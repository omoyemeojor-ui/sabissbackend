use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};

use crate::{
    app::AppState,
    middleware::admin::require_admin,
    module::{
        admin::controller::{me, wallet_challenge, wallet_connect},
        market,
    },
};

pub fn router(state: AppState) -> Router<AppState> {
    let protected_routes =
        Router::new()
            .route("/me", get(me))
            .merge(market::route::admin_router())
            .route_layer(axum_middleware::from_fn_with_state(
                state.clone(),
                require_admin,
            ));

    Router::new()
        .route("/auth/wallet/challenge", post(wallet_challenge))
        .route("/auth/wallet/connect", post(wallet_connect))
        .merge(protected_routes)
}
