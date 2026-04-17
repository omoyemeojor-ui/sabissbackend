use axum::{
    Json,
    extract::{Extension, Path, State},
};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{auth::error::AuthError, liquidity::schema::*},
    service::{
        jwt::AuthenticatedUser,
        liquidity::{
            deposit_market_collateral, deposit_market_inventory, get_event_liquidity,
            get_my_event_liquidity, get_my_market_liquidity, remove_market_liquidity,
            withdraw_market_collateral, withdraw_market_inventory,
        },
    },
};

pub async fn event_liquidity(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventLiquidityResponse>, AuthError> {
    Ok(Json(get_event_liquidity(&state, event_id).await?))
}

pub async fn my_market_liquidity(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MyMarketLiquidityResponse>, AuthError> {
    Ok(Json(
        get_my_market_liquidity(&state, authenticated_user, market_id).await?,
    ))
}

pub async fn my_event_liquidity(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<MyEventLiquidityResponse>, AuthError> {
    Ok(Json(
        get_my_event_liquidity(&state, authenticated_user, event_id).await?,
    ))
}

pub async fn deposit_inventory(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<DepositInventoryRequest>,
) -> Result<Json<LiquidityWriteResponse>, AuthError> {
    Ok(Json(
        deposit_market_inventory(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn deposit_collateral(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<DepositCollateralRequest>,
) -> Result<Json<LiquidityWriteResponse>, AuthError> {
    Ok(Json(
        deposit_market_collateral(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn remove_liquidity(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<RemoveLiquidityRequest>,
) -> Result<Json<LiquidityWriteResponse>, AuthError> {
    Ok(Json(
        remove_market_liquidity(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn withdraw_inventory(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<WithdrawInventoryRequest>,
) -> Result<Json<LiquidityWriteResponse>, AuthError> {
    Ok(Json(
        withdraw_market_inventory(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn withdraw_collateral(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<WithdrawCollateralRequest>,
) -> Result<Json<LiquidityWriteResponse>, AuthError> {
    Ok(Json(
        withdraw_market_collateral(&state, authenticated_user, market_id, payload).await?,
    ))
}
