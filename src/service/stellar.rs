use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use serde_json::Value;
use tokio::time::{Duration, sleep};

use crate::{
    config::environment::Environment,
    service::soroban_rpc::SorobanRpc,
};

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
pub struct DeployWalletContractResult {
    pub contract_id: String,
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

#[derive(Debug, Clone)]
pub struct LiquidityPositionReadResult {
    pub posted_yes_amount: String,
    pub posted_no_amount: String,
    pub idle_yes_amount: String,
    pub idle_no_amount: String,
    pub collateral_amount: String,
    pub claimable_collateral_amount: String,
    pub updated_at: Option<DateTime<Utc>>,
    pub active: bool,
}

const DEFAULT_RESOLUTION_DISPUTE_WINDOW_SECONDS: i64 = 86_400;
const STELLAR_PLACEHOLDER_TX_HASH: &str = "soroban-rpc-submitted";

pub async fn deploy_wallet_contract(
    env: &Environment,
    owner_public_key_hex: &str,
) -> Result<DeployWalletContractResult> {
    let factory_id = env
        .sabi_wallet_factory_id
        .as_deref()
        .ok_or_else(|| anyhow!("missing SABI_WALLET_FACTORY_ID for managed wallet provisioning"))?;
    let contract_id = invoke_contract(
        env,
        factory_id,
        true,
        &["create_wallet", "--owner", owner_public_key_hex],
    )
    .await
    .context("failed to create user wallet through Soroban wallet factory")?;

    Ok(DeployWalletContractResult { contract_id })
}

pub async fn register_neg_risk_event(
    env: &Environment,
    event_id: &str,
    other_market_condition_id: Option<&str>,
) -> Result<NegRiskRegistrationTxResult> {
    let event_id = bytes32_cli_arg(event_id)?;
    let other_market_condition_id = other_market_condition_id.unwrap_or(
        "0000000000000000000000000000000000000000000000000000000000000000",
    );
    let other_market_condition_id = bytes32_cli_arg(other_market_condition_id)?;

    invoke_contract(
        env,
        &env.sabi_neg_risk_id,
        true,
        &[
            "register_neg_risk_event",
            "--event-id",
            event_id.as_str(),
            "--other-market",
            other_market_condition_id.as_str(),
        ],
    )
    .await
    .context("failed to register neg-risk event on Soroban")?;

    Ok(NegRiskRegistrationTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn publish_event(
    env: &Environment,
    event_id: &str,
    group_id: &str,
    series_id: &str,
    neg_risk: bool,
) -> Result<PublishEventTxResult> {
    let event_id = bytes32_cli_arg(event_id)?;
    let group_id = bytes32_cli_arg(group_id)?;
    let series_id = bytes32_cli_arg(series_id)?;

    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &[
            "create_event",
            "--event-id",
            event_id.as_str(),
            "--group-id",
            group_id.as_str(),
            "--series-id",
            series_id.as_str(),
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
    let event_id = bytes32_cli_arg(event_id)?;
    let question_id = bytes32_cli_arg(question_id)?;
    let condition_id = invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &[
            "create_binary_market_for_event",
            "--event-id",
            event_id.as_str(),
            "--question-id",
            question_id.as_str(),
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
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &["pause_market", "--condition-id", condition_id.as_str()],
    )
    .await
    .context("failed to pause market on Soroban")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn unpause_market(env: &Environment, condition_id: &str) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &["unpause_market", "--condition-id", condition_id.as_str()],
    )
    .await
    .context("failed to unpause market on Soroban")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn propose_resolution(
    env: &Environment,
    condition_id: &str,
    winning_outcome: u64,
    _oracle_address: &str,
) -> Result<ProposeResolutionTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &[
            "propose_resolution",
            "--resolver",
            &env.admin,
            "--condition-id",
            condition_id.as_str(),
            "--winning-outcome",
            &winning_outcome.to_string(),
        ],
    )
    .await
    .context("failed to propose market resolution on Soroban")?;

    let dispute_window_seconds = invoke_contract(
        env,
        &env.sabi_market_id,
        false,
        &["get_resolution_dispute_window"],
    )
    .await
    .ok()
    .and_then(|value| value.parse::<i64>().ok())
    .unwrap_or(DEFAULT_RESOLUTION_DISPUTE_WINDOW_SECONDS);

    Ok(ProposeResolutionTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
        dispute_window_seconds,
    })
}

pub async fn dispute_resolution(
    env: &Environment,
    condition_id: &str,
) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &[
            "dispute_resolution",
            "--disputer",
            &env.admin,
            "--condition-id",
            condition_id.as_str(),
        ],
    )
    .await
    .context("failed to dispute market resolution on Soroban")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn finalize_resolution(
    env: &Environment,
    condition_id: &str,
    _oracle_address: &str,
    _winning_outcome: u64,
) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract(
        env,
        &env.sabi_market_id,
        true,
        &["finalize_resolution", "--condition-id", condition_id.as_str()],
    )
    .await
    .context("failed to finalize market resolution on Soroban")?;

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

