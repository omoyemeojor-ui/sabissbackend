pub(crate) mod view;

use chrono::Utc;
use tokio::time::{Duration, sleep};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::{
            crud as auth_crud, error::AuthError, model::ACCOUNT_KIND_STELLAR_SMART_WALLET,
        },
        liquidity::schema::{
            DepositCollateralRequest, DepositInventoryRequest, EventLiquidityResponse,
            LiquidityPositionResponse, LiquidityTotalsResponse, LiquidityWriteResponse,
            MyEventLiquidityMarketResponse, MyEventLiquidityPositionTotalsResponse,
            MyEventLiquidityResponse, MyMarketLiquidityResponse, RemoveLiquidityRequest,
            WithdrawCollateralFieldsRequest, WithdrawCollateralRequest,
            WithdrawInventoryFieldsRequest, WithdrawInventoryRequest,
        },
        market::{
            crud as market_crud,
            schema::EventOnChainResponse,
        },
    },
    service::{
        crypto::{decrypt_private_key, encode_stellar_secret_key},
        jwt::AuthenticatedUser,
        liquidity::view::build_market_responses,
        market::{get_event_by_id, get_market_by_id},
        stellar::{
            self, LiquidityPositionReadResult, MarketLiquidityReadResult,
        },
    },
};

const USDC_DECIMALS: usize = 6;
const WRITE_RETRY_ATTEMPTS: usize = 2;
const WRITE_RETRY_DELAY_MS: u64 = 1_500;

struct LiquidityWalletContext {
    wallet_address: String,
    actor_address: String,
    source_account: Option<String>,
}

#[derive(Default)]
struct PositionAccumulator {
    active_markets: i64,
    posted_yes_amount: u128,
    posted_no_amount: u128,
    idle_yes_amount: u128,
    idle_no_amount: u128,
    collateral_amount: u128,
    claimable_collateral_amount: u128,
}

impl PositionAccumulator {
    fn add(&mut self, position: &LiquidityPositionReadResult) {
        let posted_yes_amount = parse_contract_amount(&position.posted_yes_amount);
        let posted_no_amount = parse_contract_amount(&position.posted_no_amount);
        let idle_yes_amount = parse_contract_amount(&position.idle_yes_amount);
        let idle_no_amount = parse_contract_amount(&position.idle_no_amount);
        let collateral_amount = parse_contract_amount(&position.collateral_amount);
        let claimable_collateral_amount =
            parse_contract_amount(&position.claimable_collateral_amount);

        if position.active
            || posted_yes_amount > 0
            || posted_no_amount > 0
            || idle_yes_amount > 0
            || idle_no_amount > 0
            || collateral_amount > 0
            || claimable_collateral_amount > 0
        {
            self.active_markets += 1;
        }

        self.posted_yes_amount += posted_yes_amount;
        self.posted_no_amount += posted_no_amount;
        self.idle_yes_amount += idle_yes_amount;
        self.idle_no_amount += idle_no_amount;
        self.collateral_amount += collateral_amount;
        self.claimable_collateral_amount += claimable_collateral_amount;
    }

    fn into_response(self) -> MyEventLiquidityPositionTotalsResponse {
        MyEventLiquidityPositionTotalsResponse {
            active_markets: self.active_markets,
            posted_yes_amount: format_contract_amount_u128(self.posted_yes_amount),
            posted_no_amount: format_contract_amount_u128(self.posted_no_amount),
            idle_yes_amount: format_contract_amount_u128(self.idle_yes_amount),
            idle_no_amount: format_contract_amount_u128(self.idle_no_amount),
            collateral_amount: format_contract_amount_u128(self.collateral_amount),
            claimable_collateral_amount: format_contract_amount_u128(
                self.claimable_collateral_amount,
            ),
        }
    }
}

enum LiquidityWriteAction {
    DepositInventory {
        yes_amount: String,
        no_amount: String,
    },
    DepositCollateral {
        amount: String,
    },
    Remove {
        yes_amount: String,
        no_amount: String,
    },
    WithdrawInventory {
        yes_amount: String,
        no_amount: String,
        recipient: String,
    },
    WithdrawCollateral {
        amount: String,
        recipient: String,
    },
}

