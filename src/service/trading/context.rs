use chrono::Utc;
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

pub struct TradingMarketContext {
    pub event: MarketEventRecord,
    pub market: MarketRecord,
    pub condition_id: String,
}

pub async fn load_trading_market_context(
    state: &AppState,
    market_id: Uuid,
) -> Result<TradingMarketContext, AuthError> {
    let market = market_crud::get_public_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;
    ensure_market_tradeable(&market)?;

    let event = market_crud::get_public_market_event_by_id(&state.db, market.event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;
    let condition_id = market
        .condition_id
        .clone()
        .ok_or_else(|| AuthError::bad_request("market is not published on-chain"))?;

    Ok(TradingMarketContext {
        event,
        market,
        condition_id,
    })
}

pub fn outcome_label(market: &MarketRecord, outcome_index: i32) -> Result<String, AuthError> {
    let index = usize::try_from(outcome_index)
        .map_err(|_| AuthError::bad_request("trade.outcome_index must be 0 or 1"))?;
    market
        .outcomes
        .get(index)
        .cloned()
        .ok_or_else(|| AuthError::bad_request("trade.outcome_index must be 0 or 1"))
}

fn ensure_market_tradeable(market: &MarketRecord) -> Result<(), AuthError> {
    if market.outcome_count != 2 || market.outcomes.len() != 2 {
        return Err(AuthError::bad_request(
            "trade routes currently support binary markets only",
        ));
    }

    match market.trading_status.as_str() {
        "active" => {}
        "paused" => return Err(AuthError::bad_request("market is paused")),
        "resolved" => return Err(AuthError::bad_request("market already resolved")),
        _ => return Err(AuthError::bad_request("market is not currently tradable")),
    }

    if market.end_time <= Utc::now() {
        return Err(AuthError::bad_request("market trading has ended"));
    }

    Ok(())
}
