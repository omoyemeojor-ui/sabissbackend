use std::collections::{BTreeSet, HashMap};

use chrono::{DateTime, Utc};
use ethers_core::{types::U256, utils::keccak256};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::{
            crud as auth_crud, error::AuthError, model::ACCOUNT_KIND_STELLAR_SMART_WALLET,
        },
        market::{
            crud as market_crud,
            model::{MarketEventRecord, MarketRecord},
            schema::{EventOnChainResponse, EventResponse, MarketResponse},
        },
        order::{
            crud,
            model::{MarketOrderRecord, NewMarketOrderRecord, UserTradeHistoryRecord},
            schema::*,
        },
    },
    service::{
        faucet::read_usdc_balance,
        jwt::AuthenticatedUser,
        liquidity::view::build_market_responses,
        trading::{
            context::{load_trading_market_context, outcome_label},
            format::{
                bps_to_price, format_amount, parse_trade_amount, quote_usdc_amount,
                validate_trade_value_bounds,
            },
        },
        stellar,
    },
};

const ORDER_STATUS_CANCELLED: &str = "cancelled";
const ORDER_STATUS_OPEN: &str = "open";
const ORDER_STATUS_PARTIALLY_FILLED: &str = "partially_filled";
const ORDER_SIDE_BUY: &str = "buy";
const ORDER_SIDE_SELL: &str = "sell";

#[derive(Clone)]
struct OrderWalletContext {
    wallet_address: String,
    account_kind: String,
    actor_address: String,
}

struct PositionSnapshot {
    event: MarketEventRecord,
    market_record: MarketRecord,
    market_response: MarketResponse,
    yes_balance: U256,
    no_balance: U256,
}

#[derive(Default)]
struct PortfolioMarketAccumulator {
    buy_amount: U256,
    sell_amount: U256,
    portfolio_balance: U256,
    positions: Vec<PositionOutcomeResponse>,
    last_traded_at: Option<DateTime<Utc>>,
}

pub async fn place_order(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    payload: CreateOrderRequest,
) -> Result<CreateOrderResponse, AuthError> {
    let wallet = load_order_wallet_context(state, authenticated_user.user_id).await?;
    let context = load_trading_market_context(state, payload.order.market_id).await?;
    let market =
        crate::service::liquidity::view::build_market_response(state, &context.market).await?;
    let side = OrderSide::parse(&payload.order.side)?;
    let amount = parse_trade_amount(&payload.order.token_amount, "order.token_amount")?;
    let price_bps = validate_price_bps(payload.order.price_bps)?;
    let quoted_usdc_amount = quote_usdc_amount(amount, price_bps);
    validate_trade_value_bounds(quoted_usdc_amount, "quoted order value")?;
    let (expiry_epoch_seconds, _expires_at) =
        normalize_expiry(payload.order.expiry_epoch_seconds)?;
    let salt = normalize_non_empty(&payload.order.salt, "order.salt")?;
    let signature = normalize_non_empty(&payload.order.signature, "order.signature")?;
    let (order_hash, order_digest) = compute_order_identity(
        &wallet.actor_address,
        &context.condition_id,
        payload.order.outcome_index,
        side,
        price_bps,
        &amount.to_string(),
        expiry_epoch_seconds,
        &salt,
    );

    let record = crud::insert_market_order(
        &state.db,
        &NewMarketOrderRecord {
            id: Uuid::new_v4(),
            user_id: authenticated_user.user_id,
            market_id: context.market.id,
            event_id: context.event.id,
            wallet_address: wallet.actor_address.clone(),
            account_kind: wallet.account_kind.clone(),
            condition_id: context.condition_id.clone(),
            outcome_index: payload.order.outcome_index,
            side: side.as_str().to_owned(),
            price_bps: i32::try_from(price_bps)
                .map_err(|error| AuthError::internal("invalid order price", error))?,
            amount: amount.to_string(),
            filled_amount: "0".to_owned(),
            remaining_amount: amount.to_string(),
            expiry_epoch_seconds,
            salt,
            signature,
            order_hash,
            order_digest,
            status: ORDER_STATUS_OPEN.to_owned(),
        },
    )
    .await
    .map_err(map_insert_market_order_error)?;

    let order = build_order_item_response(&context.event, &context.market, market, &record)?;
    Ok(CreateOrderResponse {
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
        order,
    })
}