impl LiquidityWriteAction {
    fn name(&self) -> &'static str {
        match self {
            Self::DepositInventory { .. } => "deposit_inventory",
            Self::DepositCollateral { .. } => "deposit_collateral",
            Self::Remove { .. } => "remove",
            Self::WithdrawInventory { .. } => "withdraw_inventory",
            Self::WithdrawCollateral { .. } => "withdraw_collateral",
        }
    }
}

pub async fn get_event_liquidity(
    state: &AppState,
    event_id: Uuid,
) -> Result<EventLiquidityResponse, AuthError> {
    let detail = get_event_by_id(state, event_id).await?;
    let liquidity = load_event_liquidity(state, &detail.on_chain).await?;

    Ok(EventLiquidityResponse {
        event: detail.event,
        on_chain: detail.on_chain,
        markets_count: detail.markets_count,
        liquidity,
    })
}

pub async fn get_my_market_liquidity(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
) -> Result<MyMarketLiquidityResponse, AuthError> {
    let detail = get_market_by_id(state, market_id).await?;
    let wallet = load_liquidity_wallet_context(state, authenticated_user.user_id, false).await?;
    let position = load_market_position_response(
        state,
        detail.market.condition_id.as_deref(),
        &wallet.actor_address,
    )
    .await?;
    let market_liquidity =
        load_market_liquidity_response(state, detail.market.condition_id.as_deref()).await?;

    Ok(MyMarketLiquidityResponse {
        event: detail.event,
        on_chain: detail.on_chain,
        market: detail.market,
        wallet_address: wallet.wallet_address,
        position,
        market_liquidity,
    })
}

pub async fn get_my_event_liquidity(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    event_id: Uuid,
) -> Result<MyEventLiquidityResponse, AuthError> {
    let detail = get_event_by_id(state, event_id).await?;
    let wallet = load_liquidity_wallet_context(state, authenticated_user.user_id, false).await?;
    let markets = market_crud::list_public_markets_for_event(&state.db, event_id).await?;
    let mut market_responses = build_market_responses(state, &markets)
        .await?
        .into_iter()
        .map(|market| (market.id, market))
        .collect::<std::collections::HashMap<_, _>>();
    let mut totals = PositionAccumulator::default();
    let mut response_markets = Vec::new();

    for market in &markets {
        let position = load_market_position_raw(
            state,
            market.condition_id.as_deref(),
            &wallet.actor_address,
        )
        .await?;
        totals.add(&position);

        if !has_position_balance(&position) {
            continue;
        }

        let market_liquidity =
            load_market_liquidity_response(state, market.condition_id.as_deref()).await?;

        response_markets.push(MyEventLiquidityMarketResponse {
            market: market_responses
                .remove(&market.id)
                .unwrap_or_else(|| crate::module::market::schema::MarketResponse::from(market)),
            position: liquidity_position_response(&position),
            market_liquidity,
        });
    }

    Ok(MyEventLiquidityResponse {
        event: detail.event,
        on_chain: detail.on_chain.clone(),
        wallet_address: wallet.wallet_address,
        event_liquidity: load_event_liquidity(state, &detail.on_chain).await?,
        position_totals: totals.into_response(),
        markets: response_markets,
    })
}

pub async fn deposit_market_inventory(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: DepositInventoryRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let yes_amount = parse_amount(&payload.liquidity.yes_amount, "liquidity.yes_amount", true)?;
    let no_amount = parse_amount(&payload.liquidity.no_amount, "liquidity.no_amount", true)?;
    ensure_inventory_pair_non_zero(&yes_amount, &no_amount)?;

    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::DepositInventory {
            yes_amount,
            no_amount,
        },
    )
    .await
}

pub async fn deposit_market_collateral(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: DepositCollateralRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let amount = parse_amount(&payload.liquidity.amount, "liquidity.amount", false)?;

    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::DepositCollateral { amount },
    )
    .await
}