pub async fn buy_market_outcome(
    env: &Environment,
    source_account: &str,
    buyer: &str,
    condition_id: &str,
    outcome_index: u32,
    usdc_amount: &str,
) -> Result<ContractTxResult> {
    ensure_exchange_max_trade_amount(env, usdc_amount)
        .await
        .context("failed to raise Soroban exchange max trade amount for buy")?;
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_exchange_id,
        true,
        &[
            "buy_outcome",
            "--buyer",
            buyer,
            "--condition-id",
            condition_id.as_str(),
            "--outcome-index",
            &outcome_index.to_string(),
            "--usdc-amount",
            usdc_amount,
        ],
    )
    .await
    .context("failed to buy outcome on Soroban exchange")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn sell_market_outcome(
    env: &Environment,
    source_account: &str,
    seller: &str,
    condition_id: &str,
    outcome_index: u32,
    token_amount: &str,
) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_exchange_id,
        true,
        &[
            "sell_outcome",
            "--seller",
            seller,
            "--condition-id",
            condition_id.as_str(),
            "--outcome-index",
            &outcome_index.to_string(),
            "--token-amount",
            token_amount,
        ],
    )
    .await
    .context("failed to sell outcome on Soroban exchange")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn split_market_position(
    env: &Environment,
    source_account: &str,
    user: &str,
    condition_id: &str,
    collateral_amount: &str,
) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_ctf_id,
        true,
        &[
            "split_position",
            "--user",
            user,
            "--collateral-token",
            &env.mock_usdc_id,
            "--parent-collection-id",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "--condition-id",
            condition_id.as_str(),
            "--partition",
            "[1,2]",
            "--amount",
            collateral_amount,
        ],
    )
    .await
    .context("failed to split market position on Soroban CTF")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn merge_market_positions(
    env: &Environment,
    source_account: &str,
    user: &str,
    condition_id: &str,
    pair_token_amount: &str,
) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_ctf_id,
        true,
        &[
            "merge_positions",
            "--user",
            user,
            "--collateral-token",
            &env.mock_usdc_id,
            "--parent-collection-id",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "--condition-id",
            condition_id.as_str(),
            "--partition",
            "[1,2]",
            "--amount",
            pair_token_amount,
        ],
    )
    .await
    .context("failed to merge market positions on Soroban CTF")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn set_market_prices(
    env: &Environment,
    condition_id: &str,
    yes_bps: u32,
    no_bps: u32,
) -> Result<SetMarketPricesTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    match invoke_contract(
        env,
        &env.sabi_exchange_id,
        true,
        &[
            "set_price",
            "--condition-id",
            condition_id.as_str(),
            "--outcome-index",
            "0",
            "--price-bps",
            &yes_bps.to_string(),
        ],
    )
    .await
    {
        Ok(_) => {}
        Err(error) if is_retryable_submission_error(&error) => {
            sleep(Duration::from_millis(1_500)).await;
            let refreshed = get_market_price_bps(env, condition_id.as_str(), 0).await?;
            if refreshed != yes_bps {
                return Err(error).context("failed to set YES market price on Soroban");
            }
        }
        Err(error) => return Err(error).context("failed to set YES market price on Soroban"),
    }
    match invoke_contract(
        env,
        &env.sabi_exchange_id,
        true,
        &[
            "set_price",
            "--condition-id",
            condition_id.as_str(),
            "--outcome-index",
            "1",
            "--price-bps",
            &no_bps.to_string(),
        ],
    )
    .await
    {
        Ok(_) => {}
        Err(error) if is_retryable_submission_error(&error) => {
            sleep(Duration::from_millis(1_500)).await;
            let refreshed = get_market_price_bps(env, condition_id.as_str(), 1).await?;
            if refreshed != no_bps {
                return Err(error).context("failed to set NO market price on Soroban");
            }
        }
        Err(error) => return Err(error).context("failed to set NO market price on Soroban"),
    }

    Ok(SetMarketPricesTxResult {
        yes_price_tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
        no_price_tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn bootstrap_market_liquidity(
    env: &Environment,
    condition_id: &str,
    yes_bps: u32,
    no_bps: u32,
    inventory_usdc_amount: &str,
    exit_collateral_usdc_amount: &str,
) -> Result<BootstrapMarketLiquidityTxResult> {
    let prices = set_market_prices(env, condition_id, yes_bps, no_bps).await?;
    let condition_id = bytes32_cli_arg(condition_id)?;

    invoke_contract(
        env,
        &env.sabi_ctf_id,
        true,
        &[
            "split_position",
            "--user",
            &env.admin,
            "--collateral-token",
            &env.mock_usdc_id,
            "--parent-collection-id",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "--condition-id",
            condition_id.as_str(),
            "--amount",
            inventory_usdc_amount,
            "--partition",
            "[1,2]",
        ],
    )
    .await
    .context("failed to split bootstrap collateral on Soroban")?;
    invoke_contract(
        env,
        &env.sabi_liquidity_manager_id,
        true,
        &[
            "deposit_inventory",
            "--provider",
            &env.admin,
            "--condition-id",
            condition_id.as_str(),
            "--yes-amount",
            inventory_usdc_amount,
            "--no-amount",
            inventory_usdc_amount,
        ],
    )
    .await
    .context("failed to deposit bootstrap inventory through Soroban liquidity manager")?;
    invoke_contract(
        env,
        &env.sabi_liquidity_manager_id,
        true,
        &[
            "add_liquidity",
            "--provider",
            &env.admin,
            "--condition-id",
            condition_id.as_str(),
            "--yes-amount",
            inventory_usdc_amount,
            "--no-amount",
            inventory_usdc_amount,
        ],
    )
    .await
    .context("failed to post bootstrap liquidity on Soroban")?;

    let deposit_collateral_tx_hash = if exit_collateral_usdc_amount != "0" {
        ensure_exchange_max_trade_amount(env, exit_collateral_usdc_amount)
            .await
            .context("failed to raise Soroban exchange max trade amount for collateral bootstrap")?;
        invoke_contract(
            env,
            &env.sabi_liquidity_manager_id,
            true,
            &[
                "deposit_collateral",
                "--provider",
                &env.admin,
                "--condition-id",
                condition_id.as_str(),
                "--amount",
                exit_collateral_usdc_amount,
            ],
        )
        .await
        .context("failed to deposit exit collateral through Soroban liquidity manager")?;
        Some(STELLAR_PLACEHOLDER_TX_HASH.to_owned())
    } else {
        None
    };

    Ok(BootstrapMarketLiquidityTxResult {
        yes_price_tx_hash: prices.yes_price_tx_hash,
        no_price_tx_hash: prices.no_price_tx_hash,
        split_and_add_liquidity_tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
        deposit_collateral_tx_hash,
    })
}

pub async fn get_market_liquidity(
    env: &Environment,
    condition_id: &str,
) -> Result<MarketLiquidityReadResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    let yes_available = invoke_contract(
        env,
        &env.sabi_exchange_id,
        false,
        &[
            "get_available_liquidity",
            "--condition-id",
            condition_id.as_str(),
            "--outcome-index",
            "0",
        ],
    )
    .await
    .context("failed to read YES liquidity on Soroban exchange")?;
    let no_available = invoke_contract(
        env,
        &env.sabi_exchange_id,
        false,
        &[
            "get_available_liquidity",
            "--condition-id",
            condition_id.as_str(),
            "--outcome-index",
            "1",
        ],
    )
    .await
    .context("failed to read NO liquidity on Soroban exchange")?;
    let totals = invoke_contract(
        env,
        &env.sabi_liquidity_manager_id,
        false,
        &["get_market_liquidity", "--condition-id", condition_id.as_str()],
    )
    .await
    .context("failed to read liquidity totals on Soroban liquidity manager")?;
    let mut totals = parse_liquidity_totals(&totals)?;
    if totals.posted_yes_total == "0" && yes_available != "0" {
        totals.posted_yes_total = yes_available.clone();
    }
    if totals.posted_no_total == "0" && no_available != "0" {
        totals.posted_no_total = no_available.clone();
    }

    Ok(MarketLiquidityReadResult {
        yes_available,
        no_available,
        idle_yes_total: totals.idle_yes_total,
        idle_no_total: totals.idle_no_total,
        posted_yes_total: totals.posted_yes_total,
        posted_no_total: totals.posted_no_total,
        claimable_collateral_total: totals.claimable_collateral_total,
    })
}

pub async fn get_event_liquidity(
    env: &Environment,
    event_id: &str,
) -> Result<MarketLiquidityReadResult> {
    let event_id = bytes32_cli_arg(event_id)?;
    let totals = invoke_contract(
        env,
        &env.sabi_liquidity_manager_id,
        false,
        &["get_event_liquidity", "--event-id", event_id.as_str()],
    )
    .await
    .context("failed to read event liquidity totals on Soroban liquidity manager")?;
    let totals = parse_liquidity_totals(&totals)?;

    Ok(MarketLiquidityReadResult {
        yes_available: totals.posted_yes_total.clone(),
        no_available: totals.posted_no_total.clone(),
        idle_yes_total: totals.idle_yes_total,
        idle_no_total: totals.idle_no_total,
        posted_yes_total: totals.posted_yes_total,
        posted_no_total: totals.posted_no_total,
        claimable_collateral_total: totals.claimable_collateral_total,
    })
}

pub async fn get_liquidity_position(
    env: &Environment,
    condition_id: &str,
    provider: &str,
) -> Result<LiquidityPositionReadResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    let position = invoke_contract(
        env,
        &env.sabi_liquidity_manager_id,
        false,
        &[
            "get_liquidity_position",
            "--condition-id",
            condition_id.as_str(),
            "--provider",
            provider,
        ],
    )
    .await
    .context("failed to read liquidity position on Soroban liquidity manager")?;

    parse_liquidity_position(&position)
}

