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
    service::jwt::AuthenticatedUser,
};

fn unsupported() -> AuthError {
    AuthError::unprocessable_entity(
        "market trading writes are not implemented in this Soroban backend yet",
    )
}

pub async fn buy_market_outcome(
    _state: &AppState,
    _authenticated_user: AuthenticatedUser,
    _market_id: Uuid,
    _payload: BuyMarketRequest,
) -> Result<MarketTradeExecutionResponse, AuthError> {
    Err(unsupported())
}

pub async fn sell_market_outcome(
    _state: &AppState,
    _authenticated_user: AuthenticatedUser,
    _market_id: Uuid,
    _payload: SellMarketRequest,
) -> Result<MarketTradeExecutionResponse, AuthError> {
    Err(unsupported())
}

pub async fn split_market_collateral(
    _state: &AppState,
    _authenticated_user: AuthenticatedUser,
    _market_id: Uuid,
    _payload: SplitMarketRequest,
) -> Result<MarketPositionConversionResponse, AuthError> {
    Err(unsupported())
}

pub async fn merge_market_positions(
    _state: &AppState,
    _authenticated_user: AuthenticatedUser,
    _market_id: Uuid,
    _payload: MergeMarketRequest,
) -> Result<MarketPositionConversionResponse, AuthError> {
    Err(unsupported())
}
