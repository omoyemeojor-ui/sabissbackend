use std::collections::HashMap;

use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::{
            crud as market_crud,
            model::{MarketPriceSnapshotRecord, MarketRecord, MarketTradeStatsRecord},
            schema::{
                MarketCurrentPricesResponse, MarketQuoteSummaryResponse, MarketResponse,
                MarketStatsResponse,
            },
        },
    },
};

pub async fn build_market_response(
    state: &AppState,
    market: &MarketRecord,
) -> Result<MarketResponse, AuthError> {
    let mut responses = build_market_responses(state, std::slice::from_ref(market)).await?;
    Ok(responses
        .pop()
        .unwrap_or_else(|| MarketResponse::from(market)))
}

pub async fn build_market_responses(
    state: &AppState,
    markets: &[MarketRecord],
) -> Result<Vec<MarketResponse>, AuthError> {
    if markets.is_empty() {
        return Ok(Vec::new());
    }

    let snapshots_by_condition = load_snapshots_by_condition(state, markets).await?;
    let stats_by_market_id = load_stats_by_market_id(state, markets).await?;

    markets
        .iter()
        .map(|market| {
            build_market_response_from_maps(market, &snapshots_by_condition, &stats_by_market_id)
        })
        .collect()
}

async fn load_snapshots_by_condition(
    state: &AppState,
    markets: &[MarketRecord],
) -> Result<HashMap<String, MarketPriceSnapshotRecord>, AuthError> {
    let condition_ids = markets
        .iter()
        .filter_map(|market| market.condition_id.clone())
        .collect::<Vec<_>>();
    let snapshots =
        market_crud::list_market_price_snapshots_by_condition_ids(&state.db, &condition_ids)
            .await?;

    Ok(snapshots
        .into_iter()
        .map(|snapshot| (snapshot.condition_id.clone(), snapshot))
        .collect())
}

async fn load_stats_by_market_id(
    state: &AppState,
    markets: &[MarketRecord],
) -> Result<HashMap<Uuid, MarketTradeStatsRecord>, AuthError> {
    let market_ids = markets.iter().map(|market| market.id).collect::<Vec<_>>();
    let stats = market_crud::list_market_trade_stats_by_market_ids(&state.db, &market_ids).await?;

    Ok(stats
        .into_iter()
        .map(|record| (record.market_id, record))
        .collect())
}

fn build_market_response_from_maps(
    market: &MarketRecord,
    snapshots_by_condition: &HashMap<String, MarketPriceSnapshotRecord>,
    stats_by_market_id: &HashMap<Uuid, MarketTradeStatsRecord>,
) -> Result<MarketResponse, AuthError> {
    let snapshot = market
        .condition_id
        .as_ref()
        .and_then(|condition_id| snapshots_by_condition.get(condition_id));

    Ok(MarketResponse {
        current_prices: snapshot.map(current_prices_from_snapshot).transpose()?,
        stats: Some(MarketStatsResponse {
            volume_usd: format_usd_cents(
                stats_by_market_id
                    .get(&market.id)
                    .map_or(0, |record| record.volume_usd_cents),
            ),
        }),
        quote_summary: snapshot.map(quote_summary_from_snapshot).transpose()?,
        ..MarketResponse::from(market)
    })
}

fn current_prices_from_snapshot(
    snapshot: &MarketPriceSnapshotRecord,
) -> Result<MarketCurrentPricesResponse, AuthError> {
    let yes_bps = u32::try_from(snapshot.yes_bps)
        .map_err(|error| AuthError::internal("invalid YES snapshot price", error))?;
    let no_bps = u32::try_from(snapshot.no_bps)
        .map_err(|error| AuthError::internal("invalid NO snapshot price", error))?;

    Ok(MarketCurrentPricesResponse { yes_bps, no_bps })
}

fn quote_summary_from_snapshot(
    snapshot: &MarketPriceSnapshotRecord,
) -> Result<MarketQuoteSummaryResponse, AuthError> {
    let current_prices = current_prices_from_snapshot(snapshot)?;

    Ok(MarketQuoteSummaryResponse {
        buy_yes_bps: current_prices.yes_bps,
        buy_no_bps: current_prices.no_bps,
        as_of: snapshot.synced_at,
        source: "price_snapshot".to_owned(),
    })
}

fn format_usd_cents(value: i64) -> String {
    let whole = value / 100;
    let fractional = value.rem_euclid(100);
    format!("{whole}.{fractional:02}")
}
