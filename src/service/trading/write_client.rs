use anyhow::{Context, Result, anyhow};
use ethers_contract::Contract;
use ethers_core::types::{Address, Bytes, U256};
use ethers_providers::{Http, Provider};
use reqwest::Client;

use crate::{
    config::environment::Environment,
    service::aa::{
        SmartAccountCall, SmartAccountExecutionResult, SmartAccountSignerContext, submit_calls,
    },
    service::rpc,
};

use super::{
    abi::{conditional_tokens_abi, erc20_abi, exchange_write_abi},
    write_support::{conditional_tokens_approval_query_reverted, parse_bytes32},
};

type ReadProvider = Provider<Http>;

pub(super) struct WriteContracts {
    env: Environment,
    http_client: Client,
    signer: SmartAccountSignerContext,
    wallet_address: Address,
    exchange: Contract<ReadProvider>,
    usdc: Contract<ReadProvider>,
    conditional_tokens: Contract<ReadProvider>,
}

impl WriteContracts {
    pub(super) async fn new(
        env: &Environment,
        http_client: &Client,
        signer: &SmartAccountSignerContext,
    ) -> Result<Self> {
        let provider = rpc::monad_provider_arc(env).await?;
        let wallet_address = signer
            .wallet_address
            .parse::<Address>()
            .context("invalid smart-account wallet address")?;

        Ok(Self {
            env: env.clone(),
            http_client: http_client.clone(),
            signer: signer.clone(),
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

    pub(super) async fn buy_outcome(
        &self,
        condition_id: &str,
        outcome_index: i32,
        usdc_amount: U256,
    ) -> Result<String> {
        let mut calls = Vec::new();
        self.append_usdc_allowance_call(usdc_amount, &mut calls)
            .await?;
        calls.push(SmartAccountCall {
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
        });
        self.submit(calls).await
    }

    pub(super) async fn sell_outcome(
        &self,
        condition_id: &str,
        outcome_index: i32,
        token_amount: U256,
    ) -> Result<String> {
        let mut calls = Vec::new();
        self.append_ctf_approval_call(&mut calls).await?;
        calls.push(SmartAccountCall {
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
        });
        self.submit(calls).await
    }

    pub(super) async fn split_position(
        &self,
        condition_id: &str,
        collateral_amount: U256,
    ) -> Result<String> {
        let mut calls = Vec::new();
        self.append_usdc_allowance_for_target_call(
            self.conditional_tokens.address(),
            collateral_amount,
            &mut calls,
        )
        .await?;
        calls.push(SmartAccountCall {
            target: self.conditional_tokens.address(),
            data: self.calldata_no_return(
                &self.conditional_tokens,
                "splitPosition",
                (
                    self.env.monad_usdc_address.parse::<Address>()?,
                    [0_u8; 32],
                    parse_bytes32(condition_id)?,
                    binary_partition(),
                    collateral_amount,
                ),
            )?,
        });
        self.submit(calls).await
    }

    pub(super) async fn merge_positions(
        &self,
        condition_id: &str,
        pair_token_amount: U256,
    ) -> Result<String> {
        self.submit(vec![SmartAccountCall {
            target: self.conditional_tokens.address(),
            data: self.calldata_no_return(
                &self.conditional_tokens,
                "mergePositions",
                (
                    self.env.monad_usdc_address.parse::<Address>()?,
                    [0_u8; 32],
                    parse_bytes32(condition_id)?,
                    binary_partition(),
                    pair_token_amount,
                ),
            )?,
        }])
        .await
    }

    async fn append_usdc_allowance_call(
        &self,
        minimum_allowance: U256,
        calls: &mut Vec<SmartAccountCall>,
    ) -> Result<()> {
        self.append_usdc_allowance_for_target_call(
            self.exchange.address(),
            minimum_allowance,
            calls,
        )
        .await
    }

    async fn append_usdc_allowance_for_target_call(
        &self,
        target: Address,
        minimum_allowance: U256,
        calls: &mut Vec<SmartAccountCall>,
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

        calls.push(SmartAccountCall {
            target: self.usdc.address(),
            data: self.calldata_with_bool_return(&self.usdc, "approve", (target, U256::MAX))?,
        });
        Ok(())
    }

    async fn append_ctf_approval_call(&self, calls: &mut Vec<SmartAccountCall>) -> Result<()> {
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

        calls.push(SmartAccountCall {
            target: self.conditional_tokens.address(),
            data: self.calldata_no_return(
                &self.conditional_tokens,
                "setApprovalForAll",
                (self.exchange.address(), true),
            )?,
        });
        Ok(())
    }

    async fn submit(&self, calls: Vec<SmartAccountCall>) -> Result<String> {
        if calls.is_empty() {
            return Err(anyhow!(
                "smart-account execution requires at least one call"
            ));
        }

        let mut latest_tx_hash = None;

        // The deployed account implementation accepts single-call execution, but
        // multi-call batch simulation reverts during sponsorship. Submit approval
        // and trade actions sequentially so first-time trades can still proceed.
        for call in calls {
            let SmartAccountExecutionResult { tx_hash } =
                submit_calls(&self.env, &self.http_client, &self.signer, &[call]).await?;
            latest_tx_hash = Some(tx_hash);
        }

        latest_tx_hash
            .ok_or_else(|| anyhow!("smart-account execution did not return a transaction hash"))
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
