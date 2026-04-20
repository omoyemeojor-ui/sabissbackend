use reqwest::{Client, StatusCode};
use serde_json::json;
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
        stellar::{
            build_contract_invocation_xdr, deploy_wallet_contract, hash_transaction_xdr,
            send_transaction_xdr, sign_transaction_xdr,
        },
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

    let horizon_base = state.env.horizon_url.trim_end_matches('/');
    let account_url = format!("{horizon_base}/accounts/{owner_address}");
    let response = state
        .http_client
        .get(&account_url)
        .send()
        .await
        .map_err(|error| AuthError::internal("failed to check managed owner account", error))?;

    if response.status() == StatusCode::OK {
        return Ok(());
    }

    if response.status() != StatusCode::NOT_FOUND {
        let detail = response.text().await.unwrap_or_default();
        return Err(AuthError::internal(
            "failed to check managed owner account",
            anyhow::anyhow!("unexpected horizon response: {detail}"),
        ));
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

    Ok(())
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

    let wallet_address = wallet
        .wallet_address
        .as_deref()
        .ok_or_else(|| AuthError::conflict("wallet address not generated yet"))?;

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

    let build_source_account = if state.env.stellar_aa_relayer_url.is_some() {
        wallet_address
    } else {
        state.env.private_key.as_deref().unwrap_or(wallet_address)
    };

    let xdr = build_contract_invocation_xdr(
        &state.env,
        build_source_account,
        contract_id,
        contract_args,
    )
        .await
        .map_err(|e| AuthError::internal("failed to build XDR", e))?;

    let signed_xdr = sign_transaction_xdr(&state.env, &xdr, &secret_key_str)
        .await
        .map_err(|e| AuthError::internal("failed to sign XDR", e))?;

    if let Some(relayer_url) = state.env.stellar_aa_relayer_url.as_deref() {
        let client = Client::new();
        let payload = json!({
            "transaction": signed_xdr
        });

        let res = client.post(relayer_url)
            .json(&payload)
            .send()
            .await
            .map_err(|e| AuthError::internal("failed to send to relayer", e))?;

        if !res.status().is_success() {
            let text = res.text().await.unwrap_or_default();
            return Err(AuthError::internal("relayer rejected transaction", anyhow::anyhow!("relayer error: {}", text)));
        }

        return Ok(signed_xdr);
    }

    let sponsor_key = state.env.private_key.as_deref().ok_or_else(|| {
        AuthError::internal(
            "missing sponsor private key",
            anyhow::anyhow!("PRIVATE_KEY not configured"),
        )
    })?;
    let sponsor_signed_xdr = sign_transaction_xdr(&state.env, &signed_xdr, sponsor_key)
        .await
        .map_err(|e| AuthError::internal("failed to sign sponsored XDR", e))?;
    let tx_hash = hash_transaction_xdr(&state.env, &sponsor_signed_xdr)
        .await
        .map_err(|e| AuthError::internal("failed to hash sponsored XDR", e))?;
    send_transaction_xdr(&state.env, &sponsor_signed_xdr)
        .await
        .map_err(|e| AuthError::internal("failed to submit sponsored XDR", e))?;

    Ok(tx_hash)
}
