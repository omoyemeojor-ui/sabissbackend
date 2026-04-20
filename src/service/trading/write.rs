use chrono::Utc;
use ethers_core::types::U256;
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::{error::AuthError, model::ACCOUNT_KIND_STELLAR_SMART_WALLET},
        market::trade_schema::{
            BuyMarketRequest, MarketPositionConversionResponse, MarketTradeExecutionResponse,
            MergeMarketRequest, SellMarketRequest, SplitMarketRequest,
        },
        order::{crud as order_crud, model::NewUserMarketTradeRecord},
    },
    service::{
        jwt::AuthenticatedUser,
        liquidity::{
            view::build_market_response,
            wallet::{load_smart_account_context, load_wallet_account_context},
        },
        monad::MarketPricesReadResult,
    },
};

use super::{
    chain_write,
    context::{load_trading_market_context, outcome_label},
    format::{
        bps_to_price, build_market_quote, format_amount, parse_trade_amount, quote_token_amount,
        quote_usdc_amount, validate_trade_value_bounds,
    },
    persistence::sync_trade_state,
    prepare,
};

enum TradeWriteAction {
    Buy {
        outcome_index: i32,
        usdc_amount: U256,
    },
    Sell {
        outcome_index: i32,
        token_amount: U256,
    },
}

impl TradeWriteAction {
    fn name(&self) -> &'static str {
        match self {
            Self::Buy { .. } => "buy",
            Self::Sell { .. } => "sell",
        }
    }

    fn outcome_index(&self) -> i32 {
        match self {
            Self::Buy { outcome_index, .. } | Self::Sell { outcome_index, .. } => *outcome_index,
        }
    }
}

enum ConversionWriteAction {
    Split { collateral_amount: U256 },
    Merge { pair_token_amount: U256 },
}

impl ConversionWriteAction {
    fn name(&self) -> &'static str {
        match self {
            Self::Split { .. } => "split",
            Self::Merge { .. } => "merge",
        }
    }
}

pub async fn buy_market_outcome(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: BuyMarketRequest,
) -> Result<MarketTradeExecutionResponse, AuthError> {
    let usdc_amount = parse_trade_amount(&payload.trade.usdc_amount, "trade.usdc_amount")?;
    execute_market_trade(
        state,
        authenticated_user,
        market_id,
        TradeWriteAction::Buy {
            outcome_index: payload.trade.outcome_index,
            usdc_amount,
        },
    )
    .await
}

pub async fn sell_market_outcome(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: SellMarketRequest,
) -> Result<MarketTradeExecutionResponse, AuthError> {
    let token_amount = parse_trade_amount(&payload.trade.token_amount, "trade.token_amount")?;
    execute_market_trade(
        state,
        authenticated_user,
        market_id,
        TradeWriteAction::Sell {
            outcome_index: payload.trade.outcome_index,
            token_amount,
        },
    )
    .await
}

pub async fn split_market_collateral(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: SplitMarketRequest,
) -> Result<MarketPositionConversionResponse, AuthError> {
    let collateral_amount = parse_trade_amount(
        &payload.conversion.collateral_amount,
        "conversion.collateral_amount",
    )?;
    execute_market_conversion(
        state,
        authenticated_user,
        market_id,
        ConversionWriteAction::Split { collateral_amount },
    )
    .await
}

pub async fn merge_market_positions(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: MergeMarketRequest,
) -> Result<MarketPositionConversionResponse, AuthError> {
    let pair_token_amount = parse_trade_amount(
        &payload.conversion.pair_token_amount,
        "conversion.pair_token_amount",
    )?;
    execute_market_conversion(
        state,
        authenticated_user,
        market_id,
        ConversionWriteAction::Merge { pair_token_amount },
    )
    .await
}

async fn execute_market_trade(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    action: TradeWriteAction,
) -> Result<MarketTradeExecutionResponse, AuthError> {
    let context = load_trading_market_context(state, market_id).await?;
    let wallet = load_wallet_account_context(state, authenticated_user.user_id).await?;
    let prices = crate::service::monad::get_market_prices(&state.env, &context.condition_id)
        .await
        .map_err(|error| AuthError::internal("market price read failed", error))?;
    let price_bps = outcome_price_bps(&prices, action.outcome_index())?;
    let outcome_label = outcome_label(&context.market, action.outcome_index())?;
    let executed_amounts = executed_amounts(&action, price_bps)?;
    validate_trade_limits(&action, &executed_amounts)?;
    validate_trade_liquidity(
        state,
        &context.condition_id,
        &action,
        &executed_amounts,
        price_bps,
    )
    .await?;
    let market = build_market_response(state, &context.market).await?;

    if wallet.account_kind == ACCOUNT_KIND_STELLAR_SMART_WALLET {
        return execute_smart_account_trade(
            state,
            authenticated_user,
            context,
            wallet,
            action,
            price_bps,
            outcome_label,
            executed_amounts,
            market,
        )
        .await;
    }

    execute_external_wallet_trade(
        state,
        context,
        wallet,
        action,
        prices,
        price_bps,
        outcome_label,
        executed_amounts,
        market,
    )
    .await
}

