use anyhow::Result;
use ethers_core::types::U256;

use crate::{
    config::environment::Environment, module::market::trade_schema::PreparedWalletCallResponse,
};

use super::prepare_client::{PrepareContracts, PreparedWalletCall};

pub async fn prepare_buy_outcome(
    env: &Environment,
    wallet_address: &str,
    condition_id: &str,
    outcome_index: i32,
    usdc_amount: U256,
) -> Result<Vec<PreparedWalletCallResponse>> {
    let calls = PrepareContracts::new(env, wallet_address)
        .await?
        .prepare_buy(condition_id, outcome_index, usdc_amount)
        .await?;
    Ok(calls.into_iter().map(map_call).collect())
}

pub async fn prepare_sell_outcome(
    env: &Environment,
    wallet_address: &str,
    condition_id: &str,
    outcome_index: i32,
    token_amount: U256,
) -> Result<Vec<PreparedWalletCallResponse>> {
    let calls = PrepareContracts::new(env, wallet_address)
        .await?
        .prepare_sell(condition_id, outcome_index, token_amount)
        .await?;
    Ok(calls.into_iter().map(map_call).collect())
}

pub async fn prepare_split_position(
    env: &Environment,
    wallet_address: &str,
    condition_id: &str,
    collateral_amount: U256,
) -> Result<Vec<PreparedWalletCallResponse>> {
    let calls = PrepareContracts::new(env, wallet_address)
        .await?
        .prepare_split(condition_id, collateral_amount)
        .await?;
    Ok(calls.into_iter().map(map_call).collect())
}

pub async fn prepare_merge_positions(
    env: &Environment,
    wallet_address: &str,
    condition_id: &str,
    pair_token_amount: U256,
) -> Result<Vec<PreparedWalletCallResponse>> {
    let calls = PrepareContracts::new(env, wallet_address)
        .await?
        .prepare_merge(condition_id, pair_token_amount)
        .await?;
    Ok(calls.into_iter().map(map_call).collect())
}

fn map_call(call: PreparedWalletCall) -> PreparedWalletCallResponse {
    PreparedWalletCallResponse {
        kind: call.kind.to_owned(),
        target: format!("{:#x}", call.target),
        data: format!("0x{}", hex::encode(call.data.as_ref())),
        value: "0".to_owned(),
        description: call.description,
    }
}
