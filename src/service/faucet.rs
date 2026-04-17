use anyhow::{Context, Result, anyhow};
use chrono::Utc;
use ethers_contract::Contract;
use ethers_core::{
    abi::AbiParser,
    types::{Address, U64, U256},
};
use ethers_middleware::SignerMiddleware;
use ethers_providers::{Http, Provider};
use ethers_signers::{LocalWallet, Signer};

use crate::{
    app::AppState,
    config::environment::Environment,
    module::{
        auth::error::AuthError,
        faucet::schema::{
            FaucetUsdcBalanceQuery, FaucetUsdcBalanceResponse, FaucetUsdcRequest,
            FaucetUsdcResponse,
        },
    },
    service::{auth::normalize_wallet_address, rpc},
};

type FaucetWriteMiddleware = SignerMiddleware<Provider<Http>, LocalWallet>;
type FaucetReadProvider = Provider<Http>;

pub async fn request_usdc_faucet(
    state: &AppState,
    payload: FaucetUsdcRequest,
) -> Result<FaucetUsdcResponse, AuthError> {
    let recipient = normalize_wallet_address(&payload.address)?;
    let amount = parse_amount(&payload.amount)?;
    let tx_hash = FaucetWriteContracts::new(&state.env)
        .await
        .map_err(|error| AuthError::internal("usdc faucet setup failed", error))?
        .mint_usdc(&recipient, amount)
        .await
        .map_err(|error| map_faucet_error("usdc faucet mint failed", error))?;

    Ok(FaucetUsdcResponse {
        token_address: state.env.monad_usdc_address.clone(),
        recipient,
        amount: amount.to_string(),
        tx_hash,
        requested_at: Utc::now(),
    })
}

pub async fn get_mock_usdc_balance(
    state: &AppState,
    query: FaucetUsdcBalanceQuery,
) -> Result<FaucetUsdcBalanceResponse, AuthError> {
    let address = normalize_wallet_address(&query.address)?;
    let balance = read_usdc_balance(state, &address).await?;

    Ok(FaucetUsdcBalanceResponse {
        token_address: state.env.monad_usdc_address.clone(),
        address,
        balance: balance.to_string(),
        queried_at: Utc::now(),
    })
}

pub async fn read_usdc_balance(state: &AppState, address: &str) -> Result<U256, AuthError> {
    let address = normalize_wallet_address(address)?;
    FaucetReadContracts::new(&state.env)
        .await
        .map_err(|error| AuthError::internal("usdc balance setup failed", error))?
        .balance_of(&address)
        .await
        .map_err(|error| AuthError::internal("usdc balance query failed", error))
}

struct FaucetWriteContracts {
    usdc: Contract<FaucetWriteMiddleware>,
}

struct FaucetReadContracts {
    usdc: Contract<FaucetReadProvider>,
}

impl FaucetWriteContracts {
    async fn new(env: &Environment) -> Result<Self> {
        let private_key = env
            .monad_operator_private_key
            .as_deref()
            .ok_or_else(|| anyhow!("MONAD_OPERATOR_PRIVATE_KEY is not configured"))?;
        let wallet = private_key
            .parse::<LocalWallet>()
            .context("invalid MONAD_OPERATOR_PRIVATE_KEY")?
            .with_chain_id(env.monad_chain_id as u64);
        let client = rpc::monad_signer_middleware(env, wallet).await?;
        let usdc = Contract::new(faucet_usdc_address(env)?, faucet_usdc_abi()?, client);

        Ok(Self { usdc })
    }

    async fn mint_usdc(&self, recipient: &str, amount: U256) -> Result<String> {
        let recipient = recipient
            .parse::<Address>()
            .context("invalid faucet recipient address")?;
        let call = self
            .usdc
            .method::<_, ()>("mint", (recipient, amount))
            .context("failed to build MockUSDC.mint call")?;
        let pending_tx = call
            .send()
            .await
            .context("failed to submit MockUSDC.mint transaction")?;
        let tx_hash = format!("{:#x}", pending_tx.tx_hash());
        let receipt = pending_tx
            .await
            .context("failed while awaiting faucet transaction receipt")?
            .ok_or_else(|| anyhow!("faucet transaction dropped from mempool"))?;

        if receipt.status != Some(U64::from(1_u64)) {
            return Err(anyhow!("faucet transaction reverted: {tx_hash}"));
        }

        Ok(tx_hash)
    }
}

impl FaucetReadContracts {
    async fn new(env: &Environment) -> Result<Self> {
        let provider = rpc::monad_provider_arc(env).await?;
        let usdc = Contract::new(faucet_usdc_address(env)?, faucet_usdc_abi()?, provider);

        Ok(Self { usdc })
    }

    async fn balance_of(&self, address: &str) -> Result<U256> {
        let account = address
            .parse::<Address>()
            .context("invalid faucet balance address")?;

        self.usdc
            .method::<_, U256>("balanceOf", account)
            .context("failed to build MockUSDC.balanceOf call")?
            .call()
            .await
            .context("failed to query MockUSDC.balanceOf")
    }
}

fn faucet_usdc_address(env: &Environment) -> Result<Address> {
    env.monad_usdc_address
        .parse::<Address>()
        .context("invalid MONAD_USDC_ADDRESS")
}

fn faucet_usdc_abi() -> Result<ethers_core::abi::Abi> {
    AbiParser::default()
        .parse(&[
            "function mint(address to, uint256 amount)",
            "function balanceOf(address account) view returns (uint256)",
        ])
        .map_err(Into::into)
}

fn parse_amount(raw: &str) -> Result<U256, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request("amount is required"));
    }

    let amount = U256::from_dec_str(value)
        .map_err(|_| AuthError::bad_request("amount must be a base-10 integer string"))?;
    if amount.is_zero() {
        return Err(AuthError::bad_request("amount must be greater than zero"));
    }

    Ok(amount)
}

fn map_faucet_error(context: &'static str, error: anyhow::Error) -> AuthError {
    let message = error.to_string();
    if message.contains("MockUSDC.mint")
        || message.contains("faucet transaction reverted:")
        || message.contains("invalid faucet recipient")
    {
        return AuthError::bad_request(message);
    }

    AuthError::internal(context, error)
}

#[cfg(test)]
mod tests {
    use super::faucet_usdc_abi;

    #[test]
    fn faucet_abi_supports_mint_and_balance_of() {
        let abi = faucet_usdc_abi().expect("abi");

        assert!(abi.function("mint").is_ok());
        assert!(abi.function("balanceOf").is_ok());
    }
}
