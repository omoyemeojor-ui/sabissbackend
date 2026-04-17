use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct UserWalletAccountRecord {
    pub user_id: Uuid,
    pub wallet_address: String,
    pub network: String,
    pub created_at: DateTime<Utc>,
}
