use anyhow::Result;
use ethers_core::types::U256;
use uuid::Uuid;

use crate::{app::AppState, service::aa::submit_gasless_transaction};

pub struct LiquidityTxResult {
    pub tx_hash: String,
}

pub async fn deposit_inventory(
    state: &AppState,
    user_id: Uuid,
    condition_id: &str,
    yes_amount: U256,
    no_amount: U256,
) -> Result<LiquidityTxResult> {
    let tx_hash = submit_gasless_transaction(
        state,
        user_id,
        &state.env.mock_usdc_id, // Wait, liquidity manager!
        &[
            "deposit_inventory",
            "--condition-id",
            condition_id,
            "--yes-amount",
            &yes_amount.to_string(),
            "--no-amount",
            &no_amount.to_string(),
        ],
    )
    .await?;
    Ok(LiquidityTxResult { tx_hash })
}

pub async fn deposit_collateral(
    state: &AppState,
    user_id: Uuid,
    condition_id: &str,
    amount: U256,
) -> Result<LiquidityTxResult> {
    let tx_hash = submit_gasless_transaction(
        state,
        user_id,
        &state.env.mock_usdc_id, // Wait, liquidity manager ID
        &[
            "deposit_collateral",
            "--condition-id",
            condition_id,
            "--amount",
            &amount.to_string(),
        ],
    )
    .await?;
    Ok(LiquidityTxResult { tx_hash })
}

pub async fn remove_liquidity(
    state: &AppState,
    user_id: Uuid,
    condition_id: &str,
    yes_amount: U256,
    no_amount: U256,
) -> Result<LiquidityTxResult> {
    let tx_hash = submit_gasless_transaction(
        state,
        user_id,
        &state.env.mock_usdc_id, // Liquidity manager ID
        &[
            "remove_liquidity",
            "--condition-id",
            condition_id,
            "--yes-amount",
            &yes_amount.to_string(),
            "--no-amount",
            &no_amount.to_string(),
        ],
    )
    .await?;
    Ok(LiquidityTxResult { tx_hash })
}

pub async fn withdraw_inventory(
    state: &AppState,
    user_id: Uuid,
    condition_id: &str,
    yes_amount: U256,
    no_amount: U256,
    recipient: &str,
) -> Result<LiquidityTxResult> {
    let tx_hash = submit_gasless_transaction(
        state,
        user_id,
        &state.env.mock_usdc_id, // Liquidity manager ID
        &[
            "withdraw_inventory",
            "--condition-id",
            condition_id,
            "--yes-amount",
            &yes_amount.to_string(),
            "--no-amount",
            &no_amount.to_string(),
            "--recipient",
            recipient,
        ],
    )
    .await?;
    Ok(LiquidityTxResult { tx_hash })
}

pub async fn withdraw_collateral(
    state: &AppState,
    user_id: Uuid,
    condition_id: &str,
    amount: U256,
    recipient: &str,
) -> Result<LiquidityTxResult> {
    let tx_hash = submit_gasless_transaction(
        state,
        user_id,
        &state.env.mock_usdc_id, // Liquidity manager ID
        &[
            "withdraw_collateral",
            "--condition-id",
            condition_id,
            "--amount",
            &amount.to_string(),
            "--recipient",
            recipient,
        ],
    )
    .await?;
    Ok(LiquidityTxResult { tx_hash })
}
