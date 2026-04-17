use std::{
    collections::{BTreeSet, HashMap},
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use ethers_contract::Contract;
use ethers_core::{
    abi::{Abi, AbiParser, Token, encode},
    types::{Address, Bytes, H256, Signature, U256},
    utils::keccak256,
};
use ethers_providers::{Http, Middleware, Provider};
use tokio::task::JoinSet;

use crate::{
    app::AppState,
    config::environment::Environment,
    module::{
        auth::{error::AuthError, model::ACCOUNT_KIND_SMART_ACCOUNT},
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
        liquidity::{
            view::build_market_responses,
            wallet::{WalletAccountContext, load_wallet_account_context},
        },
        rpc,
        trading::{
            context::{load_trading_market_context, outcome_label},
            format::{
                bps_to_price, format_amount, parse_trade_amount, quote_usdc_amount,
                validate_trade_value_bounds,
            },
        },
    },
};
use uuid::Uuid;

const ORDER_STATUS_CANCELLED: &str = "cancelled";
const ORDER_STATUS_OPEN: &str = "open";
const ORDER_STATUS_PARTIALLY_FILLED: &str = "partially_filled";
const ORDER_SIDE_BUY: &str = "buy";
const ORDER_SIDE_SELL: &str = "sell";
const ORDER_DOMAIN_NAME: &str = "Sabi Exchange";
const ORDER_DOMAIN_VERSION: &str = "1";
const ORDER_DOMAIN_TYPE: &str =
    "EIP712Domain(string name,string version,uint256 chainId,address verifyingContract)";
const ORDER_TYPE: &str = "Order(address maker,bytes32 conditionId,uint256 outcomeIndex,uint8 side,uint256 price,uint256 amount,uint256 expiry,uint256 salt)";
const ERC1271_MAGIC_VALUE: [u8; 4] = [0x16, 0x26, 0xba, 0x7e];
const MAX_CONCURRENT_POSITION_ID_READS: usize = 8;
const MARKET_BALANCE_BATCH_SIZE: usize = 24;

type ReadProvider = Provider<Http>;

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

#[derive(Debug, Clone, Copy)]
struct MarketPositionIds {
    market_id: Uuid,
    yes_position_id: U256,
    no_position_id: U256,
}

pub async fn place_order(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    payload: CreateOrderRequest,
) -> Result<CreateOrderResponse, AuthError> {
    let wallet = load_wallet_account_context(state, authenticated_user.user_id).await?;

    let context = load_trading_market_context(state, payload.order.market_id).await?;
    let market =
        crate::service::liquidity::view::build_market_response(state, &context.market).await?;
    let side = OrderSide::parse(&payload.order.side)?;
    let amount = parse_trade_amount(&payload.order.token_amount, "order.token_amount")?;
    let price_bps = validate_price_bps(payload.order.price_bps)?;
    let quoted_usdc_amount = quote_usdc_amount(amount, price_bps);
    validate_trade_value_bounds(quoted_usdc_amount, "quoted order value")?;

    let (expiry_epoch_seconds, expiry) = normalize_expiry(payload.order.expiry_epoch_seconds)?;
    let salt = parse_decimal_u256(&payload.order.salt, "order.salt")?;
    let maker = parse_address(&wallet.wallet_address, "linked wallet address")?;
    let condition_id = parse_bytes32(&context.condition_id, "market condition id")?;
    let order_hash = compute_order_hash(
        maker,
        condition_id,
        payload.order.outcome_index,
        side,
        price_bps,
        amount,
        expiry,
        salt,
    )?;
    let order_digest = compute_order_digest(&state.env, order_hash)?;
    verify_order_signature(
        &state.env,
        &wallet,
        &payload.order.signature,
        order_digest,
        maker,
    )
    .await?;

    let record = crud::insert_market_order(
        &state.db,
        &NewMarketOrderRecord {
            id: Uuid::new_v4(),
            user_id: authenticated_user.user_id,
            market_id: context.market.id,
            event_id: context.event.id,
            wallet_address: wallet.wallet_address.clone(),
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
            salt: salt.to_string(),
            signature: normalize_signature_hex(&payload.order.signature)?,
            order_hash: format!("{order_hash:#x}"),
            order_digest: format!("{order_digest:#x}"),
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
    let wallet = load_wallet_account_context(state, authenticated_user.user_id).await?;

    let existing = crud::get_market_order_by_id_and_user_id(
        &state.db,
        payload.order.order_id,
        authenticated_user.user_id,
    )
    .await?
    .ok_or_else(|| AuthError::not_found("order not found"))?;

    if existing.wallet_address != wallet.wallet_address {
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
    let wallet = load_wallet_account_context(state, authenticated_user.user_id).await?;
    let orders = crud::list_market_orders_by_user_id(&state.db, authenticated_user.user_id).await?;

    if orders.is_empty() {
        return Ok(MyOrdersResponse {
            wallet_address: wallet.wallet_address,
            account_kind: wallet.account_kind,
            orders: Vec::new(),
        });
    }

    let market_ids = unique_market_ids(&orders);
    let markets = crud::list_markets_by_ids(&state.db, &market_ids).await?;
    let market_responses = build_market_responses(state, &markets).await?;
    let event_ids = unique_event_ids(&orders);
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

    let orders = orders
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
        orders,
    })
}

pub async fn get_my_positions(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
) -> Result<MyPositionsResponse, AuthError> {
    let (wallet, snapshots) = load_my_position_snapshots(state, authenticated_user).await?;

    if snapshots.is_empty() {
        return Ok(MyPositionsResponse {
            wallet_address: wallet.wallet_address,
            account_kind: wallet.account_kind,
            positions: Vec::new(),
        });
    }

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
    let wallet = load_wallet_account_context(state, authenticated_user.user_id).await?;
    let trade_history =
        crud::list_user_trade_history_by_user_id(&state.db, authenticated_user.user_id).await?;
    let cash_balance = read_usdc_balance(state, &wallet.wallet_address).await?;

    if trade_history.is_empty() {
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

    let market_ids = trade_history
        .iter()
        .map(|record| record.market_id)
        .collect::<BTreeSet<_>>();
    let market_ids = market_ids.into_iter().collect::<Vec<_>>();
    let markets = crud::list_markets_by_ids(&state.db, &market_ids).await?;
    let snapshots = load_position_snapshots_for_markets(state, &wallet, &markets).await?;
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
            let usdc_amount = parse_stored_amount(&record.usdc_amount, "stored trade usdc amount")?;
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

    let markets = market_rows
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
        markets,
        history,
    })
}

async fn load_my_position_snapshots(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
) -> Result<(WalletAccountContext, Vec<PositionSnapshot>), AuthError> {
    let wallet = load_wallet_account_context(state, authenticated_user.user_id).await?;
    let markets = crud::list_markets_with_condition_ids(&state.db).await?;

    let snapshots = load_position_snapshots_for_markets(state, &wallet, &markets).await?;

    Ok((wallet, snapshots))
}

async fn load_position_snapshots_for_markets(
    state: &AppState,
    wallet: &WalletAccountContext,
    markets: &[MarketRecord],
) -> Result<Vec<PositionSnapshot>, AuthError> {
    if markets.is_empty() {
        return Ok(Vec::new());
    }

    let reader = OrderChainReader::new(&state.env)
        .await
        .map_err(|error| AuthError::internal("market positions read failed", error))?;
    let balances_by_market_id = reader
        .get_market_outcome_balances(&wallet.wallet_address, &markets)
        .await
        .map_err(|error| AuthError::internal("market positions read failed", error))?;

    let markets_with_positions = markets
        .into_iter()
        .filter(|market| {
            balances_by_market_id
                .get(&market.id)
                .is_some_and(|(yes, no)| !yes.is_zero() || !no.is_zero())
        })
        .cloned()
        .collect::<Vec<_>>();

    if markets_with_positions.is_empty() {
        return Ok(Vec::new());
    }

    let market_responses = build_market_responses(state, &markets_with_positions).await?;
    let event_ids = markets_with_positions
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
    let market_responses_by_id = market_responses
        .into_iter()
        .map(|market| (market.id, market))
        .collect::<HashMap<_, _>>();

    let snapshots = markets_with_positions
        .into_iter()
        .map(|market| {
            let event = events_by_id
                .get(&market.event_db_id)
                .cloned()
                .ok_or_else(|| AuthError::internal("missing position event", market.event_db_id))?;
            let market_response =
                market_responses_by_id
                    .get(&market.id)
                    .cloned()
                    .ok_or_else(|| {
                        AuthError::internal("missing position market response", market.id)
                    })?;
            let (yes_balance, no_balance) = balances_by_market_id
                .get(&market.id)
                .cloned()
                .ok_or_else(|| {
                    AuthError::internal("missing on-chain position balance", market.id)
                })?;

            Ok::<PositionSnapshot, AuthError>(PositionSnapshot {
                event,
                market_record: market,
                market_response,
                yes_balance,
                no_balance,
            })
        })
        .collect::<Result<Vec<_>, _>>()?;

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

fn normalize_expiry(expiry_epoch_seconds: Option<i64>) -> Result<(Option<i64>, U256), AuthError> {
    match expiry_epoch_seconds {
        None | Some(0) => Ok((None, U256::zero())),
        Some(value) if value <= Utc::now().timestamp() => Err(AuthError::bad_request(
            "order.expiry_epoch_seconds must be in the future",
        )),
        Some(value) => {
            let expiry =
                u64::try_from(value).map_err(|_| AuthError::bad_request("invalid order expiry"))?;
            Ok((Some(value), U256::from(expiry)))
        }
    }
}

fn parse_decimal_u256(raw: &str, field_name: &str) -> Result<U256, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }

    U256::from_dec_str(value).map_err(|_| {
        AuthError::bad_request(format!("{field_name} must be a base-10 integer string",))
    })
}

