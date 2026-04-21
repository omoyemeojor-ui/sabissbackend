use anyhow::{Context, Result, anyhow, bail};
use ed25519_dalek::SigningKey;
use reqwest::Client;
use sabissbackend::{
    config::environment::Environment,
    service::{
        crypto::{create_managed_owner_key, decrypt_private_key, encode_stellar_secret_key},
        soroban_rpc::SorobanRpc,
        stellar,
    },
};
use sha2::{Digest, Sha256};
use sqlx::{FromRow, postgres::PgPoolOptions};
use stellar_strkey::{Strkey, ed25519};
use tokio::time::{Duration, sleep};
use uuid::Uuid;

const ONE_USDC: &str = "1000000";
const HALF_USDC: &str = "500000";
const TEN_USDC: &str = "10000000";

#[derive(Clone, Debug, FromRow)]
struct MarketRow {
    condition_id: String,
    oracle_address: String,
}

struct TempOwner {
    address: String,
    public_key_hex: String,
    secret_key: String,
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenvy::dotenv().ok();

    let env = Environment::load().context("failed to load environment")?;
    let admin_public_key = verify_admin_private_key(&env)?;
    let http_client = Client::builder()
        .timeout(Duration::from_secs(30))
        .build()
        .context("failed to build smoke-test HTTP client")?;
    let pool = PgPoolOptions::new()
        .max_connections(1)
        .connect(&env.database_url)
        .await
        .context("failed to connect to Postgres")?;
    let markets = load_smoke_markets(&pool).await?;
    let market = select_read_market(&env, &markets).await?;

    println!("admin_public_key={admin_public_key}");
    println!("smoke_condition_id={}", market.condition_id);
    println!("smoke_oracle={}", market.oracle_address);

    let rpc = SorobanRpc::new(&env);
    let dispute_window = rpc
        .simulate(&env.sabi_market_id, "get_resolution_dispute_window", &[])
        .await
        .context("failed to read market dispute window through Soroban RPC")?;
    let exchange_max_trade_amount = stellar::get_exchange_max_trade_amount(&env)
        .await
        .context("failed to read exchange max trade amount through Soroban RPC")?;
    let liquidity = stellar::get_market_liquidity(&env, &market.condition_id)
        .await
        .context("failed to read market liquidity through Soroban RPC")?;
    let yes_price = stellar::get_market_price_bps(&env, &market.condition_id, 0)
        .await
        .context("failed to read YES market price through Soroban RPC")?;
    let no_price = stellar::get_market_price_bps(&env, &market.condition_id, 1)
        .await
        .context("failed to read NO market price through Soroban RPC")?;

    println!("dispute_window_seconds={dispute_window}");
    println!("exchange_max_trade_amount={exchange_max_trade_amount}");
    println!(
        "market_liquidity=yes_available:{} no_available:{} posted_yes:{} posted_no:{}",
        liquidity.yes_available,
        liquidity.no_available,
        liquidity.posted_yes_total,
        liquidity.posted_no_total
    );
    println!("market_prices=yes:{yes_price} no:{no_price}");

    smoke_admin_simulations(&rpc, &env, &market).await?;
    smoke_admin_writes(&env, &market, yes_price, no_price).await?;

    if env.network != "testnet" {
        println!("runtime_network={} - write smoke skipped outside testnet", env.network);
        return Ok(());
    }

    let temp_owner = create_temp_owner(&env)?;
    fund_testnet_account(&http_client, &env, &temp_owner.address).await?;
    let mut failures = Vec::new();

    let deployed_wallet = stellar::deploy_wallet_contract(&env, &temp_owner.public_key_hex)
        .await
        .context("wallet factory RPC deployment failed")?;
    println!("deployed_wallet_contract_id={}", deployed_wallet.contract_id);

    stellar::ensure_mock_usdc_balance(&env, &temp_owner.address, TEN_USDC)
        .await
        .context("failed to mint mock USDC to temporary owner through RPC")?;
    let owner_balance = stellar::get_mock_usdc_balance(&env, &temp_owner.address)
        .await
        .context("failed to read temporary owner mock USDC balance")?;
    println!("temp_owner_mock_usdc_balance={owner_balance}");

    match select_user_flow_market(&rpc, &env, &markets, &temp_owner).await {
        Ok(user_market) => {
            println!("user_flow_condition_id={}", user_market.condition_id);
            match smoke_ctf_round_trip(&env, &user_market.condition_id, &temp_owner).await {
                Ok(()) => {}
                Err(error) => failures.push(format!("ctf_round_trip failed: {error:#}")),
            }
            match smoke_liquidity_round_trip(&env, &user_market.condition_id, &temp_owner).await {
                Ok(()) => {}
                Err(error) => failures.push(format!("liquidity_round_trip failed: {error:#}")),
            }
            match smoke_collateral_round_trip(&env, &user_market.condition_id, &temp_owner).await {
                Ok(()) => {}
                Err(error) => failures.push(format!("collateral_round_trip failed: {error:#}")),
            }
        }
        Err(error) => failures.push(format!("ctf/liquidity market selection failed: {error:#}")),
    }

