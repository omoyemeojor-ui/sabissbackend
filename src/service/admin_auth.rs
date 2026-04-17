use crate::{
    app::AppState,
    module::auth::{
        error::AuthError,
        schema::{
            AuthResponse, WalletChallengeRequest, WalletChallengeResponse, WalletConnectRequest,
        },
    },
    service::auth::{
        complete_wallet_connection, issue_wallet_challenge, load_active_wallet_challenge,
        normalize_wallet_address,
    },
};

pub async fn create_wallet_challenge(
    state: &AppState,
    payload: WalletChallengeRequest,
) -> Result<WalletChallengeResponse, AuthError> {
    let wallet_address = normalize_wallet_address(&payload.wallet_address)?;
    ensure_admin_wallet(state, &wallet_address)?;

    issue_wallet_challenge(state, &wallet_address).await
}

pub async fn connect_wallet(
    state: &AppState,
    payload: WalletConnectRequest,
) -> Result<AuthResponse, AuthError> {
    let challenge = load_active_wallet_challenge(state, payload.challenge_id).await?;
    ensure_admin_wallet(state, &challenge.wallet_address)?;

    complete_wallet_connection(
        state,
        challenge,
        payload.username.as_deref(),
        &payload.signature,
    )
    .await
}

fn ensure_admin_wallet(state: &AppState, wallet_address: &str) -> Result<(), AuthError> {
    if state.env.is_admin_wallet(wallet_address) {
        return Ok(());
    }

    Err(AuthError::forbidden("admin access required"))
}
