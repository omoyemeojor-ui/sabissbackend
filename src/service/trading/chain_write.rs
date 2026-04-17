use anyhow::Result;
use ethers_core::types::U256;
use reqwest::Client;

use crate::{config::environment::Environment, service::aa::SmartAccountSignerContext};

use super::write_client::WriteContracts;

pub struct TradeTxResult {
    pub tx_hash: String,
}

pub async fn buy_outcome(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    condition_id: &str,
    outcome_index: i32,
    usdc_amount: U256,
) -> Result<TradeTxResult> {
    let tx_hash = WriteContracts::new(env, http_client, signer)
        .await?
        .buy_outcome(condition_id, outcome_index, usdc_amount)
        .await?;
    Ok(TradeTxResult { tx_hash })
}

pub async fn sell_outcome(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    condition_id: &str,
    outcome_index: i32,
    token_amount: U256,
) -> Result<TradeTxResult> {
    let tx_hash = WriteContracts::new(env, http_client, signer)
        .await?
        .sell_outcome(condition_id, outcome_index, token_amount)
        .await?;
    Ok(TradeTxResult { tx_hash })
}

pub async fn split_position(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    condition_id: &str,
    collateral_amount: U256,
) -> Result<TradeTxResult> {
    let tx_hash = WriteContracts::new(env, http_client, signer)
        .await?
        .split_position(condition_id, collateral_amount)
        .await?;
    Ok(TradeTxResult { tx_hash })
}

pub async fn merge_positions(
    env: &Environment,
    http_client: &Client,
    signer: &SmartAccountSignerContext,
    condition_id: &str,
    pair_token_amount: U256,
) -> Result<TradeTxResult> {
    let tx_hash = WriteContracts::new(env, http_client, signer)
        .await?
        .merge_positions(condition_id, pair_token_amount)
        .await?;
    Ok(TradeTxResult { tx_hash })
}
