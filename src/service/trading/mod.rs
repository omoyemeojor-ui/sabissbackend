use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::{crud as auth_crud, error::AuthError, model::ACCOUNT_KIND_STELLAR_SMART_WALLET},
        market::{
            schema::{EventOnChainResponse, EventResponse, MarketResponse},
            trade_schema::{
                BuyMarketRequest, MarketPositionConversionResponse, MarketTradeExecutionResponse,
                MergeMarketRequest, SellMarketRequest, SplitMarketRequest,
            },
        },
        order::{crud as order_crud, model::NewUserMarketTradeRecord},
    },
    service::{
        crypto::{decrypt_private_key, encode_stellar_secret_key},
        jwt::AuthenticatedUser,
        market::get_market_quote,
        stellar,
    },
};

use self::{
    context::{load_trading_market_context, outcome_label},
    format::{
        bps_to_price, format_amount, parse_trade_amount, quote_token_amount, quote_usdc_amount,
        validate_trade_value_bounds,
    },
};

pub mod context;
pub mod format;

struct SigningWalletContext {
    wallet_address: String,
    account_kind: String,
    actor_address: String,
    source_account: String,
}

pub async fn buy_market_outcome(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: BuyMarketRequest,
) -> Result<MarketTradeExecutionResponse, AuthError> {
    let usdc_amount = parse_trade_amount(&payload.trade.usdc_amount, "trade.usdc_amount")?;
    validate_trade_value_bounds(usdc_amount, "trade.usdc_amount")?;

    let context = load_trading_market_context(state, market_id).await?;
    let signing_wallet = load_signing_wallet_context(state, authenticated_user.user_id).await?;
    let prices = load_market_prices(state, &context.condition_id).await?;
    let price_bps = price_bps_for_outcome(payload.trade.outcome_index, prices.yes_bps, prices.no_bps)?;
    let token_amount = quote_token_amount(usdc_amount, price_bps)?;
    let outcome_label = outcome_label(&context.market, payload.trade.outcome_index)?;
    ensure_buy_liquidity_available(state, &context.condition_id, payload.trade.outcome_index).await?;
    let tx = stellar::buy_market_outcome(
        &state.env,
        &signing_wallet.source_account,
        &signing_wallet.actor_address,
        &context.condition_id,
        payload.trade.outcome_index as u32,
        &usdc_amount.to_string(),
    )
    .await
    .map_err(|error| map_trade_chain_error("failed to buy market outcome", error))?;
    let persisted_tx_hash = normalize_persisted_tx_hash(&tx.tx_hash);
    order_crud::insert_user_market_trade(
        &state.db,
        &NewUserMarketTradeRecord {
            user_id: authenticated_user.user_id,
            market_id: context.market.id,
            event_id: context.event.id,
            wallet_address: signing_wallet.actor_address.clone(),
            execution_source: "market_trade".to_owned(),
            action: "buy".to_owned(),
            outcome_index: payload.trade.outcome_index,
            price_bps: i32::try_from(price_bps)
                .map_err(|error| AuthError::internal("invalid trade price", error))?,
            token_amount: token_amount.to_string(),
            usdc_amount: usdc_amount.to_string(),
            tx_hash: persisted_tx_hash,
        },
    )
    .await?;

    Ok(MarketTradeExecutionResponse {
        event: EventResponse::from(&context.event),
        on_chain: EventOnChainResponse::from(&context.event),
        market: MarketResponse::from(&context.market),
        wallet_address: signing_wallet.wallet_address,
        account_kind: signing_wallet.account_kind,
        action: "buy".to_owned(),
        outcome_index: payload.trade.outcome_index,
        outcome_label,
        execution_mode: "smart_account".to_owned(),
        execution_status: "submitted".to_owned(),
        tx_hash: Some(tx.tx_hash),
        prepared_transactions: None,
        usdc_amount: format_amount(&usdc_amount),
        token_amount: format_amount(&token_amount),
        price_bps,
        price: bps_to_price(price_bps),
        market_quote: get_market_quote(state, market_id).await?,
        requested_at: chrono::Utc::now(),
    })
}