pub async fn remove_market_liquidity(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: RemoveLiquidityRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let yes_amount = parse_amount(&payload.liquidity.yes_amount, "liquidity.yes_amount", true)?;
    let no_amount = parse_amount(&payload.liquidity.no_amount, "liquidity.no_amount", true)?;
    ensure_inventory_pair_non_zero(&yes_amount, &no_amount)?;

    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::Remove {
            yes_amount,
            no_amount,
        },
    )
    .await
}

pub async fn withdraw_market_inventory(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: WithdrawInventoryRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let yes_amount = parse_amount(&payload.liquidity.yes_amount, "liquidity.yes_amount", true)?;
    let no_amount = parse_amount(&payload.liquidity.no_amount, "liquidity.no_amount", true)?;
    ensure_inventory_pair_non_zero(&yes_amount, &no_amount)?;
    let wallet = load_liquidity_wallet_context(state, authenticated_user.user_id, false).await?;
    let recipient = resolve_recipient(payload.liquidity, &wallet.actor_address)?;

    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::WithdrawInventory {
            yes_amount,
            no_amount,
            recipient,
        },
    )
    .await
}

pub async fn withdraw_market_collateral(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: WithdrawCollateralRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    let amount = parse_amount(&payload.liquidity.amount, "liquidity.amount", false)?;
    let wallet = load_liquidity_wallet_context(state, authenticated_user.user_id, false).await?;
    let recipient = resolve_collateral_recipient(payload.liquidity, &wallet.actor_address)?;

    execute_market_write(
        state,
        authenticated_user,
        market_id,
        LiquidityWriteAction::WithdrawCollateral { amount, recipient },
    )
    .await
}

async fn execute_market_write(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    action: LiquidityWriteAction,
) -> Result<LiquidityWriteResponse, AuthError> {
    let detail = get_market_by_id(state, market_id).await?;
    let condition_id = detail
        .market
        .condition_id
        .as_deref()
        .ok_or_else(|| AuthError::bad_request("market is not published on-chain"))?;
    validate_bytes32(condition_id, "market condition_id")?;
    let wallet = load_liquidity_wallet_context(state, authenticated_user.user_id, true).await?;
    let source_account = wallet
        .source_account
        .as_deref()
        .ok_or_else(|| AuthError::unprocessable_entity("managed smart wallet is required"))?;
    let before_position =
        load_market_position_raw(state, Some(condition_id), &wallet.actor_address).await?;
    let before_market_liquidity = stellar::get_market_liquidity(&state.env, condition_id)
        .await
        .unwrap_or_else(|_| stellar::MarketLiquidityReadResult {
            yes_available: "0".to_owned(),
            no_available: "0".to_owned(),
            idle_yes_total: "0".to_owned(),
            idle_no_total: "0".to_owned(),
            posted_yes_total: "0".to_owned(),
            posted_no_total: "0".to_owned(),
            claimable_collateral_total: "0".to_owned(),
        });
    if let LiquidityWriteAction::DepositCollateral { amount } = &action {
        stellar::ensure_mock_usdc_balance(&state.env, &wallet.actor_address, amount)
            .await
            .map_err(|error| map_liquidity_chain_error("fund_mock_usdc", error))?;
    }

    let tx = match &action {
        LiquidityWriteAction::DepositInventory {
            yes_amount,
            no_amount,
        } => {
            let deposit = stellar::deposit_inventory(
                &state.env,
                source_account,
                &wallet.actor_address,
                condition_id,
                yes_amount,
                no_amount,
            )
            .await;
            match deposit {
                Ok(_) => add_liquidity_with_retry(
                    state,
                    source_account,
                    &wallet.actor_address,
                    condition_id,
                    yes_amount,
                    no_amount,
                    &before_position,
                    &before_market_liquidity,
                )
                .await,
                Err(error) => Err(error),
            }
        }
        LiquidityWriteAction::DepositCollateral { amount } => {
            let collateral_write = stellar::deposit_collateral(
                &state.env,
                source_account,
                &wallet.actor_address,
                condition_id,
                amount,
            )
            .await;

            match collateral_write {
                Ok(tx) => Ok(tx),
                Err(error)
                    if is_retryable_submission_error(&error)
                        && verify_collateral_write_applied(
                            state,
                            condition_id,
                            &wallet.actor_address,
                            &before_position,
                            &before_market_liquidity,
                            amount,
                        )
                        .await? =>
                {
                    Ok(stellar::ContractTxResult {
                        tx_hash: "soroban-rpc-submitted".to_owned(),
                    })
                }
                Err(error) => Err(error),
            }
        }
        LiquidityWriteAction::Remove {
            yes_amount,
            no_amount,
        } => {
            stellar::remove_liquidity(
                &state.env,
                source_account,
                &wallet.actor_address,
                condition_id,
                yes_amount,
                no_amount,
            )
            .await
        }
        LiquidityWriteAction::WithdrawInventory {
            yes_amount,
            no_amount,
            recipient,
        } => {
            stellar::withdraw_inventory(
                &state.env,
                source_account,
                &wallet.actor_address,
                condition_id,
                yes_amount,
                no_amount,
                recipient,
            )
            .await
        }
        LiquidityWriteAction::WithdrawCollateral { amount, recipient } => {
            stellar::withdraw_collateral(
                &state.env,
                source_account,
                &wallet.actor_address,
                condition_id,
                amount,
                recipient,
            )
            .await
        }
    }
    .map_err(|error| map_liquidity_chain_error(action.name(), error))?;

    let position = load_market_position_response(state, Some(condition_id), &wallet.actor_address).await?;
    let market_liquidity = load_market_liquidity_response(state, Some(condition_id)).await?;

    Ok(LiquidityWriteResponse {
        event: detail.event,
        on_chain: detail.on_chain,
        market: detail.market,
        wallet_address: wallet.wallet_address,
        action: action.name().to_owned(),
        tx_hash: tx.tx_hash,
        position,
        market_liquidity,
        updated_at: Utc::now(),
    })
}