fn parse_address(value: &str, field_name: &str) -> Result<Address, AuthError> {
    Address::from_str(value)
        .map_err(|_| AuthError::bad_request(format!("{field_name} is not a valid address")))
}

fn parse_bytes32(value: &str, field_name: &str) -> Result<H256, AuthError> {
    H256::from_str(value)
        .map_err(|_| AuthError::bad_request(format!("{field_name} is not a valid bytes32 value")))
}

fn normalize_signature_hex(signature: &str) -> Result<String, AuthError> {
    let signature = signature.trim();
    if signature.is_empty() {
        return Err(AuthError::bad_request("order.signature is required"));
    }

    let signature = signature
        .strip_prefix("0x")
        .map_or(signature.to_owned(), str::to_owned);
    if signature.len() != 130
        || !signature
            .chars()
            .all(|character| character.is_ascii_hexdigit())
    {
        return Err(AuthError::bad_request(
            "order.signature must be a 65-byte hex string",
        ));
    }

    Ok(format!("0x{signature}"))
}

async fn verify_order_signature(
    env: &Environment,
    wallet: &WalletAccountContext,
    signature_raw: &str,
    order_digest: H256,
    maker: Address,
) -> Result<(), AuthError> {
    let signature_hex = normalize_signature_hex(signature_raw)?;

    if wallet.account_kind != ACCOUNT_KIND_SMART_ACCOUNT {
        return verify_eoa_order_signature(&signature_hex, order_digest, maker);
    }

    let provider = rpc::monad_provider_arc(env)
        .await
        .map_err(|error| AuthError::internal("order signature validation setup failed", error))?;
    let maker_code = provider
        .get_code(maker, None)
        .await
        .map_err(|error| AuthError::internal("smart-account code lookup failed", error))?;

    if maker_code.as_ref().is_empty() {
        return Err(AuthError::bad_request(
            "linked smart-account wallet is not deployed on-chain and cannot place orderbook orders yet",
        ));
    }

    let signature_bytes = decode_signature_hex(&signature_hex)?;
    let validator = Contract::new(maker, erc1271_abi()?, provider);
    let magic_value = validator
        .method::<_, [u8; 4]>(
            "isValidSignature",
            (order_digest, Bytes::from(signature_bytes)),
        )
        .map_err(|error| {
            AuthError::internal("smart-account signature validation setup failed", error)
        })?
        .call()
        .await
        .map_err(|_| {
            AuthError::bad_request(
                "order.signature is not valid for the linked smart-account wallet",
            )
        })?;

    if magic_value != ERC1271_MAGIC_VALUE {
        return Err(AuthError::bad_request(
            "order.signature is not valid for the linked smart-account wallet",
        ));
    }

    Ok(())
}

