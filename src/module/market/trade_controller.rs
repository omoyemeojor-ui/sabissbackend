use axum::{
    Json,
    extract::{Extension, Path, State},
};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::trade_schema::{
            BuyMarketRequest, MarketPositionConversionResponse, MarketTradeExecutionResponse,
            MergeMarketRequest, SellMarketRequest, SplitMarketRequest,
        },
    },
    service::{
        jwt::AuthenticatedUser,
        trading::{
            buy_market_outcome, merge_market_positions, sell_market_outcome,
            split_market_collateral,
        },
    },
};

pub async fn market_buy(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<BuyMarketRequest>,
) -> Result<Json<MarketTradeExecutionResponse>, AuthError> {
    Ok(Json(
        buy_market_outcome(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn market_sell(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<SellMarketRequest>,
) -> Result<Json<MarketTradeExecutionResponse>, AuthError> {
    Ok(Json(
        sell_market_outcome(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn market_split(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<SplitMarketRequest>,
) -> Result<Json<MarketPositionConversionResponse>, AuthError> {
    Ok(Json(
        split_market_collateral(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn market_merge(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<MergeMarketRequest>,
) -> Result<Json<MarketPositionConversionResponse>, AuthError> {
    Ok(Json(
        merge_market_positions(&state, authenticated_user, market_id, payload).await?,
    ))
}
