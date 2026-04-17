use std::collections::HashMap;

use ethers_core::types::U256;
use tokio::task::JoinSet;
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        liquidity::schema::{
            EventLiquidityResponse, MyEventLiquidityMarketResponse, MyEventLiquidityResponse,
            MyMarketLiquidityResponse,
        },
        market::schema::{EventOnChainResponse, EventResponse, MarketResponse},
    },
    service::{
        jwt::AuthenticatedUser,
        liquidity::{
            chain_read::{self, LiquidityPositionReadResult, LiquidityTotalsReadResult},
            context::{
                load_public_event_context, load_public_event_markets, load_public_market_context,
            },
            format::{
                PositionAccumulator, has_position_balance, liquidity_position_response,
                liquidity_totals_response,
            },
            view::{build_market_response, build_market_responses},
            wallet::load_user_wallet_context,
        },
    },
};

pub async fn get_event_liquidity(
    state: &AppState,
    event_id: Uuid,
) -> Result<EventLiquidityResponse, AuthError> {
    let context = load_public_event_context(state, event_id).await?;
    let liquidity = chain_read::get_event_liquidity(&state.env, &context.event.event_id)
        .await
        .map_err(|error| AuthError::internal("event liquidity read failed", error))?;

    Ok(EventLiquidityResponse {
        event: EventResponse::from(&context.event),
        on_chain: EventOnChainResponse::from(&context.event),
        markets_count: context.markets_count,
        liquidity: liquidity_totals_response(&liquidity),
    })
}

pub async fn get_my_market_liquidity(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
) -> Result<MyMarketLiquidityResponse, AuthError> {
    let wallet = load_user_wallet_context(state, authenticated_user.user_id).await?;
    let context = load_public_market_context(state, market_id).await?;
    let market = build_market_response(state, &context.market).await?;
    let position = load_market_position(
        state,
        context.market.condition_id.as_deref(),
        &wallet.wallet_address,
    )
    .await?;
    let market_liquidity =
        load_market_liquidity(state, context.market.condition_id.as_deref()).await?;

    Ok(MyMarketLiquidityResponse {
        event: EventResponse::from(&context.event),
        on_chain: EventOnChainResponse::from(&context.event),
        market,
        wallet_address: wallet.wallet_address,
        position: liquidity_position_response(&position),
        market_liquidity: liquidity_totals_response(&market_liquidity),
    })
}

pub async fn get_my_event_liquidity(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    event_id: Uuid,
) -> Result<MyEventLiquidityResponse, AuthError> {
    let wallet = load_user_wallet_context(state, authenticated_user.user_id).await?;
    let (event, markets) = load_public_event_markets(state, event_id).await?;
    let event_liquidity = chain_read::get_event_liquidity(&state.env, &event.event_id)
        .await
        .map_err(|error| AuthError::internal("event liquidity read failed", error))?;
    let mut market_responses = build_market_responses(state, &markets)
        .await?
        .into_iter()
        .map(|market| (market.id, market))
        .collect::<HashMap<_, _>>();
    let (positions_by_market_id, liquidity_by_market_id) =
        load_event_market_states(state, &markets, &wallet.wallet_address).await?;

    let mut totals = PositionAccumulator::default();
    let mut response_markets = Vec::new();
    for market in &markets {
        let position = positions_by_market_id
            .get(&market.id)
            .cloned()
            .unwrap_or_default();
        totals.add(&position);

        if !has_position_balance(&position) {
            continue;
        }

        let market_liquidity = liquidity_by_market_id
            .get(&market.id)
            .cloned()
            .unwrap_or_else(empty_liquidity_totals);

        response_markets.push(MyEventLiquidityMarketResponse {
            market: market_responses
                .remove(&market.id)
                .unwrap_or_else(|| MarketResponse::from(market)),
            position: liquidity_position_response(&position),
            market_liquidity: liquidity_totals_response(&market_liquidity),
        });
    }

    Ok(MyEventLiquidityResponse {
        event: EventResponse::from(&event),
        on_chain: EventOnChainResponse::from(&event),
        wallet_address: wallet.wallet_address,
        event_liquidity: liquidity_totals_response(&event_liquidity),
        position_totals: totals.into_response(),
        markets: response_markets,
    })
}

async fn load_event_market_states(
    state: &AppState,
    markets: &[crate::module::market::model::MarketRecord],
    wallet_address: &str,
) -> Result<
    (
        HashMap<Uuid, LiquidityPositionReadResult>,
        HashMap<Uuid, LiquidityTotalsReadResult>,
    ),
    AuthError,
> {
    let mut join_set = JoinSet::new();

    for market in markets {
        let Some(condition_id) = market.condition_id.clone() else {
            continue;
        };
        let env = state.env.clone();
        let wallet_address = wallet_address.to_owned();
        let market_id = market.id;

        join_set.spawn(async move {
            let position =
                chain_read::get_liquidity_position(&env, &condition_id, &wallet_address).await?;
            let liquidity = chain_read::get_market_liquidity(&env, &condition_id).await?;
            Ok::<_, anyhow::Error>((market_id, position, liquidity))
        });
    }

    let mut positions_by_market_id = HashMap::new();
    let mut liquidity_by_market_id = HashMap::new();
    while let Some(result) = join_set.join_next().await {
        let (market_id, position, liquidity) = result
            .map_err(|error| AuthError::internal("event liquidity task join failed", error))?
            .map_err(|error| AuthError::internal("event market liquidity read failed", error))?;
        positions_by_market_id.insert(market_id, position);
        liquidity_by_market_id.insert(market_id, liquidity);
    }

    Ok((positions_by_market_id, liquidity_by_market_id))
}

async fn load_market_position(
    state: &AppState,
    condition_id: Option<&str>,
    wallet_address: &str,
) -> Result<LiquidityPositionReadResult, AuthError> {
    let Some(condition_id) = condition_id else {
        return Ok(LiquidityPositionReadResult::default());
    };

    chain_read::get_liquidity_position(&state.env, condition_id, wallet_address)
        .await
        .map_err(|error| AuthError::internal("market position read failed", error))
}

async fn load_market_liquidity(
    state: &AppState,
    condition_id: Option<&str>,
) -> Result<LiquidityTotalsReadResult, AuthError> {
    let Some(condition_id) = condition_id else {
        return Ok(empty_liquidity_totals());
    };

    chain_read::get_market_liquidity(&state.env, condition_id)
        .await
        .map_err(|error| AuthError::internal("market liquidity read failed", error))
}

fn empty_liquidity_totals() -> LiquidityTotalsReadResult {
    LiquidityTotalsReadResult {
        idle_yes_total: U256::zero(),
        idle_no_total: U256::zero(),
        posted_yes_total: U256::zero(),
        posted_no_total: U256::zero(),
        claimable_collateral_total: U256::zero(),
    }
}
