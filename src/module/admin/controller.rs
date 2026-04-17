use axum::{
    Json,
    extract::{Extension, State},
};

use crate::{
    app::AppState,
    module::{
        admin::{
            crud,
            schema::{
                AdminAuthResponse, AdminMeResponse, AdminWalletChallengeRequest,
                AdminWalletChallengeResponse, AdminWalletConnectRequest,
            },
        },
        auth::error::AuthError,
    },
    service::{
        admin_auth::{connect_wallet, create_wallet_challenge},
        jwt::AuthenticatedUser,
    },
};

pub async fn wallet_challenge(
    State(state): State<AppState>,
    Json(payload): Json<AdminWalletChallengeRequest>,
) -> Result<Json<AdminWalletChallengeResponse>, AuthError> {
    Ok(Json(create_wallet_challenge(&state, payload).await?))
}

pub async fn wallet_connect(
    State(state): State<AppState>,
    Json(payload): Json<AdminWalletConnectRequest>,
) -> Result<Json<AdminAuthResponse>, AuthError> {
    Ok(Json(connect_wallet(&state, payload).await?))
}

pub async fn me(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<Json<AdminMeResponse>, AuthError> {
    let profile = crud::get_admin_profile(&state.db, authenticated_user.user_id).await?;

    Ok(Json(AdminMeResponse::from_profile(
        profile,
        state.env.network.clone(),
    )))
}
