use std::{env, process::Command};

use anyhow::{Context, Result, anyhow, bail};
use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use data_encoding::BASE32_NOPAD;
use ed25519_dalek::{Signer, SigningKey};
use reqwest::Client;
use serde::{Deserialize, Serialize};

const STELLAR_SECRET_SEED_VERSION_BYTE: u8 = 18 << 3;

#[derive(Debug, Deserialize)]
struct WalletChallengeResponse {
    challenge_id: String,
    message: String,
    expires_at: String,
}

#[derive(Debug, Serialize)]
struct WalletChallengeRequest<'a> {
    wallet_address: &'a str,
}

#[derive(Debug, Serialize)]
struct WalletConnectRequest<'a> {
    challenge_id: &'a str,
    signature: &'a str,
    username: &'a str,
}

#[derive(Debug, Deserialize)]
struct AuthResponse {
    token: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let base_url =
        env::var("BACKEND_BASE_URL").unwrap_or_else(|_| "http://127.0.0.1:8080".to_owned());
    let source = env::var("SOURCE").context("missing SOURCE in env")?;
    let expected_admin = env::var("ADMIN").context("missing ADMIN in env")?;
    let username = env::var("ADMIN_TEST_USERNAME").unwrap_or_else(|_| "mac".to_owned());

    let public_key = stellar_public_key(&source)?;
    if public_key != expected_admin {
        bail!("SOURCE `{source}` resolves to `{public_key}`, but ADMIN is `{expected_admin}`");
    }

    let secret_seed = stellar_secret(&source).with_context(|| {
        format!(
            "failed to read secret for `{source}` from Stellar CLI secure storage; re-add the key with `stellar keys add {source}`"
        )
    })?;

    let client = Client::new();
    let challenge = client
        .post(format!("{base_url}/admin/auth/wallet/challenge"))
        .json(&WalletChallengeRequest {
            wallet_address: &public_key,
        })
        .send()
        .await
        .context("failed to call admin wallet challenge endpoint")?
        .error_for_status()
        .context("admin wallet challenge endpoint returned error")?
        .json::<WalletChallengeResponse>()
        .await
        .context("failed to decode wallet challenge response")?;

    let signature = sign_message(&secret_seed, challenge.message.as_bytes())?;
    let auth = client
        .post(format!("{base_url}/admin/auth/wallet/connect"))
        .json(&WalletConnectRequest {
            challenge_id: &challenge.challenge_id,
            signature: &signature,
            username: &username,
        })
        .send()
        .await
        .context("failed to call admin wallet connect endpoint")?
        .error_for_status()
        .context("admin wallet connect endpoint returned error")?
        .json::<AuthResponse>()
        .await
        .context("failed to decode auth response")?;

    println!("admin address: {public_key}");
    println!("challenge_id: {}", challenge.challenge_id);
    println!("challenge_expires_at: {}", challenge.expires_at);
    println!("token: {}", auth.token);
    println!("jwt_prefix: {}", &auth.token[..auth.token.len().min(24)]);
    println!("result: admin auth flow succeeded");

    Ok(())
}

fn stellar_public_key(name: &str) -> Result<String> {
    let output = Command::new("stellar")
        .args(["keys", "public-key", name])
        .output()
        .context("failed to execute `stellar keys public-key`")?;

    if !output.status.success() {
        bail!(
            "{}",
            String::from_utf8_lossy(&output.stderr).trim().to_owned()
        );
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn stellar_secret(name: &str) -> Result<String> {
    let output = Command::new("stellar")
        .args(["keys", "secret", name])
        .output()
        .context("failed to execute `stellar keys secret`")?;

    if !output.status.success() {
        bail!(
            "{}",
            String::from_utf8_lossy(&output.stderr).trim().to_owned()
        );
    }

    Ok(String::from_utf8(output.stdout)?.trim().to_owned())
}

fn sign_message(secret_seed: &str, message: &[u8]) -> Result<String> {
    let secret_bytes = decode_stellar_secret_seed(secret_seed)?;
    let signing_key = SigningKey::from_bytes(&secret_bytes);
    let signature = signing_key.sign(message);
    Ok(BASE64_STANDARD.encode(signature.to_bytes()))
}

fn decode_stellar_secret_seed(value: &str) -> Result<[u8; 32]> {
    let normalized = value.trim().to_ascii_uppercase();
    let decoded = BASE32_NOPAD
        .decode(normalized.as_bytes())
        .map_err(|_| anyhow!("invalid Stellar secret seed"))?;

    if decoded.len() != 35 {
        bail!("invalid Stellar secret seed length");
    }

    let payload = &decoded[..33];
    let checksum = &decoded[33..];

    if payload[0] != STELLAR_SECRET_SEED_VERSION_BYTE {
        bail!("invalid Stellar secret seed version byte");
    }

    if crc16_xmodem(payload).to_le_bytes() != [checksum[0], checksum[1]] {
        bail!("invalid Stellar secret seed checksum");
    }

    let mut key = [0_u8; 32];
    key.copy_from_slice(&payload[1..33]);
    Ok(key)
}

fn crc16_xmodem(data: &[u8]) -> u16 {
    let mut crc = 0u16;

    for byte in data {
        crc ^= u16::from(*byte) << 8;

        for _ in 0..8 {
            crc = if (crc & 0x8000) != 0 {
                (crc << 1) ^ 0x1021
            } else {
                crc << 1
            };
        }
    }

    crc
}
