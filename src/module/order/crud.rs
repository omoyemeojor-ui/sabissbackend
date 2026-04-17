use uuid::Uuid;

use crate::{config::db::DbPool, module::auth::error::AuthError};

use super::model::{MarketOrderFillRecord, MarketOrderRecord};

pub async fn list_active_market_orders_by_market_id(
    _db: &DbPool,
    _market_id: Uuid,
) -> Result<Vec<MarketOrderRecord>, AuthError> {
    Ok(Vec::new())
}

pub async fn list_market_order_fills_by_market_id(
    _db: &DbPool,
    _market_id: Uuid,
    _limit: i64,
) -> Result<Vec<MarketOrderFillRecord>, AuthError> {
    Ok(Vec::new())
}