fn verify_eoa_order_signature(
    signature_hex: &str,
    order_digest: H256,
    maker: Address,
) -> Result<(), AuthError> {
    let signature = Signature::from_str(signature_hex)
        .map_err(|_| AuthError::bad_request("order.signature is invalid"))?;
    let recovered = signature
        .recover(order_digest)
        .map_err(|_| AuthError::bad_request("order.signature does not recover a valid signer"))?;

    if recovered != maker {
        return Err(AuthError::bad_request(
            "order.signature does not match the linked wallet address",
        ));
    }

    Ok(())
}

fn decode_signature_hex(signature_hex: &str) -> Result<Vec<u8>, AuthError> {
    hex::decode(signature_hex.trim_start_matches("0x"))
        .map_err(|_| AuthError::bad_request("order.signature is invalid"))
}

fn compute_order_hash(
    maker: Address,
    condition_id: H256,
    outcome_index: i32,
    side: OrderSide,
    price_bps: u32,
    amount: U256,
    expiry: U256,
    salt: U256,
) -> Result<H256, AuthError> {
    let outcome_index = u64::try_from(outcome_index)
        .map_err(|_| AuthError::bad_request("order.outcome_index must be 0 or 1"))?;
    if outcome_index > 1 {
        return Err(AuthError::bad_request("order.outcome_index must be 0 or 1"));
    }

    Ok(H256::from(keccak256(encode(&[
        Token::FixedBytes(keccak256(ORDER_TYPE).to_vec()),
        Token::Address(maker),
        Token::FixedBytes(condition_id.as_bytes().to_vec()),
        Token::Uint(U256::from(outcome_index)),
        Token::Uint(U256::from(side.as_u8())),
        Token::Uint(U256::from(price_bps)),
        Token::Uint(amount),
        Token::Uint(expiry),
        Token::Uint(salt),
    ]))))
}