pub async fn cancel_existing_order(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    payload: CancelOrderRequest,
) -> Result<CancelOrderResponse, AuthError> {
    let wallet = load_order_wallet_context(state, authenticated_user.user_id).await?;
    let existing = crud::get_market_order_by_id_and_user_id(
        &state.db,
        payload.order.order_id,
        authenticated_user.user_id,
    )
    .await?
    .ok_or_else(|| AuthError::not_found("order not found"))?;

    if existing.wallet_address != wallet.actor_address {
        return Err(AuthError::forbidden(
            "linked wallet does not match the order maker",
        ));
    }

    if existing.status == ORDER_STATUS_CANCELLED {
        return Err(AuthError::bad_request("order is already cancelled"));
    }

    if existing.status != ORDER_STATUS_OPEN && existing.status != ORDER_STATUS_PARTIALLY_FILLED {
        return Err(AuthError::bad_request(
            "only open orders can be cancelled through this route",
        ));
    }

    let record = crud::cancel_market_order_by_id_and_user_id(
        &state.db,
        payload.order.order_id,
        authenticated_user.user_id,
    )
    .await?;
    let (event, market_record) = load_order_market_and_event(state, record.market_id).await?;
    let market =
        crate::service::liquidity::view::build_market_response(state, &market_record).await?;
    let order = build_order_item_response(&event, &market_record, market, &record)?;

    Ok(CancelOrderResponse {
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
        cancellation_scope: "offchain_registry".to_owned(),
        cancellation_status: ORDER_STATUS_CANCELLED.to_owned(),
        prepared_transactions: None,
        order,
    })
}

