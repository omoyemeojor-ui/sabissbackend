use axum::{
    Json,
    extract::{Extension, State},
    http::HeaderMap,
    response::IntoResponse,
};

use crate::{
    app::AppState,
    module::auth::{
        error::AuthError,
        schema::{
            GoogleSignInRequest, SmartWalletRegistrationRequest, WalletChallengeRequest,
            WalletConnectRequest,
        },
    },
    service::{
        auth::{
            connect_wallet, create_wallet_challenge, get_me, register_smart_wallet,
            sign_in_with_google,
        },
        jwt::AuthenticatedUser,
    },
};

pub async fn google_sign_in(
    State(state): State<AppState>,
    headers: HeaderMap,
    Json(payload): Json<GoogleSignInRequest>,
) -> Result<impl IntoResponse, AuthError> {
    Ok(Json(sign_in_with_google(&state, &headers, payload).await?))
}

pub async fn wallet_challenge(
    State(state): State<AppState>,
    Json(payload): Json<WalletChallengeRequest>,
) -> Result<impl IntoResponse, AuthError> {
    Ok(Json(create_wallet_challenge(&state, payload).await?))
}

pub async fn wallet_connect(
    State(state): State<AppState>,
    Json(payload): Json<WalletConnectRequest>,
) -> Result<impl IntoResponse, AuthError> {
    Ok(Json(connect_wallet(&state, payload).await?))
}

pub async fn me(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
) -> Result<impl IntoResponse, AuthError> {
    Ok(Json(get_me(&state, authenticated_user).await?))
}

pub async fn smart_wallet_register(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<SmartWalletRegistrationRequest>,
) -> Result<impl IntoResponse, AuthError> {
    Ok(Json(
        register_smart_wallet(&state, authenticated_user, payload).await?,
    ))
}
