use std::str::FromStr;

use anyhow::{Context, Result, anyhow};
use chrono::{DateTime, Utc};
use ethers_contract::Contract;
use ethers_core::types::{Address, H256, U256};
use ethers_providers::{Http, Provider};

use crate::{config::environment::Environment, service::rpc};

use super::abi::liquidity_manager_read_abi;

type ReadProvider = Provider<Http>;

#[derive(Debug, Clone)]
pub struct LiquidityTotalsReadResult {
    pub idle_yes_total: U256,
    pub idle_no_total: U256,
    pub posted_yes_total: U256,
    pub posted_no_total: U256,
    pub claimable_collateral_total: U256,
}

#[derive(Debug, Clone, Default)]
pub struct LiquidityPositionReadResult {
    pub posted_yes_amount: U256,
    pub posted_no_amount: U256,
    pub idle_yes_amount: U256,
    pub idle_no_amount: U256,
    pub collateral_amount: U256,
    pub claimable_collateral_amount: U256,
    pub updated_at: Option<DateTime<Utc>>,
    pub active: bool,
}

pub async fn get_event_liquidity(
    env: &Environment,
    event_id: &str,
) -> Result<LiquidityTotalsReadResult> {
    ReadContracts::new(env)
        .await?
        .get_event_liquidity(event_id)
        .await
}

pub async fn get_market_liquidity(
    env: &Environment,
    condition_id: &str,
) -> Result<LiquidityTotalsReadResult> {
    ReadContracts::new(env)
        .await?
        .get_market_liquidity(condition_id)
        .await
}

pub async fn get_liquidity_position(
    env: &Environment,
    condition_id: &str,
    provider: &str,
) -> Result<LiquidityPositionReadResult> {
    ReadContracts::new(env)
        .await?
        .get_liquidity_position(condition_id, provider)
        .await
}

struct ReadContracts {
    liquidity_manager: Contract<ReadProvider>,
}

impl ReadContracts {
    async fn new(env: &Environment) -> Result<Self> {
        let provider = rpc::monad_provider_arc(env).await?;
        let liquidity_manager = Contract::new(
            env.monad_liquidity_manager_address
                .parse::<Address>()
                .context("invalid MONAD_LIQUIDITY_MANAGER_ADDRESS")?,
            liquidity_manager_read_abi()?,
            provider,
        );

        Ok(Self { liquidity_manager })
    }

    async fn get_event_liquidity(&self, event_id: &str) -> Result<LiquidityTotalsReadResult> {
        let event_id = parse_bytes32(event_id)?;
        let values = self
            .liquidity_manager
            .method::<_, (U256, U256, U256, U256, U256)>("getEventLiquidity", event_id)?
            .call()
            .await
            .context("failed to query LiquidityManager.getEventLiquidity")?;

        Ok(liquidity_totals(values))
    }

    async fn get_market_liquidity(&self, condition_id: &str) -> Result<LiquidityTotalsReadResult> {
        let condition_id = parse_bytes32(condition_id)?;
        let values = self
            .liquidity_manager
            .method::<_, (U256, U256, U256, U256, U256)>("getMarketLiquidity", condition_id)?
            .call()
            .await
            .context("failed to query LiquidityManager.getMarketLiquidity")?;

        Ok(liquidity_totals(values))
    }

    async fn get_liquidity_position(
        &self,
        condition_id: &str,
        provider: &str,
    ) -> Result<LiquidityPositionReadResult> {
        let condition_id = parse_bytes32(condition_id)?;
        let provider = parse_address(provider)?;
        let values = self
            .liquidity_manager
            .method::<_, (U256, U256, U256, U256, U256, U256, U256, bool)>(
                "getLiquidityPosition",
                (condition_id, provider),
            )?
            .call()
            .await
            .context("failed to query LiquidityManager.getLiquidityPosition")?;

        Ok(LiquidityPositionReadResult {
            posted_yes_amount: values.0,
            posted_no_amount: values.1,
            idle_yes_amount: values.2,
            idle_no_amount: values.3,
            collateral_amount: values.4,
            claimable_collateral_amount: values.5,
            updated_at: timestamp_to_datetime(values.6),
            active: values.7,
        })
    }
}

fn liquidity_totals(values: (U256, U256, U256, U256, U256)) -> LiquidityTotalsReadResult {
    LiquidityTotalsReadResult {
        idle_yes_total: values.0,
        idle_no_total: values.1,
        posted_yes_total: values.2,
        posted_no_total: values.3,
        claimable_collateral_total: values.4,
    }
}

fn parse_bytes32(value: &str) -> Result<H256> {
    H256::from_str(value).map_err(|error| anyhow!("invalid bytes32 value: {error}"))
}

fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).map_err(|error| anyhow!("invalid address value: {error}"))
}

fn timestamp_to_datetime(value: U256) -> Option<DateTime<Utc>> {
    let seconds = value.to_string().parse::<i64>().ok()?;
    DateTime::<Utc>::from_timestamp(seconds, 0)
}
