use axum::{
    Router,
    routing::{get, post},
};

use crate::{
    app::AppState,
    module::faucet::controller::{faucet_usdc, mock_usdc_balance},
};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/faucet/usdc", post(faucet_usdc))
        .route("/faucet/usdc/balance", get(mock_usdc_balance))
}
