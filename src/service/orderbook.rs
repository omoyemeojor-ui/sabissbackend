use std::{str::FromStr, time::Duration};

use anyhow::{Context, Result};
use chrono::Utc;
use ethers_contract::Contract;
use ethers_core::{
    abi::{Abi, AbiParser},
    types::{Address, Bytes, H256, U256},
};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::{LocalWallet, Signer};
use tokio::time::{MissedTickBehavior, interval};
use uuid::Uuid;

use crate::{
    app::AppState,
    config::environment::Environment,
    module::{
        auth::error::AuthError,
        market::crud as market_crud,
        order::{
            crud,
            model::{MarketOrderFillRecord, MarketOrderRecord, NewMarketOrderFillRecord},
            schema::*,
        },
    },
    service::{
        rpc,
        trading::format::{format_amount, parse_trade_amount, quote_usdc_amount},
    },
};

type WriteProvider = SignerMiddleware<Provider<Http>, LocalWallet>;

const ORDER_STATUS_FILLED: &str = "filled";
const ORDER_STATUS_PARTIALLY_FILLED: &str = "partially_filled";
const DEFAULT_MAX_MATCH_FILLS_PER_MARKET: u32 = 16;

pub fn spawn_orderbook_matcher(state: AppState) {
    let interval_secs = state.env.orderbook_match_interval_secs;
    if interval_secs == 0 {
        tracing::info!("orderbook matcher disabled");
        return;
    }
    if state.env.monad_operator_private_key.is_none() {
        tracing::warn!("orderbook matcher disabled because MONAD_OPERATOR_PRIVATE_KEY is missing");
        return;
    }

    tokio::spawn(async move {
        tracing::info!(
            interval_secs,
            max_fills_per_market = state.env.orderbook_match_max_fills_per_market,
            "orderbook matcher started"
        );

        if let Err(error) =
            run_match_orders_once(&state, None, default_max_fills_per_market(&state.env)).await
        {
            tracing::warn!(%error, "initial orderbook match run failed");
        }

        let mut ticker = interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        ticker.tick().await;

        loop {
            ticker.tick().await;

            if let Err(error) =
                run_match_orders_once(&state, None, default_max_fills_per_market(&state.env)).await
            {
                tracing::warn!(%error, "scheduled orderbook match run failed");
            }
        }
    });
}

pub async fn fill_direct_orders(
    state: &AppState,
    payload: AdminFillDirectOrdersRequest,
) -> Result<AdminOrderFillResponse, AuthError> {
    let buy_order = load_order_by_id(state, payload.fill.buy_order_id).await?;
    let sell_order = load_order_by_id(state, payload.fill.sell_order_id).await?;
    let fill_amount =
        parse_trade_amount(&payload.fill.fill_token_amount, "fill.fill_token_amount")?;
    let settlement_price = validate_price_bps(
        payload.fill.settlement_price_bps,
        "fill.settlement_price_bps",
    )?;

    validate_direct_pair(&buy_order, &sell_order, fill_amount, settlement_price)?;
    let tx_hash = orderbook_write_client(&state.env)
        .await?
        .fill_direct_order_pair(&buy_order, &sell_order, fill_amount, settlement_price)
        .await
        .map_err(|error| AuthError::bad_request(format!("direct order fill failed: {error}")))?;

    let yes_price_bps = direct_yes_price_bps(&buy_order, settlement_price)?;
    let no_price_bps = 10_000_u32.saturating_sub(yes_price_bps);
    let collateral_amount = quote_usdc_amount(fill_amount, settlement_price);
    let updated_buy = apply_fill_to_order(state, &buy_order, fill_amount).await?;
    let updated_sell = apply_fill_to_order(state, &sell_order, fill_amount).await?;
    let fill = persist_fill(
        state,
        NewMarketOrderFillRecord {
            id: Uuid::new_v4(),
            market_id: buy_order.market_id,
            event_id: buy_order.event_id,
            condition_id: buy_order.condition_id.clone(),
            match_type: "direct".to_owned(),
            buy_order_id: Some(buy_order.id),
            sell_order_id: Some(sell_order.id),
            yes_order_id: None,
            no_order_id: None,
            outcome_index: Some(buy_order.outcome_index),
            fill_amount: fill_amount.to_string(),
            collateral_amount: collateral_amount.to_string(),
            yes_price_bps: i32::try_from(yes_price_bps)
                .map_err(|error| AuthError::internal("invalid YES price", error))?,
            no_price_bps: i32::try_from(no_price_bps)
                .map_err(|error| AuthError::internal("invalid NO price", error))?,
            tx_hash: tx_hash.clone(),
        },
    )
    .await?;

    Ok(build_admin_fill_response(
        &fill,
        tx_hash,
        vec![updated_buy, updated_sell],
    )?)
}

