use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::schema::{ConfigureMarketAutoResolveRequest, MarketAutoResolveConfigResponse},
    },
};

pub async fn configure_market_auto_resolve_coinbase(
    _state: &AppState,
    _market_id: Uuid,
    _payload: ConfigureMarketAutoResolveRequest,
) -> Result<MarketAutoResolveConfigResponse, AuthError> {
    Err(AuthError::unprocessable_entity(
        "market auto-resolution is not implemented in this Soroban backend yet",
    ))
}