pub async fn deposit_inventory(
    env: &Environment,
    source_account: &str,
    provider: &str,
    condition_id: &str,
    yes_amount: &str,
    no_amount: &str,
) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_liquidity_manager_id,
        true,
        &[
            "deposit_inventory",
            "--provider",
            provider,
            "--condition-id",
            condition_id.as_str(),
            "--yes-amount",
            yes_amount,
            "--no-amount",
            no_amount,
        ],
    )
    .await
    .context("failed to deposit inventory through Soroban liquidity manager")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn add_liquidity(
    env: &Environment,
    source_account: &str,
    provider: &str,
    condition_id: &str,
    yes_amount: &str,
    no_amount: &str,
) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_liquidity_manager_id,
        true,
        &[
            "add_liquidity",
            "--provider",
            provider,
            "--condition-id",
            condition_id.as_str(),
            "--yes-amount",
            yes_amount,
            "--no-amount",
            no_amount,
        ],
    )
    .await
    .context("failed to add liquidity through Soroban liquidity manager")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn deposit_collateral(
    env: &Environment,
    source_account: &str,
    provider: &str,
    condition_id: &str,
    amount: &str,
) -> Result<ContractTxResult> {
    ensure_exchange_max_trade_amount(env, amount)
        .await
        .context("failed to raise Soroban exchange max trade amount for collateral deposit")?;
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_liquidity_manager_id,
        true,
        &[
            "deposit_collateral",
            "--provider",
            provider,
            "--condition-id",
            condition_id.as_str(),
            "--amount",
            amount,
        ],
    )
    .await
    .context("failed to deposit collateral through Soroban liquidity manager")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn remove_liquidity(
    env: &Environment,
    source_account: &str,
    provider: &str,
    condition_id: &str,
    yes_amount: &str,
    no_amount: &str,
) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_liquidity_manager_id,
        true,
        &[
            "remove_liquidity",
            "--provider",
            provider,
            "--condition-id",
            condition_id.as_str(),
            "--yes-amount",
            yes_amount,
            "--no-amount",
            no_amount,
        ],
    )
    .await
    .context("failed to remove liquidity through Soroban liquidity manager")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn withdraw_inventory(
    env: &Environment,
    source_account: &str,
    provider: &str,
    condition_id: &str,
    yes_amount: &str,
    no_amount: &str,
    recipient: &str,
) -> Result<ContractTxResult> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_liquidity_manager_id,
        true,
        &[
            "withdraw_inventory",
            "--provider",
            provider,
            "--condition-id",
            condition_id.as_str(),
            "--yes-amount",
            yes_amount,
            "--no-amount",
            no_amount,
            "--recipient",
            recipient,
        ],
    )
    .await
    .context("failed to withdraw inventory through Soroban liquidity manager")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn withdraw_collateral(
    env: &Environment,
    source_account: &str,
    provider: &str,
    condition_id: &str,
    amount: &str,
    recipient: &str,
) -> Result<ContractTxResult> {
    ensure_exchange_max_trade_amount(env, amount)
        .await
        .context("failed to raise Soroban exchange max trade amount for collateral withdrawal")?;
    let condition_id = bytes32_cli_arg(condition_id)?;
    invoke_contract_as_source(
        env,
        source_account,
        &env.sabi_liquidity_manager_id,
        true,
        &[
            "withdraw_collateral",
            "--provider",
            provider,
            "--condition-id",
            condition_id.as_str(),
            "--amount",
            amount,
            "--recipient",
            recipient,
        ],
    )
    .await
    .context("failed to withdraw collateral through Soroban liquidity manager")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn mint_mock_usdc(
    env: &Environment,
    recipient: &str,
    amount: &str,
) -> Result<ContractTxResult> {
    invoke_contract(
        env,
        &env.mock_usdc_id,
        true,
        &["mint", "--to", recipient, "--amount", amount],
    )
    .await
    .context("failed to mint Mock USDC on Soroban")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn get_mock_usdc_balance(env: &Environment, address: &str) -> Result<String> {
    invoke_contract(env, &env.mock_usdc_id, false, &["balance", "--id", address])
        .await
        .context("failed to read Mock USDC balance on Soroban")
}

pub async fn get_exchange_max_trade_amount(env: &Environment) -> Result<String> {
    invoke_contract(env, &env.sabi_exchange_id, false, &["get_max_trade_amount"])
        .await
        .context("failed to read Soroban exchange max trade amount")
}

pub async fn get_market_price_bps(
    env: &Environment,
    condition_id: &str,
    outcome_index: u32,
) -> Result<u32> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    let raw = invoke_contract(
        env,
        &env.sabi_exchange_id,
        false,
        &[
            "get_price",
            "--condition-id",
            condition_id.as_str(),
            "--outcome-index",
            &outcome_index.to_string(),
        ],
    )
    .await
    .context("failed to read Soroban market price")?;

    raw.parse::<u32>()
        .with_context(|| format!("invalid Soroban market price `{raw}`"))
}

pub async fn set_exchange_max_trade_amount(
    env: &Environment,
    amount: &str,
) -> Result<ContractTxResult> {
    invoke_contract(
        env,
        &env.sabi_exchange_id,
        true,
        &["set_max_trade_amount", "--amount", amount],
    )
    .await
    .context("failed to set Soroban exchange max trade amount")?;

    Ok(ContractTxResult {
        tx_hash: STELLAR_PLACEHOLDER_TX_HASH.to_owned(),
    })
}

pub async fn ensure_exchange_max_trade_amount(
    env: &Environment,
    minimum_amount: &str,
) -> Result<()> {
    let required = minimum_amount
        .parse::<u128>()
        .with_context(|| format!("invalid minimum exchange max trade amount `{minimum_amount}`"))?;
    if required == 0 {
        return Ok(());
    }

    let existing = get_exchange_max_trade_amount(env)
        .await?
        .parse::<u128>()
        .context("invalid Soroban exchange max trade amount")?;
    if existing >= required {
        return Ok(());
    }

    set_exchange_max_trade_amount(env, &required.to_string())
        .await
        .context("failed to raise Soroban exchange max trade amount")?;
    Ok(())
}

pub async fn ensure_mock_usdc_balance(
    env: &Environment,
    address: &str,
    minimum_amount: &str,
) -> Result<()> {
    if env.network != "testnet" {
        return Ok(());
    }

    let required = minimum_amount
        .parse::<u128>()
        .with_context(|| format!("invalid minimum mock USDC amount `{minimum_amount}`"))?;
    if required == 0 {
        return Ok(());
    }

    let existing = get_mock_usdc_balance(env, address)
        .await?
        .parse::<u128>()
        .with_context(|| format!("invalid mock USDC balance for `{address}`"))?;
    if existing >= required {
        return Ok(());
    }

    let missing = required - existing;
    match mint_mock_usdc(env, address, &missing.to_string()).await {
        Ok(_) => {}
        Err(error) if is_retryable_submission_error(&error) => {
            sleep(Duration::from_millis(1_500)).await;
            let refreshed = get_mock_usdc_balance(env, address)
                .await?
                .parse::<u128>()
                .with_context(|| format!("invalid mock USDC balance for `{address}`"))?;
            if refreshed < required {
                return Err(error)
                    .with_context(|| format!("failed to top up mock USDC for `{address}`"));
            }
        }
        Err(error) => {
            return Err(error).with_context(|| format!("failed to top up mock USDC for `{address}`"));
        }
    }
    Ok(())
}

fn is_retryable_submission_error(error: &anyhow::Error) -> bool {
    let message = format!("{error:#}").to_ascii_lowercase();
    message.contains("transaction submission timeout")
        || message.contains("txbadseq")
        || message.contains("bad seq")
}

pub async fn get_outcome_position_balance(
    env: &Environment,
    condition_id: &str,
    holder: &str,
    outcome_index: u32,
) -> Result<String> {
    let condition_id = bytes32_cli_arg(condition_id)?;
    let collection_id = invoke_contract(
        env,
        &env.sabi_ctf_id,
        false,
        &[
            "get_collection_id",
            "--parent-collection-id",
            "0000000000000000000000000000000000000000000000000000000000000000",
            "--condition-id",
            condition_id.as_str(),
            "--index-set",
            &(1u32 << outcome_index).to_string(),
        ],
    )
    .await
    .context("failed to derive CTF collection id")?;
    let position_id = invoke_contract(
        env,
        &env.sabi_ctf_id,
        false,
        &[
            "get_position_id",
            "--collateral-token",
            &env.mock_usdc_id,
            "--collection-id",
            collection_id.as_str(),
        ],
    )
    .await
    .context("failed to derive CTF position id")?;

    invoke_contract(
        env,
        &env.sabi_ctf_id,
        false,
        &[
            "get_position_balance",
            "--user",
            holder,
            "--position-id",
            position_id.as_str(),
        ],
    )
    .await
    .context("failed to read CTF position balance")
}

pub async fn get_market_prices_batch_best_effort(
    env: &Environment,
    condition_ids: &[String],
) -> Result<std::collections::HashMap<String, MarketPricesReadResult>> {
    let mut prices = std::collections::HashMap::new();
    for condition_id in condition_ids {
        let Ok(normalized) = bytes32_cli_arg(condition_id) else {
            continue;
        };

        let yes_bps = invoke_contract(
            env,
            &env.sabi_exchange_id,
            false,
            &[
                "get_price",
                "--condition-id",
                normalized.as_str(),
                "--outcome-index",
                "0",
            ],
        )
        .await;
        let no_bps = invoke_contract(
            env,
            &env.sabi_exchange_id,
            false,
            &[
                "get_price",
                "--condition-id",
                normalized.as_str(),
                "--outcome-index",
                "1",
            ],
        )
        .await;

        let (Ok(yes_bps), Ok(no_bps)) = (yes_bps, no_bps) else {
            continue;
        };
        let (Ok(yes_bps), Ok(no_bps)) = (yes_bps.parse::<u32>(), no_bps.parse::<u32>()) else {
            continue;
        };
        prices.insert(
            normalized,
            MarketPricesReadResult { yes_bps, no_bps },
        );
    }
    Ok(prices)
}

fn bool_arg(value: bool) -> &'static str {
    if value { "true" } else { "false" }
}