    if parse_amount(&liquidity.yes_available) > 0 || parse_amount(&liquidity.no_available) > 0 {
        match smoke_exchange_round_trip(&env, &market.condition_id, &liquidity, &temp_owner).await {
            Ok(()) => {}
            Err(error) => failures.push(format!("exchange_round_trip failed: {error:#}")),
        }
    } else {
        println!("exchange_round_trip=skipped_no_live_liquidity");
    }

    if failures.is_empty() {
        println!("rpc_smoke_result=ok");
        Ok(())
    } else {
        bail!("rpc_smoke_result=failed\n{}", failures.join("\n"));
    }
}

async fn load_smoke_markets(pool: &sqlx::PgPool) -> Result<Vec<MarketRow>> {
    sqlx::query_as::<_, MarketRow>(
        r#"
        SELECT condition_id, oracle_address
        FROM markets
        WHERE condition_id IS NOT NULL
        ORDER BY created_at DESC
        LIMIT 20
        "#,
    )
    .fetch_all(pool)
    .await
    .context("failed to load recent markets for RPC smoke test")
}

async fn select_read_market(env: &Environment, candidates: &[MarketRow]) -> Result<MarketRow> {
    for candidate in candidates {
        if stellar::get_market_liquidity(env, &candidate.condition_id)
            .await
            .is_ok()
        {
            return Ok(candidate.clone());
        }
    }

    bail!("did not find a recent market with a working on-chain condition id")
}

async fn select_user_flow_market(
    rpc: &SorobanRpc,
    env: &Environment,
    candidates: &[MarketRow],
    owner: &TempOwner,
) -> Result<MarketRow> {
    for candidate in candidates {
        if can_split_market(rpc, env, &candidate.condition_id, owner)
            .await
            .is_ok()
        {
            return Ok(candidate.clone());
        }
    }

    bail!("did not find a recent market whose CTF split_position path succeeds through Soroban RPC")
}

async fn can_split_market(
    rpc: &SorobanRpc,
    env: &Environment,
    condition_id: &str,
    owner: &TempOwner,
) -> Result<()> {
    rpc.simulate(
        &env.sabi_ctf_id,
        "split_position",
        &[
            ("user", &owner.address),
            ("collateral-token", &env.mock_usdc_id),
            (
                "parent-collection-id",
                "0000000000000000000000000000000000000000000000000000000000000000",
            ),
            ("condition-id", condition_id),
            ("partition", "[1,2]"),
            ("amount", ONE_USDC),
        ],
    )
    .await
    .map(|_| ())
}

fn verify_admin_private_key(env: &Environment) -> Result<String> {
    let secret = env
        .private_key
        .as_deref()
        .ok_or_else(|| anyhow!("PRIVATE_KEY is required for RPC smoke tests"))?;
    let public_key = derive_public_key(secret)?;

    if public_key != env.admin {
        bail!(
            "PRIVATE_KEY resolves to `{public_key}`, but ADMIN is `{}`; admin writes will not authorize correctly",
            env.admin
        );
    }

    Ok(public_key)
}

fn derive_public_key(secret_key: &str) -> Result<String> {
    match Strkey::from_string(secret_key).context("invalid Stellar secret key")? {
        Strkey::PrivateKeyEd25519(secret) => {
            let signing_key = SigningKey::from_bytes(&secret.0);
            let public_key = signing_key.verifying_key().to_bytes();
            Ok(Strkey::PublicKeyEd25519(ed25519::PublicKey(public_key))
                .to_string()
                .to_string())
        }
        _ => Err(anyhow!("expected an ed25519 Stellar secret key")),
    }
}

async fn smoke_admin_simulations(rpc: &SorobanRpc, env: &Environment, market: &MarketRow) -> Result<()> {
    let event_id = random_bytes32("create_event:event_id");
    let group_id = random_bytes32("create_event:group_id");
    let series_id = random_bytes32("create_event:series_id");
    let wallet_owner = create_temp_owner(env)?;

    rpc.simulate(
        &env.sabi_market_id,
        "create_event",
        &[
            ("event-id", &event_id),
            ("group-id", &group_id),
            ("series-id", &series_id),
            ("neg-risk", "false"),
        ],
    )
    .await
    .context("failed to simulate `create_event` through Soroban RPC")?;

    rpc.simulate(
        &env.sabi_wallet_factory_id
            .clone()
            .ok_or_else(|| anyhow!("missing SABI_WALLET_FACTORY_ID"))?,
        "create_wallet",
        &[("owner", &wallet_owner.public_key_hex)],
    )
    .await
    .context("failed to simulate `create_wallet` through Soroban RPC")?;

    rpc.simulate(
        &env.sabi_exchange_id,
        "set_price",
        &[
            ("condition-id", &market.condition_id),
            ("outcome-index", "0"),
            ("price-bps", "5000"),
        ],
    )
    .await
    .context("failed to simulate `set_price` through Soroban RPC")?;

    println!("admin_simulations=ok");
    Ok(())
}

