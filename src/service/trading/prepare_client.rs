use anyhow::{Context, Result, anyhow};
use ethers_contract::Contract;
use ethers_core::types::{Address, Bytes, U256};
use ethers_providers::{Http, Provider};

use crate::{config::environment::Environment, service::rpc};

use super::{
    abi::{conditional_tokens_abi, erc20_abi, exchange_write_abi},
    write_support::{conditional_tokens_approval_query_reverted, parse_bytes32},
};

type ReadProvider = Provider<Http>;

pub(super) struct PreparedWalletCall {
    pub kind: &'static str,
    pub target: Address,
    pub data: Bytes,
    pub description: String,
}

pub(super) struct PrepareContracts {
    wallet_address: Address,
    exchange: Contract<ReadProvider>,
    usdc: Contract<ReadProvider>,
    conditional_tokens: Contract<ReadProvider>,
}

impl PrepareContracts {
    pub(super) async fn new(env: &Environment, wallet_address: &str) -> Result<Self> {
        let provider = rpc::monad_provider_arc(env).await?;
        let wallet_address = wallet_address
            .parse::<Address>()
            .context("invalid wallet address")?;

        Ok(Self {
            wallet_address,
            exchange: Contract::new(
                env.monad_exchange_address
                    .parse::<Address>()
                    .context("invalid MONAD_EXCHANGE_ADDRESS")?,
                exchange_write_abi()?,
                provider.clone(),
            ),
            usdc: Contract::new(
                env.monad_usdc_address
                    .parse::<Address>()
                    .context("invalid MONAD_USDC_ADDRESS")?,
                erc20_abi()?,
                provider.clone(),
            ),
            conditional_tokens: Contract::new(
                env.monad_conditional_tokens_address
                    .parse::<Address>()
                    .context("invalid MONAD_CONDITIONAL_TOKENS_ADDRESS")?,
                conditional_tokens_abi()?,
                provider,
            ),
        })
    }

    pub(super) async fn prepare_buy(
        &self,
        condition_id: &str,
        outcome_index: i32,
        usdc_amount: U256,
    ) -> Result<Vec<PreparedWalletCall>> {
        let mut calls = Vec::new();
        self.append_usdc_approval_if_needed(usdc_amount, &mut calls)
            .await?;
        calls.push(PreparedWalletCall {
            kind: "trade",
            target: self.exchange.address(),
            data: self.calldata_with_u256_return(
                &self.exchange,
                "buyOutcome",
                (
                    parse_bytes32(condition_id)?,
                    U256::from(outcome_index as u64),
                    usdc_amount,
                ),
            )?,
            description: "Execute buy against exchange liquidity".to_owned(),
        });
        Ok(calls)
    }

    pub(super) async fn prepare_sell(
        &self,
        condition_id: &str,
        outcome_index: i32,
        token_amount: U256,
    ) -> Result<Vec<PreparedWalletCall>> {
        let mut calls = Vec::new();
        self.append_ctf_approval_if_needed(&mut calls).await?;
        calls.push(PreparedWalletCall {
            kind: "trade",
            target: self.exchange.address(),
            data: self.calldata_with_u256_return(
                &self.exchange,
                "sellOutcome",
                (
                    parse_bytes32(condition_id)?,
                    U256::from(outcome_index as u64),
                    token_amount,
                ),
            )?,
            description: "Execute sell against exchange liquidity".to_owned(),
        });
        Ok(calls)
    }

    pub(super) async fn prepare_split(
        &self,
        condition_id: &str,
        collateral_amount: U256,
    ) -> Result<Vec<PreparedWalletCall>> {
        let mut calls = Vec::new();
        self.append_usdc_approval_if_needed_for_target(
            self.conditional_tokens.address(),
            collateral_amount,
            &mut calls,
        )
        .await?;
        calls.push(PreparedWalletCall {
            kind: "conversion",
            target: self.conditional_tokens.address(),
            data: self.calldata_no_return(
                &self.conditional_tokens,
                "splitPosition",
                (
                    self.usdc.address(),
                    [0_u8; 32],
                    parse_bytes32(condition_id)?,
                    binary_partition(),
                    collateral_amount,
                ),
            )?,
            description: "Split collateral into YES and NO tokens".to_owned(),
        });
        Ok(calls)
    }

