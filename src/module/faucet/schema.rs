use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Deserialize)]
pub struct FaucetUsdcRequest {
    pub address: String,
    pub amount: String,
}

#[derive(Debug, Deserialize)]
pub struct FaucetUsdcBalanceQuery {
    pub address: String,
}

#[derive(Debug, Serialize)]
pub struct FaucetUsdcResponse {
    pub token_address: String,
    pub recipient: String,
    pub amount: String,
    pub tx_hash: String,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct FaucetUsdcBalanceResponse {
    pub token_address: String,
    pub address: String,
    pub balance: String,
    pub queried_at: DateTime<Utc>,
}