pub async fn fill_complementary_buy_orders(
    state: &AppState,
    payload: AdminFillComplementaryBuyOrdersRequest,
) -> Result<AdminOrderFillResponse, AuthError> {
    let yes_order = load_order_by_id(state, payload.fill.yes_buy_order_id).await?;
    let no_order = load_order_by_id(state, payload.fill.no_buy_order_id).await?;
    let fill_amount =
        parse_trade_amount(&payload.fill.fill_token_amount, "fill.fill_token_amount")?;
    let yes_price_bps = validate_price_bps(
        payload.fill.yes_settlement_price_bps,
        "fill.yes_settlement_price_bps",
    )?;
    let no_price_bps = 10_000_u32
        .checked_sub(yes_price_bps)
        .ok_or_else(|| AuthError::bad_request("fill.yes_settlement_price_bps must be <= 10000"))?;

    validate_complementary_pair(
        &yes_order,
        &no_order,
        "buy",
        fill_amount,
        yes_price_bps,
        no_price_bps,
    )?;
    let tx_hash = orderbook_write_client(&state.env)
        .await?
        .fill_complementary_buy_orders(
            &yes_order,
            &no_order,
            fill_amount,
            yes_price_bps,
            no_price_bps,
        )
        .await
        .map_err(|error| {
            AuthError::bad_request(format!("complementary buy fill failed: {error}"))
        })?;

    let updated_yes = apply_fill_to_order(state, &yes_order, fill_amount).await?;
    let updated_no = apply_fill_to_order(state, &no_order, fill_amount).await?;
    let fill = persist_fill(
        state,
        NewMarketOrderFillRecord {
            id: Uuid::new_v4(),
            market_id: yes_order.market_id,
            event_id: yes_order.event_id,
            condition_id: yes_order.condition_id.clone(),
            match_type: "complementary_buy".to_owned(),
            buy_order_id: None,
            sell_order_id: None,
            yes_order_id: Some(yes_order.id),
            no_order_id: Some(no_order.id),
            outcome_index: None,
            fill_amount: fill_amount.to_string(),
            collateral_amount: fill_amount.to_string(),
            yes_price_bps: i32::try_from(yes_price_bps)
                .map_err(|error| AuthError::internal("invalid YES price", error))?,
            no_price_bps: i32::try_from(no_price_bps)
                .map_err(|error| AuthError::internal("invalid NO price", error))?,
            tx_hash: tx_hash.clone(),
        },
    )
    .await?;

    Ok(build_admin_fill_response(
        &fill,
        tx_hash,
        vec![updated_yes, updated_no],
    )?)
}

pub async fn fill_complementary_sell_orders(
    state: &AppState,
    payload: AdminFillComplementarySellOrdersRequest,
) -> Result<AdminOrderFillResponse, AuthError> {
    let yes_order = load_order_by_id(state, payload.fill.yes_sell_order_id).await?;
    let no_order = load_order_by_id(state, payload.fill.no_sell_order_id).await?;
    let fill_amount =
        parse_trade_amount(&payload.fill.fill_token_amount, "fill.fill_token_amount")?;
    let yes_price_bps = validate_price_bps(
        payload.fill.yes_settlement_price_bps,
        "fill.yes_settlement_price_bps",
    )?;
    let no_price_bps = 10_000_u32
        .checked_sub(yes_price_bps)
        .ok_or_else(|| AuthError::bad_request("fill.yes_settlement_price_bps must be <= 10000"))?;

    validate_complementary_pair(
        &yes_order,
        &no_order,
        "sell",
        fill_amount,
        yes_price_bps,
        no_price_bps,
    )?;
    let tx_hash = orderbook_write_client(&state.env)
        .await?
        .fill_complementary_sell_orders(
            &yes_order,
            &no_order,
            fill_amount,
            yes_price_bps,
            no_price_bps,
        )
        .await
        .map_err(|error| {
            AuthError::bad_request(format!("complementary sell fill failed: {error}"))
        })?;

    let updated_yes = apply_fill_to_order(state, &yes_order, fill_amount).await?;
    let updated_no = apply_fill_to_order(state, &no_order, fill_amount).await?;
    let fill = persist_fill(
        state,
        NewMarketOrderFillRecord {
            id: Uuid::new_v4(),
            market_id: yes_order.market_id,
            event_id: yes_order.event_id,
            condition_id: yes_order.condition_id.clone(),
            match_type: "complementary_sell".to_owned(),
            buy_order_id: None,
            sell_order_id: None,
            yes_order_id: Some(yes_order.id),
            no_order_id: Some(no_order.id),
            outcome_index: None,
            fill_amount: fill_amount.to_string(),
            collateral_amount: fill_amount.to_string(),
            yes_price_bps: i32::try_from(yes_price_bps)
                .map_err(|error| AuthError::internal("invalid YES price", error))?,
            no_price_bps: i32::try_from(no_price_bps)
                .map_err(|error| AuthError::internal("invalid NO price", error))?,
            tx_hash: tx_hash.clone(),
        },
    )
    .await?;

    Ok(build_admin_fill_response(
        &fill,
        tx_hash,
        vec![updated_yes, updated_no],
    )?)
}

