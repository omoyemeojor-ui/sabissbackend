use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::schema::{ConfigureMarketAutoCreateSeriesRequest, MarketAutoCreateSeriesResponse},
    },
    service::jwt::AuthenticatedUser,
};

pub async fn configure_market_auto_create_series(
    _state: &AppState,
    _authenticated_user: AuthenticatedUser,
    _payload: ConfigureMarketAutoCreateSeriesRequest,
) -> Result<MarketAutoCreateSeriesResponse, AuthError> {
    Err(AuthError::unprocessable_entity(
        "market auto-create is not implemented in this Soroban backend yet",
    ))
}