async fn add_liquidity_with_retry(
    state: &AppState,
    source_account: &str,
    actor_address: &str,
    condition_id: &str,
    yes_amount: &str,
    no_amount: &str,
    before_position: &LiquidityPositionReadResult,
    before_market_liquidity: &MarketLiquidityReadResult,
) -> Result<stellar::ContractTxResult, anyhow::Error> {
    let mut attempts = 0usize;

    loop {
        match stellar::add_liquidity(
            &state.env,
            source_account,
            actor_address,
            condition_id,
            yes_amount,
            no_amount,
        )
        .await
        {
            Ok(tx) => return Ok(tx),
            Err(_error)
                if verify_inventory_write_applied(
                    state,
                    condition_id,
                    actor_address,
                    before_position,
                    before_market_liquidity,
                    yes_amount,
                    no_amount,
                )
                .await? =>
            {
                return Ok(stellar::ContractTxResult {
                    tx_hash: "soroban-rpc-submitted".to_owned(),
                });
            }
            Err(error)
                if is_retryable_submission_error(&error) && attempts < WRITE_RETRY_ATTEMPTS =>
            {
                attempts += 1;
                sleep(Duration::from_millis(WRITE_RETRY_DELAY_MS)).await;
            }
            Err(error) => return Err(error),
        }
    }
}

async fn verify_inventory_write_applied(
    state: &AppState,
    condition_id: &str,
    actor_address: &str,
    before_position: &LiquidityPositionReadResult,
    before_market_liquidity: &MarketLiquidityReadResult,
    yes_amount: &str,
    no_amount: &str,
) -> Result<bool, anyhow::Error> {
    let after_position = stellar::get_liquidity_position(&state.env, condition_id, actor_address).await?;
    let after_market_liquidity = stellar::get_market_liquidity(&state.env, condition_id).await?;

    Ok(parse_contract_amount(&after_position.posted_yes_amount)
        >= parse_contract_amount(&before_position.posted_yes_amount) + parse_contract_amount(yes_amount)
        && parse_contract_amount(&after_position.posted_no_amount)
            >= parse_contract_amount(&before_position.posted_no_amount) + parse_contract_amount(no_amount)
        && parse_contract_amount(&after_market_liquidity.posted_yes_total)
            >= parse_contract_amount(&before_market_liquidity.posted_yes_total)
                + parse_contract_amount(yes_amount)
        && parse_contract_amount(&after_market_liquidity.posted_no_total)
            >= parse_contract_amount(&before_market_liquidity.posted_no_total)
                + parse_contract_amount(no_amount))
}

