use base64::{Engine as _, engine::general_purpose::STANDARD as BASE64_STANDARD};
use axum::http::{HeaderMap, header};
use chrono::{Duration, Utc};
use data_encoding::BASE32_NOPAD;
use ed25519_dalek::{Signature, Verifier, VerifyingKey};
use jsonwebtoken::{Algorithm, DecodingKey, Validation, decode, decode_header};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::auth::{
        crud,
        error::AuthError,
        model::{UserRecord, VerifiedGoogleToken, WalletChallengeRecord},
        schema::{
            AuthResponse, GoogleSignInRequest, MeResponse, UserResponse, WalletChallengeRequest,
            WalletChallengeResponse, WalletConnectRequest,
        },
    },
    service::{
        aa::{ensure_google_user_smart_wallet, ensure_wallet_user_smart_wallet},
        jwt::{AuthenticatedUser, create_session_token},
    },
};

const APP_NAME: &str = "Sabiss";
const WALLET_CHALLENGE_TTL_MINUTES: i64 = 10;
const STELLAR_ACCOUNT_ID_VERSION_BYTE: u8 = 6 << 3;
const STELLAR_CONTRACT_ID_VERSION_BYTE: u8 = 2 << 3;
const FREIGHTER_SIGN_MESSAGE_PREFIX: &str = "Stellar Signed Message:\n";

pub async fn sign_in_with_google(
    state: &AppState,
    headers: &HeaderMap,
    payload: GoogleSignInRequest,
) -> Result<AuthResponse, AuthError> {
    validate_google_csrf(headers, &payload)?;

    let google_client_id = state
        .env
        .google_client_id
        .as_deref()
        .ok_or_else(|| AuthError::bad_request("google sign-in is not configured"))?;

    if let Some(client_id) = payload.client_id.as_deref() {
        if client_id != google_client_id {
            return Err(AuthError::bad_request("unexpected google client id"));
        }
    }

    let verified = verify_google_id_token(state, &payload.credential, google_client_id).await?;
    let user = crud::upsert_google_user(&state.db, &verified).await?;
    ensure_google_user_smart_wallet(state, &user, &verified).await?;
    build_auth_response(state, user).await
}

pub async fn create_wallet_challenge(
    state: &AppState,
    payload: WalletChallengeRequest,
) -> Result<WalletChallengeResponse, AuthError> {
    let wallet_address = normalize_wallet_address(&payload.wallet_address)?;
    issue_wallet_challenge(state, &wallet_address).await
}

pub(crate) async fn issue_wallet_challenge(
    state: &AppState,
    wallet_address: &str,
) -> Result<WalletChallengeResponse, AuthError> {
    let challenge_id = Uuid::new_v4();
    let nonce = Uuid::new_v4().simple().to_string();
    let expires_at = Utc::now() + Duration::minutes(WALLET_CHALLENGE_TTL_MINUTES);
    let message = build_wallet_challenge_message(wallet_address, &state.env.network, &nonce);

    let challenge = crud::create_wallet_challenge(
        &state.db,
        challenge_id,
        wallet_address,
        &state.env.network,
        &nonce,
        &message,
        expires_at,
    )
    .await?;

    Ok(WalletChallengeResponse {
        challenge_id: challenge.id,
        message: challenge.message,
        expires_at: challenge.expires_at,
    })
}

pub async fn connect_wallet(
    state: &AppState,
    payload: WalletConnectRequest,
) -> Result<AuthResponse, AuthError> {
    let challenge = load_active_wallet_challenge(state, payload.challenge_id).await?;
    complete_wallet_connection(
        state,
        challenge,
        payload.username.as_deref(),
        &payload.signature,
    )
    .await
}

pub async fn get_me(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
) -> Result<MeResponse, AuthError> {
    let profile = crud::get_user_profile_by_id(&state.db, authenticated_user.user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("invalid session"))?;
    let (user, wallet) = profile.into_parts();

    Ok(MeResponse {
        user: UserResponse::from_parts(user, wallet),
    })
}

pub fn extract_cookie(headers: &HeaderMap, name: &str) -> Option<String> {
    let cookies = headers.get(header::COOKIE)?.to_str().ok()?;

    cookies
        .split(';')
        .filter_map(|part| part.split_once('='))
        .find_map(|(key, value)| {
            let key = key.trim();
            let value = value.trim();
            (key == name).then(|| value.to_owned())
        })
}

pub fn normalize_wallet_address(raw: &str) -> Result<String, AuthError> {
    let normalized = raw.trim().to_ascii_uppercase();
    decode_stellar_public_key(&normalized)
        .map_err(|_| AuthError::bad_request("invalid stellar wallet address"))?;
    Ok(normalized)
}

pub fn normalize_stellar_address(raw: &str) -> Result<String, AuthError> {
    let normalized = raw.trim().to_ascii_uppercase();
    if decode_stellar_public_key(&normalized).is_ok()
        || decode_stellar_contract_id(&normalized).is_ok()
    {
        return Ok(normalized);
    }

    Err(AuthError::bad_request("invalid stellar address"))
}

