use anyhow::Result;
use ethers_core::types::U256;
use uuid::Uuid;

use crate::{app::AppState, service::aa::submit_gasless_transaction};

pub struct TradeTxResult {
    pub tx_hash: String,
}

pub async fn buy_outcome(
    state: &AppState,
    user_id: Uuid,
    condition_id: &str,
    outcome_index: i32,
    usdc_amount: U256,
) -> Result<TradeTxResult> {
    let tx_hash = submit_gasless_transaction(
        state,
        user_id,
        &state.env.mock_usdc_id, // Wait, trading! Probably a trading contract id or same liquidity manager
        &[
            "buy_outcome",
            "--condition-id",
            condition_id,
            "--outcome-index",
            &outcome_index.to_string(),
            "--usdc-amount",
            &usdc_amount.to_string(),
        ],
    )
    .await?;
    Ok(TradeTxResult { tx_hash })
}

pub async fn sell_outcome(
    state: &AppState,
    user_id: Uuid,
    condition_id: &str,
    outcome_index: i32,
    token_amount: U256,
) -> Result<TradeTxResult> {
    let tx_hash = submit_gasless_transaction(
        state,
        user_id,
        &state.env.mock_usdc_id, // Wait, trading contract id
        &[
            "sell_outcome",
            "--condition-id",
            condition_id,
            "--outcome-index",
            &outcome_index.to_string(),
            "--token-amount",
            &token_amount.to_string(),
        ],
    )
    .await?;
    Ok(TradeTxResult { tx_hash })
}

pub async fn split_position(
    state: &AppState,
    user_id: Uuid,
    condition_id: &str,
    collateral_amount: U256,
) -> Result<TradeTxResult> {
    let tx_hash = submit_gasless_transaction(
        state,
        user_id,
        &state.env.mock_usdc_id, // Wait, conditional tokens id
        &[
            "split_position",
            "--condition-id",
            condition_id,
            "--collateral-amount",
            &collateral_amount.to_string(),
        ],
    )
    .await?;
    Ok(TradeTxResult { tx_hash })
}

pub async fn merge_positions(
    state: &AppState,
    user_id: Uuid,
    condition_id: &str,
    pair_token_amount: U256,
) -> Result<TradeTxResult> {
    let tx_hash = submit_gasless_transaction(
        state,
        user_id,
        &state.env.mock_usdc_id, // Wait, conditional tokens id
        &[
            "merge_positions",
            "--condition-id",
            condition_id,
            "--pair-token-amount",
            &pair_token_amount.to_string(),
        ],
    )
    .await?;
    Ok(TradeTxResult { tx_hash })
}
