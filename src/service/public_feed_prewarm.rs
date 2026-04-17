use std::{collections::BTreeSet, time::Instant};

use serde_json::json;
use tokio::time::{Duration, MissedTickBehavior, interval};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::schema::{EventMarketsQuery, ListEventsQuery},
    },
    service::{
        cache::{self, HOT_FEED_TTL_SECS},
        market::{get_event_markets, list_events},
    },
};

const HOT_PAGE_LIMIT: i64 = 12;
const HOT_PAGE_OFFSETS: [i64; 3] = [0, 12, 24];
const PREWARM_INTERVAL_SECS: u64 = 240;

#[derive(Debug, Default, Clone, Copy)]
struct PublicFeedPrewarmStats {
    event_pages: usize,
    event_page_failures: usize,
    event_market_pages: usize,
    event_market_failures: usize,
    unique_events: usize,
}

pub fn spawn_public_feed_prewarm(state: AppState) {
    if state.cache.is_none() {
        tracing::info!(
            "public feed prewarm disabled because redis cache is unavailable; local on-demand caching remains enabled"
        );
        return;
    }

    tokio::spawn(async move {
        tracing::info!(
            interval_secs = PREWARM_INTERVAL_SECS,
            ttl_secs = HOT_FEED_TTL_SECS,
            "public feed prewarm started"
        );

        if let Err(error) = prewarm_public_feed_once(&state).await {
            tracing::warn!(%error, "initial public feed prewarm failed");
        }

        let mut ticker = interval(Duration::from_secs(PREWARM_INTERVAL_SECS));
        ticker.set_missed_tick_behavior(MissedTickBehavior::Skip);
        ticker.tick().await;

        loop {
            ticker.tick().await;

            if let Err(error) = prewarm_public_feed_once(&state).await {
                tracing::warn!(%error, "scheduled public feed prewarm failed");
            }
        }
    });
}

async fn prewarm_public_feed_once(state: &AppState) -> Result<(), AuthError> {
    let started = Instant::now();
    let mut stats = PublicFeedPrewarmStats::default();
    let mut event_ids = BTreeSet::<Uuid>::new();
    let (page_0, page_12, page_24) = tokio::join!(
        prewarm_event_page(state, HOT_PAGE_OFFSETS[0]),
        prewarm_event_page(state, HOT_PAGE_OFFSETS[1]),
        prewarm_event_page(state, HOT_PAGE_OFFSETS[2]),
    );

    for (offset, result) in [
        (HOT_PAGE_OFFSETS[0], page_0),
        (HOT_PAGE_OFFSETS[1], page_12),
        (HOT_PAGE_OFFSETS[2], page_24),
    ] {
        match result {
            Ok(ids) => {
                stats.event_pages += 1;
                event_ids.extend(ids);
            }
            Err(error) => {
                stats.event_page_failures += 1;
                tracing::warn!(%error, offset, "public event page prewarm failed");
            }
        }
    }

    stats.unique_events = event_ids.len();

    for event_id in event_ids {
        match prewarm_event_markets_page(state, event_id).await {
            Ok(()) => {
                stats.event_market_pages += 1;
            }
            Err(error) => {
                stats.event_market_failures += 1;
                tracing::warn!(%error, %event_id, "event markets page prewarm failed");
            }
        }
    }

    tracing::info!(
        event_pages = stats.event_pages,
        event_page_failures = stats.event_page_failures,
        event_market_pages = stats.event_market_pages,
        event_market_failures = stats.event_market_failures,
        unique_events = stats.unique_events,
        elapsed_ms = started.elapsed().as_millis(),
        "public feed prewarm completed"
    );

    Ok(())
}

async fn prewarm_event_page(state: &AppState, offset: i64) -> Result<Vec<Uuid>, AuthError> {
    let query = ListEventsQuery {
        include_markets: Some(true),
        limit: Some(HOT_PAGE_LIMIT),
        offset: Some(offset),
        ..ListEventsQuery::default()
    };
    let response = list_events(state, query.clone()).await?;
    let key = cache::build_cache_key("events:index", &query)?;

    cache::store_json(state.cache.as_ref(), key, HOT_FEED_TTL_SECS, &response).await?;

    Ok(response.events.into_iter().map(|event| event.id).collect())
}

async fn prewarm_event_markets_page(state: &AppState, event_id: Uuid) -> Result<(), AuthError> {
    let query = EventMarketsQuery::default();
    let response = get_event_markets(state, event_id, query.clone()).await?;
    let key = cache::build_cache_key(
        "events:markets",
        &json!({ "event_id": event_id, "query": query }),
    )?;

    cache::store_json(state.cache.as_ref(), key, HOT_FEED_TTL_SECS, &response).await
}