async fn verify_collateral_write_applied(
    state: &AppState,
    condition_id: &str,
    actor_address: &str,
    before_position: &LiquidityPositionReadResult,
    before_market_liquidity: &MarketLiquidityReadResult,
    amount: &str,
) -> Result<bool, AuthError> {
    let after_position = stellar::get_liquidity_position(&state.env, condition_id, actor_address)
        .await
        .map_err(|error| AuthError::internal("failed to verify collateral write", error))?;
    let after_market_liquidity = stellar::get_market_liquidity(&state.env, condition_id)
        .await
        .map_err(|error| AuthError::internal("failed to verify collateral write", error))?;

    Ok(parse_contract_amount(&after_position.claimable_collateral_amount)
        >= parse_contract_amount(&before_position.claimable_collateral_amount)
            + parse_contract_amount(amount)
        && parse_contract_amount(&after_market_liquidity.claimable_collateral_total)
            >= parse_contract_amount(&before_market_liquidity.claimable_collateral_total)
                + parse_contract_amount(amount))
}

fn is_retryable_submission_error(error: &anyhow::Error) -> bool {
    let message = format!("{error:#}").to_ascii_lowercase();
    message.contains("txbadseq")
        || message.contains("bad seq")
        || message.contains("transaction submission timeout")
}

async fn load_event_liquidity(
    state: &AppState,
    on_chain: &EventOnChainResponse,
) -> Result<LiquidityTotalsResponse, AuthError> {
    if !is_valid_bytes32(&on_chain.event_id) {
        return Ok(empty_liquidity_totals());
    }

    let totals = stellar::get_event_liquidity(&state.env, &on_chain.event_id)
        .await
        .map_err(|error| AuthError::internal("event liquidity read failed", error))?;
    Ok(liquidity_totals_response(&totals))
}

async fn load_market_position_response(
    state: &AppState,
    condition_id: Option<&str>,
    provider: &str,
) -> Result<LiquidityPositionResponse, AuthError> {
    Ok(liquidity_position_response(
        &load_market_position_raw(state, condition_id, provider).await?,
    ))
}

async fn load_market_position_raw(
    state: &AppState,
    condition_id: Option<&str>,
    provider: &str,
) -> Result<LiquidityPositionReadResult, AuthError> {
    let Some(condition_id) = condition_id else {
        return Ok(empty_liquidity_position_read());
    };
    if !is_valid_bytes32(condition_id) {
        return Ok(empty_liquidity_position_read());
    }

    stellar::get_liquidity_position(&state.env, condition_id, provider)
        .await
        .map_err(|error| AuthError::internal("market position read failed", error))
}

async fn load_market_liquidity_response(
    state: &AppState,
    condition_id: Option<&str>,
) -> Result<LiquidityTotalsResponse, AuthError> {
    let Some(condition_id) = condition_id else {
        return Ok(empty_liquidity_totals());
    };
    if !is_valid_bytes32(condition_id) {
        return Ok(empty_liquidity_totals());
    }

    let totals = stellar::get_market_liquidity(&state.env, condition_id)
        .await
        .map_err(|error| AuthError::internal("market liquidity read failed", error))?;
    Ok(liquidity_totals_response(&totals))
}

