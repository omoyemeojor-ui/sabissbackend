use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};

use crate::{
    app::AppState,
    middleware::user::require_auth,
    module::liquidity::controller::{
        deposit_collateral, deposit_inventory, event_liquidity, my_event_liquidity,
        my_market_liquidity, remove_liquidity, withdraw_collateral, withdraw_inventory,
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new().route("/events/{event_id}/liquidity", get(event_liquidity))
}

pub fn me_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/me/liquidity/markets/{market_id}", get(my_market_liquidity))
        .route("/me/liquidity/events/{event_id}", get(my_event_liquidity))
        .route(
            "/me/liquidity/markets/{market_id}/deposit-inventory",
            post(deposit_inventory),
        )
        .route(
            "/me/liquidity/markets/{market_id}/deposit-collateral",
            post(deposit_collateral),
        )
        .route("/me/liquidity/markets/{market_id}/remove", post(remove_liquidity))
        .route(
            "/me/liquidity/markets/{market_id}/withdraw-inventory",
            post(withdraw_inventory),
        )
        .route(
            "/me/liquidity/markets/{market_id}/withdraw-collateral",
            post(withdraw_collateral),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, require_auth))
}