pub fn normalize_username(raw: &str) -> Result<String, AuthError> {
    let username = raw.trim().to_ascii_lowercase();

    if !(3..=24).contains(&username.len()) {
        return Err(AuthError::bad_request(
            "username must be between 3 and 24 characters",
        ));
    }

    if !username
        .chars()
        .all(|value| value.is_ascii_lowercase() || value.is_ascii_digit() || value == '_')
    {
        return Err(AuthError::bad_request(
            "username can only contain lowercase letters, numbers, and underscores",
        ));
    }

    Ok(username)
}

async fn build_auth_response(
    state: &AppState,
    user: UserRecord,
) -> Result<AuthResponse, AuthError> {
    let wallet = crud::get_wallet_for_user(&state.db, user.id).await?;
    let token = create_session_token(&state.env, &user)
        .map_err(|error| AuthError::internal("jwt encoding failed", error))?;

    Ok(AuthResponse {
        token,
        user: UserResponse::from_parts(user, wallet),
    })
}

pub(crate) async fn load_active_wallet_challenge(
    state: &AppState,
    challenge_id: Uuid,
) -> Result<WalletChallengeRecord, AuthError> {
    let challenge = crud::get_wallet_challenge_by_id(&state.db, challenge_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("invalid wallet challenge"))?;

    validate_wallet_challenge(&challenge)?;
    Ok(challenge)
}

pub(crate) async fn complete_wallet_connection(
    state: &AppState,
    challenge: WalletChallengeRecord,
    username: Option<&str>,
    raw_signature: &str,
) -> Result<AuthResponse, AuthError> {
    verify_wallet_signature(&challenge, raw_signature)?;

    if let Some(user) =
        crud::find_user_by_wallet_address(&state.db, &challenge.wallet_address).await?
    {
        if !crud::consume_wallet_challenge(&state.db, challenge.id).await? {
            return Err(AuthError::unauthorized("invalid wallet challenge"));
        }

        ensure_wallet_user_smart_wallet(state, &user, &challenge.wallet_address).await?;
        return build_auth_response(state, user).await;
    }

    let username = username
        .ok_or_else(|| AuthError::bad_request("username is required for new wallet users"))?;
    let username = normalize_username(username)?;

    if !crud::consume_wallet_challenge(&state.db, challenge.id).await? {
        return Err(AuthError::unauthorized("invalid wallet challenge"));
    }

    let user = crud::create_wallet_user(
        &state.db,
        &username,
        &challenge.wallet_address,
        &challenge.network,
    )
    .await?;

    ensure_wallet_user_smart_wallet(state, &user, &challenge.wallet_address).await?;
    build_auth_response(state, user).await
}

fn validate_wallet_challenge(challenge: &WalletChallengeRecord) -> Result<(), AuthError> {
    if challenge.consumed_at.is_some() || challenge.expires_at <= Utc::now() {
        return Err(AuthError::unauthorized("invalid wallet challenge"));
    }

    Ok(())
}

fn verify_wallet_signature(
    challenge: &WalletChallengeRecord,
    raw_signature: &str,
) -> Result<(), AuthError> {
    let public_key_bytes = decode_stellar_public_key(&challenge.wallet_address)
        .map_err(|_| AuthError::internal("invalid stored wallet address", "bad public key"))?;
    let verifying_key = VerifyingKey::from_bytes(&public_key_bytes)
        .map_err(|error| AuthError::internal("invalid stored wallet address", error))?;
    let signature_bytes = decode_signature(raw_signature)?;
    let signature = Signature::from_bytes(&signature_bytes);

    // Try verifying against the raw message first
    if verifying_key
        .verify(challenge.message.as_bytes(), &signature)
        .is_ok()
    {
        return Ok(());
    }

    // Freighter signMessage signs SHA256("Stellar Signed Message:\n" + message)
    let freighter_signed_payload =
        format!("{FREIGHTER_SIGN_MESSAGE_PREFIX}{}", challenge.message);
    let freighter_message_hash = Sha256::digest(freighter_signed_payload.as_bytes());
    if verifying_key
        .verify(freighter_message_hash.as_slice(), &signature)
        .is_ok()
    {
        return Ok(());
    }

    // If raw verification fails, try SEP-0010 envelope format
    // SEP-0010 wraps messages as: "{domain} auth {timestamp}\n{message}"
    // For web auth, we use a standardized format similar to: "localhost:8080 auth 1234567890\n{message}"
    let sep10_message = format!("Sabiss auth 0\n{}", challenge.message);
    verifying_key
        .verify(sep10_message.as_bytes(), &signature)
        .map_err(|_| AuthError::unauthorized("wallet signature verification failed"))
}

fn build_wallet_challenge_message(wallet_address: &str, network: &str, nonce: &str) -> String {
    format!(
        "Sign this message to sign in to {APP_NAME}.\n\nWallet: {wallet_address}\nNetwork: {network}\nNonce: {nonce}"
    )
}

