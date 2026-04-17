use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{auth::error::AuthError, market::crud},
    service::monad,
};

use super::format::{last_trade_yes_bps, volume_usd_cents};

pub struct TradeStateSnapshot {
    pub yes_bps: u32,
    pub no_bps: u32,
    pub last_trade_yes_bps: u32,
    pub as_of: DateTime<Utc>,
}

pub async fn sync_trade_state(
    state: &AppState,
    market_id: Uuid,
    condition_id: &str,
    outcome_index: i32,
    outcome_price_bps: u32,
    usdc_amount: &ethers_core::types::U256,
) -> Result<TradeStateSnapshot, AuthError> {
    let prices = monad::get_market_prices(&state.env, condition_id)
        .await
        .map_err(|error| AuthError::internal("market price refresh failed", error))?;
    let snapshot = upsert_market_price_snapshot(
        state,
        market_id,
        condition_id,
        prices.yes_bps,
        prices.no_bps,
    )
    .await?;
    let last_trade_yes_bps = last_trade_yes_bps(outcome_index, outcome_price_bps)?;
    let volume_usd_cents = volume_usd_cents(usdc_amount)?;

    crud::upsert_market_trade_execution(
        &state.db,
        market_id,
        volume_usd_cents,
        last_trade_yes_bps,
        Utc::now(),
    )
    .await?;

    Ok(TradeStateSnapshot {
        yes_bps: prices.yes_bps,
        no_bps: prices.no_bps,
        last_trade_yes_bps: u32::try_from(last_trade_yes_bps)
            .map_err(|error| AuthError::internal("invalid last trade price", error))?,
        as_of: snapshot.synced_at,
    })
}

async fn upsert_market_price_snapshot(
    state: &AppState,
    market_id: Uuid,
    condition_id: &str,
    yes_bps: u32,
    no_bps: u32,
) -> Result<crate::module::market::model::MarketPriceSnapshotRecord, AuthError> {
    let yes_bps =
        i32::try_from(yes_bps).map_err(|error| AuthError::internal("invalid YES price", error))?;
    let no_bps =
        i32::try_from(no_bps).map_err(|error| AuthError::internal("invalid NO price", error))?;

    if yes_bps + no_bps != 10_000 {
        return Err(AuthError::bad_request("market prices must sum to 10000"));
    }

    let existing = crud::get_market_price_snapshot_by_market_id(&state.db, market_id).await?;
    let snapshot =
        crud::upsert_market_price_snapshot(&state.db, market_id, condition_id, yes_bps, no_bps)
            .await?;
    let should_append_history = existing.as_ref().is_none_or(|record| {
        record.yes_bps != yes_bps || record.no_bps != no_bps || record.condition_id != condition_id
    });

    if should_append_history {
        crud::insert_market_price_history_snapshot(
            &state.db,
            market_id,
            condition_id,
            yes_bps,
            no_bps,
        )
        .await?;
    }

    Ok(snapshot)
}