async fn execute_market_conversion(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    action: ConversionWriteAction,
) -> Result<MarketPositionConversionResponse, AuthError> {
    let context = load_trading_market_context(state, market_id).await?;
    let wallet = load_wallet_account_context(state, authenticated_user.user_id).await?;
    let market = build_market_response(state, &context.market).await?;

    if wallet.account_kind == ACCOUNT_KIND_STELLAR_SMART_WALLET {
        let _signer = load_smart_account_context(state, authenticated_user.user_id).await?;
        let tx = match action {
            ConversionWriteAction::Split { collateral_amount } => {
                chain_write::split_position(
                    state,
                    authenticated_user.user_id,
                    &context.condition_id,
                    collateral_amount,
                )
                .await
            }
            ConversionWriteAction::Merge { pair_token_amount } => {
                chain_write::merge_positions(
                    state,
                    authenticated_user.user_id,
                    &context.condition_id,
                    pair_token_amount,
                )
                .await
            }
        }
        .map_err(|error| map_trade_chain_error(action.name(), error))?;

        return Ok(MarketPositionConversionResponse {
            event: (&context.event).into(),
            on_chain: (&context.event).into(),
            market,
            wallet_address: wallet.wallet_address,
            account_kind: wallet.account_kind,
            action: action.name().to_owned(),
            execution_mode: "smart_account".to_owned(),
            execution_status: "submitted".to_owned(),
            tx_hash: Some(tx.tx_hash),
            prepared_transactions: None,
            collateral_amount: format_amount(&conversion_collateral_amount(&action)),
            token_amount: format_amount(&conversion_token_amount(&action)),
            requested_at: Utc::now(),
        });
    }

    let prepared_transactions = match action {
        ConversionWriteAction::Split { collateral_amount } => {
            prepare::prepare_split_position(
                &state.env,
                &wallet.wallet_address,
                &context.condition_id,
                collateral_amount,
            )
            .await
        }
        ConversionWriteAction::Merge { pair_token_amount } => {
            prepare::prepare_merge_positions(
                &state.env,
                &wallet.wallet_address,
                &context.condition_id,
                pair_token_amount,
            )
            .await
        }
    }
    .map_err(|error| map_trade_chain_error(action.name(), error))?;

    Ok(MarketPositionConversionResponse {
        event: (&context.event).into(),
        on_chain: (&context.event).into(),
        market,
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
        action: action.name().to_owned(),
        execution_mode: "external_wallet".to_owned(),
        execution_status: "prepared".to_owned(),
        tx_hash: None,
        prepared_transactions: Some(prepared_transactions),
        collateral_amount: format_amount(&conversion_collateral_amount(&action)),
        token_amount: format_amount(&conversion_token_amount(&action)),
        requested_at: Utc::now(),
    })
}