fn bytes32_cli_arg(value: &str) -> Result<String> {
    let normalized = value.trim();
    let normalized = normalized.trim_matches('"');
    let normalized = normalized.strip_prefix("0x").unwrap_or(normalized);
    let normalized = normalized.strip_prefix("0X").unwrap_or(normalized);

    if normalized.len() != 64 || !normalized.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(anyhow!("invalid bytes32 argument `{value}`"));
    }

    Ok(normalized.to_ascii_lowercase())
}

async fn invoke_contract(
    env: &Environment,
    contract_id: &str,
    send: bool,
    contract_args: &[&str],
) -> Result<String> {
    let source_account = env.private_key.as_deref().unwrap_or(&env.admin);
    invoke_contract_as_source(env, source_account, contract_id, send, contract_args).await
}

async fn invoke_contract_as_source(
    env: &Environment,
    source_account: &str,
    contract_id: &str,
    send: bool,
    contract_args: &[&str],
) -> Result<String> {
    let rpc = SorobanRpc::new(env);
    let (method, args) = contract_invocation_args(contract_args)?;

    if send {
        let secret_key = if source_account.trim().starts_with('S') {
            source_account
        } else {
            env.private_key
                .as_deref()
                .ok_or_else(|| anyhow!("PRIVATE_KEY not configured"))?
        };
        rpc.invoke(contract_id, method, &args, source_account, secret_key)
            .await
            .map(|response| response.value)
            .with_context(|| format!("failed to invoke `{method}` on `{contract_id}`"))
    } else {
        rpc.simulate(contract_id, method, &args)
            .await
            .with_context(|| format!("failed to simulate `{method}` on `{contract_id}`"))
    }
}