pub async fn match_orders(
    state: &AppState,
    payload: AdminMatchOrdersRequest,
) -> Result<AdminMatchOrdersResponse, AuthError> {
    let max_fills_per_market = normalize_max_fills_per_market(
        payload.matching.max_fills_per_market,
        default_max_fills_per_market(&state.env),
    )?;
    run_match_orders_once(state, payload.matching.market_id, max_fills_per_market).await
}

async fn run_match_orders_once(
    state: &AppState,
    requested_market_id: Option<Uuid>,
    max_fills_per_market: u32,
) -> Result<AdminMatchOrdersResponse, AuthError> {
    let market_ids = match requested_market_id {
        Some(market_id) => vec![market_id],
        None => crud::list_market_ids_with_active_orders(&state.db).await?,
    };

    let mut markets = Vec::new();
    let mut executed_fills = 0_usize;

    for matched_market_id in market_ids.iter().copied() {
        let fills =
            match_market_orders_for_market(state, matched_market_id, max_fills_per_market).await?;
        executed_fills += fills.len();

        if requested_market_id.is_some() || !fills.is_empty() {
            markets.push(AdminMatchedMarketResponse {
                market_id: matched_market_id,
                executed_fills: fills.len(),
                fills,
            });
        }
    }

    let markets_matched = markets
        .iter()
        .filter(|market| market.executed_fills > 0)
        .count();
    let scope = if requested_market_id.is_some() {
        "single_market"
    } else {
        "all_open_markets"
    };

    Ok(AdminMatchOrdersResponse {
        scope: scope.to_owned(),
        market_id: requested_market_id,
        markets_scanned: market_ids.len(),
        markets_matched,
        executed_fills,
        max_fills_per_market,
        markets,
        completed_at: Utc::now(),
    })
}

async fn match_market_orders_for_market(
    state: &AppState,
    market_id: Uuid,
    max_fills_per_market: u32,
) -> Result<Vec<AdminOrderFillResponse>, AuthError> {
    let mut fills = Vec::new();

    for _ in 0..max_fills_per_market {
        let orders = crud::list_active_market_orders_by_market_id(&state.db, market_id).await?;
        let Some(candidate) = select_next_match_candidate(&orders)? else {
            break;
        };

        let fill = execute_match_candidate(state, candidate).await?;
        fills.push(fill);
    }

    Ok(fills)
}

async fn execute_match_candidate(
    state: &AppState,
    candidate: MatchCandidate,
) -> Result<AdminOrderFillResponse, AuthError> {
    match candidate {
        MatchCandidate::Direct(payload) => fill_direct_orders(state, payload).await,
        MatchCandidate::ComplementaryBuy(payload) => {
            fill_complementary_buy_orders(state, payload).await
        }
        MatchCandidate::ComplementarySell(payload) => {
            fill_complementary_sell_orders(state, payload).await
        }
    }
}

fn select_next_match_candidate(
    orders: &[MarketOrderRecord],
) -> Result<Option<MatchCandidate>, AuthError> {
    if let Some(candidate) = find_direct_match_candidate(orders, 0)? {
        return Ok(Some(candidate));
    }
    if let Some(candidate) = find_direct_match_candidate(orders, 1)? {
        return Ok(Some(candidate));
    }
    if let Some(candidate) = find_complementary_buy_match_candidate(orders)? {
        return Ok(Some(candidate));
    }
    if let Some(candidate) = find_complementary_sell_match_candidate(orders)? {
        return Ok(Some(candidate));
    }

    Ok(None)
}