pub async fn get_my_orders(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
) -> Result<MyOrdersResponse, AuthError> {
    let wallet = load_order_wallet_context(state, authenticated_user.user_id).await?;
    let orders = crud::list_market_orders_by_user_id(&state.db, authenticated_user.user_id).await?;

    if orders.is_empty() {
        return Ok(MyOrdersResponse {
            wallet_address: wallet.wallet_address,
            account_kind: wallet.account_kind,
            orders: Vec::new(),
        });
    }

    let market_ids = unique_market_ids(&orders);
    let event_ids = unique_event_ids(&orders);
    let markets = crud::list_markets_by_ids(&state.db, &market_ids).await?;
    let market_responses = build_market_responses(state, &markets).await?;
    let events = crud::list_market_events_by_ids(&state.db, &event_ids).await?;
    let market_records_by_id = markets
        .into_iter()
        .map(|market| (market.id, market))
        .collect::<HashMap<_, _>>();
    let market_responses_by_id = market_responses
        .into_iter()
        .map(|market| (market.id, market))
        .collect::<HashMap<_, _>>();
    let events_by_id = events
        .into_iter()
        .map(|event| (event.id, event))
        .collect::<HashMap<_, _>>();

    let items = orders
        .iter()
        .map(|order| {
            let event = events_by_id
                .get(&order.event_id)
                .ok_or_else(|| AuthError::internal("missing order event", order.event_id))?;
            let market_record = market_records_by_id
                .get(&order.market_id)
                .ok_or_else(|| AuthError::internal("missing order market", order.market_id))?;
            let market = market_responses_by_id
                .get(&order.market_id)
                .cloned()
                .ok_or_else(|| {
                    AuthError::internal("missing order market response", order.market_id)
                })?;
            build_order_item_response(event, market_record, market, order)
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MyOrdersResponse {
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
        orders: items,
    })
}

pub async fn get_my_positions(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
) -> Result<MyPositionsResponse, AuthError> {
    let (wallet, snapshots) = load_my_position_snapshots(state, authenticated_user).await?;
    let positions = snapshots
        .iter()
        .map(|snapshot| {
            build_position_item_response(
                &snapshot.event,
                &snapshot.market_record,
                snapshot.market_response.clone(),
                snapshot.yes_balance,
                snapshot.no_balance,
            )
        })
        .collect::<Result<Vec<_>, _>>()?;

    Ok(MyPositionsResponse {
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
        positions,
    })
}

pub async fn get_my_portfolio(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
) -> Result<MyPortfolioResponse, AuthError> {
    let wallet = load_order_wallet_context(state, authenticated_user.user_id).await?;
    let trade_history =
        crud::list_user_trade_history_by_user_id(&state.db, authenticated_user.user_id).await?;
    let cash_balance = parse_stored_amount(
        &read_usdc_balance(state, &wallet.actor_address).await?,
        "cash balance",
    )?;

    let market_ids = trade_history
        .iter()
        .map(|record| record.market_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let mut markets = crud::list_markets_by_ids(&state.db, &market_ids).await?;
    let snapshots = load_position_snapshots_for_markets(state, &wallet, &markets).await?;
    for snapshot in &snapshots {
        if !market_ids.contains(&snapshot.market_record.id) {
            markets.push(snapshot.market_record.clone());
        }
    }
    let unique_markets = markets
        .into_iter()
        .map(|market| (market.id, market))
        .collect::<HashMap<_, _>>();
    let markets = unique_markets.into_values().collect::<Vec<_>>();

    if markets.is_empty() {
        return Ok(MyPortfolioResponse {
            wallet_address: wallet.wallet_address,
            account_kind: wallet.account_kind,
            summary: PortfolioSummaryResponse {
                cash_balance: format_amount(&cash_balance),
                portfolio_balance: "0".to_owned(),
                total_balance: format_amount(&cash_balance),
                total_buy_amount: "0".to_owned(),
                total_sell_amount: "0".to_owned(),
            },
            markets: Vec::new(),
            history: Vec::new(),
        });
    }

    let market_responses = build_market_responses(state, &markets).await?;
    let event_ids = markets
        .iter()
        .map(|market| market.event_db_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let events = crud::list_market_events_by_ids(&state.db, &event_ids).await?;
    let market_records_by_id = markets
        .into_iter()
        .map(|market| (market.id, market))
        .collect::<HashMap<_, _>>();
    let market_responses_by_id = market_responses
        .into_iter()
        .map(|market| (market.id, market))
        .collect::<HashMap<_, _>>();
    let events_by_id = events
        .into_iter()
        .map(|event| (event.id, event))
        .collect::<HashMap<_, _>>();

    let mut markets_by_id = HashMap::<Uuid, PortfolioMarketAccumulator>::new();
    let mut portfolio_balance = U256::zero();

    for snapshot in &snapshots {
        let current_value = estimate_position_value(
            &snapshot.market_response,
            snapshot.yes_balance,
            snapshot.no_balance,
        )
        .unwrap_or_default();
        let positions = build_position_outcomes_response(
            &snapshot.market_record,
            &snapshot.market_response,
            snapshot.yes_balance,
            snapshot.no_balance,
        )?;
        let market_state = markets_by_id.entry(snapshot.market_record.id).or_default();
        market_state.portfolio_balance = current_value;
        market_state.positions = positions;
        portfolio_balance += current_value;
    }

    let mut total_buy_amount = U256::zero();
    let mut total_sell_amount = U256::zero();
    let history = trade_history
        .iter()
        .map(|record| {
            let history_item = build_portfolio_trade_history_item_response(
                &market_records_by_id,
                &market_responses_by_id,
                &events_by_id,
                record,
            )?;
            let usdc_amount = parse_stored_amount(&record.usdc_amount, "stored trade amount")?;
            let market_state = markets_by_id.entry(record.market_id).or_default();
            update_last_traded_at(&mut market_state.last_traded_at, record.executed_at);
            match record.action.as_str() {
                ORDER_SIDE_BUY => {
                    total_buy_amount += usdc_amount;
                    market_state.buy_amount += usdc_amount;
                }
                ORDER_SIDE_SELL => {
                    total_sell_amount += usdc_amount;
                    market_state.sell_amount += usdc_amount;
                }
                _ => {
                    return Err(AuthError::internal(
                        "invalid stored trade action",
                        record.action.clone(),
                    ));
                }
            }
            Ok(history_item)
        })
        .collect::<Result<Vec<_>, _>>()?;

    let mut market_rows = markets_by_id.into_iter().collect::<Vec<_>>();
    market_rows.sort_by(|(left_id, left), (right_id, right)| {
        right
            .portfolio_balance
            .cmp(&left.portfolio_balance)
            .then_with(|| right.last_traded_at.cmp(&left.last_traded_at))
            .then_with(|| left_id.cmp(right_id))
    });

    let market_rows = market_rows
        .into_iter()
        .map(|(market_id, aggregate)| {
            let market_record = market_records_by_id
                .get(&market_id)
                .ok_or_else(|| AuthError::internal("missing portfolio market", market_id))?;
            let event = events_by_id
                .get(&market_record.event_db_id)
                .ok_or_else(|| {
                    AuthError::internal("missing portfolio event", market_record.event_db_id)
                })?;
            let market = market_responses_by_id
                .get(&market_id)
                .cloned()
                .ok_or_else(|| {
                    AuthError::internal("missing portfolio market response", market_id)
                })?;
            Ok::<PortfolioMarketSummaryResponse, AuthError>(PortfolioMarketSummaryResponse {
                event: EventResponse::from(event),
                on_chain: EventOnChainResponse::from(event),
                market,
                buy_amount: format_amount(&aggregate.buy_amount),
                sell_amount: format_amount(&aggregate.sell_amount),
                portfolio_balance: format_amount(&aggregate.portfolio_balance),
                positions: aggregate.positions,
                last_traded_at: aggregate.last_traded_at,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

    let total_balance = cash_balance + portfolio_balance;

    Ok(MyPortfolioResponse {
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
        summary: PortfolioSummaryResponse {
            cash_balance: format_amount(&cash_balance),
            portfolio_balance: format_amount(&portfolio_balance),
            total_balance: format_amount(&total_balance),
            total_buy_amount: format_amount(&total_buy_amount),
            total_sell_amount: format_amount(&total_sell_amount),
        },
        markets: market_rows,
        history,
    })
}

async fn load_my_position_snapshots(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
) -> Result<(OrderWalletContext, Vec<PositionSnapshot>), AuthError> {
    let wallet = load_order_wallet_context(state, authenticated_user.user_id).await?;
    let markets = crud::list_markets_with_condition_ids(&state.db).await?;
    let snapshots = load_position_snapshots_for_markets(state, &wallet, &markets).await?;
    Ok((wallet, snapshots))
}

async fn load_position_snapshots_for_markets(
    state: &AppState,
    wallet: &OrderWalletContext,
    markets: &[MarketRecord],
) -> Result<Vec<PositionSnapshot>, AuthError> {
    if markets.is_empty() {
        return Ok(Vec::new());
    }

    let market_responses = build_market_responses(state, markets).await?;
    let market_responses_by_id = market_responses
        .into_iter()
        .map(|market| (market.id, market))
        .collect::<HashMap<_, _>>();
    let event_ids = markets
        .iter()
        .map(|market| market.event_db_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let events = crud::list_market_events_by_ids(&state.db, &event_ids).await?;
    let events_by_id = events
        .into_iter()
        .map(|event| (event.id, event))
        .collect::<HashMap<_, _>>();

    let mut snapshots = Vec::new();
    for market in markets {
        let Some(condition_id) = market.condition_id.as_deref() else {
            continue;
        };

        let yes_balance = parse_stored_amount(
            &stellar::get_outcome_position_balance(
                &state.env,
                condition_id,
                &wallet.actor_address,
                0,
            )
            .await
            .map_err(|error| AuthError::internal("market positions read failed", error))?,
            "yes balance",
        )?;
        let no_balance = parse_stored_amount(
            &stellar::get_outcome_position_balance(
                &state.env,
                condition_id,
                &wallet.actor_address,
                1,
            )
            .await
            .map_err(|error| AuthError::internal("market positions read failed", error))?,
            "no balance",
        )?;

        if yes_balance.is_zero() && no_balance.is_zero() {
            continue;
        }

        let event = events_by_id
            .get(&market.event_db_id)
            .cloned()
            .ok_or_else(|| AuthError::internal("missing position event", market.event_db_id))?;
        let market_response = market_responses_by_id
            .get(&market.id)
            .cloned()
            .ok_or_else(|| AuthError::internal("missing position market response", market.id))?;

        snapshots.push(PositionSnapshot {
            event,
            market_record: market.clone(),
            market_response,
            yes_balance,
            no_balance,
        });
    }

    Ok(snapshots)
}

fn validate_price_bps(price_bps: u32) -> Result<u32, AuthError> {
    if price_bps == 0 || price_bps > 10_000 {
        return Err(AuthError::bad_request(
            "order.price_bps must be between 1 and 10000",
        ));
    }
    Ok(price_bps)
}

fn normalize_expiry(expiry_epoch_seconds: Option<i64>) -> Result<(Option<i64>, Option<DateTime<Utc>>), AuthError> {
    match expiry_epoch_seconds {
        None | Some(0) => Ok((None, None)),
        Some(value) if value <= Utc::now().timestamp() => Err(AuthError::bad_request(
            "order.expiry_epoch_seconds must be in the future",
        )),
        Some(value) => {
            let datetime = DateTime::<Utc>::from_timestamp(value, 0)
                .ok_or_else(|| AuthError::internal("invalid order expiry timestamp", value))?;
            Ok((Some(value), Some(datetime)))
        }
    }
}

fn normalize_non_empty(raw: &str, field_name: &str) -> Result<String, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }
    Ok(value.to_owned())
}

fn compute_order_identity(
    maker: &str,
    condition_id: &str,
    outcome_index: i32,
    side: OrderSide,
    price_bps: u32,
    amount: &str,
    expiry_epoch_seconds: Option<i64>,
    salt: &str,
) -> (String, String) {
    let hash_input = format!(
        "{maker}|{condition_id}|{outcome_index}|{}|{price_bps}|{amount}|{}|{salt}",
        side.as_str(),
        expiry_epoch_seconds.unwrap_or_default()
    );
    let order_hash = hex::encode(keccak256(hash_input.as_bytes()));
    let digest_input = format!("sabi-stellar-order|{order_hash}");
    let order_digest = hex::encode(keccak256(digest_input.as_bytes()));
    (order_hash, order_digest)
}

fn build_order_item_response(
    event: &MarketEventRecord,
    market_record: &MarketRecord,
    market: MarketResponse,
    record: &MarketOrderRecord,
) -> Result<OrderItemResponse, AuthError> {
    let price_bps = u32::try_from(record.price_bps)
        .map_err(|error| AuthError::internal("invalid order price", error))?;
    let amount = parse_stored_amount(&record.amount, "stored order amount")?;
    let filled_amount = parse_stored_amount(&record.filled_amount, "stored filled amount")?;
    let remaining_amount =
        parse_stored_amount(&record.remaining_amount, "stored remaining amount")?;
    let quoted_usdc_amount = quote_usdc_amount(amount, price_bps);
    let expires_at = expiry_datetime(record.expiry_epoch_seconds)?;

    Ok(OrderItemResponse {
        event: EventResponse::from(event),
        on_chain: EventOnChainResponse::from(event),
        market,
        order: OrderResponse::from_record(
            record,
            outcome_label(market_record, record.outcome_index)?,
            price_bps,
            bps_to_price(price_bps),
            format_amount(&amount),
            format_amount(&filled_amount),
            format_amount(&remaining_amount),
            format_amount(&quoted_usdc_amount),
            expires_at,
        ),
    })
}

fn build_position_item_response(
    event: &MarketEventRecord,
    market_record: &MarketRecord,
    market: MarketResponse,
    yes_balance: U256,
    no_balance: U256,
) -> Result<PositionItemResponse, AuthError> {
    let outcomes =
        build_position_outcomes_response(market_record, &market, yes_balance, no_balance)?;
    let total_estimated_value = estimate_position_value(&market, yes_balance, no_balance);

    Ok(PositionItemResponse {
        event: EventResponse::from(event),
        on_chain: EventOnChainResponse::from(event),
        market,
        outcomes,
        total_token_amount: format_amount(&(yes_balance + no_balance)),
        total_estimated_value_usdc: total_estimated_value.as_ref().map(format_amount),
        updated_at: Utc::now(),
    })
}

fn build_position_outcomes_response(
    market_record: &MarketRecord,
    market: &MarketResponse,
    yes_balance: U256,
    no_balance: U256,
) -> Result<Vec<PositionOutcomeResponse>, AuthError> {
    let mut outcomes = Vec::new();

    if !yes_balance.is_zero() {
        outcomes.push(PositionOutcomeResponse {
            outcome_index: 0,
            outcome_label: outcome_label(market_record, 0)?,
            token_amount: format_amount(&yes_balance),
            estimated_value_usdc: market.current_prices.as_ref().map(|current_prices| {
                format_amount(&quote_usdc_amount(yes_balance, current_prices.yes_bps))
            }),
        });
    }

    if !no_balance.is_zero() {
        outcomes.push(PositionOutcomeResponse {
            outcome_index: 1,
            outcome_label: outcome_label(market_record, 1)?,
            token_amount: format_amount(&no_balance),
            estimated_value_usdc: market.current_prices.as_ref().map(|current_prices| {
                format_amount(&quote_usdc_amount(no_balance, current_prices.no_bps))
            }),
        });
    }

    Ok(outcomes)
}

fn estimate_position_value(
    market: &MarketResponse,
    yes_balance: U256,
    no_balance: U256,
) -> Option<U256> {
    market.current_prices.as_ref().map(|current_prices| {
        quote_usdc_amount(yes_balance, current_prices.yes_bps)
            + quote_usdc_amount(no_balance, current_prices.no_bps)
    })
}

fn build_portfolio_trade_history_item_response(
    market_records_by_id: &HashMap<Uuid, MarketRecord>,
    market_responses_by_id: &HashMap<Uuid, MarketResponse>,
    events_by_id: &HashMap<Uuid, MarketEventRecord>,
    record: &UserTradeHistoryRecord,
) -> Result<PortfolioTradeHistoryItemResponse, AuthError> {
    let market_record = market_records_by_id
        .get(&record.market_id)
        .ok_or_else(|| AuthError::internal("missing portfolio trade market", record.market_id))?;
    let market = market_responses_by_id
        .get(&record.market_id)
        .cloned()
        .ok_or_else(|| {
            AuthError::internal("missing portfolio trade market response", record.market_id)
        })?;
    let event = events_by_id
        .get(&record.event_id)
        .ok_or_else(|| AuthError::internal("missing portfolio trade event", record.event_id))?;
    let price_bps = u32::try_from(record.price_bps)
        .map_err(|error| AuthError::internal("invalid stored trade price", error))?;
    let usdc_amount = parse_stored_amount(&record.usdc_amount, "stored usdc amount")?;
    let token_amount = parse_stored_amount(&record.token_amount, "stored token amount")?;

    Ok(PortfolioTradeHistoryItemResponse {
        id: record.history_key.clone(),
        execution_source: record.execution_source.clone(),
        event: EventResponse::from(event),
        on_chain: EventOnChainResponse::from(event),
        market,
        action: record.action.clone(),
        outcome_index: record.outcome_index,
        outcome_label: outcome_label(market_record, record.outcome_index)?,
        usdc_amount: format_amount(&usdc_amount),
        token_amount: format_amount(&token_amount),
        price_bps,
        price: bps_to_price(price_bps),
        tx_hash: record.tx_hash.clone(),
        executed_at: record.executed_at,
    })
}

fn update_last_traded_at(current: &mut Option<DateTime<Utc>>, next: DateTime<Utc>) {
    if current.is_none_or(|value| next > value) {
        *current = Some(next);
    }
}

fn parse_stored_amount(raw: &str, _field_name: &str) -> Result<U256, AuthError> {
    U256::from_dec_str(raw).map_err(|error| AuthError::internal("invalid stored amount", error))
}

fn expiry_datetime(expiry_epoch_seconds: Option<i64>) -> Result<Option<DateTime<Utc>>, AuthError> {
    expiry_epoch_seconds
        .map(|seconds| {
            DateTime::<Utc>::from_timestamp(seconds, 0)
                .ok_or_else(|| AuthError::internal("invalid stored order expiry", seconds))
        })
        .transpose()
}

async fn load_order_market_and_event(
    state: &AppState,
    market_id: Uuid,
) -> Result<(MarketEventRecord, MarketRecord), AuthError> {
    let market = market_crud::get_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;
    let event = market_crud::get_market_event_by_id(&state.db, market.event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;
    Ok((event, market))
}

async fn load_order_wallet_context(
    state: &AppState,
    user_id: Uuid,
) -> Result<OrderWalletContext, AuthError> {
    let wallet = auth_crud::get_wallet_for_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("wallet not linked to user"))?;
    let deployed_wallet_address = wallet
        .wallet_address
        .ok_or_else(|| AuthError::forbidden("wallet is not deployed"))?;
    let actor_address = wallet
        .owner_address
        .clone()
        .unwrap_or_else(|| deployed_wallet_address.clone());
    let account_kind = if wallet.account_kind == ACCOUNT_KIND_STELLAR_SMART_WALLET {
        "smart_account".to_owned()
    } else {
        wallet.account_kind
    };

    Ok(OrderWalletContext {
        wallet_address: actor_address.clone(),
        account_kind,
        actor_address,
    })
}

fn unique_market_ids(orders: &[MarketOrderRecord]) -> Vec<Uuid> {
    orders
        .iter()
        .map(|order| order.market_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn unique_event_ids(orders: &[MarketOrderRecord]) -> Vec<Uuid> {
    orders
        .iter()
        .map(|order| order.event_id)
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn map_insert_market_order_error(error: sqlx::Error) -> AuthError {
    match unique_constraint(&error) {
        Some("market_orders_order_hash_key") | Some("market_orders_order_digest_key") => {
            AuthError::conflict("order already submitted")
        }
        _ => AuthError::from(error),
    }
}

fn unique_constraint(error: &sqlx::Error) -> Option<&str> {
    match error {
        sqlx::Error::Database(database_error)
            if database_error.code().as_deref() == Some("23505") =>
        {
            database_error.constraint()
        }
        _ => None,
    }
}

#[derive(Clone, Copy)]
enum OrderSide {
    Buy,
    Sell,
}

impl OrderSide {
    fn parse(value: &str) -> Result<Self, AuthError> {
        match value.trim().to_ascii_lowercase().as_str() {
            ORDER_SIDE_BUY => Ok(Self::Buy),
            ORDER_SIDE_SELL => Ok(Self::Sell),
            _ => Err(AuthError::bad_request("order.side must be `buy` or `sell`")),
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Buy => ORDER_SIDE_BUY,
            Self::Sell => ORDER_SIDE_SELL,
        }
    }
}
