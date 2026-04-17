use std::{
    env,
    fmt::Display,
    net::{IpAddr, Ipv4Addr, SocketAddr},
    str::FromStr,
};

use anyhow::{Context, Result, anyhow};
use dotenvy::dotenv;

#[derive(Clone)]
pub struct Environment {
    pub host: IpAddr,
    pub port: u16,
    pub database_url: String,
    pub db_max_connections: u32,
    pub db_acquire_timeout_ms: u64,
    pub redis_url: Option<String>,
    pub rpc_url: String,
    pub cors_allowed_origins: Vec<String>,
    pub google_client_id: Option<String>,
    pub google_jwks_url: String,
    pub stellar_aa_relayer_kind: String,
    pub stellar_aa_relayer_url: Option<String>,
    pub stellar_aa_sponsor_address: String,
    pub sep45_web_auth_contract_id: Option<String>,
    pub sep45_web_auth_domain: Option<String>,
    pub jwt_secret: String,
    pub jwt_ttl_hours: i64,
    pub market_price_sync_interval_secs: u64,
    pub admin_wallet_addresses: Vec<String>,
    pub network: String,
    pub source: String,
    pub private_key: Option<String>,
    pub admin: String,
    pub operator: String,
    pub fee_recipient: String,
    pub mock_usdc_id: String,
    pub sabi_wallet_id: Option<String>,
    pub sabi_ctf_id: String,
    pub sabi_market_id: String,
    pub sabi_exchange_id: String,
}

impl Environment {
    pub fn load() -> Result<Self> {
        dotenv().ok();

        let configured_admins = parse_wallet_list_env("ADMIN_WALLET_ADDRESSES")?;

        Ok(Self {
            host: parse_env("HOST", IpAddr::V4(Ipv4Addr::UNSPECIFIED))?,
            port: parse_env("PORT", 8080)?,
            database_url: required_env("DATABASE_URL")?,
            db_max_connections: parse_env("DB_MAX_CONNECTIONS", 20)?,
            db_acquire_timeout_ms: parse_env("DB_ACQUIRE_TIMEOUT_MS", 10_000)?,
            redis_url: optional_env("REDIS_URL"),
            rpc_url: required_env("RPC_URL")?,
            cors_allowed_origins: parse_cors_allowed_origins()?,
            google_client_id: optional_env("GOOGLE_CLIENT_ID"),
            google_jwks_url: parse_env(
                "GOOGLE_JWKS_URL",
                "https://www.googleapis.com/oauth2/v3/certs".to_owned(),
            )?,
            stellar_aa_relayer_kind: parse_env("STELLAR_AA_RELAYER_KIND", "server".to_owned())?,
            stellar_aa_relayer_url: optional_env("STELLAR_AA_RELAYER_URL"),
            stellar_aa_sponsor_address: normalize_wallet_address(
                &required_env("STELLAR_AA_SPONSOR_ADDRESS")
                    .or_else(|_| required_env("OPERATOR"))
                    .or_else(|_| required_env("ADMIN"))?,
            )?,
            sep45_web_auth_contract_id: Some(parse_env(
                "SEP45_WEB_AUTH_CONTRACT_ID",
                "CD3LA6RKF5D2FN2R2L57MWXLBRSEWWENE74YBEFZSSGNJRJGICFGQXMX".to_owned(),
            )?),
            sep45_web_auth_domain: Some(parse_env(
                "SEP45_WEB_AUTH_DOMAIN",
                "localhost:8080".to_owned(),
            )?),
            jwt_secret: required_env("JWT_SECRET")?,
            jwt_ttl_hours: parse_env("JWT_TTL_HOURS", 24)?,
            market_price_sync_interval_secs: parse_env("MARKET_PRICE_SYNC_INTERVAL_SECS", 900)?,
            admin_wallet_addresses: configured_admins.clone(),
            network: required_env("NETWORK")?,
            source: required_env("SOURCE")?,
            private_key: optional_env("PRIVATE_KEY"),
            admin: normalize_wallet_address(&required_env("ADMIN")?)?,
            operator: normalize_wallet_address(&required_env("OPERATOR")?)?,
            fee_recipient: normalize_wallet_address(&required_env("FEE_RECIPIENT")?)?,
            mock_usdc_id: required_env("MOCK_USDC_ID")?,
            sabi_wallet_id: optional_env("SABI_WALLET_ID"),
            sabi_ctf_id: required_env("SABI_CTF_ID")?,
            sabi_market_id: required_env("SABI_MARKET_ID")?,
            sabi_exchange_id: required_env("SABI_EXCHANGE_ID")?,
        })
        .map(|mut env| {
            env.admin_wallet_addresses = if configured_admins.is_empty() {
                vec![env.admin.clone()]
            } else {
                configured_admins
            };
            env
        })
    }

    pub fn bind_address(&self) -> SocketAddr {
        SocketAddr::from((self.host, self.port))
    }

    pub fn is_admin_wallet(&self, wallet_address: &str) -> bool {
        self.admin_wallet_addresses
            .iter()
            .any(|value| value == wallet_address)
    }
}

fn required_env(key: &str) -> Result<String> {
    env::var(key).with_context(|| format!("missing required env var `{key}`"))
}

fn optional_env(key: &str) -> Option<String> {
    env::var(key)
        .ok()
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty())
}

fn parse_cors_allowed_origins() -> Result<Vec<String>> {
    if let Some(origins) = optional_env("CORS_ALLOWED_ORIGINS") {
        return Ok(split_csv_values(&origins));
    }

    Ok(vec![
        "http://localhost:3000".to_owned(),
        "http://localhost:5173".to_owned(),
        "http://localhost:5174".to_owned(),
    ])
}

fn parse_wallet_list_env(key: &str) -> Result<Vec<String>> {
    let raw = env::var(key).unwrap_or_default();
    if raw.trim().is_empty() {
        return Ok(Vec::new());
    }

    split_csv_values(&raw)
        .into_iter()
        .map(|value| normalize_wallet_address(&value))
        .collect()
}

fn normalize_wallet_address(raw: &str) -> Result<String> {
    let normalized = raw.trim().to_ascii_uppercase();
    if normalized.is_empty() {
        return Err(anyhow!("wallet address cannot be empty"));
    }

    Ok(normalized)
}

fn parse_env<T>(key: &str, default: T) -> Result<T>
where
    T: FromStr + ToString,
    T::Err: Display,
{
    let raw = env::var(key).unwrap_or_else(|_| default.to_string());
    raw.parse::<T>()
        .map_err(|error| anyhow!("invalid value for env var `{key}`: {error}"))
}

fn split_csv_values(raw: &str) -> Vec<String> {
    raw.split(',')
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
        .collect()
}