async fn execute_smart_account_trade(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    context: super::context::TradingMarketContext,
    wallet: crate::service::liquidity::wallet::WalletAccountContext,
    action: TradeWriteAction,
    price_bps: u32,
    outcome_label: String,
    executed_amounts: ExecutedAmounts,
    market: crate::module::market::schema::MarketResponse,
) -> Result<MarketTradeExecutionResponse, AuthError> {
    let _signer = load_smart_account_context(state, authenticated_user.user_id).await?;
    let tx = match action {
        TradeWriteAction::Buy {
            outcome_index,
            usdc_amount,
        } => {
            chain_write::buy_outcome(
                state,
                authenticated_user.user_id,
                &context.condition_id,
                outcome_index,
                usdc_amount,
            )
            .await
        }
        TradeWriteAction::Sell {
            outcome_index,
            token_amount,
        } => {
            chain_write::sell_outcome(
                state,
                authenticated_user.user_id,
                &context.condition_id,
                outcome_index,
                token_amount,
            )
            .await
        }
    }
    .map_err(|error| map_trade_chain_error(action.name(), error))?;

    order_crud::insert_user_market_trade(
        &state.db,
        &NewUserMarketTradeRecord {
            user_id: authenticated_user.user_id,
            market_id: context.market.id,
            event_id: context.event.id,
            wallet_address: wallet.wallet_address.clone(),
            execution_source: "market_trade".to_owned(),
            action: action.name().to_owned(),
            outcome_index: action.outcome_index(),
            price_bps: i32::try_from(price_bps)
                .map_err(|error| AuthError::internal("invalid market trade price", error))?,
            token_amount: executed_amounts.token_amount.to_string(),
            usdc_amount: executed_amounts.usdc_amount.to_string(),
            tx_hash: Some(tx.tx_hash.clone()),
        },
    )
    .await?;

    let trade_state = sync_trade_state(
        state,
        context.market.id,
        &context.condition_id,
        action.outcome_index(),
        price_bps,
        &executed_amounts.usdc_amount,
    )
    .await?;

    Ok(MarketTradeExecutionResponse {
        event: (&context.event).into(),
        on_chain: (&context.event).into(),
        market,
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
        action: action.name().to_owned(),
        outcome_index: action.outcome_index(),
        outcome_label,
        execution_mode: "smart_account".to_owned(),
        execution_status: "submitted".to_owned(),
        tx_hash: Some(tx.tx_hash),
        prepared_transactions: None,
        usdc_amount: format_amount(&executed_amounts.usdc_amount),
        token_amount: format_amount(&executed_amounts.token_amount),
        price_bps,
        price: bps_to_price(price_bps),
        market_quote: build_market_quote(
            context.market.id,
            &context.condition_id,
            trade_state.yes_bps,
            trade_state.no_bps,
            trade_state.last_trade_yes_bps,
            trade_state.as_of,
        ),
        requested_at: Utc::now(),
    })
}

async fn execute_external_wallet_trade(
    state: &AppState,
    context: super::context::TradingMarketContext,
    wallet: crate::service::liquidity::wallet::WalletAccountContext,
    action: TradeWriteAction,
    prices: MarketPricesReadResult,
    price_bps: u32,
    outcome_label: String,
    executed_amounts: ExecutedAmounts,
    market: crate::module::market::schema::MarketResponse,
) -> Result<MarketTradeExecutionResponse, AuthError> {
    let prepared_transactions = match action {
        TradeWriteAction::Buy {
            outcome_index,
            usdc_amount,
        } => {
            prepare::prepare_buy_outcome(
                &state.env,
                &wallet.wallet_address,
                &context.condition_id,
                outcome_index,
                usdc_amount,
            )
            .await
        }
        TradeWriteAction::Sell {
            outcome_index,
            token_amount,
        } => {
            prepare::prepare_sell_outcome(
                &state.env,
                &wallet.wallet_address,
                &context.condition_id,
                outcome_index,
                token_amount,
            )
            .await
        }
    }
    .map_err(|error| map_trade_chain_error(action.name(), error))?;

    Ok(MarketTradeExecutionResponse {
        event: (&context.event).into(),
        on_chain: (&context.event).into(),
        market,
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
        action: action.name().to_owned(),
        outcome_index: action.outcome_index(),
        outcome_label,
        execution_mode: "external_wallet".to_owned(),
        execution_status: "prepared".to_owned(),
        tx_hash: None,
        prepared_transactions: Some(prepared_transactions),
        usdc_amount: format_amount(&executed_amounts.usdc_amount),
        token_amount: format_amount(&executed_amounts.token_amount),
        price_bps,
        price: bps_to_price(price_bps),
        market_quote: build_market_quote(
            context.market.id,
            &context.condition_id,
            prices.yes_bps,
            prices.no_bps,
            default_last_trade_yes_bps(action.outcome_index(), price_bps),
            Utc::now(),
        ),
        requested_at: Utc::now(),
    })
}

struct ExecutedAmounts {
    usdc_amount: U256,
    token_amount: U256,
}

fn executed_amounts(
    action: &TradeWriteAction,
    price_bps: u32,
) -> Result<ExecutedAmounts, AuthError> {
    match action {
        TradeWriteAction::Buy { usdc_amount, .. } => Ok(ExecutedAmounts {
            usdc_amount: *usdc_amount,
            token_amount: quote_token_amount(*usdc_amount, price_bps)?,
        }),
        TradeWriteAction::Sell { token_amount, .. } => Ok(ExecutedAmounts {
            usdc_amount: quote_usdc_amount(*token_amount, price_bps),
            token_amount: *token_amount,
        }),
    }
}

