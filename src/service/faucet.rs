use anyhow::Result;
use chrono::Utc;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        faucet::schema::{
            FaucetUsdcBalanceQuery, FaucetUsdcBalanceResponse, FaucetUsdcRequest,
            FaucetUsdcResponse,
        },
    },
    service::{auth::normalize_stellar_address, stellar},
};

pub async fn request_usdc_faucet(
    state: &AppState,
    payload: FaucetUsdcRequest,
) -> Result<FaucetUsdcResponse, AuthError> {
    let recipient = normalize_stellar_address(&payload.address)?;
    let amount = parse_amount(&payload.amount)?;
    let tx_hash = stellar::mint_mock_usdc(&state.env, &recipient, &amount)
        .await
        .map_err(|error| map_faucet_error("usdc faucet mint failed", error))?
        .tx_hash;

    Ok(FaucetUsdcResponse {
        token_address: state.env.mock_usdc_id.clone(),
        recipient,
        amount,
        tx_hash,
        requested_at: Utc::now(),
    })
}

pub async fn get_mock_usdc_balance(
    state: &AppState,
    query: FaucetUsdcBalanceQuery,
) -> Result<FaucetUsdcBalanceResponse, AuthError> {
    let address = normalize_stellar_address(&query.address)?;
    let balance = read_usdc_balance(state, &address).await?;

    Ok(FaucetUsdcBalanceResponse {
        token_address: state.env.mock_usdc_id.clone(),
        address,
        balance,
        queried_at: Utc::now(),
    })
}

pub async fn read_usdc_balance(state: &AppState, address: &str) -> Result<String, AuthError> {
    stellar::get_mock_usdc_balance(&state.env, &normalize_stellar_address(address)?)
        .await
        .map_err(|error| AuthError::internal("usdc balance query failed", error))
}

fn parse_amount(raw: &str) -> Result<String, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request("amount is required"));
    }

    let amount = value
        .parse::<u128>()
        .map_err(|_| AuthError::bad_request("amount must be a base-10 integer string"))?;
    if amount == 0 {
        return Err(AuthError::bad_request("amount must be greater than zero"));
    }

    Ok(amount.to_string())
}

fn map_faucet_error(context: &'static str, error: anyhow::Error) -> AuthError {
    let message = error.to_string();
    if message.contains("mint")
        || message.contains("faucet transaction reverted:")
        || message.contains("invalid faucet recipient")
    {
        return AuthError::bad_request(message);
    }

    AuthError::internal(context, error)
}
