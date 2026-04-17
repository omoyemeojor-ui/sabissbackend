use aes_gcm::{
    Aes256Gcm, Nonce,
    aead::{Aead, KeyInit, OsRng, rand_core::RngCore},
};
use base64::{Engine as _, engine::general_purpose::STANDARD};
use ethers_signers::{LocalWallet, Signer};
use rand::thread_rng;
use thiserror::Error;

use crate::config::environment::Environment;

#[derive(Debug, Error)]
pub enum WalletCryptoError {
    #[error("AA owner encryption key must be a 32-byte hex string")]
    InvalidEncryptionKey,
    #[error("wallet encryption failed")]
    EncryptFailed,
    #[error("wallet decryption failed")]
    DecryptFailed,
}

#[derive(Debug, Clone)]
pub struct ManagedOwnerKeyMaterial {
    pub owner_address: String,
    pub encrypted_private_key: String,
    pub encryption_nonce: String,
    pub key_version: i32,
}

pub fn create_managed_owner_key(
    env: &Environment,
) -> Result<ManagedOwnerKeyMaterial, WalletCryptoError> {
    let wallet = LocalWallet::new(&mut thread_rng());
    let secret_key = wallet.signer().to_bytes();
    let encrypted = encrypt_private_key(env, secret_key.as_slice())?;

    Ok(ManagedOwnerKeyMaterial {
        owner_address: format!("{:#x}", wallet.address()),
        encrypted_private_key: encrypted.ciphertext,
        encryption_nonce: encrypted.nonce,
        key_version: env.aa_owner_encryption_key_version,
    })
}

pub fn decrypt_private_key(
    env: &Environment,
    ciphertext_b64: &str,
    nonce_b64: &str,
) -> Result<Vec<u8>, WalletCryptoError> {
    let cipher = build_cipher(env)?;
    let nonce_bytes = STANDARD
        .decode(nonce_b64)
        .map_err(|_| WalletCryptoError::DecryptFailed)?;
    let ciphertext = STANDARD
        .decode(ciphertext_b64)
        .map_err(|_| WalletCryptoError::DecryptFailed)?;

    cipher
        .decrypt(Nonce::from_slice(&nonce_bytes), ciphertext.as_ref())
        .map_err(|_| WalletCryptoError::DecryptFailed)
}

struct EncryptedWalletKey {
    ciphertext: String,
    nonce: String,
}

fn encrypt_private_key(
    env: &Environment,
    private_key_bytes: &[u8],
) -> Result<EncryptedWalletKey, WalletCryptoError> {
    let cipher = build_cipher(env)?;
    let mut nonce_bytes = [0_u8; 12];
    OsRng.fill_bytes(&mut nonce_bytes);
    let ciphertext = cipher
        .encrypt(Nonce::from_slice(&nonce_bytes), private_key_bytes)
        .map_err(|_| WalletCryptoError::EncryptFailed)?;

    Ok(EncryptedWalletKey {
        ciphertext: STANDARD.encode(ciphertext),
        nonce: STANDARD.encode(nonce_bytes),
    })
}

fn build_cipher(env: &Environment) -> Result<Aes256Gcm, WalletCryptoError> {
    let key_bytes = hex::decode(&env.aa_owner_encryption_key)
        .map_err(|_| WalletCryptoError::InvalidEncryptionKey)?;

    if key_bytes.len() != 32 {
        return Err(WalletCryptoError::InvalidEncryptionKey);
    }

    Aes256Gcm::new_from_slice(&key_bytes).map_err(|_| WalletCryptoError::InvalidEncryptionKey)
}