fn find_direct_match_candidate(
    orders: &[MarketOrderRecord],
    outcome_index: i32,
) -> Result<Option<MatchCandidate>, AuthError> {
    let buys = orders
        .iter()
        .filter(|order| order.side == "buy" && order.outcome_index == outcome_index)
        .collect::<Vec<_>>();
    let sells = orders
        .iter()
        .filter(|order| order.side == "sell" && order.outcome_index == outcome_index)
        .collect::<Vec<_>>();

    for buy in buys {
        let buy_price = order_price_bps(buy)?;
        for sell in &sells {
            if buy.wallet_address == sell.wallet_address {
                continue;
            }

            let sell_price = order_price_bps(sell)?;
            if buy_price < sell_price {
                break;
            }

            let fill_amount = match_fill_amount(buy, sell)?;
            if fill_amount.is_zero() {
                continue;
            }

            let settlement_price_bps = older_order_price_bps(buy, sell)?;
            return Ok(Some(MatchCandidate::Direct(AdminFillDirectOrdersRequest {
                fill: AdminFillDirectOrdersFieldsRequest {
                    buy_order_id: buy.id,
                    sell_order_id: sell.id,
                    fill_token_amount: fill_amount.to_string(),
                    settlement_price_bps,
                },
            })));
        }
    }

    Ok(None)
}

fn find_complementary_buy_match_candidate(
    orders: &[MarketOrderRecord],
) -> Result<Option<MatchCandidate>, AuthError> {
    let yes_buys = orders
        .iter()
        .filter(|order| order.side == "buy" && order.outcome_index == 0)
        .collect::<Vec<_>>();
    let no_buys = orders
        .iter()
        .filter(|order| order.side == "buy" && order.outcome_index == 1)
        .collect::<Vec<_>>();

    for yes_order in yes_buys {
        let yes_price = order_price_bps(yes_order)?;
        for no_order in &no_buys {
            if yes_order.wallet_address == no_order.wallet_address {
                continue;
            }

            let no_price = order_price_bps(no_order)?;
            if yes_price.saturating_add(no_price) < 10_000 {
                break;
            }

            let fill_amount = match_fill_amount(yes_order, no_order)?;
            if fill_amount.is_zero() {
                continue;
            }

            let yes_settlement_price_bps =
                complementary_buy_yes_settlement_price_bps(yes_order, no_order)?;
            return Ok(Some(MatchCandidate::ComplementaryBuy(
                AdminFillComplementaryBuyOrdersRequest {
                    fill: AdminFillComplementaryBuyOrdersFieldsRequest {
                        yes_buy_order_id: yes_order.id,
                        no_buy_order_id: no_order.id,
                        fill_token_amount: fill_amount.to_string(),
                        yes_settlement_price_bps,
                    },
                },
            )));
        }
    }

    Ok(None)
}

fn find_complementary_sell_match_candidate(
    orders: &[MarketOrderRecord],
) -> Result<Option<MatchCandidate>, AuthError> {
    let yes_sells = orders
        .iter()
        .filter(|order| order.side == "sell" && order.outcome_index == 0)
        .collect::<Vec<_>>();
    let no_sells = orders
        .iter()
        .filter(|order| order.side == "sell" && order.outcome_index == 1)
        .collect::<Vec<_>>();

    for yes_order in yes_sells {
        let yes_price = order_price_bps(yes_order)?;
        for no_order in &no_sells {
            if yes_order.wallet_address == no_order.wallet_address {
                continue;
            }

            let no_price = order_price_bps(no_order)?;
            if yes_price.saturating_add(no_price) > 10_000 {
                break;
            }

            let fill_amount = match_fill_amount(yes_order, no_order)?;
            if fill_amount.is_zero() {
                continue;
            }

            let yes_settlement_price_bps =
                complementary_sell_yes_settlement_price_bps(yes_order, no_order)?;
            return Ok(Some(MatchCandidate::ComplementarySell(
                AdminFillComplementarySellOrdersRequest {
                    fill: AdminFillComplementarySellOrdersFieldsRequest {
                        yes_sell_order_id: yes_order.id,
                        no_sell_order_id: no_order.id,
                        fill_token_amount: fill_amount.to_string(),
                        yes_settlement_price_bps,
                    },
                },
            )));
        }
    }

    Ok(None)
}

fn order_price_bps(order: &MarketOrderRecord) -> Result<u32, AuthError> {
    u32::try_from(order.price_bps)
        .map_err(|error| AuthError::internal("invalid order price", error))
}

fn older_order_price_bps(
    left: &MarketOrderRecord,
    right: &MarketOrderRecord,
) -> Result<u32, AuthError> {
    if left.created_at <= right.created_at {
        order_price_bps(left)
    } else {
        order_price_bps(right)
    }
}

fn complementary_buy_yes_settlement_price_bps(
    yes_order: &MarketOrderRecord,
    no_order: &MarketOrderRecord,
) -> Result<u32, AuthError> {
    let yes_price = order_price_bps(yes_order)?;
    let no_price = order_price_bps(no_order)?;
    if yes_order.created_at <= no_order.created_at {
        Ok(yes_price)
    } else {
        Ok(10_000_u32.saturating_sub(no_price))
    }
}

