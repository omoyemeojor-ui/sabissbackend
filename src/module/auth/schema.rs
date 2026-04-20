use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::module::auth::model::{UserRecord, WalletRecord};

#[derive(Debug, Deserialize)]
pub struct GoogleSignInRequest {
    pub credential: String,
    pub g_csrf_token: Option<String>,
    pub client_id: Option<String>,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WalletChallengeRequest {
    pub wallet_address: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct WalletConnectRequest {
    pub challenge_id: Uuid,
    pub signature: String,
    pub username: Option<String>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct AuthResponse {
    pub token: String,
    pub user: UserResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletChallengeResponse {
    pub challenge_id: Uuid,
    pub message: String,
    pub expires_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct MeResponse {
    pub user: UserResponse,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct UserResponse {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub wallet: Option<WalletResponse>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct WalletResponse {
    pub wallet_address: String,
    pub chain_id: i64,
    pub account_kind: String,
    pub owner_address: Option<String>,
    pub owner_provider: Option<String>,
    pub factory_address: Option<String>,
    pub entry_point_address: Option<String>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

impl UserResponse {
    pub fn from_parts(user: UserRecord, wallet: Option<WalletRecord>) -> Self {
        Self {
            id: user.id,
            email: user.email,
            username: user.username,
            display_name: user.display_name,
            avatar_url: user.avatar_url,
            wallet: wallet.map(WalletResponse::from),
            created_at: user.created_at,
            updated_at: user.updated_at,
        }
    }
}

impl From<WalletRecord> for WalletResponse {
    fn from(value: WalletRecord) -> Self {
        let wallet_address = frontend_wallet_address(&value);
        Self {
            wallet_address,
            chain_id: wallet_chain_id(&value.network),
            account_kind: frontend_account_kind(&value.account_kind),
            owner_address: value.owner_address,
            owner_provider: value.owner_provider,
            factory_address: value.factory_contract_id,
            entry_point_address: None,
            created_at: value.created_at,
        }
    }
}

fn wallet_chain_id(network: &str) -> i64 {
    match network {
        "testnet" => 10143,
        "mainnet" => 1,
        _ => 10143,
    }
}

fn frontend_account_kind(account_kind: &str) -> String {
    match account_kind {
        "stellar_smart_wallet" => "smart_account".to_owned(),
        "classic_account" => "external_eoa".to_owned(),
        other => other.to_owned(),
    }
}

fn frontend_wallet_address(wallet: &WalletRecord) -> String {
    if wallet.account_kind == "stellar_smart_wallet" {
        return wallet
            .owner_address
            .clone()
            .or_else(|| wallet.wallet_address.clone())
            .unwrap_or_default();
    }

    wallet.wallet_address.clone().unwrap_or_default()
}