async fn smoke_admin_writes(
    env: &Environment,
    market: &MarketRow,
    yes_price: u32,
    no_price: u32,
) -> Result<()> {
    stellar::set_market_prices(env, &market.condition_id, yes_price, no_price)
        .await
        .context("failed to re-submit current market prices through Soroban RPC")?;

    if env.network == "testnet" {
        let event_id = random_bytes32("publish_market:event_id");
        let group_id = random_bytes32("publish_market:group_id");
        let series_id = random_bytes32("publish_market:series_id");
        let question_id = random_bytes32("publish_market:question_id");
        let published = stellar::publish_standalone_binary_market(
            env,
            &event_id,
            &group_id,
            &series_id,
            false,
            &question_id,
            1_999_999_999,
            &market.oracle_address,
        )
        .await
        .context("failed to publish an isolated smoke-test market through Soroban RPC")?;
        println!("published_smoke_market_condition_id={}", published.condition_id);
    }

    println!("admin_writes=ok");
    Ok(())
}

fn create_temp_owner(env: &Environment) -> Result<TempOwner> {
    let owner = create_managed_owner_key(env)
        .map_err(|error| anyhow!("failed to create temporary managed owner key: {error}"))?;
    let decrypted = decrypt_private_key(env, &owner.encrypted_private_key, &owner.encryption_nonce)
        .map_err(|error| anyhow!("failed to decrypt temporary managed owner key: {error}"))?;
    let secret_bytes: [u8; 32] = decrypted
        .as_slice()
        .try_into()
        .map_err(|_| anyhow!("temporary owner private key length was not 32 bytes"))?;

    Ok(TempOwner {
        address: owner.owner_address,
        public_key_hex: owner.owner_public_key_hex,
        secret_key: encode_stellar_secret_key(&secret_bytes),
    })
}

async fn fund_testnet_account(client: &Client, env: &Environment, address: &str) -> Result<()> {
    let rpc = SorobanRpc::new(env);
    let response = client
        .get("https://friendbot.stellar.org")
        .query(&[("addr", address)])
        .send()
        .await
        .context("failed to call Friendbot")?;

    if !response.status().is_success() {
        let status = response.status();
        let body = response.text().await.unwrap_or_default();
        bail!("Friendbot funding failed with {status}: {body}");
    }

    for _ in 0..10 {
        if rpc
            .account_exists(address)
            .await
            .with_context(|| format!("failed to check RPC account `{address}`"))?
        {
            println!("temp_owner_funded={address}");
            return Ok(());
        }
        sleep(Duration::from_millis(500)).await;
    }

    bail!("funded temporary owner `{address}`, but Soroban RPC never observed the account")
}

async fn smoke_ctf_round_trip(env: &Environment, condition_id: &str, owner: &TempOwner) -> Result<()> {
    let before_yes = parse_amount(
        &stellar::get_outcome_position_balance(env, condition_id, &owner.address, 0)
            .await
            .context("failed to read YES position before split")?,
    );
    let before_no = parse_amount(
        &stellar::get_outcome_position_balance(env, condition_id, &owner.address, 1)
            .await
            .context("failed to read NO position before split")?,
    );

    stellar::split_market_position(env, &owner.secret_key, &owner.address, condition_id, ONE_USDC)
        .await
        .context("split_position RPC write failed")?;

    let after_split_yes = parse_amount(
        &stellar::get_outcome_position_balance(env, condition_id, &owner.address, 0)
            .await
            .context("failed to read YES position after split")?,
    );
    let after_split_no = parse_amount(
        &stellar::get_outcome_position_balance(env, condition_id, &owner.address, 1)
            .await
            .context("failed to read NO position after split")?,
    );
    if after_split_yes <= before_yes || after_split_no <= before_no {
        bail!("split_position submitted but outcome balances did not increase");
    }

    stellar::merge_market_positions(env, &owner.secret_key, &owner.address, condition_id, ONE_USDC)
        .await
        .context("merge_positions RPC write failed")?;

    let after_merge_yes = parse_amount(
        &stellar::get_outcome_position_balance(env, condition_id, &owner.address, 0)
            .await
            .context("failed to read YES position after merge")?,
    );
    let after_merge_no = parse_amount(
        &stellar::get_outcome_position_balance(env, condition_id, &owner.address, 1)
            .await
            .context("failed to read NO position after merge")?,
    );
    if after_merge_yes >= after_split_yes || after_merge_no >= after_split_no {
        bail!("merge_positions submitted but outcome balances did not decrease");
    }

    println!("ctf_round_trip=ok");
    Ok(())
}