    pub(super) async fn prepare_merge(
        &self,
        condition_id: &str,
        pair_token_amount: U256,
    ) -> Result<Vec<PreparedWalletCall>> {
        Ok(vec![PreparedWalletCall {
            kind: "conversion",
            target: self.conditional_tokens.address(),
            data: self.calldata_no_return(
                &self.conditional_tokens,
                "mergePositions",
                (
                    self.usdc.address(),
                    [0_u8; 32],
                    parse_bytes32(condition_id)?,
                    binary_partition(),
                    pair_token_amount,
                ),
            )?,
            description: "Merge YES and NO tokens back into collateral".to_owned(),
        }])
    }

    async fn append_usdc_approval_if_needed(
        &self,
        minimum_allowance: U256,
        calls: &mut Vec<PreparedWalletCall>,
    ) -> Result<()> {
        self.append_usdc_approval_if_needed_for_target(
            self.exchange.address(),
            minimum_allowance,
            calls,
        )
        .await
    }

    async fn append_usdc_approval_if_needed_for_target(
        &self,
        target: Address,
        minimum_allowance: U256,
        calls: &mut Vec<PreparedWalletCall>,
    ) -> Result<()> {
        let allowance = self
            .usdc
            .method::<_, U256>("allowance", (self.wallet_address, target))?
            .call()
            .await
            .context("failed to query USDC allowance")?;
        if allowance >= minimum_allowance {
            return Ok(());
        }

        calls.push(PreparedWalletCall {
            kind: "approval",
            target: self.usdc.address(),
            data: self.calldata_with_bool_return(&self.usdc, "approve", (target, U256::MAX))?,
            description: "Approve USDC for the target contract".to_owned(),
        });
        Ok(())
    }

    async fn append_ctf_approval_if_needed(
        &self,
        calls: &mut Vec<PreparedWalletCall>,
    ) -> Result<()> {
        let approval_query = self.conditional_tokens.method::<_, bool>(
            "isApprovedForAll",
            (self.wallet_address, self.exchange.address()),
        )?;
        let approved = match approval_query.call().await {
            Ok(approved) => approved,
            Err(error) if conditional_tokens_approval_query_reverted(&error) => {
                tracing::warn!(
                    %error,
                    wallet = %self.wallet_address,
                    operator = %self.exchange.address(),
                    "ConditionalTokens.isApprovedForAll reverted; falling back to unconditional approval"
                );
                false
            }
            Err(error) => {
                return Err(error).context("failed to query ConditionalTokens.isApprovedForAll");
            }
        };
        if approved {
            return Ok(());
        }

        calls.push(PreparedWalletCall {
            kind: "approval",
            target: self.conditional_tokens.address(),
            data: self.calldata_no_return(
                &self.conditional_tokens,
                "setApprovalForAll",
                (self.exchange.address(), true),
            )?,
            description: "Approve conditional tokens for the exchange".to_owned(),
        });
        Ok(())
    }

    fn calldata_no_return<T>(
        &self,
        contract: &Contract<ReadProvider>,
        method: &str,
        data: T,
    ) -> Result<Bytes>
    where
        T: ethers_core::abi::Tokenize,
    {
        contract
            .method::<_, ()>(method, data)?
            .calldata()
            .ok_or_else(|| anyhow!("missing calldata for `{method}`"))
    }

    fn calldata_with_bool_return<T>(
        &self,
        contract: &Contract<ReadProvider>,
        method: &str,
        data: T,
    ) -> Result<Bytes>
    where
        T: ethers_core::abi::Tokenize,
    {
        contract
            .method::<_, bool>(method, data)?
            .calldata()
            .ok_or_else(|| anyhow!("missing calldata for `{method}`"))
    }

    fn calldata_with_u256_return<T>(
        &self,
        contract: &Contract<ReadProvider>,
        method: &str,
        data: T,
    ) -> Result<Bytes>
    where
        T: ethers_core::abi::Tokenize,
    {
        contract
            .method::<_, U256>(method, data)?
            .calldata()
            .ok_or_else(|| anyhow!("missing calldata for `{method}`"))
    }
}

fn binary_partition() -> Vec<U256> {
    vec![U256::from(1_u64), U256::from(2_u64)]
}
