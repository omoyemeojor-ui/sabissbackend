use chrono::Utc;

use crate::{
    app::AppState,
    module::{auth::error::AuthError, order::schema::*},
};

pub async fn fill_direct_orders(
    _state: &AppState,
    _payload: AdminFillDirectOrdersRequest,
) -> Result<AdminOrderFillResponse, AuthError> {
    Err(AuthError::unprocessable_entity(
        "Soroban admin order settlement is not implemented yet",
    ))
}

pub async fn fill_complementary_buy_orders(
    _state: &AppState,
    _payload: AdminFillComplementaryBuyOrdersRequest,
) -> Result<AdminOrderFillResponse, AuthError> {
    Err(AuthError::unprocessable_entity(
        "Soroban complementary buy settlement is not implemented yet",
    ))
}

pub async fn fill_complementary_sell_orders(
    _state: &AppState,
    _payload: AdminFillComplementarySellOrdersRequest,
) -> Result<AdminOrderFillResponse, AuthError> {
    Err(AuthError::unprocessable_entity(
        "Soroban complementary sell settlement is not implemented yet",
    ))
}

pub async fn match_orders(
    _state: &AppState,
    payload: AdminMatchOrdersRequest,
) -> Result<AdminMatchOrdersResponse, AuthError> {
    let max_fills_per_market = payload.matching.max_fills_per_market.unwrap_or(16);
    Ok(AdminMatchOrdersResponse {
        scope: if payload.matching.market_id.is_some() {
            "market".to_owned()
        } else {
            "all_markets".to_owned()
        },
        market_id: payload.matching.market_id,
        markets_scanned: usize::from(payload.matching.market_id.is_some()),
        markets_matched: 0,
        executed_fills: 0,
        max_fills_per_market,
        markets: Vec::new(),
        completed_at: Utc::now(),
    })
}
