use axum::{
    Json,
    extract::{Extension, State},
};

use crate::{
    app::AppState,
    module::{auth::error::AuthError, order::schema::*},
    service::{jwt::AuthenticatedUser, order::*},
};

pub async fn create_order(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateOrderRequest>,
) -> Result<Json<CreateOrderResponse>, AuthError> {
    Ok(Json(
        place_order(&state, authenticated_user, payload).await?,
    ))
}

pub async fn cancel_order(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CancelOrderRequest>,
) -> Result<Json<CancelOrderResponse>, AuthError> {
    Ok(Json(
        cancel_existing_order(&state, authenticated_user, payload).await?,
    ))
}

pub async fn my_orders(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<MyOrdersResponse>, AuthError> {
    Ok(Json(get_my_orders(&state, authenticated_user).await?))
}

pub async fn my_positions(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<MyPositionsResponse>, AuthError> {
    Ok(Json(get_my_positions(&state, authenticated_user).await?))
}

pub async fn my_portfolio(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<MyPortfolioResponse>, AuthError> {
    Ok(Json(get_my_portfolio(&state, authenticated_user).await?))
}

pub async fn admin_fill_direct_orders(
    State(state): State<AppState>,
    Json(payload): Json<AdminFillDirectOrdersRequest>,
) -> Result<Json<AdminOrderFillResponse>, AuthError> {
    Ok(Json(
        crate::service::orderbook::fill_direct_orders(&state, payload).await?,
    ))
}

pub async fn admin_fill_complementary_buy_orders(
    State(state): State<AppState>,
    Json(payload): Json<AdminFillComplementaryBuyOrdersRequest>,
) -> Result<Json<AdminOrderFillResponse>, AuthError> {
    Ok(Json(
        crate::service::orderbook::fill_complementary_buy_orders(&state, payload).await?,
    ))
}

pub async fn admin_fill_complementary_sell_orders(
    State(state): State<AppState>,
    Json(payload): Json<AdminFillComplementarySellOrdersRequest>,
) -> Result<Json<AdminOrderFillResponse>, AuthError> {
    Ok(Json(
        crate::service::orderbook::fill_complementary_sell_orders(&state, payload).await?,
    ))
}

pub async fn admin_match_orders(
    State(state): State<AppState>,
    Json(payload): Json<AdminMatchOrdersRequest>,
) -> Result<Json<AdminMatchOrdersResponse>, AuthError> {
    Ok(Json(
        crate::service::orderbook::match_orders(&state, payload).await?,
    ))
}