async fn load_liquidity_wallet_context(
    state: &AppState,
    user_id: Uuid,
    require_managed: bool,
) -> Result<LiquidityWalletContext, AuthError> {
    let wallet = auth_crud::get_wallet_for_user(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::unauthorized("wallet not linked to user"))?;

    let deployed_wallet_address = wallet
        .wallet_address
        .ok_or_else(|| AuthError::forbidden("wallet is not deployed"))?;
    let actor_address = wallet
        .owner_address
        .clone()
        .unwrap_or_else(|| deployed_wallet_address.clone());
    let wallet_address = actor_address.clone();

    if !require_managed {
        return Ok(LiquidityWalletContext {
            wallet_address,
            actor_address,
            source_account: None,
        });
    }

    if wallet.account_kind != ACCOUNT_KIND_STELLAR_SMART_WALLET {
        return Err(AuthError::unprocessable_entity(
            "liquidity writes currently require a managed smart wallet",
        ));
    }

    let encrypted_private_key = wallet
        .owner_encrypted_private_key
        .ok_or_else(|| AuthError::forbidden("wallet owner key is missing"))?;
    let encryption_nonce = wallet
        .owner_encryption_nonce
        .ok_or_else(|| AuthError::forbidden("wallet owner nonce is missing"))?;
    let decrypted = decrypt_private_key(&state.env, &encrypted_private_key, &encryption_nonce)
        .map_err(|error| AuthError::internal("failed to decrypt managed wallet owner key", error))?;
    let secret_seed_bytes: [u8; 32] = decrypted
        .as_slice()
        .try_into()
        .map_err(|_| AuthError::internal("invalid managed wallet owner key length", "expected 32 bytes"))?;

    Ok(LiquidityWalletContext {
        wallet_address,
        actor_address,
        source_account: Some(encode_stellar_secret_key(&secret_seed_bytes)),
    })
}

fn liquidity_totals_response(value: &MarketLiquidityReadResult) -> LiquidityTotalsResponse {
    LiquidityTotalsResponse {
        idle_yes_total: format_contract_amount(&value.idle_yes_total),
        idle_no_total: format_contract_amount(&value.idle_no_total),
        posted_yes_total: format_contract_amount(&value.posted_yes_total),
        posted_no_total: format_contract_amount(&value.posted_no_total),
        claimable_collateral_total: format_contract_amount(&value.claimable_collateral_total),
    }
}

fn liquidity_position_response(value: &LiquidityPositionReadResult) -> LiquidityPositionResponse {
    LiquidityPositionResponse {
        posted_yes_amount: format_contract_amount(&value.posted_yes_amount),
        posted_no_amount: format_contract_amount(&value.posted_no_amount),
        idle_yes_amount: format_contract_amount(&value.idle_yes_amount),
        idle_no_amount: format_contract_amount(&value.idle_no_amount),
        collateral_amount: format_contract_amount(&value.collateral_amount),
        claimable_collateral_amount: format_contract_amount(&value.claimable_collateral_amount),
        updated_at: value.updated_at,
        active: value.active,
    }
}

fn resolve_recipient(
    liquidity: WithdrawInventoryFieldsRequest,
    default_recipient: &str,
) -> Result<String, AuthError> {
        liquidity
            .recipient
            .as_deref()
        .map(crate::service::auth::normalize_stellar_address)
        .transpose()?
        .or_else(|| Some(default_recipient.to_owned()))
        .ok_or_else(|| AuthError::bad_request("recipient is required"))
}

fn resolve_collateral_recipient(
    liquidity: WithdrawCollateralFieldsRequest,
    default_recipient: &str,
) -> Result<String, AuthError> {
        liquidity
            .recipient
            .as_deref()
        .map(crate::service::auth::normalize_stellar_address)
        .transpose()?
        .or_else(|| Some(default_recipient.to_owned()))
        .ok_or_else(|| AuthError::bad_request("recipient is required"))
}

fn parse_amount(raw: &str, field_name: &str, allow_zero: bool) -> Result<String, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }

    let parsed = value.parse::<u128>().map_err(|_| {
        AuthError::bad_request(format!("{field_name} must be a base-10 integer string"))
    })?;

    if parsed == 0 && !allow_zero {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be greater than zero"
        )));
    }

    Ok(parsed.to_string())
}

