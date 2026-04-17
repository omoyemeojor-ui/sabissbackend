use std::process::Stdio;

use anyhow::{Context, Result, anyhow};
use tokio::process::Command;

use crate::config::environment::Environment;

#[derive(Debug, Clone)]
pub struct NegRiskRegistrationTxResult {
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct PublishEventTxResult {
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct PublishBinaryMarketTxResult {
    pub tx_hash: String,
    pub condition_id: String,
}

#[derive(Debug, Clone)]
pub struct ContractTxResult {
    pub tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct ProposeResolutionTxResult {
    pub tx_hash: String,
    pub dispute_window_seconds: i64,
}

#[derive(Debug, Clone)]
pub struct SetMarketPricesTxResult {
    pub yes_price_tx_hash: String,
    pub no_price_tx_hash: String,
}

#[derive(Debug, Clone)]
pub struct BootstrapMarketLiquidityTxResult {
    pub yes_price_tx_hash: String,
    pub no_price_tx_hash: String,
    pub split_and_add_liquidity_tx_hash: String,
    pub deposit_collateral_tx_hash: Option<String>,
}

#[derive(Debug, Clone)]
pub struct MarketLiquidityReadResult {
    pub yes_available: String,
    pub no_available: String,
    pub idle_yes_total: String,
    pub idle_no_total: String,
    pub posted_yes_total: String,
    pub posted_no_total: String,
    pub claimable_collateral_total: String,
}

#[derive(Debug, Clone)]
pub struct MarketPricesReadResult {
    pub yes_bps: u32,
    pub no_bps: u32,
}

const DEFAULT_RESOLUTION_DISPUTE_WINDOW_SECONDS: i64 = 86_400;
const STELLAR_PLACEHOLDER_TX_HASH: &str = "stellar-cli-submitted";

pub async fn register_neg_risk_event(
    _env: &Environment,
    _event_id: &str,
    _other_market_condition_id: Option<&str>,
) -> Result<NegRiskRegistrationTxResult> {
    Err(anyhow!(
        "neg-risk registration is not implemented by the current Soroban contracts"
    ))
}

pub async fn publish_event(
    env: &Environment,
    event_id: &str,
    group_id: &str,
    series_id: &str,
    neg_risk: bool,
) -> Result<PublishEventTxResult> {
    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &[
            "create_event",
            "--event-id",
            event_id,
            "--group-id",
            group_id,
            "--series-id",
            series_id,
            "--neg-risk",
            bool_arg(neg_risk),
        ],
    )
    .await
    .context("failed to publish event on Soroban")?;

    Ok(PublishEventTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn publish_standalone_binary_market(
    env: &Environment,
    event_id: &str,
    group_id: &str,
    series_id: &str,
    neg_risk: bool,
    question_id: &str,
    end_time: u64,
    oracle: &str,
) -> Result<PublishBinaryMarketTxResult> {
    publish_event(env, event_id, group_id, series_id, neg_risk).await?;

    publish_event_market(env, event_id, question_id, end_time, oracle).await
}

pub async fn publish_event_market(
    env: &Environment,
    event_id: &str,
    question_id: &str,
    end_time: u64,
    oracle: &str,
) -> Result<PublishBinaryMarketTxResult> {
    let condition_id = invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &[
            "create_binary_market_for_event",
            "--event-id",
            event_id,
            "--question-id",
            question_id,
            "--end-time",
            &end_time.to_string(),
            "--oracle",
            oracle,
        ],
    )
    .await
    .context("failed to publish event market on Soroban")?;

    Ok(PublishBinaryMarketTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
        condition_id,
    })
}

pub async fn find_existing_event_binary_market(
    _env: &Environment,
    _event_id: &str,
    _question_id: &str,
) -> Result<Option<String>> {
    Ok(None)
}

pub async fn pause_market(env: &Environment, condition_id: &str) -> Result<ContractTxResult> {
    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &["pause_market", "--condition-id", condition_id],
    )
    .await
    .context("failed to pause market on Soroban")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn unpause_market(env: &Environment, condition_id: &str) -> Result<ContractTxResult> {
    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &["unpause_market", "--condition-id", condition_id],
    )
    .await
    .context("failed to unpause market on Soroban")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn propose_resolution(
    _env: &Environment,
    _condition_id: &str,
    _winning_outcome: u64,
    _oracle_address: &str,
) -> Result<ProposeResolutionTxResult> {
    Ok(ProposeResolutionTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
        dispute_window_seconds: DEFAULT_RESOLUTION_DISPUTE_WINDOW_SECONDS,
    })
}

pub async fn dispute_resolution(
    _env: &Environment,
    _condition_id: &str,
) -> Result<ContractTxResult> {
    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn finalize_resolution(
    env: &Environment,
    condition_id: &str,
    oracle_address: &str,
    winning_outcome: u64,
) -> Result<ContractTxResult> {
    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &[
            "resolve_market",
            "--oracle",
            oracle_address,
            "--condition-id",
            condition_id,
            "--winning-outcome",
            &winning_outcome.to_string(),
        ],
    )
    .await
    .context("failed to resolve market on Soroban")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn emergency_resolve_market(
    env: &Environment,
    condition_id: &str,
    oracle_address: &str,
    winning_outcome: u64,
) -> Result<ContractTxResult> {
    finalize_resolution(env, condition_id, oracle_address, winning_outcome).await
}

pub async fn set_market_prices(
    _env: &Environment,
    _condition_id: &str,
    _yes_bps: u32,
    _no_bps: u32,
) -> Result<SetMarketPricesTxResult> {
    Err(anyhow!(
        "price-setting is not implemented by the current Soroban contracts"
    ))
}

pub async fn bootstrap_market_liquidity(
    _env: &Environment,
    _condition_id: &str,
    _yes_bps: u32,
    _no_bps: u32,
    _inventory_usdc_amount: &str,
    _exit_collateral_usdc_amount: &str,
) -> Result<BootstrapMarketLiquidityTxResult> {
    Err(anyhow!(
        "liquidity bootstrap is not implemented by the current Soroban contracts"
    ))
}

pub async fn get_market_liquidity(
    _env: &Environment,
    _condition_id: &str,
) -> Result<MarketLiquidityReadResult> {
    Ok(MarketLiquidityReadResult {
        yes_available: "0".to_owned(),
        no_available: "0".to_owned(),
        idle_yes_total: "0".to_owned(),
        idle_no_total: "0".to_owned(),
        posted_yes_total: "0".to_owned(),
        posted_no_total: "0".to_owned(),
        claimable_collateral_total: "0".to_owned(),
    })
}

pub async fn get_market_prices_batch_best_effort(
    _env: &Environment,
    _condition_ids: &[String],
) -> Result<std::collections::HashMap<String, MarketPricesReadResult>> {
    Ok(std::collections::HashMap::new())
}

fn bool_arg(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

async fn invoke_contract(
    env: &Environment,
    contract_id: &str,
    send: bool,
    contract_args: &[&str],
) -> Result<String> {
    let source_account = env.private_key.as_deref().unwrap_or(&env.source);

    let mut command = Command::new("stellar");
    command
        .arg("contract")
        .arg("invoke")
        .arg("--network")
        .arg(&env.network)
        .arg("--source-account")
        .arg(source_account)
        .arg("--id")
        .arg(contract_id)
        .arg("--send")
        .arg(if send { "yes" } else { "no" })
        .arg("--")
        .args(contract_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());

    let output = command
        .output()
        .await
        .with_context(|| format!("failed to execute `stellar contract invoke` for `{contract_id}`"))?;

    if !output.status.success() {
        let stderr = String::from_utf8_lossy(&output.stderr).trim().to_owned();
        let stdout = String::from_utf8_lossy(&output.stdout).trim().to_owned();
        let detail = if !stderr.is_empty() { stderr } else { stdout };
        return Err(anyhow!(
            "stellar contract invoke failed for `{contract_id}`: {detail}"
        ));
    }

    Ok(String::from_utf8_lossy(&output.stdout).trim().to_owned())
}