fn compute_order_digest(env: &Environment, order_hash: H256) -> Result<H256, AuthError> {
    let verifying_contract = parse_address(
        &env.monad_orderbook_exchange_address,
        "MONAD_ORDERBOOK_EXCHANGE_ADDRESS",
    )?;
    let chain_id = u64::try_from(env.monad_chain_id)
        .map_err(|_| AuthError::bad_request("MONAD_CHAIN_ID must be non-negative"))?;
    let domain_separator = H256::from(keccak256(encode(&[
        Token::FixedBytes(keccak256(ORDER_DOMAIN_TYPE).to_vec()),
        Token::FixedBytes(keccak256(ORDER_DOMAIN_NAME).to_vec()),
        Token::FixedBytes(keccak256(ORDER_DOMAIN_VERSION).to_vec()),
        Token::Uint(U256::from(chain_id)),
        Token::Address(verifying_contract),
    ])));

    let mut encoded = Vec::with_capacity(66);
    encoded.extend_from_slice(b"\x19\x01");
    encoded.extend_from_slice(domain_separator.as_bytes());
    encoded.extend_from_slice(order_hash.as_bytes());

    Ok(H256::from(keccak256(encoded)))
}

fn erc1271_abi() -> Result<Abi, AuthError> {
    AbiParser::default()
        .parse(&["function isValidSignature(bytes32 hash, bytes signature) view returns (bytes4 magicValue)"])
        .map_err(|error| AuthError::internal("failed to build ERC-1271 ABI", error))
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
    let filled_amount = parse_stored_amount(&record.filled_amount, "stored filled order amount")?;
    let remaining_amount =
        parse_stored_amount(&record.remaining_amount, "stored remaining order amount")?;
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
    let usdc_amount = parse_stored_amount(&record.usdc_amount, "stored trade usdc amount")?;
    let token_amount = parse_stored_amount(&record.token_amount, "stored trade token amount")?;

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

#[derive(Debug, Clone, Copy)]
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

    fn as_u8(self) -> u8 {
        match self {
            Self::Buy => 0,
            Self::Sell => 1,
        }
    }
}

#[derive(Clone)]
struct OrderChainReader {
    conditional_tokens: Contract<ReadProvider>,
    collateral_token: Address,
}