fn ensure_inventory_pair_non_zero(yes_amount: &str, no_amount: &str) -> Result<(), AuthError> {
    if parse_contract_amount(yes_amount) == 0 && parse_contract_amount(no_amount) == 0 {
        return Err(AuthError::bad_request(
            "at least one liquidity amount must be greater than zero",
        ));
    }

    Ok(())
}

fn empty_liquidity_totals() -> LiquidityTotalsResponse {
    LiquidityTotalsResponse {
        idle_yes_total: "0".to_owned(),
        idle_no_total: "0".to_owned(),
        posted_yes_total: "0".to_owned(),
        posted_no_total: "0".to_owned(),
        claimable_collateral_total: "0".to_owned(),
    }
}

fn empty_liquidity_position_read() -> LiquidityPositionReadResult {
    LiquidityPositionReadResult {
        posted_yes_amount: "0".to_owned(),
        posted_no_amount: "0".to_owned(),
        idle_yes_amount: "0".to_owned(),
        idle_no_amount: "0".to_owned(),
        collateral_amount: "0".to_owned(),
        claimable_collateral_amount: "0".to_owned(),
        updated_at: None,
        active: false,
    }
}

fn has_position_balance(position: &LiquidityPositionReadResult) -> bool {
    position.active
        || parse_contract_amount(&position.posted_yes_amount) > 0
        || parse_contract_amount(&position.posted_no_amount) > 0
        || parse_contract_amount(&position.idle_yes_amount) > 0
        || parse_contract_amount(&position.idle_no_amount) > 0
        || parse_contract_amount(&position.collateral_amount) > 0
        || parse_contract_amount(&position.claimable_collateral_amount) > 0
}

fn parse_contract_amount(raw: &str) -> u128 {
    raw.parse::<u128>().unwrap_or(0)
}

fn format_contract_amount(raw: &str) -> String {
    format_contract_amount_u128(parse_contract_amount(raw))
}

fn format_contract_amount_u128(value: u128) -> String {
    format_decimal_str(&value.to_string())
}

fn format_decimal_str(raw: &str) -> String {
    if raw.is_empty() {
        return "0".to_owned();
    }

    let raw = raw.trim_start_matches('0');
    if raw.is_empty() {
        return "0".to_owned();
    }

    if raw.len() <= USDC_DECIMALS {
        return format_fractional("0", &format!("{raw:0>6}"));
    }

    let split_index = raw.len() - USDC_DECIMALS;
    format_fractional(&raw[..split_index], &raw[split_index..])
}

fn format_fractional(whole: &str, fractional: &str) -> String {
    let trimmed_fractional = fractional.trim_end_matches('0');
    if trimmed_fractional.is_empty() {
        whole.to_owned()
    } else {
        format!("{whole}.{trimmed_fractional}")
    }
}

fn validate_bytes32(value: &str, field_name: &str) -> Result<(), AuthError> {
    if is_valid_bytes32(value) {
        return Ok(());
    }

    Err(AuthError::bad_request(format!(
        "{field_name} must be a 32-byte hex string"
    )))
}

fn is_valid_bytes32(value: &str) -> bool {
    let normalized = value.trim().trim_matches('"');
    let normalized = normalized.strip_prefix("0x").unwrap_or(normalized);
    let normalized = normalized.strip_prefix("0X").unwrap_or(normalized);
    normalized.len() == 64 && normalized.chars().all(|ch| ch.is_ascii_hexdigit())
}

fn map_liquidity_chain_error(context: &'static str, error: anyhow::Error) -> AuthError {
    let message = format!("{error:#}");
    let lower = message.to_ascii_lowercase();
    if lower.contains("insufficient")
        || lower.contains("allowance")
        || lower.contains("balance")
        || lower.contains("txbadseq")
        || lower.contains("bad seq")
        || lower.contains("transaction submission timeout")
        || lower.contains("negative")
        || lower.contains("invalid")
        || lower.contains("invalidaction")
        || lower.contains("simulation failed")
        || lower.contains("account not found")
        || lower.contains("not published")
    {
        return AuthError::bad_request(message);
    }

    AuthError::internal(context, error)
}
