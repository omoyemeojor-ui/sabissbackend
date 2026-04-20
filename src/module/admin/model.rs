use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

use crate::module::auth::model::{UserRecord, WalletRecord};

#[derive(Debug, Clone)]
pub struct AdminProfile {
    pub user: UserRecord,
    pub wallet: WalletRecord,
}

#[derive(Debug, Clone, FromRow)]
pub struct AdminUploadAssetRecord {
    pub id: Uuid,
    pub storage_provider: String,
    pub bucket_name: String,
    pub scope: String,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub cid: String,
    pub ipfs_url: String,
    pub gateway_url: String,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct NewAdminUploadAssetRecord {
    pub id: Uuid,
    pub storage_provider: String,
    pub bucket_name: String,
    pub scope: String,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub cid: String,
    pub ipfs_url: String,
    pub gateway_url: String,
    pub created_by_user_id: Uuid,
}
