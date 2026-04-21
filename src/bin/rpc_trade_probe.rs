use anyhow::{Context, Result};
use reqwest::Client;
use sabissbackend::{
    config::environment::Environment,
    service::{
        crypto::{create_managed_owner_key, decrypt_private_key, encode_stellar_secret_key},
        soroban_rpc::SorobanRpc,
        stellar,
    },
};
use tokio::time::{Duration, sleep};

struct TempOwner {
    address: String,
    public_key_hex: String,
    secret_key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let env = Environment::load().context("failed to load environment")?;
    let condition_id = std::env::var("PROBE_CONDITION_ID").unwrap_or_else(|_| {
        "c73b3039e3d698e6f6bf16879c9c19a7a730be2fc8941229a8f2da8ef4c0e7c5".to_owned()
    });
    let probe_usdc_amount =
        std::env::var("PROBE_USDC_AMOUNT").unwrap_or_else(|_| "100000000".to_owned());
    let http = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build HTTP client")?;

    let owner = create_temp_owner(&env)?;
    fund_testnet_account(&http, &env, &owner.address).await?;
    let wallet = stellar::deploy_wallet_contract(&env, &owner.public_key_hex)
        .await
        .context("failed to deploy wallet contract")?;
    let liquidity = stellar::get_market_liquidity(&env, &condition_id)
        .await
        .context("failed to read market liquidity")?;
    let outcome_index = if liquidity.yes_available != "0" { 0 } else { 1 };
    let yes_price_bps = stellar::get_market_price_bps(&env, &condition_id, 0)
        .await
        .context("failed to read YES price")?;
    let no_price_bps = stellar::get_market_price_bps(&env, &condition_id, 1)
        .await
        .context("failed to read NO price")?;
    let max_trade_amount = stellar::get_exchange_max_trade_amount(&env)
        .await
        .context("failed to read exchange max trade amount")?;

    println!("owner_address={}", owner.address);
    println!("wallet_contract_id={}", wallet.contract_id);
    println!("condition_id={condition_id}");
    println!(
        "liquidity=yes:{} no:{}",
        liquidity.yes_available, liquidity.no_available
    );
    println!("prices=yes:{yes_price_bps} no:{no_price_bps}");
    println!("max_trade_amount={max_trade_amount}");
    println!("probe_usdc_amount={probe_usdc_amount}");

    probe_buy_path(
        &env,
        "wallet_contract",
        &owner.secret_key,
        &wallet.contract_id,
        &condition_id,
        outcome_index,
        &probe_usdc_amount,
    )
    .await?;
    probe_buy_path(
        &env,
        "owner_account",
        &owner.secret_key,
        &owner.address,
        &condition_id,
        outcome_index,
        &probe_usdc_amount,
    )
    .await?;

    Ok(())
}