fn outcome_price_bps(
    prices: &MarketPricesReadResult,
    outcome_index: i32,
) -> Result<u32, AuthError> {
    match outcome_index {
        0 => Ok(prices.yes_bps),
        1 => Ok(prices.no_bps),
        _ => Err(AuthError::bad_request("trade.outcome_index must be 0 or 1")),
    }
}

fn default_last_trade_yes_bps(outcome_index: i32, outcome_price_bps: u32) -> u32 {
    match outcome_index {
        0 => outcome_price_bps,
        1 => 10_000_u32.saturating_sub(outcome_price_bps),
        _ => outcome_price_bps,
    }
}

fn conversion_collateral_amount(action: &ConversionWriteAction) -> U256 {
    match action {
        ConversionWriteAction::Split { collateral_amount } => *collateral_amount,
        ConversionWriteAction::Merge { pair_token_amount } => *pair_token_amount,
    }
}

fn conversion_token_amount(action: &ConversionWriteAction) -> U256 {
    match action {
        ConversionWriteAction::Split { collateral_amount } => *collateral_amount,
        ConversionWriteAction::Merge { pair_token_amount } => *pair_token_amount,
    }
}

fn validate_trade_limits(
    action: &TradeWriteAction,
    executed_amounts: &ExecutedAmounts,
) -> Result<(), AuthError> {
    match action {
        TradeWriteAction::Buy { .. } => {
            validate_trade_value_bounds(executed_amounts.usdc_amount, "trade.usdc_amount")
        }
        TradeWriteAction::Sell { .. } => {
            validate_trade_value_bounds(executed_amounts.usdc_amount, "quoted sell value")
        }
    }
}

async fn validate_trade_liquidity(
    state: &AppState,
    condition_id: &str,
    action: &TradeWriteAction,
    executed_amounts: &ExecutedAmounts,
    price_bps: u32,
) -> Result<(), AuthError> {
    let TradeWriteAction::Buy { outcome_index, .. } = action else {
        return Ok(());
    };

    let liquidity = crate::service::monad::get_market_liquidity(&state.env, condition_id)
        .await
        .map_err(|error| AuthError::internal("market liquidity read failed", error))?;
    let available_raw = match outcome_index {
        0 => &liquidity.yes_available,
        1 => &liquidity.no_available,
        _ => return Err(AuthError::bad_request("trade.outcome_index must be 0 or 1")),
    };
    let available = U256::from_dec_str(available_raw)
        .map_err(|error| AuthError::internal("invalid on-chain liquidity amount", error))?;

    if executed_amounts.token_amount <= available {
        return Ok(());
    }

    let max_usdc_amount = quote_usdc_amount(available, price_bps);
    let outcome_name = if *outcome_index == 0 { "YES" } else { "NO" };
    Err(AuthError::bad_request(format!(
        "insufficient on-chain liquidity for {outcome_name}: requested {} tokens from {} USDC, but only {} tokens are currently available. Maximum buy size at the current price is {} USDC",
        format_amount(&executed_amounts.token_amount),
        format_amount(&executed_amounts.usdc_amount),
        format_amount(&available),
        format_amount(&max_usdc_amount),
    )))
}

fn map_trade_chain_error(context: &'static str, error: anyhow::Error) -> AuthError {
    let messages = error
        .chain()
        .map(|cause| cause.to_string())
        .collect::<Vec<_>>();
    let contains = |needle: &str| messages.iter().any(|message| message.contains(needle));

    if contains("AA21 didn't pay prefund") {
        return AuthError::bad_request(
            "smart-account wallet has no native MON to prefund gas (AA21 didn't pay prefund)",
        );
    }

    if contains("maxFeePerGas must be at least") || contains("pimlico_getUserOperationGasPrice") {
        return AuthError::bad_request(
            "bundler rejected the user-operation gas price because network gas moved; retry the trade",
        );
    }

    if contains("Insufficient liquidity")
        || contains("Price not set")
        || contains("Trade amount too small")
        || contains("Trade amount too large")
        || contains("USDC transfer failed")
        || contains("UserOperation reverted during simulation with reason:")
        || contains("transaction reverted:")
        || contains("execution reverted")
    {
        return AuthError::bad_request(messages.join(": "));
    }

    AuthError::internal(context, error)
}