pub async fn sell_market_outcome(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: SellMarketRequest,
) -> Result<MarketTradeExecutionResponse, AuthError> {
    let token_amount = parse_trade_amount(&payload.trade.token_amount, "trade.token_amount")?;

    let context = load_trading_market_context(state, market_id).await?;
    let signing_wallet = load_signing_wallet_context(state, authenticated_user.user_id).await?;
    let prices = load_market_prices(state, &context.condition_id).await?;
    let price_bps = price_bps_for_outcome(payload.trade.outcome_index, prices.yes_bps, prices.no_bps)?;
    let usdc_amount = quote_usdc_amount(token_amount, price_bps);
    validate_trade_value_bounds(usdc_amount, "trade.token_amount")?;
    let outcome_label = outcome_label(&context.market, payload.trade.outcome_index)?;
    let tx = stellar::sell_market_outcome(
        &state.env,
        &signing_wallet.source_account,
        &signing_wallet.actor_address,
        &context.condition_id,
        payload.trade.outcome_index as u32,
        &token_amount.to_string(),
    )
    .await
    .map_err(|error| map_trade_chain_error("failed to sell market outcome", error))?;
    let persisted_tx_hash = normalize_persisted_tx_hash(&tx.tx_hash);
    order_crud::insert_user_market_trade(
        &state.db,
        &NewUserMarketTradeRecord {
            user_id: authenticated_user.user_id,
            market_id: context.market.id,
            event_id: context.event.id,
            wallet_address: signing_wallet.actor_address.clone(),
            execution_source: "market_trade".to_owned(),
            action: "sell".to_owned(),
            outcome_index: payload.trade.outcome_index,
            price_bps: i32::try_from(price_bps)
                .map_err(|error| AuthError::internal("invalid trade price", error))?,
            token_amount: token_amount.to_string(),
            usdc_amount: usdc_amount.to_string(),
            tx_hash: persisted_tx_hash,
        },
    )
    .await?;

    Ok(MarketTradeExecutionResponse {
        event: EventResponse::from(&context.event),
        on_chain: EventOnChainResponse::from(&context.event),
        market: MarketResponse::from(&context.market),
        wallet_address: signing_wallet.wallet_address,
        account_kind: signing_wallet.account_kind,
        action: "sell".to_owned(),
        outcome_index: payload.trade.outcome_index,
        outcome_label,
        execution_mode: "smart_account".to_owned(),
        execution_status: "submitted".to_owned(),
        tx_hash: Some(tx.tx_hash),
        prepared_transactions: None,
        usdc_amount: format_amount(&usdc_amount),
        token_amount: format_amount(&token_amount),
        price_bps,
        price: bps_to_price(price_bps),
        market_quote: get_market_quote(state, market_id).await?,
        requested_at: chrono::Utc::now(),
    })
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
    validate_trade_value_bounds(collateral_amount, "conversion.collateral_amount")?;

    let context = load_trading_market_context(state, market_id).await?;
    let signing_wallet = load_signing_wallet_context(state, authenticated_user.user_id).await?;
    let collateral_amount_arg = collateral_amount.to_string();
    stellar::ensure_mock_usdc_balance(
        &state.env,
        &signing_wallet.actor_address,
        &collateral_amount_arg,
    )
    .await
    .map_err(|error| map_trade_chain_error("failed to fund split collateral", error))?;
    let tx = stellar::split_market_position(
        &state.env,
        &signing_wallet.source_account,
        &signing_wallet.actor_address,
        &context.condition_id,
        &collateral_amount_arg,
    )
    .await
    .map_err(|error| map_trade_chain_error("failed to split market collateral", error))?;

    Ok(MarketPositionConversionResponse {
        event: EventResponse::from(&context.event),
        on_chain: EventOnChainResponse::from(&context.event),
        market: MarketResponse::from(&context.market),
        wallet_address: signing_wallet.wallet_address,
        account_kind: signing_wallet.account_kind,
        action: "split".to_owned(),
        execution_mode: "smart_account".to_owned(),
        execution_status: "submitted".to_owned(),
        tx_hash: Some(tx.tx_hash),
        prepared_transactions: None,
        collateral_amount: format_amount(&collateral_amount),
        token_amount: format_amount(&collateral_amount),
        requested_at: chrono::Utc::now(),
    })
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

    let context = load_trading_market_context(state, market_id).await?;
    let signing_wallet = load_signing_wallet_context(state, authenticated_user.user_id).await?;
    let pair_token_amount_arg = pair_token_amount.to_string();
    let tx = stellar::merge_market_positions(
        &state.env,
        &signing_wallet.source_account,
        &signing_wallet.actor_address,
        &context.condition_id,
        &pair_token_amount_arg,
    )
    .await
    .map_err(|error| map_trade_chain_error("failed to merge market positions", error))?;

    Ok(MarketPositionConversionResponse {
        event: EventResponse::from(&context.event),
        on_chain: EventOnChainResponse::from(&context.event),
        market: MarketResponse::from(&context.market),
        wallet_address: signing_wallet.wallet_address,
        account_kind: signing_wallet.account_kind,
        action: "merge".to_owned(),
        execution_mode: "smart_account".to_owned(),
        execution_status: "submitted".to_owned(),
        tx_hash: Some(tx.tx_hash),
        prepared_transactions: None,
        collateral_amount: format_amount(&pair_token_amount),
        token_amount: format_amount(&pair_token_amount),
        requested_at: chrono::Utc::now(),
    })
}