fn complementary_sell_yes_settlement_price_bps(
    yes_order: &MarketOrderRecord,
    no_order: &MarketOrderRecord,
) -> Result<u32, AuthError> {
    let yes_price = order_price_bps(yes_order)?;
    let no_price = order_price_bps(no_order)?;
    if yes_order.created_at <= no_order.created_at {
        Ok(yes_price)
    } else {
        Ok(10_000_u32.saturating_sub(no_price))
    }
}

fn match_fill_amount(
    left: &MarketOrderRecord,
    right: &MarketOrderRecord,
) -> Result<U256, AuthError> {
    let left_remaining = parse_amount(&left.remaining_amount, "stored remaining amount")?;
    let right_remaining = parse_amount(&right.remaining_amount, "stored remaining amount")?;
    Ok(left_remaining.min(right_remaining))
}

fn default_max_fills_per_market(env: &Environment) -> u32 {
    if env.orderbook_match_max_fills_per_market == 0 {
        DEFAULT_MAX_MATCH_FILLS_PER_MARKET
    } else {
        env.orderbook_match_max_fills_per_market
    }
}

fn normalize_max_fills_per_market(
    requested: Option<u32>,
    default_value: u32,
) -> Result<u32, AuthError> {
    let value = requested.unwrap_or(default_value);
    if value == 0 {
        return Err(AuthError::bad_request(
            "matching.max_fills_per_market must be greater than zero",
        ));
    }

    Ok(value)
}

enum MatchCandidate {
    Direct(AdminFillDirectOrdersRequest),
    ComplementaryBuy(AdminFillComplementaryBuyOrdersRequest),
    ComplementarySell(AdminFillComplementarySellOrdersRequest),
}

fn build_admin_fill_response(
    fill: &MarketOrderFillRecord,
    tx_hash: String,
    orders: Vec<MarketOrderRecord>,
) -> Result<AdminOrderFillResponse, AuthError> {
    Ok(AdminOrderFillResponse {
        market_id: fill.market_id,
        condition_id: fill.condition_id.clone(),
        match_type: fill.match_type.clone(),
        tx_hash,
        fill_token_amount: format_amount(&parse_amount(&fill.fill_amount, "fill amount")?),
        collateral_amount: format_amount(&parse_amount(
            &fill.collateral_amount,
            "collateral amount",
        )?),
        yes_price_bps: u32::try_from(fill.yes_price_bps)
            .map_err(|error| AuthError::internal("invalid YES fill price", error))?,
        no_price_bps: u32::try_from(fill.no_price_bps)
            .map_err(|error| AuthError::internal("invalid NO fill price", error))?,
        orders: orders
            .iter()
            .map(|order| -> Result<OrderFillOrderStateResponse, AuthError> {
                Ok(OrderFillOrderStateResponse {
                    order_id: order.id,
                    side: order.side.clone(),
                    outcome_index: order.outcome_index,
                    status: order.status.clone(),
                    filled_token_amount: format_amount(&parse_amount(
                        &order.filled_amount,
                        "filled amount",
                    )?),
                    remaining_token_amount: format_amount(&parse_amount(
                        &order.remaining_amount,
                        "remaining amount",
                    )?),
                })
            })
            .collect::<Result<Vec<_>, _>>()?,
        executed_at: fill.created_at,
    })
}

async fn persist_fill(
    state: &AppState,
    fill: NewMarketOrderFillRecord,
) -> Result<MarketOrderFillRecord, AuthError> {
    let record = crud::insert_market_order_fill(&state.db, &fill).await?;
    let volume_usd_cents = volume_usd_cents_from_amount(&fill.collateral_amount)?;
    market_crud::upsert_market_trade_execution(
        &state.db,
        fill.market_id,
        volume_usd_cents,
        fill.yes_price_bps,
        Utc::now(),
    )
    .await?;
    Ok(record)
}

async fn apply_fill_to_order(
    state: &AppState,
    order: &MarketOrderRecord,
    fill_amount: U256,
) -> Result<MarketOrderRecord, AuthError> {
    let filled_amount = parse_amount(&order.filled_amount, "stored filled amount")?;
    let remaining_amount = parse_amount(&order.remaining_amount, "stored remaining amount")?;
    if fill_amount > remaining_amount {
        return Err(AuthError::bad_request(
            "fill amount exceeds order remaining amount",
        ));
    }

    let next_filled = filled_amount + fill_amount;
    let next_remaining = remaining_amount - fill_amount;
    let next_status = if next_remaining.is_zero() {
        ORDER_STATUS_FILLED
    } else {
        ORDER_STATUS_PARTIALLY_FILLED
    };

    crud::update_market_order_fill_state(
        &state.db,
        order.id,
        &next_filled.to_string(),
        &next_remaining.to_string(),
        next_status,
    )
    .await
}

