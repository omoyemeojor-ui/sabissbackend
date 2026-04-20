use chrono::Utc;
use ethers_core::types::U256;
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{auth::error::AuthError, liquidity::schema::*},
    service::{
        auth::normalize_wallet_address,
        jwt::AuthenticatedUser,
        liquidity::{
            chain_read, chain_write,
            format::{liquidity_position_response, liquidity_totals_response, parse_amount},
            view::build_market_response,
            wallet::load_smart_account_context,
        },
    },
};

use super::context::load_public_market_context;

enum LiquidityWriteAction {
    DepositInventory {
        yes_amount: U256,
        no_amount: U256,
    },
    DepositCollateral {
        amount: U256,
    },
    Remove {
        yes_amount: U256,
        no_amount: U256,
    },
    WithdrawInventory {
        yes_amount: U256,
        no_amount: U256,
        recipient: String,
    },
    WithdrawCollateral {
        amount: U256,
        recipient: String,
    },
}

impl LiquidityWriteAction {
    fn name(&self) -> &'static str {
        match self {
            Self::DepositInventory { .. } => "deposit_inventory",
            Self::DepositCollateral { .. } => "deposit_collateral",
            Self::Remove { .. } => "remove",
            Self::WithdrawInventory { .. } => "withdraw_inventory",
            Self::WithdrawCollateral { .. } => "withdraw_collateral",
        }
    }
}

pub async fn deposit_market_inventory(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: DepositInventoryRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let yes_amount = parse_amount(&payload.liquidity.yes_amount, "liquidity.yes_amount", true)?;
    let no_amount = parse_amount(&payload.liquidity.no_amount, "liquidity.no_amount", true)?;
    ensure_inventory_pair_non_zero(yes_amount, no_amount)?;
    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::DepositInventory {
            yes_amount,
            no_amount,
        },
    )
    .await
}

pub async fn deposit_market_collateral(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: DepositCollateralRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let amount = parse_amount(&payload.liquidity.amount, "liquidity.amount", false)?;
    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::DepositCollateral { amount },
    )
    .await
}

pub async fn remove_market_liquidity(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: RemoveLiquidityRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let yes_amount = parse_amount(&payload.liquidity.yes_amount, "liquidity.yes_amount", true)?;
    let no_amount = parse_amount(&payload.liquidity.no_amount, "liquidity.no_amount", true)?;
    ensure_inventory_pair_non_zero(yes_amount, no_amount)?;
    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::Remove {
            yes_amount,
            no_amount,
        },
    )
    .await
}

pub async fn withdraw_market_inventory(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: WithdrawInventoryRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let yes_amount = parse_amount(&payload.liquidity.yes_amount, "liquidity.yes_amount", true)?;
    let no_amount = parse_amount(&payload.liquidity.no_amount, "liquidity.no_amount", true)?;
    ensure_inventory_pair_non_zero(yes_amount, no_amount)?;
    let wallet = load_smart_account_context(state, authenticated_user.user_id).await?;
    let recipient = resolve_recipient(
        payload.liquidity.recipient.as_deref(),
        &wallet.wallet_address,
    )?;
    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::WithdrawInventory {
            yes_amount,
            no_amount,
            recipient,
        },
    )
    .await
}

pub async fn withdraw_market_collateral(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: WithdrawCollateralRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let amount = parse_amount(&payload.liquidity.amount, "liquidity.amount", false)?;
    let wallet = load_smart_account_context(state, authenticated_user.user_id).await?;
    let recipient = resolve_recipient(
        payload.liquidity.recipient.as_deref(),
        &wallet.wallet_address,
    )?;
    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::WithdrawCollateral { amount, recipient },
    )
    .await
}

async fn execute_market_write(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    action: LiquidityWriteAction,
) -> Result<LiquidityWriteResponse, AuthError> {
    let context = load_public_market_context(state, market_id).await?;
    let wallet = load_smart_account_context(state, authenticated_user.user_id).await?;
    let condition_id = context
        .market
        .condition_id
        .as_deref()
        .ok_or_else(|| AuthError::bad_request("market is not published on-chain"))?;
    let tx = match &action {
        LiquidityWriteAction::DepositInventory {
            yes_amount,
            no_amount,
        } => {
            chain_write::deposit_inventory(
                state,
                authenticated_user.user_id,
                condition_id,
                *yes_amount,
                *no_amount,
            )
            .await
        }
        LiquidityWriteAction::DepositCollateral { amount } => {
            chain_write::deposit_collateral(
                state,
                authenticated_user.user_id,
                condition_id,
                *amount,
            )
            .await
        }
        LiquidityWriteAction::Remove {
            yes_amount,
            no_amount,
        } => {
            chain_write::remove_liquidity(
                state,
                authenticated_user.user_id,
                condition_id,
                *yes_amount,
                *no_amount,
            )
            .await
        }
        LiquidityWriteAction::WithdrawInventory {
            yes_amount,
            no_amount,
            recipient,
        } => {
            chain_write::withdraw_inventory(
                state,
                authenticated_user.user_id,
                condition_id,
                *yes_amount,
                *no_amount,
                recipient,
            )
            .await
        }
        LiquidityWriteAction::WithdrawCollateral { amount, recipient } => {
            chain_write::withdraw_collateral(
                state,
                authenticated_user.user_id,
                condition_id,
                *amount,
                recipient,
            )
            .await
        }
    }
    .map_err(|error| map_liquidity_chain_error(action.name(), error))?;

    let position =
        chain_read::get_liquidity_position(&state.env, condition_id, &wallet.wallet_address)
            .await
            .map_err(|error| AuthError::internal("market position read failed", error))?;
    let market_liquidity = chain_read::get_market_liquidity(&state.env, condition_id)
        .await
        .map_err(|error| AuthError::internal("market liquidity read failed", error))?;

    Ok(LiquidityWriteResponse {
        event: (&context.event).into(),
        on_chain: (&context.event).into(),
        market: build_market_response(state, &context.market).await?,
        wallet_address: wallet.wallet_address,
        action: action.name().to_owned(),
        tx_hash: tx.tx_hash,
        position: liquidity_position_response(&position),
        market_liquidity: liquidity_totals_response(&market_liquidity),
        updated_at: Utc::now(),
    })
}

fn ensure_inventory_pair_non_zero(yes_amount: U256, no_amount: U256) -> Result<(), AuthError> {
    if yes_amount.is_zero() && no_amount.is_zero() {
        return Err(AuthError::bad_request(
            "at least one liquidity amount must be greater than zero",
        ));
    }

    Ok(())
}

fn resolve_recipient(raw: Option<&str>, default_wallet: &str) -> Result<String, AuthError> {
    match raw {
        Some(value) => normalize_wallet_address(value),
        None => Ok(default_wallet.to_owned()),
    }
}

fn map_liquidity_chain_error(context: &'static str, error: anyhow::Error) -> AuthError {
    let message = error.to_string();
    if message.contains("MARKET_MAKER_ROLE") {
        return AuthError::forbidden(message);
    }
    if message.contains("SecurityGuards:")
        || message.contains("Insufficient")
        || message.contains("No active position")
        || message.contains("transaction reverted:")
    {
        return AuthError::bad_request(message);
    }

    AuthError::internal(context, error)
}
