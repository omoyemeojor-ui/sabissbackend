use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::{
            crud as market_crud,
            model::{MarketEventRecord, MarketRecord},
        },
    },
};

pub struct PublicEventLiquidityContext {
    pub event: MarketEventRecord,
    pub markets_count: i64,
}

pub struct PublicMarketLiquidityContext {
    pub event: MarketEventRecord,
    pub market: MarketRecord,
}

pub async fn load_public_event_context(
    state: &AppState,
    event_id: Uuid,
) -> Result<PublicEventLiquidityContext, AuthError> {
    let event = market_crud::get_public_market_event_by_id(&state.db, event_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;
    let markets_count = market_crud::count_public_markets_for_event(&state.db, event_id).await?;

    Ok(PublicEventLiquidityContext {
        event,
        markets_count,
    })
}

pub async fn load_public_market_context(
    state: &AppState,
    market_id: Uuid,
) -> Result<PublicMarketLiquidityContext, AuthError> {
    let market = market_crud::get_public_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;
    let event = market_crud::get_public_market_event_by_id(&state.db, market.event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;

    Ok(PublicMarketLiquidityContext { event, market })
}

pub async fn load_public_event_markets(
    state: &AppState,
    event_id: Uuid,
) -> Result<(MarketEventRecord, Vec<MarketRecord>), AuthError> {
    let event = market_crud::get_public_market_event_by_id(&state.db, event_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;
    let markets = market_crud::list_public_markets_for_event(&state.db, event_id).await?;

    Ok((event, markets))
}