impl OrderChainReader {
    async fn new(env: &Environment) -> Result<Self> {
        let provider = rpc::monad_provider_arc(env).await?;
        let conditional_tokens = Contract::new(
            parse_address(
                &env.monad_conditional_tokens_address,
                "MONAD_CONDITIONAL_TOKENS_ADDRESS",
            )
            .map_err(|error| anyhow!(error.to_string()))?,
            conditional_tokens_read_abi()?,
            provider,
        );
        let collateral_token = parse_address(&env.monad_usdc_address, "MONAD_USDC_ADDRESS")
            .map_err(|error| anyhow!(error.to_string()))?;

        Ok(Self {
            conditional_tokens,
            collateral_token,
        })
    }

    async fn get_market_outcome_balances(
        &self,
        wallet_address: &str,
        markets: &[MarketRecord],
    ) -> Result<HashMap<Uuid, (U256, U256)>> {
        let wallet = Address::from_str(wallet_address).context("invalid wallet address")?;
        let position_ids = self.load_market_position_ids(markets).await?;

        if position_ids.is_empty() {
            return Ok(HashMap::new());
        }

        self.load_market_balances(wallet_address, wallet, &position_ids)
            .await
    }

    async fn load_market_position_ids(
        &self,
        markets: &[MarketRecord],
    ) -> Result<Vec<MarketPositionIds>> {
        let mut join_set = JoinSet::new();
        let mut next_market_index = 0_usize;

        while next_market_index < markets.len() && join_set.len() < MAX_CONCURRENT_POSITION_ID_READS
        {
            let reader = self.clone();
            let market = markets[next_market_index].clone();
            next_market_index += 1;
            join_set.spawn(async move {
                let market_id = market.id;
                let result = reader.get_market_position_ids(&market).await;
                (market_id, result)
            });
        }

        let mut position_ids = Vec::with_capacity(markets.len());
        let mut first_error = None;
        while let Some(result) = join_set.join_next().await {
            let (market_id, position_result) =
                result.context("market position id task join failed")?;

            match position_result {
                Ok(position) => position_ids.push(position),
                Err(error) => {
                    tracing::warn!(
                        %market_id,
                        error = %error,
                        "skipping market during conditional token position id read"
                    );
                    if first_error.is_none() {
                        first_error = Some(
                            error.context(format!("market {market_id} position id read failed")),
                        );
                    }
                }
            }

            if next_market_index < markets.len() {
                let reader = self.clone();
                let market = markets[next_market_index].clone();
                next_market_index += 1;
                join_set.spawn(async move {
                    let market_id = market.id;
                    let result = reader.get_market_position_ids(&market).await;
                    (market_id, result)
                });
            }
        }

        if position_ids.is_empty() && !markets.is_empty() {
            return Err(first_error
                .unwrap_or_else(|| anyhow!("failed to compute conditional token position ids")));
        }

        Ok(position_ids)
    }

    async fn get_market_position_ids(&self, market: &MarketRecord) -> Result<MarketPositionIds> {
        let condition_id = market
            .condition_id
            .as_deref()
            .ok_or_else(|| anyhow!("market {} is missing a condition id", market.id))?;
        let condition_id = H256::from_str(condition_id).context("invalid condition id")?;

        let yes_collection = self
            .conditional_tokens
            .method::<_, H256>(
                "getCollectionId",
                (H256::zero(), condition_id, U256::from(1_u64)),
            )?
            .call()
            .await
            .context("failed to compute YES collection id")?;
        let no_collection = self
            .conditional_tokens
            .method::<_, H256>(
                "getCollectionId",
                (H256::zero(), condition_id, U256::from(2_u64)),
            )?
            .call()
            .await
            .context("failed to compute NO collection id")?;

        let yes_position_id = self
            .conditional_tokens
            .method::<_, U256>("getPositionId", (self.collateral_token, yes_collection))?
            .call()
            .await
            .context("failed to compute YES position id")?;
        let no_position_id = self
            .conditional_tokens
            .method::<_, U256>("getPositionId", (self.collateral_token, no_collection))?
            .call()
            .await
            .context("failed to compute NO position id")?;

        Ok(MarketPositionIds {
            market_id: market.id,
            yes_position_id,
            no_position_id,
        })
    }

