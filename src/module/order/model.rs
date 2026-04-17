use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct MarketOrderRecord {
    pub id: Uuid,
    pub user_id: Uuid,
    pub market_id: Uuid,
    pub event_id: Uuid,
    pub wallet_address: String,
    pub account_kind: String,
    pub condition_id: String,
    pub outcome_index: i32,
    pub side: String,
    pub price_bps: i32,
    pub amount: String,
    pub filled_amount: String,
    pub remaining_amount: String,
    pub expiry_epoch_seconds: Option<i64>,
    pub salt: String,
    pub signature: String,
    pub order_hash: String,
    pub order_digest: String,
    pub status: String,
    pub cancelled_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketOrderFillRecord {
    pub id: Uuid,
    pub market_id: Uuid,
    pub event_id: Uuid,
    pub condition_id: String,
    pub match_type: String,
    pub buy_order_id: Option<Uuid>,
    pub sell_order_id: Option<Uuid>,
    pub yes_order_id: Option<Uuid>,
    pub no_order_id: Option<Uuid>,
    pub outcome_index: Option<i32>,
    pub fill_amount: String,
    pub collateral_amount: String,
    pub yes_price_bps: i32,
    pub no_price_bps: i32,
    pub tx_hash: String,
    pub created_at: DateTime<Utc>,
}
