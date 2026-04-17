use anyhow::Result;
use ethers_core::types::U256;
use reqwest::Client;

use crate::{config::environment::Environment, service::aa::SmartAccountSignerContext};

use super::write_client::WriteContracts;

pub struct LiquidityTxResult {
    pub tx_hash: String,
}

pub async fn deposit_inventory(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    condition_id: &str,
    yes_amount: U256,
    no_amount: U256,
) -> Result<LiquidityTxResult> {
    let tx_hash = WriteContracts::new(env, http_client, signer)
        .await?
        .deposit_inventory(condition_id, yes_amount, no_amount)
        .await?;
    Ok(LiquidityTxResult { tx_hash })
}

pub async fn deposit_collateral(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    condition_id: &str,
    amount: U256,
) -> Result<LiquidityTxResult> {
    let tx_hash = WriteContracts::new(env, http_client, signer)
        .await?
        .deposit_collateral(condition_id, amount)
        .await?;
    Ok(LiquidityTxResult { tx_hash })
}

pub async fn remove_liquidity(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    condition_id: &str,
    yes_amount: U256,
    no_amount: U256,
) -> Result<LiquidityTxResult> {
    let tx_hash = WriteContracts::new(env, http_client, signer)
        .await?
        .remove_liquidity(condition_id, yes_amount, no_amount)
        .await?;
    Ok(LiquidityTxResult { tx_hash })
}

pub async fn withdraw_inventory(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    condition_id: &str,
    yes_amount: U256,
    no_amount: U256,
    recipient: &str,
) -> Result<LiquidityTxResult> {
    let tx_hash = WriteContracts::new(env, http_client, signer)
        .await?
        .withdraw_inventory(condition_id, yes_amount, no_amount, recipient)
        .await?;
    Ok(LiquidityTxResult { tx_hash })
}

pub async fn withdraw_collateral(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    condition_id: &str,
    amount: U256,
    recipient: &str,
) -> Result<LiquidityTxResult> {
    let tx_hash = WriteContracts::new(env, http_client, signer)
        .await?
        .withdraw_collateral(condition_id, amount, recipient)
        .await?;
    Ok(LiquidityTxResult { tx_hash })
}
