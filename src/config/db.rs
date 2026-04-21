use std::{
    net::{IpAddr, Ipv4Addr},
    time::Duration,
};

use anyhow::{Result, anyhow};
use reqwest::Url;
use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::config::environment::Environment;

pub type DbPool = PgPool;

const DEFAULT_POSTGRES_PORT: u16 = 5432;
const NEON_HOST_SUFFIX: &str = ".neon.tech";
const NEON_POOLER_SUFFIX: &str = "-pooler";

pub fn sanitize_database_url(database_url: &str) -> String {
    let Ok(mut url) = Url::parse(database_url) else {
        return database_url.to_owned();
    };

    let mut removed_channel_binding = false;
    let query_pairs = url
        .query_pairs()
        .filter_map(|(key, value)| {
            if key == "channel_binding" {
                removed_channel_binding = true;
                None
            } else {
                Some((key.into_owned(), value.into_owned()))
            }
        })
        .collect::<Vec<_>>();

    if !removed_channel_binding {
        return database_url.to_owned();
    }

    {
        let mut pairs = url.query_pairs_mut();
        pairs.clear();

        for (key, value) in query_pairs {
            pairs.append_pair(&key, &value);
        }
    }

    url.to_string()
}

async fn prepare_database_url(database_url: &str) -> String {
    let database_url = sanitize_database_url(database_url);
    let Some((host, port, endpoint_id)) = neon_connection_target(&database_url) else {
        return database_url;
    };

    let Some(hostaddr) = resolve_neon_ipv4(&host, port).await else {
        return database_url;
    };

    tracing::info!(%host, %hostaddr, "using resolved IPv4 for Neon Postgres connection");

    rewrite_neon_database_url(&database_url, hostaddr, &endpoint_id)
}

fn neon_connection_target(database_url: &str) -> Option<(String, u16, String)> {
    let url = Url::parse(database_url).ok()?;
    if url.query_pairs().any(|(key, _)| key == "hostaddr") {
        return None;
    }

    let host = url.host_str()?.to_owned();
    let endpoint_id = neon_endpoint_id(&host)?;

    Some((
        host,
        url.port().unwrap_or(DEFAULT_POSTGRES_PORT),
        endpoint_id,
    ))
}

fn neon_endpoint_id(host: &str) -> Option<String> {
    if !host.ends_with(NEON_HOST_SUFFIX) {
        return None;
    }

    let label = host.split('.').next()?;
    if !label.starts_with("ep-") {
        return None;
    }

    Some(label.trim_end_matches(NEON_POOLER_SUFFIX).to_owned())
}

async fn resolve_neon_ipv4(host: &str, port: u16) -> Option<Ipv4Addr> {
    let lookup = tokio::time::timeout(
        Duration::from_secs(2),
        tokio::net::lookup_host((host, port)),
    )
    .await
    .ok()?
    .ok()?;

    lookup.into_iter().find_map(|addr| match addr.ip() {
        IpAddr::V4(ipv4) => Some(ipv4),
        IpAddr::V6(_) => None,
    })
}

fn rewrite_neon_database_url(database_url: &str, hostaddr: Ipv4Addr, endpoint_id: &str) -> String {
    let Ok(mut url) = Url::parse(database_url) else {
        return database_url.to_owned();
    };

    let endpoint_option = format!("endpoint={endpoint_id}");
    let mut hostaddr_set = false;
    let mut endpoint_set = false;
    let query_pairs = url
        .query_pairs()
        .map(|(key, value)| {
            let key = key.into_owned();
            let mut value = value.into_owned();

            match key.as_str() {
                "hostaddr" => {
                    value = hostaddr.to_string();
                    hostaddr_set = true;
                }
                "options" => {
                    if has_neon_endpoint_option(&value) {
                        endpoint_set = true;
                    } else if value.is_empty() {
                        value = endpoint_option.clone();
                        endpoint_set = true;
                    } else {
                        value.push(' ');
                        value.push_str(&endpoint_option);
                        endpoint_set = true;
                    }
                }
                _ => {}
            }

            (key, value)
        })
        .collect::<Vec<_>>();

    {
        let mut pairs = url.query_pairs_mut();
        pairs.clear();

        for (key, value) in query_pairs {
            pairs.append_pair(&key, &value);
        }

        if !hostaddr_set {
            pairs.append_pair("hostaddr", &hostaddr.to_string());
        }

        if !endpoint_set {
            pairs.append_pair("options", &endpoint_option);
        }
    }

    url.to_string()
}

fn has_neon_endpoint_option(options: &str) -> bool {
    options
        .split_whitespace()
        .any(|option| option.starts_with("endpoint="))
}

pub async fn create_pool(env: &Environment) -> Result<DbPool> {
    let sanitized_database_url = sanitize_database_url(&env.database_url);
    let prepared_database_url = prepare_database_url(&sanitized_database_url).await;
    let acquire_timeout = Duration::from_millis(env.db_acquire_timeout_ms);

    tracing::info!(
        max_connections = env.db_max_connections,
        acquire_timeout_ms = env.db_acquire_timeout_ms,
        "configuring postgres pool"
    );

    let mut failures = Vec::new();
    for max_connections in max_connection_candidates(env.db_max_connections) {
        for (label, database_url) in
            database_url_candidates(&prepared_database_url, &sanitized_database_url)
        {
            match PgPoolOptions::new()
                .max_connections(max_connections)
                .acquire_timeout(acquire_timeout)
                .connect(database_url)
                .await
            {
                Ok(pool) => {
                    if label != "prepared" || max_connections != env.db_max_connections {
                        tracing::warn!(
                            candidate = label,
                            configured_max_connections = env.db_max_connections,
                            effective_max_connections = max_connections,
                            "postgres connection succeeded after fallback"
                        );
                    }
                    return Ok(pool);
                }
                Err(error) => failures.push(format!(
                    "{label} database URL with max_connections={max_connections} failed: {error:#}"
                )),
            }
        }
    }

    Err(anyhow!(
        "failed to connect to postgres: {}",
        failures.join(" | ")
    ))
}

fn database_url_candidates<'a>(prepared: &'a str, sanitized: &'a str) -> Vec<(&'static str, &'a str)> {
    let mut candidates = vec![("prepared", prepared)];
    if prepared != sanitized {
        candidates.push(("sanitized", sanitized));
    }
    candidates
}

fn max_connection_candidates(configured: u32) -> Vec<u32> {
    let mut candidates = vec![configured];
    if configured > 1 {
        candidates.push(1);
    }
    candidates
}
