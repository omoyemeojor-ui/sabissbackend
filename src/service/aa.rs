use tokio::time::{Duration, sleep};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::auth::{
        crud::{self, ManagedWalletUpsert},
        error::AuthError,
        model::{ACCOUNT_KIND_STELLAR_SMART_WALLET, UserRecord, VerifiedGoogleToken},
    },
    service::{
        crypto::{self, create_managed_owner_key},
        soroban_rpc::SorobanRpc,
        stellar::{deploy_wallet_contract, submit_contract_as_smart_wallet, submit_contract_as_source},
    },
};

pub async fn ensure_google_user_smart_wallet(
    state: &AppState,
    user: &UserRecord,
    verified: &VerifiedGoogleToken,
) -> Result<(), AuthError> {
    ensure_user_managed_smart_wallet(state, user, "google_oidc", &verified.google_sub).await
}

pub async fn ensure_wallet_user_smart_wallet(
    state: &AppState,
    user: &UserRecord,
    wallet_address: &str,
) -> Result<(), AuthError> {
    ensure_user_managed_smart_wallet(state, user, "stellar_wallet", wallet_address).await
}

async fn ensure_user_managed_smart_wallet(
    state: &AppState,
    user: &UserRecord,
    owner_provider: &str,
    owner_ref: &str,
) -> Result<(), AuthError> {
    if let Some(existing_wallet) = crud::get_wallet_for_user(&state.db, user.id).await? {
        if existing_wallet.account_kind == ACCOUNT_KIND_STELLAR_SMART_WALLET
            && existing_wallet.wallet_address.is_some()
        {
            if let Some(owner_address) = existing_wallet.owner_address.as_deref() {
                ensure_managed_owner_account(state, owner_address).await?;
            }
            return Ok(());
        }
    }

    let owner = create_managed_owner_key(&state.env)
        .map_err(|error| AuthError::internal("failed to create smart-wallet owner key", error))?;
    ensure_managed_owner_account(state, &owner.owner_address).await?;
    let deployed_wallet = deploy_wallet_contract(&state.env, &owner.owner_public_key_hex)
        .await
        .map_err(|error| AuthError::internal("failed to deploy smart wallet", error))?;

    match crud::upsert_managed_wallet(
        &state.db,
        &state.env,
        user.id,
        owner_provider,
        &ManagedWalletUpsert {
            wallet_address: &deployed_wallet.contract_id,
            owner_address: &owner.owner_address,
            owner_ref,
            owner_encrypted_private_key: &owner.encrypted_private_key,
            owner_encryption_nonce: &owner.encryption_nonce,
            owner_key_version: owner.key_version,
        },
    )
    .await
    {
        Ok(_) => Ok(()),
        Err(error) if error.is_conflict() => {
            let existing_wallet = crud::get_wallet_for_user(&state.db, user.id).await?;

            if matches!(existing_wallet, Some(wallet) if wallet.account_kind == ACCOUNT_KIND_STELLAR_SMART_WALLET && wallet.wallet_address.is_some())
            {
                Ok(())
            } else {
                Err(error)
            }
        }
        Err(error) => Err(error),
    }
}

async fn ensure_managed_owner_account(
    state: &AppState,
    owner_address: &str,
) -> Result<(), AuthError> {
    if state.env.network != "testnet" {
        return Ok(());
    }

    let rpc = SorobanRpc::new(&state.env);
    if rpc
        .account_exists(owner_address)
        .await
        .map_err(|error| AuthError::internal("failed to check managed owner account", error))?
    {
        return Ok(());
    }

    let response = state
        .http_client
        .get("https://friendbot.stellar.org")
        .query(&[("addr", owner_address)])
        .send()
        .await
        .map_err(|error| AuthError::internal("failed to fund managed owner account", error))?;

    if !response.status().is_success() {
        let detail = response.text().await.unwrap_or_default();
        return Err(AuthError::internal(
            "failed to fund managed owner account",
            anyhow::anyhow!("friendbot rejected funding request: {detail}"),
        ));
    }

    for _ in 0..10 {
        if rpc
            .account_exists(owner_address)
            .await
            .map_err(|error| AuthError::internal("failed to confirm managed owner account funding", error))?
        {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }

    Err(AuthError::internal(
        "failed to confirm managed owner account funding",
        anyhow::anyhow!("Friendbot funded `{owner_address}`, but Soroban RPC never observed the account"),
    ))
}

pub async fn submit_gasless_transaction(
    state: &AppState,
    user_id: Uuid,
    contract_id: &str,
    contract_args: &[&str],
) -> Result<String, AuthError> {
    let wallet = crud::get_wallet_for_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("user has no wallet"))?;

    let ciphertext = wallet.owner_encrypted_private_key.as_ref().ok_or_else(|| {
        AuthError::internal("missing owner_encrypted_private_key", anyhow::anyhow!("missing encrypted private key"))
    })?;
    let nonce = wallet.owner_encryption_nonce.as_ref().ok_or_else(|| {
        AuthError::internal("missing owner_encryption_nonce", anyhow::anyhow!("missing encryption nonce"))
    })?;

    let private_key_bytes = crypto::decrypt_private_key(&state.env, ciphertext, nonce)
        .map_err(|e| AuthError::internal("failed to decrypt wallet key", e))?;
    
    let private_key_array: [u8; 32] = private_key_bytes.try_into()
        .map_err(|_| AuthError::internal("invalid private key length", anyhow::anyhow!("invalid private key length")))?;
    
    let secret_key_str = crypto::encode_stellar_secret_key(&private_key_array);
    let tx = if wallet.account_kind == ACCOUNT_KIND_STELLAR_SMART_WALLET {
        let wallet_contract_id = wallet
            .wallet_address
            .as_deref()
            .ok_or_else(|| AuthError::forbidden("wallet is not deployed"))?;
        submit_contract_as_smart_wallet(
            &state.env,
            &secret_key_str,
            wallet_contract_id,
            contract_id,
            contract_args,
        )
        .await
        .map_err(|e| AuthError::internal("failed to submit smart-wallet transaction", e))?
    } else {
        // Submit through the backend RPC flow so this path does not depend on
        // the local Stellar CLI being installed in the deployment environment.
        submit_contract_as_source(&state.env, &secret_key_str, contract_id, contract_args)
            .await
            .map_err(|e| AuthError::internal("failed to submit wallet transaction", e))?
    };

    Ok(tx.tx_hash)
}
