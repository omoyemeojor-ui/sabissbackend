use serde::Serialize;

use crate::module::{
    admin::model::AdminProfile,
    auth::schema::{
        AuthResponse, UserResponse, WalletChallengeRequest, WalletChallengeResponse,
        WalletConnectRequest,
    },
};

pub type AdminWalletChallengeRequest = WalletChallengeRequest;
pub type AdminWalletChallengeResponse = WalletChallengeResponse;
pub type AdminWalletConnectRequest = WalletConnectRequest;
pub type AdminAuthResponse = AuthResponse;

#[derive(Debug, Serialize)]
pub struct AdminMeResponse {
    pub user: UserResponse,
    pub chain_id: i64,
    pub stellar_network: String,
    pub rpc_url: String,
    pub horizon_url: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub monad_chain_id: Option<i64>,
}

impl AdminMeResponse {
    pub fn from_profile(
        profile: AdminProfile,
        chain_id: i64,
        stellar_network: String,
        rpc_url: String,
        horizon_url: String,
    ) -> Self {
        Self {
            user: UserResponse::from_parts(profile.user, Some(profile.wallet)),
            chain_id,
            stellar_network,
            rpc_url,
            horizon_url,
            monad_chain_id: Some(chain_id),
        }
    }
}

#[derive(Debug, Serialize)]
pub struct AdminImageUploadResponse {
    pub asset: AdminImageAssetResponse,
}

#[derive(Debug, Serialize)]
pub struct AdminImageAssetResponse {
    pub id: String,
    pub storage_provider: String,
    pub bucket_name: String,
    pub scope: String,
    pub file_name: String,
    pub content_type: String,
    pub size_bytes: i64,
    pub cid: String,
    pub ipfs_url: String,
    pub gateway_url: String,
    pub created_at: String,
}

impl AdminImageUploadResponse {
    pub fn from_record(record: crate::module::admin::model::AdminUploadAssetRecord) -> Self {
        Self {
            asset: AdminImageAssetResponse {
                id: record.id.to_string(),
                storage_provider: record.storage_provider,
                bucket_name: record.bucket_name,
                scope: record.scope,
                file_name: record.file_name,
                content_type: record.content_type,
                size_bytes: record.size_bytes,
                cid: record.cid,
                ipfs_url: record.ipfs_url,
                gateway_url: record.gateway_url,
                created_at: record.created_at.to_rfc3339(),
            },
        }
    }
}
