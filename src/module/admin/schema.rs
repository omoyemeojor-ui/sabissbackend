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
    pub network: String,
}

impl AdminMeResponse {
    pub fn from_profile(profile: AdminProfile, network: String) -> Self {
        Self {
            user: UserResponse::from_parts(profile.user, Some(profile.wallet)),
            network,
        }
    }
}
