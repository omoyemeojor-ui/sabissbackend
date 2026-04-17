use axum::{
    Router, middleware as axum_middleware,
    routing::{get, post},
};

use crate::{
    app::AppState,
    middleware::user::require_auth,
    module::order::controller::{
        admin_fill_complementary_buy_orders, admin_fill_complementary_sell_orders,
        admin_fill_direct_orders, admin_match_orders, cancel_order, create_order, my_orders,
        my_portfolio, my_positions,
    },
};

pub fn me_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/orders", post(create_order))
        .route("/orders/cancel", post(cancel_order))
        .route("/me/orders", get(my_orders))
        .route("/me/positions", get(my_positions))
        .route("/me/portfolio", get(my_portfolio))
        .route_layer(axum_middleware::from_fn_with_state(state, require_auth))
}

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route("/orders/match", post(admin_match_orders))
        .route("/orders/fill/direct", post(admin_fill_direct_orders))
        .route(
            "/orders/fill/complementary-buy",
            post(admin_fill_complementary_buy_orders),
        )
        .route(
            "/orders/fill/complementary-sell",
            post(admin_fill_complementary_sell_orders),
        )
}