    async fn load_market_balances(
        &self,
        wallet_address: &str,
        wallet: Address,
        position_ids: &[MarketPositionIds],
    ) -> Result<HashMap<Uuid, (U256, U256)>> {
        let mut balances_by_market_id = HashMap::with_capacity(position_ids.len());
        let mut first_error = None;

        for chunk in position_ids.chunks(MARKET_BALANCE_BATCH_SIZE) {
            match self.read_market_balance_chunk(wallet, chunk).await {
                Ok(chunk_balances) => {
                    balances_by_market_id.extend(chunk_balances);
                }
                Err(error) => {
                    tracing::warn!(
                        %wallet_address,
                        market_count = chunk.len(),
                        error = %error,
                        "conditional token batch balance read failed; retrying per market"
                    );
                    if first_error.is_none() {
                        first_error =
                            Some(error.context("conditional token batch balance read failed"));
                    }

                    for position in chunk {
                        match self.read_single_market_balances(wallet, position).await {
                            Ok((yes_balance, no_balance)) => {
                                balances_by_market_id
                                    .insert(position.market_id, (yes_balance, no_balance));
                            }
                            Err(error) => {
                                tracing::warn!(
                                    %wallet_address,
                                    market_id = %position.market_id,
                                    error = %error,
                                    "skipping market during conditional token balance read"
                                );
                                if first_error.is_none() {
                                    first_error = Some(error.context(format!(
                                        "conditional token balance read failed for market {}",
                                        position.market_id
                                    )));
                                }
                            }
                        }
                    }
                }
            }
        }

        if balances_by_market_id.is_empty() && !position_ids.is_empty() {
            return Err(first_error
                .unwrap_or_else(|| anyhow!("failed to query conditional token balances")));
        }

        Ok(balances_by_market_id)
    }

    async fn read_market_balance_chunk(
        &self,
        wallet: Address,
        position_ids: &[MarketPositionIds],
    ) -> Result<HashMap<Uuid, (U256, U256)>> {
        let mut token_ids = Vec::with_capacity(position_ids.len() * 2);
        let mut offsets = Vec::with_capacity(position_ids.len());

        for position in position_ids {
            let yes_index = token_ids.len();
            token_ids.push(position.yes_position_id);
            let no_index = token_ids.len();
            token_ids.push(position.no_position_id);
            offsets.push((position.market_id, yes_index, no_index));
        }

        let accounts = vec![wallet; token_ids.len()];
        let balances = self
            .conditional_tokens
            .method::<_, Vec<U256>>("balanceOfBatch", (accounts, token_ids))?
            .call()
            .await
            .context("failed to query conditional token balances")?;

        let mut balances_by_market_id = HashMap::with_capacity(position_ids.len());
        for (market_id, yes_index, no_index) in offsets {
            let yes_balance = balances
                .get(yes_index)
                .copied()
                .ok_or_else(|| anyhow!("missing YES balance entry"))?;
            let no_balance = balances
                .get(no_index)
                .copied()
                .ok_or_else(|| anyhow!("missing NO balance entry"))?;
            balances_by_market_id.insert(market_id, (yes_balance, no_balance));
        }

        Ok(balances_by_market_id)
    }

    async fn read_single_market_balances(
        &self,
        wallet: Address,
        position: &MarketPositionIds,
    ) -> Result<(U256, U256)> {
        let yes_balance = self
            .conditional_tokens
            .method::<_, U256>("balanceOf", (wallet, position.yes_position_id))?
            .call()
            .await
            .context("failed to query YES conditional token balance")?;
        let no_balance = self
            .conditional_tokens
            .method::<_, U256>("balanceOf", (wallet, position.no_position_id))?
            .call()
            .await
            .context("failed to query NO conditional token balance")?;

        Ok((yes_balance, no_balance))
    }
}

fn conditional_tokens_read_abi() -> Result<Abi> {
    AbiParser::default()
        .parse(&[
            "function getCollectionId(bytes32 parentCollectionId, bytes32 conditionId, uint256 indexSet) view returns (bytes32)",
            "function getPositionId(address collateralToken, bytes32 collectionId) view returns (uint256)",
            "function balanceOf(address account, uint256 id) view returns (uint256)",
            "function balanceOfBatch(address[] accounts, uint256[] ids) view returns (uint256[])",
        ])
        .map_err(Into::into)
}