async fn load_order_by_id(
    state: &AppState,
    order_id: Uuid,
) -> Result<MarketOrderRecord, AuthError> {
    let order = crud::get_market_order_by_id(&state.db, order_id)
        .await?
        .ok_or_else(|| AuthError::not_found("order not found"))?;
    ensure_order_open(&order)?;
    Ok(order)
}

fn ensure_order_open(order: &MarketOrderRecord) -> Result<(), AuthError> {
    if order.status != "open" && order.status != "partially_filled" {
        return Err(AuthError::bad_request("order is not fillable"));
    }
    if let Some(expiry) = order.expiry_epoch_seconds {
        if expiry > 0 && expiry < Utc::now().timestamp() {
            return Err(AuthError::bad_request("order is expired"));
        }
    }
    Ok(())
}

fn validate_direct_pair(
    buy_order: &MarketOrderRecord,
    sell_order: &MarketOrderRecord,
    fill_amount: U256,
    settlement_price: u32,
) -> Result<(), AuthError> {
    if buy_order.market_id != sell_order.market_id
        || buy_order.condition_id != sell_order.condition_id
    {
        return Err(AuthError::bad_request("orders must target the same market"));
    }
    if buy_order.side != "buy" || sell_order.side != "sell" {
        return Err(AuthError::bad_request(
            "direct fill requires one buy order and one sell order",
        ));
    }
    if buy_order.outcome_index != sell_order.outcome_index {
        return Err(AuthError::bad_request(
            "direct fill orders must target the same outcome",
        ));
    }
    if buy_order.wallet_address == sell_order.wallet_address {
        return Err(AuthError::bad_request("orders must have different makers"));
    }
    if settlement_price > u32::try_from(buy_order.price_bps).unwrap_or_default() {
        return Err(AuthError::bad_request(
            "settlement price is above the buy order price",
        ));
    }
    if settlement_price < u32::try_from(sell_order.price_bps).unwrap_or_default() {
        return Err(AuthError::bad_request(
            "settlement price is below the sell order price",
        ));
    }
    ensure_fill_amount_within_remaining(buy_order, fill_amount)?;
    ensure_fill_amount_within_remaining(sell_order, fill_amount)?;
    Ok(())
}

fn validate_complementary_pair(
    yes_order: &MarketOrderRecord,
    no_order: &MarketOrderRecord,
    expected_side: &str,
    fill_amount: U256,
    yes_price_bps: u32,
    no_price_bps: u32,
) -> Result<(), AuthError> {
    if yes_order.market_id != no_order.market_id || yes_order.condition_id != no_order.condition_id
    {
        return Err(AuthError::bad_request("orders must target the same market"));
    }
    if yes_order.side != expected_side || no_order.side != expected_side {
        return Err(AuthError::bad_request(
            "orders do not match the expected complementary side",
        ));
    }
    if yes_order.outcome_index != 0 || no_order.outcome_index != 1 {
        return Err(AuthError::bad_request(
            "complementary fills require YES outcome 0 and NO outcome 1 orders",
        ));
    }
    if yes_order.wallet_address == no_order.wallet_address {
        return Err(AuthError::bad_request("orders must have different makers"));
    }
    if yes_price_bps + no_price_bps != 10_000 {
        return Err(AuthError::bad_request(
            "settlement prices must sum to 10000",
        ));
    }
    let yes_limit = u32::try_from(yes_order.price_bps)
        .map_err(|error| AuthError::internal("invalid YES order price", error))?;
    let no_limit = u32::try_from(no_order.price_bps)
        .map_err(|error| AuthError::internal("invalid NO order price", error))?;
    match expected_side {
        "buy" => {
            if yes_price_bps > yes_limit || no_price_bps > no_limit {
                return Err(AuthError::bad_request(
                    "settlement price is above one of the order bids",
                ));
            }
        }
        "sell" => {
            if yes_price_bps < yes_limit || no_price_bps < no_limit {
                return Err(AuthError::bad_request(
                    "settlement price is below one of the order asks",
                ));
            }
        }
        _ => {
            return Err(AuthError::internal(
                "invalid complementary side",
                expected_side,
            ));
        }
    }
    ensure_fill_amount_within_remaining(yes_order, fill_amount)?;
    ensure_fill_amount_within_remaining(no_order, fill_amount)?;
    Ok(())
}

