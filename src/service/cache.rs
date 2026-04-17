use std::{
    collections::HashMap,
    future::Future,
    sync::{Arc, Mutex, OnceLock},
    time::{Duration, Instant},
};

use axum::{
    body::Body,
    http::{StatusCode, header},
    response::Response,
};
use ethers_core::utils::keccak256;
use redis::aio::{ConnectionManager, ConnectionManagerConfig};
use serde::Serialize;

use crate::module::auth::error::AuthError;

const CACHE_NAMESPACE: &str = "sabibackend";
const REDIS_CONNECTION_TIMEOUT: Duration = Duration::from_millis(500);
const REDIS_RESPONSE_TIMEOUT: Duration = Duration::from_millis(500);
const REDIS_RECONNECT_MAX_DELAY: Duration = Duration::from_secs(3);
const REDIS_RECONNECT_RETRIES: usize = 6;
const REDIS_PIPELINE_BUFFER_SIZE: usize = 256;
const REDIS_FAILURE_COOLDOWN: Duration = Duration::from_secs(30);

pub const SHORT_TTL_SECS: u64 = 5;
pub const STANDARD_TTL_SECS: u64 = 30;
pub const LONG_TTL_SECS: u64 = 120;
pub const HOT_FEED_TTL_SECS: u64 = 300;

static LOCAL_RESPONSE_CACHE: OnceLock<Mutex<HashMap<String, LocalCacheEntry>>> = OnceLock::new();

#[derive(Clone)]
pub struct CacheClient {
    connection: ConnectionManager,
    failure_state: Arc<Mutex<Option<Instant>>>,
}

enum CacheLookup {
    Hit(String),
    Miss,
    Bypass,
}

enum CacheStore {
    Stored,
    Bypass,
}

struct LocalCacheEntry {
    body: String,
    expires_at: Instant,
}

impl CacheClient {
    pub async fn connect(redis_url: &str) -> redis::RedisResult<Self> {
        let client = redis::Client::open(redis_url)?;
        let config = ConnectionManagerConfig::new()
            .set_connection_timeout(Some(REDIS_CONNECTION_TIMEOUT))
            .set_response_timeout(Some(REDIS_RESPONSE_TIMEOUT))
            .set_max_delay(REDIS_RECONNECT_MAX_DELAY)
            .set_number_of_retries(REDIS_RECONNECT_RETRIES)
            .set_pipeline_buffer_size(REDIS_PIPELINE_BUFFER_SIZE);
        let mut connection = client.get_connection_manager_with_config(config).await?;
        redis::cmd("PING")
            .query_async::<String>(&mut connection)
            .await?;

        Ok(Self {
            connection,
            failure_state: Arc::new(Mutex::new(None)),
        })
    }

    async fn get(&self, key: &str) -> CacheLookup {
        if self.should_bypass() {
            return CacheLookup::Bypass;
        }

        let mut connection = self.connection.clone();

        match redis::cmd("GET")
            .arg(key)
            .query_async::<Option<String>>(&mut connection)
            .await
        {
            Ok(Some(value)) => {
                self.mark_success();
                CacheLookup::Hit(value)
            }
            Ok(None) => {
                self.mark_success();
                CacheLookup::Miss
            }
            Err(error) => {
                self.mark_failure("GET", key, &error);
                CacheLookup::Bypass
            }
        }
    }

    async fn set_with_ttl(&self, key: &str, value: &str, ttl_secs: u64) -> CacheStore {
        if self.should_bypass() {
            return CacheStore::Bypass;
        }

        let mut connection = self.connection.clone();

        match redis::cmd("SET")
            .arg(key)
            .arg(value)
            .arg("EX")
            .arg(ttl_secs)
            .query_async::<()>(&mut connection)
            .await
        {
            Ok(()) => {
                self.mark_success();
                CacheStore::Stored
            }
            Err(error) => {
                self.mark_failure("SET", key, &error);
                CacheStore::Bypass
            }
        }
    }

    fn should_bypass(&self) -> bool {
        let until = self
            .failure_state
            .lock()
            .expect("cache failure state poisoned")
            .clone();
        let Some(until) = until else {
            return false;
        };

        if Instant::now() >= until {
            return false;
        }

        true
    }

