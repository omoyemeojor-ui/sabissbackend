use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;

pub const ACCOUNT_KIND_CLASSIC_ACCOUNT: &str = "classic_account";
pub const ACCOUNT_KIND_STELLAR_SMART_WALLET: &str = "stellar_smart_wallet";
pub const WALLET_STATUS_ACTIVE: &str = "active";
pub const WALLET_STATUS_PENDING_REGISTRATION: &str = "pending_registration";

#[derive(Debug, Clone, FromRow)]
pub struct UserRecord {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct WalletRecord {
    pub wallet_address: Option<String>,
    pub network: String,
    pub account_kind: String,
    pub wallet_status: String,
    pub wallet_standard: Option<String>,
    pub owner_provider: Option<String>,
    pub owner_ref: Option<String>,
    pub sponsor_address: Option<String>,
    pub relayer_kind: Option<String>,
    pub relayer_url: Option<String>,
    pub web_auth_contract_id: Option<String>,
    pub web_auth_domain: Option<String>,
    pub deployed_at: Option<DateTime<Utc>>,
    pub last_authenticated_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct UserProfileRecord {
    pub id: Uuid,
    pub email: Option<String>,
    pub username: Option<String>,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub wallet_address: Option<String>,
    pub wallet_network: Option<String>,
    pub wallet_account_kind: Option<String>,
    pub wallet_status: Option<String>,
    pub wallet_standard: Option<String>,
    pub wallet_owner_provider: Option<String>,
    pub wallet_owner_ref: Option<String>,
    pub wallet_sponsor_address: Option<String>,
    pub wallet_relayer_kind: Option<String>,
    pub wallet_relayer_url: Option<String>,
    pub wallet_web_auth_contract_id: Option<String>,
    pub wallet_web_auth_domain: Option<String>,
    pub wallet_deployed_at: Option<DateTime<Utc>>,
    pub wallet_last_authenticated_at: Option<DateTime<Utc>>,
    pub wallet_created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, FromRow)]
pub struct WalletChallengeRecord {
    pub id: Uuid,
    pub wallet_address: String,
    pub network: String,
    pub nonce: String,
    pub message: String,
    pub expires_at: DateTime<Utc>,
    pub consumed_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone)]
pub struct VerifiedGoogleToken {
    pub google_sub: String,
    pub email: Option<String>,
    pub email_verified: bool,
    pub display_name: Option<String>,
    pub avatar_url: Option<String>,
}

impl UserProfileRecord {
    pub fn into_parts(self) -> (UserRecord, Option<WalletRecord>) {
        let user = UserRecord {
            id: self.id,
            email: self.email,
            username: self.username,
            display_name: self.display_name,
            avatar_url: self.avatar_url,
            created_at: self.created_at,
            updated_at: self.updated_at,
        };

        let wallet = match (
            self.wallet_address,
            self.wallet_network,
            self.wallet_account_kind,
            self.wallet_status,
            self.wallet_created_at,
        ) {
            (wallet_address, Some(network), Some(account_kind), Some(wallet_status), Some(created_at)) => Some(WalletRecord {
                wallet_address,
                network,
                account_kind,
                wallet_status,
                wallet_standard: self.wallet_standard,
                owner_provider: self.wallet_owner_provider,
                owner_ref: self.wallet_owner_ref,
                sponsor_address: self.wallet_sponsor_address,
                relayer_kind: self.wallet_relayer_kind,
                relayer_url: self.wallet_relayer_url,
                web_auth_contract_id: self.wallet_web_auth_contract_id,
                web_auth_domain: self.wallet_web_auth_domain,
                deployed_at: self.wallet_deployed_at,
                last_authenticated_at: self.wallet_last_authenticated_at,
                created_at,
            }),
            _ => None,
        };

        (user, wallet)
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct JwtClaims {
    pub sub: String,
    pub exp: usize,
    pub iat: usize,
    pub email: Option<String>,
}
