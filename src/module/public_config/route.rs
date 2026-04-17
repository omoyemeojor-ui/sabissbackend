use axum::{Router, routing::get};

use crate::{app::AppState, module::public_config::controller::contracts_config};

pub fn public_router() -> Router<AppState> {
    Router::new().route("/config/contracts", get(contracts_config))
}