    fn mark_success(&self) {
        let mut failure_state = self
            .failure_state
            .lock()
            .expect("cache failure state poisoned");
        if failure_state.take().is_some() {
            tracing::info!("redis cache recovered");
        }
    }

    fn mark_failure(&self, operation: &'static str, key: &str, error: &redis::RedisError) {
        let now = Instant::now();
        let cooldown_until = now + REDIS_FAILURE_COOLDOWN;
        let mut failure_state = self
            .failure_state
            .lock()
            .expect("cache failure state poisoned");
        let was_active = failure_state.as_ref().is_some_and(|until| until > &now);

        *failure_state = Some(cooldown_until);

        if !was_active {
            tracing::warn!(
                %error,
                %key,
                operation,
                cooldown_secs = REDIS_FAILURE_COOLDOWN.as_secs(),
                "redis cache temporarily bypassed after failure"
            );
        }
    }
}

pub fn build_cache_key<T>(scope: &str, input: &T) -> Result<String, AuthError>
where
    T: Serialize,
{
    let payload = serde_json::to_vec(input)
        .map_err(|error| AuthError::internal("cache key serialization failed", error))?;

    Ok(format!(
        "{CACHE_NAMESPACE}:{scope}:{}",
        hex::encode(keccak256(payload))
    ))
}

pub async fn cached_json_response<T, F, Fut>(
    cache: Option<&CacheClient>,
    key: String,
    ttl_secs: u64,
    fetch: F,
) -> Result<Response, AuthError>
where
    T: Serialize,
    F: FnOnce() -> Fut,
    Fut: Future<Output = Result<T, AuthError>>,
{
    let mut cache_status = "BYPASS";

    if let Some(cache) = cache {
        match cache.get(&key).await {
            CacheLookup::Hit(body) => return Ok(json_response(body, "HIT")),
            CacheLookup::Miss => {
                cache_status = "MISS";
            }
            CacheLookup::Bypass => {
                if let Some(body) = local_cache_get(&key) {
                    return Ok(json_response(body, "LOCAL_HIT"));
                }
            }
        }
    } else if let Some(body) = local_cache_get(&key) {
        return Ok(json_response(body, "LOCAL_HIT"));
    } else {
        cache_status = "LOCAL_MISS";
    }

    if cache_status == "BYPASS" {
        if let Some(body) = local_cache_get(&key) {
            return Ok(json_response(body, "LOCAL_HIT"));
        }
    }

    let payload = fetch().await?;
    let body = serde_json::to_string(&payload)
        .map_err(|error| AuthError::internal("json serialization failed", error))?;

    local_cache_set(&key, &body, ttl_secs);

    if let Some(cache) = cache {
        if matches!(
            cache.set_with_ttl(&key, &body, ttl_secs).await,
            CacheStore::Bypass
        ) {
            cache_status = "BYPASS";
        }
    }

    Ok(json_response(body, cache_status))
}

pub async fn store_json<T>(
    cache: Option<&CacheClient>,
    key: String,
    ttl_secs: u64,
    payload: &T,
) -> Result<(), AuthError>
where
    T: Serialize,
{
    let body = serde_json::to_string(payload)
        .map_err(|error| AuthError::internal("json serialization failed", error))?;

    local_cache_set(&key, &body, ttl_secs);

    if let Some(cache) = cache {
        let _ = cache.set_with_ttl(&key, &body, ttl_secs).await;
    }

    Ok(())
}

fn local_cache_get(key: &str) -> Option<String> {
    let cache = LOCAL_RESPONSE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let now = Instant::now();
    let mut cache = cache.lock().expect("local response cache poisoned");

    match cache.get(key) {
        Some(entry) if entry.expires_at > now => Some(entry.body.clone()),
        Some(_) => {
            cache.remove(key);
            None
        }
        None => None,
    }
}

fn local_cache_set(key: &str, body: &str, ttl_secs: u64) {
    let cache = LOCAL_RESPONSE_CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let expires_at = Instant::now() + Duration::from_secs(ttl_secs);
    let mut cache = cache.lock().expect("local response cache poisoned");

    cache.insert(
        key.to_owned(),
        LocalCacheEntry {
            body: body.to_owned(),
            expires_at,
        },
    );
}

fn json_response(body: String, cache_status: &'static str) -> Response {
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/json")
        .header("x-cache", cache_status)
        .body(Body::from(body))
        .expect("valid cached JSON response")
}