async fn verify_google_id_token(
    state: &AppState,
    id_token: &str,
    google_client_id: &str,
) -> Result<VerifiedGoogleToken, AuthError> {
    let header = decode_header(id_token)
        .map_err(|_| AuthError::unauthorized("invalid google credential header"))?;

    if header.alg != Algorithm::RS256 {
        return Err(AuthError::unauthorized(
            "unsupported google credential algorithm",
        ));
    }

    let key_id = header
        .kid
        .ok_or_else(|| AuthError::unauthorized("google credential is missing key id"))?;
    let jwks = state
        .http_client
        .get(&state.env.google_jwks_url)
        .send()
        .await
        .map_err(|error| AuthError::internal("failed to fetch google jwks", error))?
        .error_for_status()
        .map_err(|error| AuthError::internal("google jwks request failed", error))?
        .json::<GoogleJwks>()
        .await
        .map_err(|error| AuthError::internal("failed to decode google jwks", error))?;

    let jwk = jwks
        .keys
        .into_iter()
        .find(|value| value.kid == key_id)
        .ok_or_else(|| AuthError::unauthorized("google signing key not found"))?;

    let decoding_key = DecodingKey::from_rsa_components(&jwk.n, &jwk.e)
        .map_err(|error| AuthError::internal("failed to build google decoding key", error))?;

    let mut validation = Validation::new(Algorithm::RS256);
    validation.set_audience(&[google_client_id]);
    validation.set_issuer(&["accounts.google.com", "https://accounts.google.com"]);
    validation.validate_exp = true;

    let claims = decode::<GoogleIdTokenClaims>(id_token, &decoding_key, &validation)
        .map_err(|_| AuthError::unauthorized("invalid google credential"))?
        .claims;

    Ok(VerifiedGoogleToken {
        google_sub: claims.sub,
        email: claims.email,
        email_verified: claims.email_verified.unwrap_or(false),
        display_name: claims.name,
        avatar_url: claims.picture,
    })
}

fn validate_google_csrf(
    headers: &HeaderMap,
    payload: &GoogleSignInRequest,
) -> Result<(), AuthError> {
    let cookie_token = extract_cookie(headers, "g_csrf_token");
    let body_token = payload.g_csrf_token.as_deref();

    match (cookie_token.as_deref(), body_token) {
        (None, None) => Ok(()),
        (Some(cookie), Some(body)) if cookie == body => Ok(()),
        _ => Err(AuthError::unauthorized("invalid google csrf token")),
    }
}

fn decode_signature(raw: &str) -> Result<[u8; 64], AuthError> {
    let value = raw.trim();

    if let Ok(bytes) = hex::decode(value.trim_start_matches("0x")) {
        if let Ok(signature) = <[u8; 64]>::try_from(bytes.as_slice()) {
            return Ok(signature);
        }
    }

    let decoded = BASE64_STANDARD
        .decode(value)
        .map_err(|_| AuthError::bad_request("invalid wallet signature"))?;

    <[u8; 64]>::try_from(decoded.as_slice())
        .map_err(|_| AuthError::bad_request("invalid wallet signature"))
}

fn decode_stellar_public_key(value: &str) -> Result<[u8; 32], AuthError> {
    decode_stellar_address(value, STELLAR_ACCOUNT_ID_VERSION_BYTE, "invalid stellar wallet address")
}

fn decode_stellar_contract_id(value: &str) -> Result<[u8; 32], AuthError> {
    decode_stellar_address(value, STELLAR_CONTRACT_ID_VERSION_BYTE, "invalid stellar contract address")
}

fn decode_stellar_address(
    value: &str,
    expected_version_byte: u8,
    error_message: &'static str,
) -> Result<[u8; 32], AuthError> {
    let normalized = value.trim().to_ascii_uppercase();
    let decoded = BASE32_NOPAD
        .decode(normalized.as_bytes())
        .map_err(|_| AuthError::bad_request(error_message))?;

    if decoded.len() != 35 {
        return Err(AuthError::bad_request(error_message));
    }

    let payload = &decoded[..33];
    let checksum = &decoded[33..];

    if payload[0] != expected_version_byte {
        return Err(AuthError::bad_request(error_message));
    }

    if crc16_xmodem(payload).to_le_bytes() != [checksum[0], checksum[1]] {
        return Err(AuthError::bad_request(error_message));
    }

    let mut key = [0_u8; 32];
    key.copy_from_slice(&payload[1..33]);
    Ok(key)
}

#[derive(Debug, Deserialize)]
struct GoogleJwks {
    keys: Vec<GoogleJwk>,
}

#[derive(Debug, Deserialize)]
struct GoogleJwk {
    kid: String,
    n: String,
    e: String,
}

#[derive(Debug, Deserialize)]
struct GoogleIdTokenClaims {
    sub: String,
    email: Option<String>,
    email_verified: Option<bool>,
    name: Option<String>,
    picture: Option<String>,
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
