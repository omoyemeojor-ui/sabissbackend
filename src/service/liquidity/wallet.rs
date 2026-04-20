use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::{error::AuthError, model::ACCOUNT_KIND_STELLAR_SMART_WALLET},
        liquidity::crud,
    },
};

pub struct UserWalletContext {
    pub wallet_address: String,
}

pub struct WalletAccountContext {
    pub wallet_address: String,
    pub account_kind: String,
}

pub async fn load_user_wallet_context(
    state: &AppState,
    user_id: Uuid,
) -> Result<UserWalletContext, AuthError> {
    let wallet = load_wallet_account(state, user_id).await?;
    Ok(UserWalletContext {
        wallet_address: wallet.wallet_address,
    })
}

pub async fn load_wallet_account_context(
    state: &AppState,
    user_id: Uuid,
) -> Result<WalletAccountContext, AuthError> {
    let wallet = load_wallet_account(state, user_id).await?;
    Ok(WalletAccountContext {
        wallet_address: wallet.wallet_address,
        account_kind: wallet.account_kind,
    })
}

pub async fn load_smart_account_context(
    state: &AppState,
    user_id: Uuid,
) -> Result<UserWalletContext, AuthError> {
    let wallet = load_wallet_account(state, user_id).await?;
    if wallet.account_kind != ACCOUNT_KIND_STELLAR_SMART_WALLET {
        return Err(AuthError::forbidden(
            "write routes require a stellar smart wallet",
        ));
    }

    Ok(UserWalletContext {
        wallet_address: wallet.wallet_address,
    })
}

async fn load_wallet_account(
    state: &AppState,
    user_id: Uuid,
) -> Result<crate::module::liquidity::model::UserWalletAccountRecord, AuthError> {
    let wallet = crud::get_user_wallet_account_by_user_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("wallet not linked to user"))?;

    // Note: Stellar doesn't have EVM chain IDs in the same way, but keeping this simple.
    // If you need to check stellar network here you can.

    Ok(wallet)
}
