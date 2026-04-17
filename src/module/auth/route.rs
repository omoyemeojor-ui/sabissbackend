use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};

use crate::{
    app::AppState,
    middleware::user::require_auth,
    module::auth::controller::{
        google_sign_in, me, smart_wallet_register, wallet_challenge, wallet_connect,
    },
};

pub fn router(state: AppState) -> Router<AppState> {
    let protected_routes = Router::new()
        .route("/me", get(me))
        .route("/smart-wallet/register", post(smart_wallet_register))
        .route_layer(axum_middleware::from_fn_with_state(state, require_auth));

    Router::new()
        .route("/google/sign-in", post(google_sign_in))
        .route("/wallet/challenge", post(wallet_challenge))
        .route("/wallet/connect", post(wallet_connect))
        .merge(protected_routes)
}