fn ensure_fill_amount_within_remaining(
    order: &MarketOrderRecord,
    fill_amount: U256,
) -> Result<(), AuthError> {
    let remaining = parse_amount(&order.remaining_amount, "stored remaining amount")?;
    if fill_amount > remaining {
        return Err(AuthError::bad_request(
            "fill amount exceeds order remaining amount",
        ));
    }
    Ok(())
}

fn direct_yes_price_bps(
    order: &MarketOrderRecord,
    settlement_price: u32,
) -> Result<u32, AuthError> {
    match order.outcome_index {
        0 => Ok(settlement_price),
        1 => Ok(10_000_u32.saturating_sub(settlement_price)),
        _ => Err(AuthError::bad_request("order outcome_index must be 0 or 1")),
    }
}

fn validate_price_bps(price_bps: u32, field_name: &str) -> Result<u32, AuthError> {
    if price_bps > 10_000 {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be between 0 and 10000"
        )));
    }
    Ok(price_bps)
}

fn parse_amount(raw: &str, field_name: &str) -> Result<U256, AuthError> {
    U256::from_dec_str(raw).map_err(|_| {
        AuthError::bad_request(format!("{field_name} must be a base-10 integer string"))
    })
}

fn volume_usd_cents_from_amount(raw_amount: &str) -> Result<i64, AuthError> {
    let amount = parse_amount(raw_amount, "volume amount")?;
    let cents = amount
        .checked_mul(U256::from(100_u64))
        .ok_or_else(|| AuthError::bad_request("volume amount is too large"))?
        / U256::from(1_000_000_u64);
    i64::try_from(cents).map_err(|error| AuthError::internal("volume amount overflowed i64", error))
}

struct OrderbookWriteClient {
    exchange: Contract<WriteProvider>,
}

impl OrderbookWriteClient {
    async fn fill_direct_order_pair(
        &self,
        buy_order: &MarketOrderRecord,
        sell_order: &MarketOrderRecord,
        fill_amount: U256,
        settlement_price_bps: u32,
    ) -> Result<String> {
        let call = self
            .exchange
            .method::<_, ()>(
                "fillDirectOrderPair",
                (
                    encode_contract_order(buy_order)?,
                    decode_signature_bytes(&buy_order.signature)?,
                    encode_contract_order(sell_order)?,
                    decode_signature_bytes(&sell_order.signature)?,
                    fill_amount,
                    U256::from(settlement_price_bps),
                ),
            )
            .context("failed to build Exchange.fillDirectOrderPair call")?;
        let pending = call
            .send()
            .await
            .context("failed to submit Exchange.fillDirectOrderPair transaction")?;
        Ok(format!("{:#x}", pending.tx_hash()))
    }

    async fn fill_complementary_buy_orders(
        &self,
        yes_order: &MarketOrderRecord,
        no_order: &MarketOrderRecord,
        fill_amount: U256,
        yes_price_bps: u32,
        no_price_bps: u32,
    ) -> Result<String> {
        let call = self
            .exchange
            .method::<_, ()>(
                "fillComplementaryBuyOrders",
                (
                    encode_contract_order(yes_order)?,
                    decode_signature_bytes(&yes_order.signature)?,
                    encode_contract_order(no_order)?,
                    decode_signature_bytes(&no_order.signature)?,
                    fill_amount,
                    U256::from(yes_price_bps),
                    U256::from(no_price_bps),
                ),
            )
            .context("failed to build Exchange.fillComplementaryBuyOrders call")?;
        let pending = call
            .send()
            .await
            .context("failed to submit Exchange.fillComplementaryBuyOrders transaction")?;
        Ok(format!("{:#x}", pending.tx_hash()))
    }

    async fn fill_complementary_sell_orders(
        &self,
        yes_order: &MarketOrderRecord,
        no_order: &MarketOrderRecord,
        fill_amount: U256,
        yes_price_bps: u32,
        no_price_bps: u32,
    ) -> Result<String> {
        let call = self
            .exchange
            .method::<_, ()>(
                "fillComplementarySellOrders",
                (
                    encode_contract_order(yes_order)?,
                    decode_signature_bytes(&yes_order.signature)?,
                    encode_contract_order(no_order)?,
                    decode_signature_bytes(&no_order.signature)?,
                    fill_amount,
                    U256::from(yes_price_bps),
                    U256::from(no_price_bps),
                ),
            )
            .context("failed to build Exchange.fillComplementarySellOrders call")?;
        let pending = call
            .send()
            .await
            .context("failed to submit Exchange.fillComplementarySellOrders transaction")?;
        Ok(format!("{:#x}", pending.tx_hash()))
    }
}