async fn probe_buy_path(
    env: &Environment,
    label: &str,
    source_account: &str,
    actor_address: &str,
    condition_id: &str,
    outcome_index: u32,
    usdc_amount: &str,
) -> Result<()> {
    println!("probe_actor={label} address={actor_address}");
    stellar::ensure_mock_usdc_balance(env, actor_address, usdc_amount)
        .await
        .with_context(|| format!("failed to mint mock USDC to {label} `{actor_address}`"))?;
    stellar::ensure_mock_usdc_allowance(
        env,
        source_account,
        actor_address,
        &env.sabi_exchange_id,
        usdc_amount,
    )
    .await
    .with_context(|| format!("failed to approve exchange USDC allowance for {label}"))?;

    let allowance = stellar::get_mock_usdc_allowance(env, actor_address, &env.sabi_exchange_id)
        .await
        .with_context(|| format!("failed to read exchange USDC allowance for {label}"))?;
    let before = stellar::get_outcome_position_balance(env, condition_id, actor_address, outcome_index)
        .await
        .with_context(|| format!("failed to read {label} position before buy"))?;
    println!("{label}_exchange_usdc_allowance={allowance}");
    println!("{label}_before_position={before}");

    match stellar::buy_market_outcome(
        env,
        source_account,
        actor_address,
        condition_id,
        outcome_index,
        usdc_amount,
    )
    .await
    {
        Ok(buy) => {
            println!("{label}_buy_tx_hash={}", buy.tx_hash);
            let after_buy =
                stellar::get_outcome_position_balance(env, condition_id, actor_address, outcome_index)
                    .await
                    .with_context(|| format!("failed to read {label} position after buy"))?;
            println!("{label}_after_buy_position={after_buy}");

            if after_buy != "0" {
                match stellar::sell_market_outcome(
                    env,
                    source_account,
                    actor_address,
                    condition_id,
                    outcome_index,
                    &after_buy,
                )
                .await
                {
                    Ok(sell) => {
                        println!("{label}_sell_tx_hash={}", sell.tx_hash);
                        let after_sell = stellar::get_outcome_position_balance(
                            env,
                            condition_id,
                            actor_address,
                            outcome_index,
                        )
                        .await
                        .with_context(|| format!("failed to read {label} position after sell"))?;
                        println!("{label}_after_sell_position={after_sell}");
                    }
                    Err(error) => println!("{label}_sell_error={error:#}"),
                }
            }
        }
        Err(error) => println!("{label}_buy_error={error:#}"),
    }

    match stellar::split_market_position(env, source_account, actor_address, condition_id, usdc_amount)
        .await
    {
        Ok(split) => {
            println!("{label}_split_tx_hash={}", split.tx_hash);
            let split_position =
                stellar::get_outcome_position_balance(env, condition_id, actor_address, outcome_index)
                    .await
                    .with_context(|| format!("failed to read {label} position after split"))?;
            println!("{label}_after_split_position={split_position}");
            if split_position != "0" {
                match stellar::sell_market_outcome(
                    env,
                    source_account,
                    actor_address,
                    condition_id,
                    outcome_index,
                    &split_position,
                )
                .await
                {
                    Ok(sell) => {
                        println!("{label}_sell_from_split_tx_hash={}", sell.tx_hash);
                        let after_sell = stellar::get_outcome_position_balance(
                            env,
                            condition_id,
                            actor_address,
                            outcome_index,
                        )
                        .await
                        .with_context(|| format!("failed to read {label} position after split sell"))?;
                        println!("{label}_after_split_sell_position={after_sell}");
                    }
                    Err(error) => println!("{label}_sell_from_split_error={error:#}"),
                }
            }
        }
        Err(error) => println!("{label}_split_error={error:#}"),
    }

    Ok(())
}

fn create_temp_owner(env: &Environment) -> Result<TempOwner> {
    let owner = create_managed_owner_key(env).map_err(anyhow::Error::from)?;
    let decrypted = decrypt_private_key(env, &owner.encrypted_private_key, &owner.encryption_nonce)
        .map_err(anyhow::Error::from)?;
    let secret_bytes: [u8; 32] = decrypted
        .as_slice()
        .try_into()
        .context("temporary owner private key length was not 32 bytes")?;

    Ok(TempOwner {
        address: owner.owner_address,
        public_key_hex: owner.owner_public_key_hex,
        secret_key: encode_stellar_secret_key(&secret_bytes),
    })
}

async fn fund_testnet_account(client: &Client, env: &Environment, address: &str) -> Result<()> {
    let rpc = SorobanRpc::new(env);

    client
        .get("https://friendbot.stellar.org")
        .query(&[("addr", address)])
        .send()
        .await
        .context("failed to call Friendbot")?
        .error_for_status()
        .context("Friendbot funding failed")?;

    for _ in 0..10 {
        if rpc
            .account_exists(address)
            .await
            .with_context(|| format!("failed to check RPC account `{address}`"))?
        {
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }

    anyhow::bail!("funded temporary owner `{address}`, but Soroban RPC never observed the account")
}
