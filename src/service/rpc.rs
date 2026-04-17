use std::{sync::Arc, time::Duration};

use anyhow::{Context, Result, anyhow};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Middleware, Provider};
use ethers_signers::LocalWallet;

use crate::config::environment::Environment;

const RPC_POLL_INTERVAL_MS: u64 = 1_000;

pub async fn monad_provider(env: &Environment) -> Result<Provider<Http>> {
    let expected_chain_id = u64::try_from(env.monad_chain_id)
        .map_err(|_| anyhow!("MONAD_CHAIN_ID must be non-negative"))?;
    let mut failures = Vec::new();

    for rpc_url in monad_rpc_candidates(env) {
        let provider = Provider::<Http>::try_from(rpc_url.as_str())
            .with_context(|| format!("invalid Monad RPC URL `{rpc_url}`"))?
            .interval(Duration::from_millis(RPC_POLL_INTERVAL_MS));

        match provider.get_chainid().await {
            Ok(chain_id) if chain_id == expected_chain_id.into() => {
                return Ok(provider);
            }
            Ok(chain_id) => {
                failures.push(format!(
                    "{rpc_url} returned unexpected chain id {chain_id} (expected {expected_chain_id})"
                ));
            }
            Err(error) => {
                failures.push(format!("{rpc_url} failed health check: {error}"));
            }
        }
    }

    Err(anyhow!(
        "all Monad RPC endpoints failed: {}",
        failures.join(" | ")
    ))
}

pub async fn monad_provider_arc(env: &Environment) -> Result<Arc<Provider<Http>>> {
    Ok(Arc::new(monad_provider(env).await?))
}

pub async fn monad_signer_middleware(
    env: &Environment,
    wallet: LocalWallet,
) -> Result<Arc<SignerMiddleware<Provider<Http>, LocalWallet>>> {
    let provider = monad_provider(env).await?;
    Ok(Arc::new(SignerMiddleware::new(provider, wallet)))
}

fn monad_rpc_candidates(env: &Environment) -> Vec<String> {
    let mut urls = env.monad_rpc_urls.clone();
    if urls.is_empty() {
        urls.push(env.monad_rpc_url.clone());
    } else if !urls.iter().any(|value| value == &env.monad_rpc_url) {
        urls.insert(0, env.monad_rpc_url.clone());
    }
    urls
}
