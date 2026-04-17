use std::collections::HashMap;

use anyhow::{Context, Result};
use tokio::time::{Duration, MissedTickBehavior, interval};

use crate::{
    app::AppState, module::market::crud, service::monad::get_market_prices_batch_best_effort,
};

#[derive(Debug, Default, Clone, Copy)]
struct MarketPriceSyncStats {
    scanned: usize,
    fetched: usize,
    updated: usize,
    unchanged: usize,
    invalid: usize,
    failed_reads: usize,
}

pub fn spawn_market_price_sync(state: AppState) {
    let interval_secs = state.env.market_price_sync_interval_secs;
    if interval_secs == 0 {
        tracing::info!("market price sync disabled");
        return;
    }

    tokio::spawn(async move {
        tracing::info!(interval_secs, "market price sync started");

        if let Err(error) = sync_market_price_snapshots_once(&state).await {
            tracing::warn!(%error, "initial market price sync failed");
        }

        let mut ticker = interval(Duration::from_secs(interval_secs));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        ticker.tick().await;

        loop {
            ticker.tick().await;

            if let Err(error) = sync_market_price_snapshots_once(&state).await {
                tracing::warn!(%error, "scheduled market price sync failed");
            }
        }
    });
}

pub async fn sync_market_price_snapshots_once(state: &AppState) -> Result<()> {
    let markets = crud::list_published_market_condition_ids(&state.db)
        .await
        .context("failed to load published market condition ids")?;
    if markets.is_empty() {
        tracing::debug!("market price sync skipped because there are no published markets");
        return Ok(());
    }

    let condition_ids = markets
        .iter()
        .map(|market| market.condition_id.clone())
        .collect::<Vec<_>>();
    let existing = crud::list_market_price_snapshots_by_condition_ids(&state.db, &condition_ids)
        .await
        .context("failed to load existing market price snapshots")?;
    let existing_by_condition = existing
        .into_iter()
        .map(|snapshot| (snapshot.condition_id.clone(), snapshot))
        .collect::<HashMap<_, _>>();

    let fetched = get_market_prices_batch_best_effort(&state.env, &condition_ids)
        .await
        .context("failed to fetch market prices from RPC")?;

    let mut stats = MarketPriceSyncStats {
        scanned: markets.len(),
        fetched: fetched.len(),
        ..MarketPriceSyncStats::default()
    };

    for market in markets {
        let Some(prices) = fetched.get(&market.condition_id) else {
            stats.failed_reads += 1;
            continue;
        };

        if prices.yes_bps + prices.no_bps != 10_000 {
            stats.invalid += 1;
            tracing::debug!(
                market_id = %market.market_id,
                condition_id = %market.condition_id,
                yes_bps = prices.yes_bps,
                no_bps = prices.no_bps,
                "skipping invalid market price snapshot"
            );
            continue;
        }

        if existing_by_condition
            .get(&market.condition_id)
            .is_some_and(|snapshot| {
                snapshot.yes_bps == prices.yes_bps as i32 && snapshot.no_bps == prices.no_bps as i32
            })
        {
            stats.unchanged += 1;
            continue;
        }

        crud::upsert_market_price_snapshot(
            &state.db,
            market.market_id,
            &market.condition_id,
            prices.yes_bps as i32,
            prices.no_bps as i32,
        )
        .await
        .with_context(|| {
            format!(
                "failed to upsert market price snapshot for {}",
                market.market_id
            )
        })?;
        crud::insert_market_price_history_snapshot(
            &state.db,
            market.market_id,
            &market.condition_id,
            prices.yes_bps as i32,
            prices.no_bps as i32,
        )
        .await
        .with_context(|| {
            format!(
                "failed to append market price history snapshot for {}",
                market.market_id
            )
        })?;
        stats.updated += 1;
    }

    tracing::info!(
        scanned = stats.scanned,
        fetched = stats.fetched,
        updated = stats.updated,
        unchanged = stats.unchanged,
        invalid = stats.invalid,
        failed_reads = stats.failed_reads,
        "market price sync completed"
    );

    Ok(())
}
