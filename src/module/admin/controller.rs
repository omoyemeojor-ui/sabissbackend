use axum::{
    Json,
    extract::{Extension, Multipart, State},
};

use crate::{
    app::AppState,
    module::{
        admin::{
            crud,
            schema::{
                AdminAuthResponse, AdminImageUploadResponse, AdminMeResponse, AdminWalletChallengeRequest,
                AdminWalletChallengeResponse, AdminWalletConnectRequest,
            },
        },
        auth::error::AuthError,
    },
    service::{
        admin_auth::{connect_wallet, create_wallet_challenge},
        jwt::AuthenticatedUser,
        upload::upload_admin_image,
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
    let chain_id = match state.env.network.as_str() {
        "mainnet" => 1,
        _ => 10143,
    };

    Ok(Json(AdminMeResponse::from_profile(
        profile,
        chain_id,
        state.env.network.clone(),
        state.env.rpc_url.clone(),
        state.env.horizon_url.clone(),
    )))
}

pub async fn upload_image(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    multipart: Multipart,
) -> Result<Json<AdminImageUploadResponse>, AuthError> {
    Ok(Json(
        upload_admin_image(&state, authenticated_user, multipart).await?,
    ))
}