async fn load_signing_wallet_context(
    state: &AppState,
    user_id: Uuid,
) -> Result<SigningWalletContext, AuthError> {
    let wallet = auth_crud::get_wallet_for_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("wallet not linked to user"))?;

    if wallet.account_kind != ACCOUNT_KIND_STELLAR_SMART_WALLET {
        return Err(AuthError::unprocessable_entity(
            "market trading currently requires a managed smart wallet",
        ));
    }

    let _contract_wallet_address = wallet
        .wallet_address
        .ok_or_else(|| AuthError::forbidden("wallet is not deployed"))?;
    let owner_address = wallet
        .owner_address
        .ok_or_else(|| AuthError::forbidden("wallet owner metadata is missing"))?;
    let encrypted_private_key = wallet
        .owner_encrypted_private_key
        .ok_or_else(|| AuthError::forbidden("wallet owner key is missing"))?;
    let encryption_nonce = wallet
        .owner_encryption_nonce
        .ok_or_else(|| AuthError::forbidden("wallet owner nonce is missing"))?;

    let decrypted = decrypt_private_key(&state.env, &encrypted_private_key, &encryption_nonce)
        .map_err(|error| AuthError::internal("failed to decrypt managed wallet owner key", error))?;
    let secret_seed_bytes: [u8; 32] = decrypted
        .as_slice()
        .try_into()
        .map_err(|_| AuthError::internal("invalid managed wallet owner key length", "expected 32 bytes"))?;
    let source_account = encode_stellar_secret_key(&secret_seed_bytes);

    Ok(SigningWalletContext {
        wallet_address: owner_address.clone(),
        account_kind: "smart_account".to_owned(),
        actor_address: owner_address,
        source_account,
    })
}

async fn ensure_buy_liquidity_available(
    state: &AppState,
    condition_id: &str,
    outcome_index: i32,
) -> Result<(), AuthError> {
    let liquidity = stellar::get_market_liquidity(&state.env, condition_id)
        .await
        .map_err(|error| AuthError::internal("market liquidity read failed", error))?;
    let available = match outcome_index {
        0 => liquidity.yes_available.as_str(),
        1 => liquidity.no_available.as_str(),
        _ => return Err(AuthError::bad_request("outcome_index is out of range")),
    };

    if available.trim() == "0" {
        return Err(AuthError::bad_request(
            "market currently has no available on-chain liquidity for the selected outcome",
        ));
    }

    Ok(())
}

fn map_trade_chain_error(context: &'static str, error: anyhow::Error) -> AuthError {
    let message = format!("{error:#}");
    let lower = message.to_ascii_lowercase();

    if lower.contains("account not found") {
        return AuthError::bad_request(
            "smart-account owner is not funded on-chain yet; reconnect the wallet and try again",
        );
    }

    if lower.contains("insufficient")
        || lower.contains("allowance")
        || lower.contains("balance")
        || lower.contains("invalidaction")
        || lower.contains("simulation failed")
        || lower.contains("invalid")
    {
        return AuthError::bad_request(message);
    }

    AuthError::internal(context, error)
}

fn normalize_persisted_tx_hash(tx_hash: &str) -> Option<String> {
    if tx_hash == "soroban-rpc-submitted" {
        None
    } else {
        Some(tx_hash.to_owned())
    }
}

async fn load_market_prices(
    state: &AppState,
    condition_id: &str,
) -> Result<stellar::MarketPricesReadResult, AuthError> {
    let prices = stellar::get_market_prices_batch_best_effort(&state.env, &[condition_id.to_owned()])
        .await
        .map_err(|error| AuthError::internal("failed to load on-chain market prices", error))?;

    prices
        .into_iter()
        .next()
        .map(|(_, prices)| prices)
        .ok_or_else(|| AuthError::not_found("market quote unavailable"))
}

fn price_bps_for_outcome(
    outcome_index: i32,
    yes_bps: u32,
    no_bps: u32,
) -> Result<u32, AuthError> {
    match outcome_index {
        0 => Ok(yes_bps),
        1 => Ok(no_bps),
        _ => Err(AuthError::bad_request("trade.outcome_index must be 0 or 1")),
    }
}