async fn orderbook_write_client(env: &Environment) -> Result<OrderbookWriteClient, AuthError> {
    let private_key = env.monad_operator_private_key.as_ref().ok_or_else(|| {
        AuthError::internal(
            "missing MONAD_OPERATOR_PRIVATE_KEY",
            "operator key not configured",
        )
    })?;
    let wallet = private_key
        .parse::<LocalWallet>()
        .context("invalid MONAD_OPERATOR_PRIVATE_KEY")
        .map_err(|error| AuthError::internal("orderbook operator setup failed", error))?
        .with_chain_id(env.monad_chain_id as u64);
    let client = rpc::monad_signer_middleware(env, wallet)
        .await
        .map_err(|error| AuthError::internal("orderbook operator setup failed", error))?;
    let exchange = Contract::new(
        parse_address(
            &env.monad_orderbook_exchange_address,
            "MONAD_ORDERBOOK_EXCHANGE_ADDRESS",
        )?,
        exchange_fill_abi()
            .map_err(|error| AuthError::internal("orderbook ABI build failed", error))?,
        client,
    );

    Ok(OrderbookWriteClient { exchange })
}

fn exchange_fill_abi() -> Result<Abi> {
    AbiParser::default().parse(&[
        "function fillDirectOrderPair((address maker, bytes32 conditionId, uint256 outcomeIndex, uint8 side, uint256 price, uint256 amount, uint256 expiry, uint256 salt) buyOrder, bytes buySignature, (address maker, bytes32 conditionId, uint256 outcomeIndex, uint8 side, uint256 price, uint256 amount, uint256 expiry, uint256 salt) sellOrder, bytes sellSignature, uint256 fillAmount, uint256 settlementPrice)",
        "function fillComplementaryBuyOrders((address maker, bytes32 conditionId, uint256 outcomeIndex, uint8 side, uint256 price, uint256 amount, uint256 expiry, uint256 salt) yesBuyOrder, bytes yesBuySignature, (address maker, bytes32 conditionId, uint256 outcomeIndex, uint8 side, uint256 price, uint256 amount, uint256 expiry, uint256 salt) noBuyOrder, bytes noBuySignature, uint256 fillAmount, uint256 yesSettlementPrice, uint256 noSettlementPrice)",
        "function fillComplementarySellOrders((address maker, bytes32 conditionId, uint256 outcomeIndex, uint8 side, uint256 price, uint256 amount, uint256 expiry, uint256 salt) yesSellOrder, bytes yesSellSignature, (address maker, bytes32 conditionId, uint256 outcomeIndex, uint8 side, uint256 price, uint256 amount, uint256 expiry, uint256 salt) noSellOrder, bytes noSellSignature, uint256 fillAmount, uint256 yesSettlementPrice, uint256 noSettlementPrice)",
    ]).map_err(Into::into)
}

fn encode_contract_order(
    order: &MarketOrderRecord,
) -> Result<(Address, H256, U256, u8, U256, U256, U256, U256), AuthError> {
    let side = match order.side.as_str() {
        "buy" => 0_u8,
        "sell" => 1_u8,
        _ => return Err(AuthError::bad_request("order.side must be `buy` or `sell`")),
    };
    Ok((
        parse_address(&order.wallet_address, "order wallet address")?,
        parse_bytes32(&order.condition_id, "order condition id")?,
        U256::from(
            u64::try_from(order.outcome_index)
                .map_err(|_| AuthError::bad_request("order.outcome_index must be 0 or 1"))?,
        ),
        side,
        U256::from(
            u32::try_from(order.price_bps)
                .map_err(|error| AuthError::internal("invalid order price", error))?,
        ),
        parse_amount(&order.amount, "order amount")?,
        match order.expiry_epoch_seconds {
            Some(value) if value > 0 => U256::from(
                u64::try_from(value)
                    .map_err(|_| AuthError::bad_request("order expiry must be non-negative"))?,
            ),
            _ => U256::zero(),
        },
        parse_amount(&order.salt, "order salt")?,
    ))
}

fn decode_signature_bytes(signature: &str) -> Result<Bytes, AuthError> {
    let normalized = signature.trim().trim_start_matches("0x");
    let decoded = hex::decode(normalized)
        .map_err(|_| AuthError::bad_request("stored order signature is invalid"))?;
    Ok(Bytes::from(decoded))
}

fn parse_address(raw: &str, field_name: &str) -> Result<Address, AuthError> {
    Address::from_str(raw)
        .map_err(|_| AuthError::bad_request(format!("{field_name} is not a valid address")))
}

fn parse_bytes32(raw: &str, field_name: &str) -> Result<H256, AuthError> {
    H256::from_str(raw)
        .map_err(|_| AuthError::bad_request(format!("{field_name} is not a valid bytes32 value")))
}
