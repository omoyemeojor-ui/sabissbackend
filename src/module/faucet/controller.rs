use axum::{
    Json,
    extract::{Query, State},
};

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        faucet::schema::{
            FaucetUsdcBalanceQuery, FaucetUsdcBalanceResponse, FaucetUsdcRequest,
            FaucetUsdcResponse,
        },
    },
    service::faucet::{get_mock_usdc_balance, request_usdc_faucet},
};

pub async fn faucet_usdc(
    State(state): State<AppState>,
    Json(payload): Json<FaucetUsdcRequest>,
) -> Result<Json<FaucetUsdcResponse>, AuthError> {
    Ok(Json(request_usdc_faucet(&state, payload).await?))
}

pub async fn mock_usdc_balance(
    State(state): State<AppState>,
    Query(query): Query<FaucetUsdcBalanceQuery>,
) -> Result<Json<FaucetUsdcBalanceResponse>, AuthError> {
    Ok(Json(get_mock_usdc_balance(&state, query).await?))
}