pub async fn submit_contract_as_source(
    env: &Environment,
    source_secret_key: &str,
    contract_id: &str,
    contract_args: &[&str],
) -> Result<ContractTxResult> {
    let rpc = SorobanRpc::new(env);
    let (method, args) = contract_invocation_args(contract_args)?;
    let response = rpc
        .invoke(contract_id, method, &args, source_secret_key, source_secret_key)
        .await
        .with_context(|| format!("failed to submit `{method}` on `{contract_id}` through RPC"))?;

    Ok(ContractTxResult {
        tx_hash: response.tx_hash,
    })
}

fn contract_invocation_args<'a>(contract_args: &'a [&'a str]) -> Result<(&'a str, Vec<(&'a str, &'a str)>)> {
    if contract_args.is_empty() {
        return Err(anyhow!("contract_args cannot be empty"));
    }

    let method = contract_args[0];
    let raw_args = &contract_args[1..];
    let args = raw_args
        .chunks(2)
        .filter(|chunk| chunk.len() == 2)
        .map(|chunk| {
            let key = chunk[0].strip_prefix("--").unwrap_or(chunk[0]);
            (key, chunk[1])
        })
        .collect();

    Ok((method, args))
}

struct LiquidityTotalsParsed {
    idle_yes_total: String,
    idle_no_total: String,
    posted_yes_total: String,
    posted_no_total: String,
    claimable_collateral_total: String,
}