async fn smoke_liquidity_round_trip(
    env: &Environment,
    condition_id: &str,
    owner: &TempOwner,
) -> Result<()> {
    stellar::split_market_position(env, &owner.secret_key, &owner.address, condition_id, ONE_USDC)
        .await
        .context("pre-liquidity split_position RPC write failed")?;

    stellar::deposit_inventory(
        env,
        &owner.secret_key,
        &owner.address,
        condition_id,
        ONE_USDC,
        ONE_USDC,
    )
    .await
    .context("deposit_inventory RPC write failed")?;
    stellar::add_liquidity(
        env,
        &owner.secret_key,
        &owner.address,
        condition_id,
        ONE_USDC,
        ONE_USDC,
    )
    .await
    .context("add_liquidity RPC write failed")?;

    let after_add = stellar::get_liquidity_position(env, condition_id, &owner.address)
        .await
        .context("failed to read liquidity position after add_liquidity")?;
    if parse_amount(&after_add.posted_yes_amount) == 0 || parse_amount(&after_add.posted_no_amount) == 0 {
        bail!("add_liquidity submitted but posted liquidity stayed at zero");
    }

    stellar::remove_liquidity(
        env,
        &owner.secret_key,
        &owner.address,
        condition_id,
        ONE_USDC,
        ONE_USDC,
    )
    .await
    .context("remove_liquidity RPC write failed")?;
    stellar::withdraw_inventory(
        env,
        &owner.secret_key,
        &owner.address,
        condition_id,
        ONE_USDC,
        ONE_USDC,
        &owner.address,
    )
    .await
    .context("withdraw_inventory RPC write failed")?;
    stellar::merge_market_positions(env, &owner.secret_key, &owner.address, condition_id, ONE_USDC)
        .await
        .context("post-liquidity merge_positions RPC write failed")?;

    println!("liquidity_round_trip=ok");
    Ok(())
}

async fn smoke_collateral_round_trip(
    env: &Environment,
    condition_id: &str,
    owner: &TempOwner,
) -> Result<()> {
    stellar::deposit_collateral(env, &owner.secret_key, &owner.address, condition_id, HALF_USDC)
        .await
        .context("deposit_collateral RPC write failed")?;

    let after_deposit = stellar::get_liquidity_position(env, condition_id, &owner.address)
        .await
        .context("failed to read liquidity position after deposit_collateral")?;
    if parse_amount(&after_deposit.collateral_amount) == 0 {
        bail!("deposit_collateral submitted but collateral amount stayed at zero");
    }

    stellar::withdraw_collateral(
        env,
        &owner.secret_key,
        &owner.address,
        condition_id,
        HALF_USDC,
        &owner.address,
    )
    .await
    .context("withdraw_collateral RPC write failed")?;

    println!("collateral_round_trip=ok");
    Ok(())
}

async fn smoke_exchange_round_trip(
    env: &Environment,
    condition_id: &str,
    liquidity: &stellar::MarketLiquidityReadResult,
    owner: &TempOwner,
) -> Result<()> {
    let outcome_index = if parse_amount(&liquidity.yes_available) > 0 { 0 } else { 1 };
    let before = parse_amount(
        &stellar::get_outcome_position_balance(env, condition_id, &owner.address, outcome_index)
            .await
            .context("failed to read outcome position before buy_outcome")?,
    );

    stellar::buy_market_outcome(
        env,
        &owner.secret_key,
        &owner.address,
        condition_id,
        outcome_index,
        "100000",
    )
    .await
    .context("buy_outcome RPC write failed")?;

    let after_buy = parse_amount(
        &stellar::get_outcome_position_balance(env, condition_id, &owner.address, outcome_index)
            .await
            .context("failed to read outcome position after buy_outcome")?,
    );
    if after_buy <= before {
        bail!("buy_outcome submitted but the purchased outcome balance did not increase");
    }

    let purchased = after_buy - before;
    stellar::sell_market_outcome(
        env,
        &owner.secret_key,
        &owner.address,
        condition_id,
        outcome_index,
        &purchased.to_string(),
    )
    .await
    .context("sell_outcome RPC write failed")?;

    println!("exchange_round_trip=ok");
    Ok(())
}

fn parse_amount(value: &str) -> u128 {
    value.trim().parse::<u128>().unwrap_or(0)
}

fn random_bytes32(label: &str) -> String {
    hex::encode(Sha256::digest(format!("{label}:{}", Uuid::new_v4()).as_bytes()))
}
