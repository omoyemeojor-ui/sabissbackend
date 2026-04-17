use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::{error::AuthError, model::ACCOUNT_KIND_SMART_ACCOUNT},
        liquidity::crud,
    },
    service::aa::SmartAccountSignerContext,
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
) -> Result<SmartAccountSignerContext, AuthError> {
    let wallet = load_wallet_account(state, user_id).await?;
    if wallet.account_kind != ACCOUNT_KIND_SMART_ACCOUNT {
        return Err(AuthError::forbidden(
            "write routes require a smart-account wallet",
        ));
    }

    let owner_address = wallet
        .owner_address
        .ok_or_else(|| AuthError::forbidden("smart-account wallet is missing owner metadata"))?;
    let owner_provider = wallet
        .owner_provider
        .ok_or_else(|| AuthError::forbidden("smart-account wallet is missing owner provider"))?;
    let owner_ref = wallet
        .owner_ref
        .ok_or_else(|| AuthError::forbidden("smart-account wallet is missing account salt"))?;
    let factory_address = wallet
        .factory_address
        .ok_or_else(|| AuthError::forbidden("smart-account wallet is missing factory metadata"))?;
    let entry_point_address = wallet.entry_point_address.ok_or_else(|| {
        AuthError::forbidden("smart-account wallet is missing entry-point metadata")
    })?;
    let owner_encrypted_private_key = wallet.owner_encrypted_private_key.ok_or_else(|| {
        AuthError::forbidden("smart-account wallet is missing owner key material")
    })?;
    let owner_encryption_nonce = wallet
        .owner_encryption_nonce
        .ok_or_else(|| AuthError::forbidden("smart-account wallet is missing owner key nonce"))?;

    Ok(SmartAccountSignerContext {
        wallet_address: wallet.wallet_address,
        owner_address,
        owner_provider,
        owner_ref,
        factory_address,
        entry_point_address,
        owner_encrypted_private_key,
        owner_encryption_nonce,
    })
}

async fn load_wallet_account(
    state: &AppState,
    user_id: Uuid,
) -> Result<crate::module::liquidity::model::UserWalletAccountRecord, AuthError> {
    let wallet = crud::get_user_wallet_account_by_user_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("wallet not linked to user"))?;

    if wallet.chain_id != state.env.monad_chain_id {
        return Err(AuthError::bad_request(
            "linked wallet is not configured for the active chain",
        ));
    }

    Ok(wallet)
}
