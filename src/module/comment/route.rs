use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};

use crate::{
    app::AppState,
    middleware::user::require_auth,
    module::comment::controller::{
        create_market_comment, create_market_comment_reply, like_comment, market_comments,
        unlike_comment,
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new().route("/markets/{market_id}/comments", get(market_comments))
}

pub fn me_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/markets/{market_id}/comments", post(create_market_comment))
        .route(
            "/markets/{market_id}/comments/{comment_id}/replies",
            post(create_market_comment_reply),
        )
        .route(
            "/comments/{comment_id}/likes",
            post(like_comment).delete(unlike_comment),
        )
        .route_layer(axum_middleware::from_fn_with_state(state, require_auth))
}
