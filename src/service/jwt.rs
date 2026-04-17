use axum::http::{HeaderMap, header};
use chrono::{Duration, Utc};
use jsonwebtoken::{Algorithm, DecodingKey, EncodingKey, Header, Validation, decode, encode};
use uuid::Uuid;

use crate::{
    config::environment::Environment,
    module::auth::{
        error::AuthError,
        model::{JwtClaims, UserRecord},
    },
};

#[derive(Debug, Clone)]
pub struct AuthenticatedUser {
    pub user_id: Uuid,
}

pub fn create_session_token(
    env: &Environment,
    user: &UserRecord,
) -> Result<String, jsonwebtoken::errors::Error> {
    let issued_at = Utc::now();
    let expires_at = issued_at + Duration::hours(env.jwt_ttl_hours);
    let claims = JwtClaims {
        sub: user.id.to_string(),
        exp: expires_at.timestamp() as usize,
        iat: issued_at.timestamp() as usize,
        email: user.email.clone(),
    };

    encode(
        &Header::new(Algorithm::HS256),
        &claims,
        &EncodingKey::from_secret(env.jwt_secret.as_bytes()),
    )
}

fn decode_session_token(env: &Environment, token: &str) -> Result<JwtClaims, AuthError> {
    let mut validation = Validation::new(Algorithm::HS256);
    validation.validate_exp = true;

    decode::<JwtClaims>(
        token,
        &DecodingKey::from_secret(env.jwt_secret.as_bytes()),
        &validation,
    )
    .map(|token| token.claims)
    .map_err(|_| AuthError::unauthorized("invalid bearer token"))
}

fn bearer_token(headers: &HeaderMap) -> Option<&str> {
    let value = headers.get(header::AUTHORIZATION)?.to_str().ok()?;
    value.strip_prefix("Bearer ")
}

pub fn authenticate_headers(
    headers: &HeaderMap,
    env: &Environment,
) -> Result<AuthenticatedUser, AuthError> {
    let token =
        bearer_token(headers).ok_or_else(|| AuthError::unauthorized("missing bearer token"))?;
    let claims = decode_session_token(env, token)?;
    let user_id = Uuid::parse_str(&claims.sub)
        .map_err(|_| AuthError::unauthorized("invalid bearer token"))?;

    Ok(AuthenticatedUser { user_id })
}