struct LiquidityPositionParsed {
    posted_yes_amount: String,
    posted_no_amount: String,
    idle_yes_amount: String,
    idle_no_amount: String,
    collateral_amount: String,
    claimable_collateral_amount: String,
    timestamp: u64,
    active: bool,
}

fn parse_liquidity_totals(raw: &str) -> Result<LiquidityTotalsParsed> {
    let value: Value = serde_json::from_str(raw)
        .with_context(|| format!("failed to decode liquidity totals output: {raw}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("liquidity totals output was not an object"))?;

    Ok(LiquidityTotalsParsed {
        idle_yes_total: json_string_field(object, "idle_yes_total")?,
        idle_no_total: json_string_field(object, "idle_no_total")?,
        posted_yes_total: json_string_field(object, "posted_yes_total")?,
        posted_no_total: json_string_field(object, "posted_no_total")?,
        claimable_collateral_total: json_string_field(object, "claimable_collateral_total")?,
    })
}

fn parse_liquidity_position(raw: &str) -> Result<LiquidityPositionReadResult> {
    let parsed = parse_liquidity_position_fields(raw)?;
    Ok(LiquidityPositionReadResult {
        posted_yes_amount: parsed.posted_yes_amount,
        posted_no_amount: parsed.posted_no_amount,
        idle_yes_amount: parsed.idle_yes_amount,
        idle_no_amount: parsed.idle_no_amount,
        collateral_amount: parsed.collateral_amount,
        claimable_collateral_amount: parsed.claimable_collateral_amount,
        updated_at: timestamp_to_datetime(parsed.timestamp),
        active: parsed.active,
    })
}

fn parse_liquidity_position_fields(raw: &str) -> Result<LiquidityPositionParsed> {
    let value: Value = serde_json::from_str(raw)
        .with_context(|| format!("failed to decode liquidity position output: {raw}"))?;
    let object = value
        .as_object()
        .ok_or_else(|| anyhow!("liquidity position output was not an object"))?;

    Ok(LiquidityPositionParsed {
        posted_yes_amount: json_string_field(object, "yes_amount")?,
        posted_no_amount: json_string_field(object, "no_amount")?,
        idle_yes_amount: json_string_field(object, "idle_yes_amount")?,
        idle_no_amount: json_string_field(object, "idle_no_amount")?,
        collateral_amount: json_string_field(object, "collateral_amount")?,
        claimable_collateral_amount: json_string_field(object, "claimable_collateral_amount")?,
        timestamp: json_u64_field(object, "timestamp")?,
        active: json_bool_field(object, "active")?,
    })
}

fn json_string_field(
    object: &serde_json::Map<String, Value>,
    field: &str,
) -> Result<String> {
    let value = object
        .get(field)
        .ok_or_else(|| anyhow!("missing field `{field}` in contract output"))?;
    match value {
        Value::String(value) => Ok(value.clone()),
        Value::Number(value) => Ok(value.to_string()),
        _ => Err(anyhow!("unexpected value type for contract field `{field}`")),
    }
}

fn json_u64_field(object: &serde_json::Map<String, Value>, field: &str) -> Result<u64> {
    let value = object
        .get(field)
        .ok_or_else(|| anyhow!("missing field `{field}` in contract output"))?;
    match value {
        Value::String(value) => value
            .parse::<u64>()
            .with_context(|| format!("invalid integer for contract field `{field}`")),
        Value::Number(value) => value
            .as_u64()
            .ok_or_else(|| anyhow!("invalid integer for contract field `{field}`")),
        _ => Err(anyhow!("unexpected value type for contract field `{field}`")),
    }
}

fn json_bool_field(object: &serde_json::Map<String, Value>, field: &str) -> Result<bool> {
    let value = object
        .get(field)
        .ok_or_else(|| anyhow!("missing field `{field}` in contract output"))?;
    match value {
        Value::Bool(value) => Ok(*value),
        _ => Err(anyhow!("unexpected value type for contract field `{field}`")),
    }
}

fn timestamp_to_datetime(timestamp: u64) -> Option<DateTime<Utc>> {
    if timestamp == 0 {
        return None;
    }

    DateTime::from_timestamp(timestamp as i64, 0)
}
