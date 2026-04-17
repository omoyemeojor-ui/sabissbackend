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

#[derive(Debug, Deserialize, Serialize)]
pub struct SmartWalletRegistrationRequest {
    pub wallet_address: String,
    pub wallet_standard: Option<String>,
    pub relayer_url: Option<String>,
    pub web_auth_domain: Option<String>,
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
    pub wallet_address: Option<String>,
    pub network: String,
    pub account_kind: String,
    pub wallet_status: String,
    pub wallet_standard: Option<String>,
    pub owner_provider: Option<String>,
    pub sponsor_address: Option<String>,
    pub relayer_kind: Option<String>,
    pub relayer_url: Option<String>,
    pub web_auth_contract_id: Option<String>,
    pub web_auth_domain: Option<String>,
    pub deployed_at: Option<DateTime<Utc>>,
    pub last_authenticated_at: Option<DateTime<Utc>>,
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
        Self {
            wallet_address: value.wallet_address,
            network: value.network,
            account_kind: value.account_kind,
            wallet_status: value.wallet_status,
            wallet_standard: value.wallet_standard,
            owner_provider: value.owner_provider,
            sponsor_address: value.sponsor_address,
            relayer_kind: value.relayer_kind,
            relayer_url: value.relayer_url,
            web_auth_contract_id: value.web_auth_contract_id,
            web_auth_domain: value.web_auth_domain,
            deployed_at: value.deployed_at,
            last_authenticated_at: value.last_authenticated_at,
            created_at: value.created_at,
        }
    }
}
