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

use super::abi::{conditional_tokens_abi, erc20_abi, liquidity_manager_write_abi};
use super::write_support::{
    conditional_tokens_approval_query_reverted, market_maker_role, parse_address, parse_bytes32,
};

pub(super) type ReadProvider = Provider<Http>;

pub(super) struct WriteContracts {
    env: Environment,
    http_client: Client,
    signer: SmartAccountSignerContext,
    wallet_address: Address,
    liquidity_manager: Contract<ReadProvider>,
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
            liquidity_manager: Contract::new(
                env.monad_liquidity_manager_address
                    .parse::<Address>()
                    .context("invalid MONAD_LIQUIDITY_MANAGER_ADDRESS")?,
                liquidity_manager_write_abi()?,
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

    pub(super) async fn deposit_inventory(
        &self,
        condition_id: &str,
        yes_amount: U256,
        no_amount: U256,
    ) -> Result<String> {
        self.ensure_market_maker_role().await?;
        let mut calls = Vec::new();
        self.append_ctf_approval_call(&mut calls).await?;
        calls.push(SmartAccountCall {
            target: self.liquidity_manager.address(),
            data: self.calldata_no_return(
                &self.liquidity_manager,
                "depositInventory",
                (parse_bytes32(condition_id)?, yes_amount, no_amount),
            )?,
        });
        self.submit(calls).await
    }

    pub(super) async fn deposit_collateral(
        &self,
        condition_id: &str,
        amount: U256,
    ) -> Result<String> {
        self.ensure_market_maker_role().await?;
        let mut calls = Vec::new();
        self.append_usdc_allowance_call(amount, &mut calls).await?;
        calls.push(SmartAccountCall {
            target: self.liquidity_manager.address(),
            data: self.calldata_no_return(
                &self.liquidity_manager,
                "depositCollateral",
                (parse_bytes32(condition_id)?, amount),
            )?,
        });
        self.submit(calls).await
    }

    pub(super) async fn remove_liquidity(
        &self,
        condition_id: &str,
        yes_amount: U256,
        no_amount: U256,
    ) -> Result<String> {
        self.ensure_market_maker_role().await?;
        self.submit(vec![SmartAccountCall {
            target: self.liquidity_manager.address(),
            data: self.calldata_no_return(
                &self.liquidity_manager,
                "removeLiquidity",
                (parse_bytes32(condition_id)?, yes_amount, no_amount),
            )?,
        }])
        .await
    }

    pub(super) async fn withdraw_inventory(
        &self,
        condition_id: &str,
        yes_amount: U256,
        no_amount: U256,
        recipient: &str,
    ) -> Result<String> {
        self.ensure_market_maker_role().await?;
        self.submit(vec![SmartAccountCall {
            target: self.liquidity_manager.address(),
            data: self.calldata_no_return(
                &self.liquidity_manager,
                "withdrawInventory",
                (
                    parse_bytes32(condition_id)?,
                    yes_amount,
                    no_amount,
                    parse_address(recipient)?,
                ),
            )?,
        }])
        .await
    }

    pub(super) async fn withdraw_collateral(
        &self,
        condition_id: &str,
        amount: U256,
        recipient: &str,
    ) -> Result<String> {
        self.ensure_market_maker_role().await?;
        self.submit(vec![SmartAccountCall {
            target: self.liquidity_manager.address(),
            data: self.calldata_no_return(
                &self.liquidity_manager,
                "withdrawCollateral",
                (
                    parse_bytes32(condition_id)?,
                    amount,
                    parse_address(recipient)?,
                ),
            )?,
        }])
        .await
    }

    async fn ensure_market_maker_role(&self) -> Result<()> {
        let has_role = self
            .liquidity_manager
            .method::<_, bool>("hasRole", (market_maker_role(), self.wallet_address))?
            .call()
            .await
            .context("failed to query LiquidityManager.hasRole")?;
        if has_role {
            return Ok(());
        }

        Err(anyhow!("smart account does not have MARKET_MAKER_ROLE"))
    }

    async fn append_usdc_allowance_call(
        &self,
        minimum_allowance: U256,
        calls: &mut Vec<SmartAccountCall>,
    ) -> Result<()> {
        let allowance = self
            .usdc
            .method::<_, U256>(
                "allowance",
                (self.wallet_address, self.liquidity_manager.address()),
            )?
            .call()
            .await
            .context("failed to query USDC allowance")?;
        if allowance >= minimum_allowance {
            return Ok(());
        }

        calls.push(SmartAccountCall {
            target: self.usdc.address(),
            data: self.calldata_with_bool_return(
                &self.usdc,
                "approve",
                (self.liquidity_manager.address(), U256::MAX),
            )?,
        });
        Ok(())
    }

    async fn append_ctf_approval_call(&self, calls: &mut Vec<SmartAccountCall>) -> Result<()> {
        let approval_query = self.conditional_tokens.method::<_, bool>(
            "isApprovedForAll",
            (self.wallet_address, self.liquidity_manager.address()),
        )?;
        let approved = match approval_query.call().await {
            Ok(approved) => approved,
            Err(error) if conditional_tokens_approval_query_reverted(&error) => {
                tracing::warn!(
                    %error,
                    wallet = %self.wallet_address,
                    operator = %self.liquidity_manager.address(),
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
                (self.liquidity_manager.address(), true),
            )?,
        });
        Ok(())
    }

    async fn submit(&self, calls: Vec<SmartAccountCall>) -> Result<String> {
        let SmartAccountExecutionResult { tx_hash } =
            submit_calls(&self.env, &self.http_client, &self.signer, &calls).await?;
        Ok(tx_hash)
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
}
