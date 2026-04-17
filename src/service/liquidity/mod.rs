use chrono::Utc;
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        liquidity::{
            crud,
            schema::{
                empty_liquidity_position, empty_liquidity_totals, DepositCollateralRequest,
                DepositInventoryRequest, EventLiquidityResponse, LiquidityWriteResponse,
                MyEventLiquidityMarketResponse, MyEventLiquidityPositionTotalsResponse,
                MyEventLiquidityResponse, MyMarketLiquidityResponse, RemoveLiquidityRequest,
                WithdrawCollateralRequest, WithdrawInventoryRequest,
            },
        },
    },
    service::{
        jwt::AuthenticatedUser,
        market::{get_event_by_id, get_event_markets, get_market_by_id},
    },
};

pub async fn get_event_liquidity(
    state: &AppState,
    event_id: Uuid,
) -> Result<EventLiquidityResponse, AuthError> {
    let detail = get_event_by_id(state, event_id).await?;
    Ok(EventLiquidityResponse {
        event: detail.event,
        on_chain: detail.on_chain,
        markets_count: detail.markets_count,
        liquidity: empty_liquidity_totals(),
    })
}

pub async fn get_my_market_liquidity(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
) -> Result<MyMarketLiquidityResponse, AuthError> {
    let detail = get_market_by_id(state, market_id).await?;
    let wallet = load_wallet_address(state, authenticated_user.user_id).await?;

    Ok(MyMarketLiquidityResponse {
        event: detail.event,
        on_chain: detail.on_chain,
        market: detail.market,
        wallet_address: wallet,
        position: empty_liquidity_position(),
        market_liquidity: empty_liquidity_totals(),
    })
}

pub async fn get_my_event_liquidity(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    event_id: Uuid,
) -> Result<MyEventLiquidityResponse, AuthError> {
    let detail = get_event_by_id(state, event_id).await?;
    let markets = get_event_markets(state, event_id, Default::default()).await?;
    let wallet = load_wallet_address(state, authenticated_user.user_id).await?;

    Ok(MyEventLiquidityResponse {
        event: detail.event,
        on_chain: detail.on_chain,
        wallet_address: wallet,
        event_liquidity: empty_liquidity_totals(),
        position_totals: MyEventLiquidityPositionTotalsResponse {
            active_markets: 0,
            posted_yes_amount: "0".to_owned(),
            posted_no_amount: "0".to_owned(),
            idle_yes_amount: "0".to_owned(),
            idle_no_amount: "0".to_owned(),
            collateral_amount: "0".to_owned(),
            claimable_collateral_amount: "0".to_owned(),
        },
        markets: markets
            .markets
            .into_iter()
            .map(|market| MyEventLiquidityMarketResponse {
                market,
                position: empty_liquidity_position(),
                market_liquidity: empty_liquidity_totals(),
            })
            .collect(),
    })
}

pub async fn deposit_market_inventory(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    _payload: DepositInventoryRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    unsupported_write(state, authenticated_user, market_id, "deposit_inventory").await
}

pub async fn deposit_market_collateral(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    _payload: DepositCollateralRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    unsupported_write(state, authenticated_user, market_id, "deposit_collateral").await
}

pub async fn remove_market_liquidity(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    _payload: RemoveLiquidityRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    unsupported_write(state, authenticated_user, market_id, "remove").await
}

pub async fn withdraw_market_inventory(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    _payload: WithdrawInventoryRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    unsupported_write(state, authenticated_user, market_id, "withdraw_inventory").await
}

pub async fn withdraw_market_collateral(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    _payload: WithdrawCollateralRequest,
) -> Result<LiquidityWriteResponse, AuthError> {
    unsupported_write(state, authenticated_user, market_id, "withdraw_collateral").await
}

async fn unsupported_write(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    action: &str,
) -> Result<LiquidityWriteResponse, AuthError> {
    let detail = get_market_by_id(state, market_id).await?;
    let wallet = load_wallet_address(state, authenticated_user.user_id).await?;

    let response = LiquidityWriteResponse {
        event: detail.event,
        on_chain: detail.on_chain,
        market: detail.market,
        wallet_address: wallet,
        action: action.to_owned(),
        tx_hash: String::new(),
        position: empty_liquidity_position(),
        market_liquidity: empty_liquidity_totals(),
        updated_at: Utc::now(),
    };

    Err(AuthError::unprocessable_entity(format!(
        "liquidity writes are not implemented in this Soroban backend yet for action `{}`",
        response.action
    )))
}

async fn load_wallet_address(state: &AppState, user_id: Uuid) -> Result<String, AuthError> {
    let wallet = crud::get_user_wallet_account_by_user_id(&state.db, user_id)
        .await?
        .ok_or_else(|| AuthError::bad_request("wallet account not linked"))?;
    Ok(wallet.wallet_address)
}
