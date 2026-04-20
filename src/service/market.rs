use std::collections::{BTreeSet, HashMap};

use anyhow::anyhow;
use chrono::{Duration, Utc};
use ethers_core::{
    abi::{Token, encode},
    types::U256,
    utils::keccak256,
};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::{
            crud,
            model::{
                MarketEventRecord, MarketPriceHistorySnapshotRecord, MarketPriceSnapshotRecord,
                MarketRecord, MarketTradeStatsRecord, NewMarketEventNegRiskConfigRecord,
                NewMarketEventRecord, NewMarketRecord, NewMarketResolutionRecord,
                PublicMarketSummaryRecord,
            },
            schema::{
                AdminEventDetailResponse, AdminEventListResponse, AdminEventMarketsQuery,
                AdminEventMarketsResponse, AdminListEventsQuery, BootstrapEventLiquidityRequest,
                BootstrapMarketLiquidityRequest, CategoriesResponse, CategoryDetailResponse,
                CategoryMarketsQuery, CreateEventMarketLadderRequest, CreateEventMarketRequest,
                CreateEventMarketsRequest, CreateEventMarketsResponse, CreateEventPublishMode,
                CreateEventRequest, CreateEventResponse, CreateMarketRequest, CreateMarketResponse,
                CreatePriceLadderTemplateRequest, DisputeMarketResolutionRequest,
                EmergencyMarketResolutionRequest, EventDetailResponse,
                EventLiquidityBootstrapItemResponse, EventLiquidityBootstrapResponse,
                EventListResponse, EventMarketsQuery, EventMarketsResponse, ListEventsQuery,
                ListMarketsQuery, MarketActivityItemResponse, MarketActivityResponse,
                MarketCurrentPricesResponse, MarketDetailResponse,
                MarketLiquidityBootstrapResponse, MarketLiquidityBootstrapStateResponse,
                MarketLiquidityOutcomeResponse, MarketLiquidityResponse, MarketListResponse,
                MarketOrderbookResponse, MarketOutcomesResponse, MarketPriceHistoryPointResponse,
                MarketPriceHistoryQuery, MarketPriceHistoryResponse, MarketPricesResponse,
                MarketPricesStateResponse, MarketQuoteResponse, MarketQuoteSummaryResponse,
                MarketResolutionReadResponse, MarketResolutionWorkflowResponse, MarketResponse,
                MarketStatsResponse, MarketTradeFillResponse, MarketTradesResponse,
                MarketTradingStatusResponse, MarketsHomeQuery, MarketsHomeResponse,
                NegRiskRegistrationResponse, OrderbookLevelResponse, PoolLiquidityResponse,
                ProposeMarketResolutionRequest, PublicEventCardResponse, PublicMarketCardResponse,
                RegisterNegRiskEventRequest, RelatedMarketsResponse, SearchMarketsQuery,
                SetMarketPricesRequest, TagsResponse, UpdateMarketRequest, UpdateMarketResponse,
            },
        },
        order::{
            crud as order_crud,
            model::{MarketOrderFillRecord, MarketOrderRecord},
        },
    },
    service::{
        auth::normalize_wallet_address,
        jwt::AuthenticatedUser,
        stellar::{
            bootstrap_market_liquidity as bootstrap_market_liquidity_tx,
            dispute_resolution as dispute_resolution_tx, emergency_resolve_market,
            ensure_mock_usdc_balance,
            finalize_resolution as finalize_resolution_tx, find_existing_event_binary_market,
            get_market_liquidity as get_market_liquidity_on_chain,
            get_market_prices_batch_best_effort, pause_market as pause_market_tx,
            propose_resolution as propose_resolution_tx, publish_event as publish_event_tx,
            publish_event_market as publish_event_market_tx, publish_standalone_binary_market,
            register_neg_risk_event, set_market_prices as set_market_prices_tx,
            unpause_market as unpause_market_tx,
        },
    },
};

const DEFAULT_RESOLUTION_DISPUTE_WINDOW_SECONDS: i64 = 86_400;
const PUBLICATION_STATUS_DRAFT: &str = "draft";
const PUBLICATION_STATUS_PUBLISHED: &str = "published";
const DEFAULT_HOME_LIMIT: i64 = 6;
const DEFAULT_LIST_LIMIT: i64 = 20;
const MAX_LIST_LIMIT: i64 = 100;
const DEFAULT_RELATED_LIMIT: usize = 6;
const MARKET_PRICE_BPS_SCALE: u32 = 10_000;
const DEFAULT_BINARY_MARKET_YES_BPS: u32 = 5_000;
const USDC_DECIMALS: usize = 6;
const PRICE_HISTORY_LOOKBACK_MULTIPLIER: i64 = 48;
const MAX_PRICE_HISTORY_SNAPSHOT_FETCH: i64 = 2_000;

pub struct PublicMarketContext {
    pub market: MarketRecord,
    pub event: MarketEventRecord,
}

struct NormalizedMarketSearchQuery {
    raw: String,
    tsquery: String,
}

pub async fn get_markets_home(
    state: &AppState,
    query: MarketsHomeQuery,
) -> Result<MarketsHomeResponse, AuthError> {
    let limit = normalize_limit(query.limit, DEFAULT_HOME_LIMIT)?;

    let featured = crud::list_public_market_summaries(
        &state.db,
        crud::PublicMarketListFilters {
            category_slug: None,
            subcategory_slug: None,
            tag_slug: None,
            q: None,
            featured: Some(true),
            breaking: None,
            trading_status: None,
            limit,
            offset: 0,
        },
        crud::PublicMarketOrder::Featured,
    )
    .await?;
    let breaking = crud::list_public_market_summaries(
        &state.db,
        crud::PublicMarketListFilters {
            category_slug: None,
            subcategory_slug: None,
            tag_slug: None,
            q: None,
            featured: None,
            breaking: Some(true),
            trading_status: None,
            limit,
            offset: 0,
        },
        crud::PublicMarketOrder::Featured,
    )
    .await?;
    let newest = crud::list_public_market_summaries(
        &state.db,
        crud::PublicMarketListFilters {
            category_slug: None,
            subcategory_slug: None,
            tag_slug: None,
            q: None,
            featured: None,
            breaking: None,
            trading_status: None,
            limit,
            offset: 0,
        },
        crud::PublicMarketOrder::Newest,
    )
    .await?;

    let featured = build_public_market_cards_with_current_prices(state, &featured).await?;
    let breaking = build_public_market_cards_with_current_prices(state, &breaking).await?;
    let newest = build_public_market_cards_with_current_prices(state, &newest).await?;

    Ok(MarketsHomeResponse::new(featured, breaking, newest))
}

pub async fn list_markets(
    state: &AppState,
    query: ListMarketsQuery,
) -> Result<MarketListResponse, AuthError> {
    let category_slug = query
        .category_slug
        .as_deref()
        .map(|value| normalize_slug(value, "category slug"))
        .transpose()?;
    let subcategory_slug = query
        .subcategory_slug
        .as_deref()
        .map(|value| normalize_slug(value, "subcategory slug"))
        .transpose()?;
    let tag_slug = query
        .tag_slug
        .as_deref()
        .map(|value| normalize_slug(value, "tag slug"))
        .transpose()?;
    let q = normalize_optional_text(query.q.as_deref());
    let trading_status = normalize_optional_trading_status(query.trading_status.as_deref())?;
    let limit = normalize_limit(query.limit, DEFAULT_LIST_LIMIT)?;
    let offset = normalize_offset(query.offset)?;

    let markets = if let Some(ref q) = q {
        let search = normalize_market_search_query(Some(q.as_str()))?;

        crud::search_public_market_summaries(
            &state.db,
            crud::PublicMarketSearchFilters {
                query: &search.raw,
                tsquery: &search.tsquery,
                category_slug: category_slug.as_deref(),
                subcategory_slug: subcategory_slug.as_deref(),
                tag_slug: tag_slug.as_deref(),
                featured: query.featured,
                breaking: query.breaking,
                trading_status: trading_status.as_deref(),
                limit,
                offset,
            },
        )
        .await?
    } else {
        crud::list_public_market_summaries(
            &state.db,
            crud::PublicMarketListFilters {
                category_slug: category_slug.as_deref(),
                subcategory_slug: subcategory_slug.as_deref(),
                tag_slug: tag_slug.as_deref(),
                q: None,
                featured: query.featured,
                breaking: query.breaking,
                trading_status: trading_status.as_deref(),
                limit,
                offset,
            },
            crud::PublicMarketOrder::Featured,
        )
        .await?
    };
    let markets = build_public_market_cards_with_current_prices(state, &markets).await?;

    Ok(MarketListResponse::new(markets, limit, offset))
}

pub async fn search_markets(
    state: &AppState,
    query: SearchMarketsQuery,
) -> Result<MarketListResponse, AuthError> {
    let category_slug = query
        .category_slug
        .as_deref()
        .map(|value| normalize_slug(value, "category slug"))
        .transpose()?;
    let subcategory_slug = query
        .subcategory_slug
        .as_deref()
        .map(|value| normalize_slug(value, "subcategory slug"))
        .transpose()?;
    let tag_slug = query
        .tag_slug
        .as_deref()
        .map(|value| normalize_slug(value, "tag slug"))
        .transpose()?;
    let search = normalize_market_search_query(query.q.as_deref())?;
    let trading_status = normalize_optional_trading_status(query.trading_status.as_deref())?;
    let limit = normalize_limit(query.limit, DEFAULT_LIST_LIMIT)?;
    let offset = normalize_offset(query.offset)?;

    let markets = crud::search_public_market_summaries(
        &state.db,
        crud::PublicMarketSearchFilters {
            query: &search.raw,
            tsquery: &search.tsquery,
            category_slug: category_slug.as_deref(),
            subcategory_slug: subcategory_slug.as_deref(),
            tag_slug: tag_slug.as_deref(),
            featured: None,
            breaking: None,
            trading_status: trading_status.as_deref(),
            limit,
            offset,
        },
    )
    .await?;
    let markets = build_public_market_cards_with_current_prices(state, &markets).await?;

    Ok(MarketListResponse::new(markets, limit, offset))
}

pub async fn get_market_by_id(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketDetailResponse, AuthError> {
    let market = crud::get_public_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;

    build_market_detail_response(state, market).await
}

pub async fn get_market_by_slug(
    state: &AppState,
    slug: String,
) -> Result<MarketDetailResponse, AuthError> {
    let slug = normalize_slug(&slug, "market slug")?;
    let market = crud::get_public_market_by_slug(&state.db, &slug)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;

    build_market_detail_response(state, market).await
}

pub async fn get_market_by_condition_id(
    state: &AppState,
    condition_id: String,
) -> Result<MarketDetailResponse, AuthError> {
    let condition_id = normalize_bytes32(&condition_id, "condition_id")?;
    let market = crud::get_public_market_by_condition_id(&state.db, &condition_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;

    build_market_detail_response(state, market).await
}

pub async fn get_market_outcomes(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketOutcomesResponse, AuthError> {
    let market = load_public_market(state, market_id).await?;
    let resolution = crud::get_market_resolution_by_market_id(&state.db, market_id).await?;

    Ok(MarketOutcomesResponse::from_records(
        &market,
        resolution.as_ref(),
    ))
}

pub async fn get_market_activity(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketActivityResponse, AuthError> {
    let market = load_public_market(state, market_id).await?;
    let resolution = crud::get_market_resolution_by_market_id(&state.db, market_id).await?;
    let mut items = vec![MarketActivityItemResponse {
        activity_type: "market_created".to_owned(),
        occurred_at: market.created_at,
        actor_user_id: None,
        details: Some("Market published".to_owned()),
    }];

    if let Some(resolution) = resolution {
        items.push(MarketActivityItemResponse {
            activity_type: "resolution_proposed".to_owned(),
            occurred_at: resolution.proposed_at,
            actor_user_id: Some(resolution.proposed_by_user_id),
            details: resolution.notes.clone(),
        });

        if let Some(disputed_at) = resolution.disputed_at {
            items.push(MarketActivityItemResponse {
                activity_type: "resolution_disputed".to_owned(),
                occurred_at: disputed_at,
                actor_user_id: resolution.disputed_by_user_id,
                details: resolution.dispute_reason.clone(),
            });
        }

        if let Some(finalized_at) = resolution.finalized_at {
            items.push(MarketActivityItemResponse {
                activity_type: "resolution_finalized".to_owned(),
                occurred_at: finalized_at,
                actor_user_id: resolution.finalized_by_user_id,
                details: None,
            });
        }

        if let Some(emergency_resolved_at) = resolution.emergency_resolved_at {
            items.push(MarketActivityItemResponse {
                activity_type: "resolution_emergency".to_owned(),
                occurred_at: emergency_resolved_at,
                actor_user_id: resolution.emergency_resolved_by_user_id,
                details: resolution.notes.clone(),
            });
        }
    }

    items.sort_by(|left, right| right.occurred_at.cmp(&left.occurred_at));

    Ok(MarketActivityResponse::new(market.id, items))
}

pub async fn get_market_price_history(
    state: &AppState,
    market_id: Uuid,
    query: MarketPriceHistoryQuery,
) -> Result<MarketPriceHistoryResponse, AuthError> {
    let market = load_public_market(state, market_id).await?;
    let interval = normalize_history_interval(query.interval.as_deref())?;
    let limit = normalize_limit(query.limit, DEFAULT_LIST_LIMIT)?;
    let fills = order_crud::list_market_order_fills_by_market_id(
        &state.db,
        market.id,
        market_price_history_fetch_limit(limit),
    )
    .await?;
    if !fills.is_empty() {
        let points =
            build_market_price_history_points_from_fills(&market, &fills, &interval, limit)?;
        if !points.is_empty() {
            return Ok(MarketPriceHistoryResponse::from_points(
                market.id,
                market.condition_id.clone(),
                "order_fill_history",
                interval,
                points,
            ));
        }
    }
    let snapshots = crud::list_market_price_history_snapshots(
        &state.db,
        market.id,
        market_price_history_fetch_limit(limit),
    )
    .await?;
    let mut points = build_market_price_history_points(&market, &snapshots, &interval, limit)?;

    if points.is_empty() {
        if let Some(snapshot) = load_or_fetch_current_market_snapshot(state, &market).await? {
            points = build_history_points_from_snapshot(
                &market,
                &snapshot.synced_at,
                snapshot.yes_bps,
                snapshot.no_bps,
            )?;
        }
    }

    Ok(MarketPriceHistoryResponse::from_points(
        market.id,
        market.condition_id.clone(),
        "price_snapshot_history",
        interval,
        points,
    ))
}

pub async fn get_market_quote(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketQuoteResponse, AuthError> {
    let market = load_public_market(state, market_id).await?;
    let open_orders =
        order_crud::list_active_market_orders_by_market_id(&state.db, market.id).await?;
    if !open_orders.is_empty() {
        let clob = build_clob_orderbook(&market, &open_orders)?;
        let last_trade_yes_bps = current_last_trade_yes_bps_from_fills(state, market.id).await?;
        let buy_yes_bps = best_ask_price_bps(&clob.asks, 0)
            .or(last_trade_yes_bps)
            .unwrap_or_default();
        let buy_no_bps = best_ask_price_bps(&clob.asks, 1)
            .or(last_trade_yes_bps.map(|price| 10_000_u32.saturating_sub(price)))
            .unwrap_or_default();
        let sell_yes_bps = best_bid_price_bps(&clob.bids, 0)
            .or(last_trade_yes_bps)
            .unwrap_or_default();
        let sell_no_bps = best_bid_price_bps(&clob.bids, 1)
            .or(last_trade_yes_bps.map(|price| 10_000_u32.saturating_sub(price)))
            .unwrap_or_default();
        return Ok(MarketQuoteResponse {
            market_id: market.id,
            condition_id: market.condition_id.clone(),
            source: "clob_resting_orders".to_owned(),
            as_of: Utc::now(),
            buy_yes_bps,
            buy_no_bps,
            sell_yes_bps,
            sell_no_bps,
            last_trade_yes_bps: last_trade_yes_bps.unwrap_or(buy_yes_bps),
            spread_bps: buy_yes_bps.saturating_sub(sell_yes_bps),
        });
    }

    let trade_stats = crud::get_market_trade_stats_by_market_id(&state.db, market.id).await?;
    let snapshot = load_or_fetch_current_market_snapshot(state, &market).await?;
    let (current_prices, source, as_of) = match snapshot.as_ref() {
        Some(snapshot) => (
            market_current_prices_from_snapshot(snapshot)?,
            "fixed_price_pool".to_owned(),
            snapshot.synced_at,
        ),
        None => (
            fallback_current_prices_from_trade_stats(trade_stats.as_ref())?,
            "fixed_price_pool_fallback".to_owned(),
            Utc::now(),
        ),
    };
    let last_trade_yes_bps =
        derive_last_trade_yes_bps(trade_stats.as_ref(), current_prices.yes_bps)?;

    Ok(MarketQuoteResponse {
        market_id: market.id,
        condition_id: market.condition_id.clone(),
        source,
        as_of,
        buy_yes_bps: current_prices.yes_bps,
        buy_no_bps: current_prices.no_bps,
        sell_yes_bps: current_prices.yes_bps,
        sell_no_bps: current_prices.no_bps,
        last_trade_yes_bps,
        spread_bps: 0,
    })
}

pub async fn get_market_orderbook(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketOrderbookResponse, AuthError> {
    let market = load_public_market(state, market_id).await?;
    let open_orders =
        order_crud::list_active_market_orders_by_market_id(&state.db, market.id).await?;
    if !open_orders.is_empty() {
        let clob = build_clob_orderbook(&market, &open_orders)?;
        return Ok(MarketOrderbookResponse {
            market_id: market.id,
            condition_id: market.condition_id.clone(),
            source: "clob_resting_orders".to_owned(),
            as_of: Utc::now(),
            spread_bps: clob.spread_bps,
            last_trade_yes_bps: current_last_trade_yes_bps_from_fills(state, market.id)
                .await?
                .unwrap_or_else(|| best_ask_price_bps(&clob.asks, 0).unwrap_or_default()),
            bids: clob.bids,
            asks: clob.asks,
        });
    }

    let trade_stats = crud::get_market_trade_stats_by_market_id(&state.db, market.id).await?;
    let snapshot = load_or_fetch_current_market_snapshot(state, &market).await?;
    let (current_prices, source, as_of) = match snapshot.as_ref() {
        Some(snapshot) => (
            market_current_prices_from_snapshot(snapshot)?,
            "synthetic_fixed_price_pool".to_owned(),
            snapshot.synced_at,
        ),
        None => (
            fallback_current_prices_from_trade_stats(trade_stats.as_ref())?,
            "synthetic_fixed_price_pool_fallback".to_owned(),
            Utc::now(),
        ),
    };
    let liquidity = match get_market_liquidity_on_chain(&state.env, required_condition_id(&market)?)
        .await
    {
        Ok(liquidity) => liquidity,
        Err(error) => {
            tracing::warn!(
                market_id = %market.id,
                condition_id = ?market.condition_id,
                error = %error,
                "unable to read on-chain market liquidity for orderbook fallback"
            );
            crate::service::stellar::MarketLiquidityReadResult {
                yes_available: "0".to_owned(),
                no_available: "0".to_owned(),
                idle_yes_total: "0".to_owned(),
                idle_no_total: "0".to_owned(),
                posted_yes_total: "0".to_owned(),
                posted_no_total: "0".to_owned(),
                claimable_collateral_total: "0".to_owned(),
            }
        }
    };
    let last_trade_yes_bps =
        derive_last_trade_yes_bps(trade_stats.as_ref(), current_prices.yes_bps)?;

    Ok(MarketOrderbookResponse {
        market_id: market.id,
        condition_id: market.condition_id.clone(),
        source,
        as_of,
        spread_bps: 0,
        last_trade_yes_bps,
        bids: build_synthetic_pool_bids(
            &market,
            &current_prices,
            &liquidity.claimable_collateral_total,
        )?,
        asks: build_synthetic_pool_asks(
            &market,
            &current_prices,
            &liquidity.yes_available,
            &liquidity.no_available,
        )?,
    })
}

pub async fn get_market_trades(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketTradesResponse, AuthError> {
    let market = load_public_market(state, market_id).await?;
    let fills =
        order_crud::list_market_order_fills_by_market_id(&state.db, market.id, MAX_LIST_LIMIT)
            .await?;

    Ok(MarketTradesResponse {
        market_id: market.id,
        condition_id: market.condition_id.clone(),
        source: "order_fill_history".to_owned(),
        trades: fills
            .iter()
            .map(build_market_trade_fill_response)
            .collect::<Result<Vec<_>, _>>()?,
    })
}

pub async fn get_market_liquidity(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketLiquidityResponse, AuthError> {
    let market = load_public_market(state, market_id).await?;
    build_market_liquidity_response(state, &market).await
}

pub async fn get_market_resolution(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketResolutionReadResponse, AuthError> {
    let market = load_public_market(state, market_id).await?;
    let resolution = crud::get_market_resolution_by_market_id(&state.db, market_id).await?;

    Ok(MarketResolutionReadResponse::new(
        market.id,
        resolution.as_ref(),
    ))
}

pub async fn get_related_markets(
    state: &AppState,
    market_id: Uuid,
) -> Result<RelatedMarketsResponse, AuthError> {
    let market = load_public_market(state, market_id).await?;
    let event = load_public_event(state, market.event_db_id).await?;
    let sibling_markets =
        crud::list_public_markets_for_event(&state.db, market.event_db_id).await?;

    let sibling_related_markets = sibling_markets
        .iter()
        .filter(|value| value.id != market.id)
        .cloned()
        .collect::<Vec<_>>();
    let mut related = build_public_market_cards_from_market_records_with_current_prices(
        state,
        &event,
        &sibling_related_markets,
    )
    .await?;

    if related.len() < DEFAULT_RELATED_LIMIT {
        let fallback_summaries = crud::list_public_market_summaries(
            &state.db,
            crud::PublicMarketListFilters {
                category_slug: Some(event.category_slug.as_str()),
                subcategory_slug: event.subcategory_slug.as_deref(),
                tag_slug: None,
                q: None,
                featured: None,
                breaking: None,
                trading_status: None,
                limit: MAX_LIST_LIMIT,
                offset: 0,
            },
            crud::PublicMarketOrder::Featured,
        )
        .await?;
        let filtered_fallback_summaries = fallback_summaries
            .into_iter()
            .filter(|value| value.market_id != market.id)
            .filter(|value| {
                !related
                    .iter()
                    .any(|existing| existing.id == value.market_id)
            })
            .collect::<Vec<_>>();
        let mut fallback =
            build_public_market_cards_with_current_prices(state, &filtered_fallback_summaries)
                .await?;

        related.append(&mut fallback);
    }

    related.truncate(DEFAULT_RELATED_LIMIT);

    Ok(RelatedMarketsResponse::new(market.id, related))
}

pub async fn list_events(
    state: &AppState,
    query: ListEventsQuery,
) -> Result<EventListResponse, AuthError> {
    let category_slug = query
        .category_slug
        .as_deref()
        .map(|value| normalize_slug(value, "category slug"))
        .transpose()?;
    let subcategory_slug = query
        .subcategory_slug
        .as_deref()
        .map(|value| normalize_slug(value, "subcategory slug"))
        .transpose()?;
    let tag_slug = query
        .tag_slug
        .as_deref()
        .map(|value| normalize_slug(value, "tag slug"))
        .transpose()?;
    let limit = normalize_limit(query.limit, DEFAULT_LIST_LIMIT)?;
    let offset = normalize_offset(query.offset)?;
    let include_markets = query.include_markets.unwrap_or(false);

    let events = crud::list_public_event_summaries(
        &state.db,
        crud::PublicEventListFilters {
            public_only: true,
            publication_status: None,
            category_slug: category_slug.as_deref(),
            subcategory_slug: subcategory_slug.as_deref(),
            tag_slug: tag_slug.as_deref(),
            featured: query.featured,
            breaking: query.breaking,
            limit,
            offset,
        },
    )
    .await?;

    if !include_markets {
        return Ok(EventListResponse::new(events, limit, offset));
    }

    let event_ids = events
        .iter()
        .map(|event| event.event_id)
        .collect::<Vec<_>>();
    let markets = crud::list_public_market_summaries_for_event_ids(&state.db, &event_ids).await?;
    let mut markets_by_event =
        build_public_market_cards_by_event_with_current_prices(state, &markets).await?;

    let events = events
        .into_iter()
        .map(|event| PublicEventCardResponse {
            markets: Some(markets_by_event.remove(&event.event_id).unwrap_or_default()),
            ..PublicEventCardResponse::from(&event)
        })
        .collect();

    Ok(EventListResponse {
        events,
        limit,
        offset,
    })
}

pub async fn admin_list_events(
    state: &AppState,
    query: AdminListEventsQuery,
) -> Result<AdminEventListResponse, AuthError> {
    let publication_status =
        normalize_optional_publication_status(query.publication_status.as_deref())?;
    let category_slug = query
        .category_slug
        .as_deref()
        .map(|value| normalize_slug(value, "category slug"))
        .transpose()?;
    let subcategory_slug = query
        .subcategory_slug
        .as_deref()
        .map(|value| normalize_slug(value, "subcategory slug"))
        .transpose()?;
    let tag_slug = query
        .tag_slug
        .as_deref()
        .map(|value| normalize_slug(value, "tag slug"))
        .transpose()?;
    let limit = normalize_limit(query.limit, DEFAULT_LIST_LIMIT)?;
    let offset = normalize_offset(query.offset)?;

    let events = crud::list_public_event_summaries(
        &state.db,
        crud::PublicEventListFilters {
            public_only: false,
            publication_status: publication_status.as_deref(),
            category_slug: category_slug.as_deref(),
            subcategory_slug: subcategory_slug.as_deref(),
            tag_slug: tag_slug.as_deref(),
            featured: query.featured,
            breaking: query.breaking,
            limit,
            offset,
        },
    )
    .await?;

    Ok(AdminEventListResponse::new(events, limit, offset))
}

pub async fn get_event_by_id(
    state: &AppState,
    event_id: Uuid,
) -> Result<EventDetailResponse, AuthError> {
    let (event, markets_count) = tokio::try_join!(
        load_public_event(state, event_id),
        crud::count_public_markets_for_event(&state.db, event_id),
    )?;

    Ok(EventDetailResponse::from_records(&event, markets_count))
}

pub async fn admin_get_event_by_id(
    state: &AppState,
    event_id: Uuid,
) -> Result<AdminEventDetailResponse, AuthError> {
    let event = crud::get_market_event_by_id(&state.db, event_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;
    let markets_count = crud::count_markets_for_event(&state.db, event_id).await?;

    Ok(AdminEventDetailResponse::from_records(
        &event,
        markets_count,
    ))
}

pub async fn get_event_markets(
    state: &AppState,
    event_id: Uuid,
    query: EventMarketsQuery,
) -> Result<EventMarketsResponse, AuthError> {
    let limit = query
        .limit
        .map(|value| normalize_limit(Some(value), DEFAULT_LIST_LIMIT))
        .transpose()?;
    let offset = query
        .offset
        .map(|value| normalize_offset(Some(value)))
        .transpose()?;
    let (event, markets) = tokio::try_join!(
        load_public_event(state, event_id),
        crud::list_public_markets_for_event_window(&state.db, event_id, limit, offset),
    )?;
    let market_refs = markets
        .iter()
        .filter_map(|market| {
            market
                .condition_id
                .clone()
                .map(|condition_id| (market.id, condition_id))
        })
        .collect();
    let market_ids = markets.iter().map(|market| market.id).collect();
    let (snapshots_by_condition, stats_by_market_id) = tokio::try_join!(
        load_market_price_snapshots_by_market_refs_db_only(state, market_refs),
        load_market_trade_stats_by_market_id(state, market_ids),
    )?;
    let market_responses =
        build_event_market_responses(&markets, &snapshots_by_condition, &stats_by_market_id)?;

    Ok(EventMarketsResponse {
        event: crate::module::market::schema::EventResponse::from(&event),
        on_chain: crate::module::market::schema::EventOnChainResponse::from(&event),
        markets: market_responses,
    })
}

pub async fn admin_get_event_markets(
    state: &AppState,
    event_id: Uuid,
    query: AdminEventMarketsQuery,
) -> Result<AdminEventMarketsResponse, AuthError> {
    let publication_status =
        normalize_optional_publication_status(query.publication_status.as_deref())?;
    let limit = query
        .limit
        .map(|value| normalize_limit(Some(value), DEFAULT_LIST_LIMIT))
        .transpose()?;
    let offset = query
        .offset
        .map(|value| normalize_offset(Some(value)))
        .transpose()?;
    let event = crud::get_market_event_by_id(&state.db, event_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;
    let markets = crud::list_markets_for_event_filtered(
        &state.db,
        event_id,
        publication_status.as_deref(),
        limit,
        offset,
    )
    .await?;

    Ok(AdminEventMarketsResponse::from_records(&event, &markets))
}

pub async fn list_categories(state: &AppState) -> Result<CategoriesResponse, AuthError> {
    let categories = crud::list_category_summaries(&state.db).await?;
    Ok(CategoriesResponse::new(categories))
}

pub async fn get_category_by_slug(
    state: &AppState,
    slug: String,
    query: CategoryMarketsQuery,
) -> Result<CategoryDetailResponse, AuthError> {
    let slug = normalize_slug(&slug, "category slug")?;
    let limit = normalize_limit(query.limit, DEFAULT_LIST_LIMIT)?;
    let offset = normalize_offset(query.offset)?;
    let category = crud::get_category_summary_by_slug(&state.db, &slug)
        .await?
        .ok_or_else(|| AuthError::not_found("category not found"))?;
    let markets = crud::list_public_market_summaries(
        &state.db,
        crud::PublicMarketListFilters {
            category_slug: Some(slug.as_str()),
            subcategory_slug: None,
            tag_slug: None,
            q: None,
            featured: None,
            breaking: None,
            trading_status: None,
            limit,
            offset,
        },
        crud::PublicMarketOrder::Featured,
    )
    .await?;
    let markets = build_public_market_cards_with_current_prices(state, &markets).await?;

    Ok(CategoryDetailResponse::new(&category, markets))
}

pub async fn list_tags(state: &AppState) -> Result<TagsResponse, AuthError> {
    let tags = crud::list_tag_summaries(&state.db).await?;
    Ok(TagsResponse::new(tags))
}

pub async fn create_event(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    payload: CreateEventRequest,
) -> Result<CreateEventResponse, AuthError> {
    let event_title = normalize_required_text(&payload.event.title, "event title")?;
    let event_slug = normalize_slug(&payload.event.slug, "event slug")?;
    let category_slug = normalize_slug(&payload.event.category_slug, "category slug")?;
    let subcategory_slug = normalize_optional_slug(payload.event.subcategory_slug.as_deref())?;
    let tag_slugs = normalize_slug_list(&payload.event.tag_slugs, "tag slug")?;
    let image_url = normalize_optional_text(payload.event.image_url.as_deref());
    let summary_text = normalize_optional_text(payload.event.summary.as_deref());
    let rules_text = normalize_required_text(&payload.event.rules, "rules")?;
    let context_text = normalize_optional_text(payload.event.context.as_deref());
    let additional_context = normalize_optional_text(payload.event.additional_context.as_deref());
    let resolution_sources = normalize_text_list(&payload.event.resolution_sources);
    let resolution_timezone =
        normalize_required_text(&payload.event.resolution_timezone, "resolution timezone")?;
    let group_key = normalize_required_text(&payload.chain.group_key, "group key")?;
    let series_key = normalize_required_text(&payload.chain.series_key, "series key")?;

    let event_db_id = Uuid::new_v4();
    let event_id = derive_bytes32_id("event", &event_slug);
    let group_id = derive_bytes32_id("group", &group_key);
    let series_id = derive_bytes32_id("series", &series_key);
    let (publication_status, published_tx_hash) = match payload.publish.mode {
        CreateEventPublishMode::Draft => (PUBLICATION_STATUS_DRAFT.to_owned(), None),
        CreateEventPublishMode::Publish => {
            let tx = publish_event_tx(
                &state.env,
                &event_id,
                &group_id,
                &series_id,
                payload.chain.neg_risk,
            )
            .await
            .map_err(|error| map_admin_chain_error("market event publish", error))?;
            (PUBLICATION_STATUS_PUBLISHED.to_owned(), Some(tx.tx_hash))
        }
        CreateEventPublishMode::Prepare => {
            return Err(AuthError::bad_request(
                "prepare mode is not supported for /admin/events today",
            ));
        }
    };

    let event_record = NewMarketEventRecord {
        id: event_db_id,
        title: event_title,
        slug: event_slug.clone(),
        category_slug,
        subcategory_slug,
        tag_slugs,
        image_url,
        summary_text,
        rules_text,
        context_text,
        additional_context,
        resolution_sources,
        resolution_timezone,
        starts_at: payload.event.starts_at,
        sort_at: payload.event.sort_at,
        featured: payload.event.featured,
        breaking: payload.event.breaking,
        searchable: payload.event.searchable,
        visible: payload.event.visible,
        hide_resolved_by_default: payload.event.hide_resolved_by_default,
        group_key: group_key.clone(),
        series_key: series_key.clone(),
        event_id,
        group_id,
        series_id,
        neg_risk: payload.chain.neg_risk,
        oracle_address: None,
        publication_status,
        published_tx_hash,
        created_by_user_id: authenticated_user.user_id,
    };

    let created_event = crud::create_market_event(&state.db, &event_record).await?;

    Ok(CreateEventResponse::from_record(created_event))
}

pub async fn create_market(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    payload: CreateMarketRequest,
) -> Result<CreateMarketResponse, AuthError> {
    let title = normalize_required_text(&payload.market.title, "market title")?;
    let slug = normalize_slug(&payload.market.slug, "market slug")?;
    let category_slug = normalize_slug(&payload.market.category_slug, "category slug")?;
    let subcategory_slug = normalize_optional_slug(payload.market.subcategory_slug.as_deref())?;
    let tag_slugs = normalize_slug_list(&payload.market.tag_slugs, "tag slug")?;
    let image_url = normalize_optional_text(payload.market.image_url.as_deref());
    let summary_text = normalize_optional_text(payload.market.summary.as_deref());
    let rules_text = normalize_required_text(&payload.market.rules, "rules")?;
    let context_text = normalize_optional_text(payload.market.context.as_deref());
    let additional_context = normalize_optional_text(payload.market.additional_context.as_deref());
    let resolution_sources = normalize_text_list(&payload.market.resolution_sources);
    let resolution_timezone =
        normalize_required_text(&payload.market.resolution_timezone, "resolution timezone")?;
    let oracle_address = normalize_wallet_address(&payload.chain.oracle_address)?;

    let now = Utc::now();
    if payload.market.end_time <= now {
        return Err(AuthError::bad_request(
            "market end_time must be in the future",
        ));
    }

    let outcomes = normalize_outcomes(&payload.market.outcomes)?;
    let event_db_id = Uuid::new_v4();
    let event_id = derive_bytes32_id("event", &slug);
    let group_key = format!("{slug}-group");
    let series_key = format!("{slug}-series");
    let group_id = derive_bytes32_id("group", &group_key);
    let series_id = derive_bytes32_id("series", &series_key);
    let question_id = derive_bytes32_id("question", &slug);
    let (publication_status, published_tx_hash, condition_id) = match payload.publish.mode {
        CreateEventPublishMode::Draft => (PUBLICATION_STATUS_DRAFT.to_owned(), None, None),
        CreateEventPublishMode::Publish => {
            let tx = publish_standalone_binary_market(
                &state.env,
                &event_id,
                &group_id,
                &series_id,
                payload.chain.neg_risk,
                &question_id,
                unix_timestamp_u64(payload.market.end_time, "market end_time")?,
                &oracle_address,
            )
            .await
            .map_err(|error| map_admin_chain_error("standalone market publish", error))?;
            (
                PUBLICATION_STATUS_PUBLISHED.to_owned(),
                Some(tx.tx_hash),
                Some(tx.condition_id),
            )
        }
        CreateEventPublishMode::Prepare => {
            return Err(AuthError::bad_request(
                "prepare mode is not supported for /admin/markets today",
            ));
        }
    };

    let event_record = NewMarketEventRecord {
        id: event_db_id,
        title: title.clone(),
        slug: slug.clone(),
        category_slug,
        subcategory_slug,
        tag_slugs,
        image_url,
        summary_text,
        rules_text,
        context_text,
        additional_context,
        resolution_sources,
        resolution_timezone,
        starts_at: payload.market.starts_at,
        sort_at: payload.market.sort_at,
        featured: payload.market.featured,
        breaking: payload.market.breaking,
        searchable: payload.market.searchable,
        visible: payload.market.visible,
        hide_resolved_by_default: payload.market.hide_resolved_by_default,
        group_key,
        series_key,
        event_id,
        group_id,
        series_id,
        neg_risk: payload.chain.neg_risk,
        oracle_address: Some(oracle_address.clone()),
        publication_status: publication_status.clone(),
        published_tx_hash,
        created_by_user_id: authenticated_user.user_id,
    };

    let market_record = NewMarketRecord {
        id: Uuid::new_v4(),
        event_db_id,
        slug: slug.clone(),
        label: title.clone(),
        question: title,
        question_id,
        condition_id,
        market_type: "binary".to_owned(),
        outcome_count: outcomes.len() as i32,
        outcomes,
        end_time: payload.market.end_time,
        sort_order: 1,
        publication_status,
        trading_status: "active".to_owned(),
        metadata_hash: None,
        oracle_address,
    };

    let (created_event, created_market) =
        crud::create_market_bundle(&state.db, &event_record, &market_record).await?;

    Ok(CreateMarketResponse::from_records(
        &created_event,
        &created_market,
    ))
}

pub async fn create_event_markets(
    state: &AppState,
    event_db_id: Uuid,
    payload: CreateEventMarketsRequest,
) -> Result<CreateEventMarketsResponse, AuthError> {
    if payload.markets.is_empty() {
        return Err(AuthError::bad_request("at least one market is required"));
    }

    let event = crud::get_market_event_by_id(&state.db, event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;

    let now = Utc::now();
    let publication_status = match payload.publish.mode {
        CreateEventPublishMode::Draft => PUBLICATION_STATUS_DRAFT.to_owned(),
        CreateEventPublishMode::Publish => {
            if event.publication_status != PUBLICATION_STATUS_PUBLISHED {
                return Err(AuthError::bad_request(
                    "event must be published before child markets can be published",
                ));
            }

            PUBLICATION_STATUS_PUBLISHED.to_owned()
        }
        CreateEventPublishMode::Prepare => {
            return Err(AuthError::bad_request(
                "prepare mode is not supported for /admin/events/:event_id/markets today",
            ));
        }
    };
    let mut markets = Vec::with_capacity(payload.markets.len());
    let mut seen_market_slugs = BTreeSet::new();
    let mut seen_sort_orders = BTreeSet::new();

    for (index, market) in payload.markets.iter().enumerate() {
        let built_market = build_market_record(event.id, market, index, now, &publication_status)?;

        if !seen_market_slugs.insert(built_market.slug.clone()) {
            return Err(AuthError::conflict("market slug already exists in request"));
        }

        if !seen_sort_orders.insert(built_market.sort_order) {
            return Err(AuthError::bad_request(
                "market sort_order values must be unique within the request",
            ));
        }

        markets.push(built_market);
    }

    let market_records = if matches!(payload.publish.mode, CreateEventPublishMode::Draft) {
        crud::create_market_records(&state.db, &markets).await?
    } else {
        let mut created_markets = Vec::with_capacity(markets.len());
        for market in markets {
            let tx = publish_event_market_tx(
                &state.env,
                &event.event_id,
                &market.question_id,
                unix_timestamp_u64(market.end_time, "market end_time")?,
                &market.oracle_address,
            )
            .await
            .map_err(|error| map_admin_chain_error("event market publish", error))?;

            let published_market = NewMarketRecord {
                condition_id: Some(tx.condition_id),
                ..market
            };
            created_markets.push(crud::create_market_record(&state.db, &published_market).await?);
        }

        created_markets
    };

    Ok(CreateEventMarketsResponse::from_records(
        &event,
        &market_records,
    ))
}

pub async fn create_event_market_ladder(
    state: &AppState,
    event_db_id: Uuid,
    payload: CreateEventMarketLadderRequest,
) -> Result<CreateEventMarketsResponse, AuthError> {
    let CreateEventMarketLadderRequest { template, publish } = payload;
    let event = crud::get_market_event_by_id(&state.db, event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;
    let generated_markets = build_price_ladder_markets(&event, template)?;

    create_event_markets(
        state,
        event_db_id,
        CreateEventMarketsRequest {
            markets: generated_markets,
            publish,
        },
    )
    .await
}

pub async fn update_market(
    state: &AppState,
    market_id: Uuid,
    payload: UpdateMarketRequest,
) -> Result<UpdateMarketResponse, AuthError> {
    let existing_market = crud::get_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;

    if existing_market.publication_status != "draft" {
        return Err(AuthError::bad_request(
            "only draft markets can be updated today",
        ));
    }

    let event = crud::get_market_event_by_id(&state.db, existing_market.event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;
    let should_sync_event_wrapper = crud::count_markets_for_event(&state.db, event.id).await? == 1
        && is_standalone_event_wrapper(&event, &existing_market);

    let updated_market = build_updated_market_record(existing_market, payload.market)?;
    let updated_market = crud::update_market(&state.db, &updated_market).await?;

    let updated_event = if should_sync_event_wrapper {
        sync_standalone_event_wrapper(state, event, &updated_market).await?
    } else {
        event
    };

    Ok(UpdateMarketResponse::from_records(
        &updated_event,
        &updated_market,
    ))
}

pub async fn set_market_prices(
    state: &AppState,
    market_id: Uuid,
    payload: SetMarketPricesRequest,
) -> Result<MarketPricesResponse, AuthError> {
    let market = crud::get_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;
    ensure_market_is_bootstrappable(&market)?;
    let event = load_market_event(state, market.event_db_id).await?;
    let (yes_bps, no_bps) = normalize_binary_prices(payload.prices.yes_bps, payload.prices.no_bps)?;

    let tx = set_market_prices_tx(&state.env, required_condition_id(&market)?, yes_bps, no_bps)
        .await
        .map_err(|error| map_admin_chain_error("market price update", error))?;
    persist_market_price_snapshot(
        state,
        market.id,
        required_condition_id(&market)?,
        yes_bps,
        no_bps,
    )
    .await?;

    Ok(MarketPricesResponse::from_records(
        &event,
        &market,
        MarketPricesStateResponse::new(yes_bps, no_bps, tx.yes_price_tx_hash, tx.no_price_tx_hash),
    ))
}

pub async fn bootstrap_market_liquidity(
    state: &AppState,
    market_id: Uuid,
    payload: BootstrapMarketLiquidityRequest,
) -> Result<MarketLiquidityBootstrapResponse, AuthError> {
    let market = crud::get_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;
    ensure_market_is_bootstrappable(&market)?;
    let event = load_market_event(state, market.event_db_id).await?;

    let liquidity = payload.liquidity;
    let (yes_bps, no_bps) = normalize_binary_prices(liquidity.yes_bps, liquidity.no_bps)?;
    let inventory_usdc_amount = normalize_u256_decimal(
        &liquidity.inventory_usdc_amount,
        "inventory_usdc_amount",
        false,
    )?;
    let exit_collateral_usdc_amount = normalize_u256_decimal(
        &liquidity.exit_collateral_usdc_amount,
        "exit_collateral_usdc_amount",
        true,
    )?;
    let required_admin_collateral = sum_u128_decimal_strings(
        &inventory_usdc_amount,
        &exit_collateral_usdc_amount,
        "bootstrap collateral",
    )?;
    ensure_mock_usdc_balance(
        &state.env,
        &state.env.admin,
        &required_admin_collateral.to_string(),
    )
    .await
    .map_err(|error| map_admin_chain_error("market liquidity bootstrap funding", error))?;

    let tx = bootstrap_market_liquidity_tx(
        &state.env,
        required_condition_id(&market)?,
        yes_bps,
        no_bps,
        &inventory_usdc_amount,
        &exit_collateral_usdc_amount,
    )
    .await
    .map_err(|error| map_admin_chain_error("market liquidity bootstrap", error))?;
    persist_market_price_snapshot(
        state,
        market.id,
        required_condition_id(&market)?,
        yes_bps,
        no_bps,
    )
    .await?;

    let liquidity_response = build_market_liquidity_response(state, &market).await?;

    Ok(MarketLiquidityBootstrapResponse::from_records(
        &event,
        &market,
        MarketLiquidityBootstrapStateResponse::new(
            yes_bps,
            no_bps,
            inventory_usdc_amount,
            exit_collateral_usdc_amount,
            tx.yes_price_tx_hash,
            tx.no_price_tx_hash,
            tx.split_and_add_liquidity_tx_hash,
            tx.deposit_collateral_tx_hash,
        ),
        liquidity_response,
    ))
}

pub async fn bootstrap_event_liquidity(
    state: &AppState,
    event_id: Uuid,
    payload: BootstrapEventLiquidityRequest,
) -> Result<EventLiquidityBootstrapResponse, AuthError> {
    let event = load_market_event(state, event_id).await?;

    if payload.liquidity.markets.is_empty() {
        return Err(AuthError::bad_request(
            "at least one market bootstrap config is required",
        ));
    }

    let mut seen_market_ids = BTreeSet::new();
    let mut results = Vec::with_capacity(payload.liquidity.markets.len());

    for config in payload.liquidity.markets {
        if !seen_market_ids.insert(config.market_id) {
            return Err(AuthError::bad_request(
                "bootstrap request contains duplicate market ids",
            ));
        }

        let market = crud::get_market_by_id(&state.db, config.market_id)
            .await?
            .ok_or_else(|| AuthError::not_found("market not found"))?;

        if market.event_db_id != event.id {
            return Err(AuthError::bad_request(
                "all bootstrap markets must belong to the target event",
            ));
        }

        ensure_market_is_bootstrappable(&market)?;
        let (yes_bps, no_bps) = normalize_binary_prices(config.yes_bps, config.no_bps)?;
        let inventory_usdc_amount = normalize_u256_decimal(
            &config.inventory_usdc_amount,
            "inventory_usdc_amount",
            false,
        )?;
        let exit_collateral_usdc_amount = normalize_u256_decimal(
            &config.exit_collateral_usdc_amount,
            "exit_collateral_usdc_amount",
            true,
        )?;
        let required_admin_collateral = sum_u128_decimal_strings(
            &inventory_usdc_amount,
            &exit_collateral_usdc_amount,
            "event bootstrap collateral",
        )?;
        ensure_mock_usdc_balance(
            &state.env,
            &state.env.admin,
            &required_admin_collateral.to_string(),
        )
        .await
        .map_err(|error| map_admin_chain_error("event liquidity bootstrap funding", error))?;

        let tx = bootstrap_market_liquidity_tx(
            &state.env,
            required_condition_id(&market)?,
            yes_bps,
            no_bps,
            &inventory_usdc_amount,
            &exit_collateral_usdc_amount,
        )
        .await
        .map_err(|error| map_admin_chain_error("event liquidity bootstrap", error))?;
        persist_market_price_snapshot(
            state,
            market.id,
            required_condition_id(&market)?,
            yes_bps,
            no_bps,
        )
        .await?;

        let liquidity_response = build_market_liquidity_response(state, &market).await?;
        let bootstrap = MarketLiquidityBootstrapStateResponse::new(
            yes_bps,
            no_bps,
            inventory_usdc_amount,
            exit_collateral_usdc_amount,
            tx.yes_price_tx_hash,
            tx.no_price_tx_hash,
            tx.split_and_add_liquidity_tx_hash,
            tx.deposit_collateral_tx_hash,
        );
        results.push(EventLiquidityBootstrapItemResponse::new(
            &market,
            bootstrap,
            liquidity_response,
        ));
    }

    Ok(EventLiquidityBootstrapResponse::from_records(
        &event, results,
    ))
}

pub async fn publish_existing_event(
    state: &AppState,
    event_db_id: Uuid,
) -> Result<EventDetailResponse, AuthError> {
    let event = crud::get_market_event_by_id(&state.db, event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;

    let published_event = if event.publication_status == PUBLICATION_STATUS_PUBLISHED {
        event
    } else {
        let tx = publish_event_tx(
            &state.env,
            &event.event_id,
            &event.group_id,
            &event.series_id,
            event.neg_risk,
        )
        .await
        .map_err(|error| map_admin_chain_error("market event publish", error))?;

        crud::update_market_event_publication_status(
            &state.db,
            event.id,
            PUBLICATION_STATUS_PUBLISHED,
            Some(&tx.tx_hash),
        )
        .await?
    };

    let markets_count = crud::count_markets_for_event(&state.db, published_event.id).await?;

    Ok(EventDetailResponse::from_records(
        &published_event,
        markets_count,
    ))
}

pub async fn publish_existing_event_markets(
    state: &AppState,
    event_db_id: Uuid,
) -> Result<EventMarketsResponse, AuthError> {
    let event = crud::get_market_event_by_id(&state.db, event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;

    if event.publication_status != PUBLICATION_STATUS_PUBLISHED {
        return Err(AuthError::bad_request(
            "event must be published before child markets can be published",
        ));
    }

    let markets = crud::list_markets_for_event(&state.db, event.id).await?;
    let mut published_markets = Vec::with_capacity(markets.len());

    for market in markets {
        if market.publication_status == PUBLICATION_STATUS_PUBLISHED {
            published_markets.push(market);
            continue;
        }

        if market.end_time <= Utc::now() {
            return Err(AuthError::bad_request(format!(
                "draft market `{}` has already ended and can no longer be published",
                market.slug
            )));
        }

        if let Some(existing_condition_id) =
            find_existing_event_binary_market(&state.env, &event.event_id, &market.question_id)
                .await
                .map_err(|error| {
                    map_admin_chain_error("event market publish reconciliation", error)
                })?
        {
            let published_market = crud::update_market_publication_status(
                &state.db,
                market.id,
                PUBLICATION_STATUS_PUBLISHED,
                Some(&existing_condition_id),
            )
            .await?;
            published_markets.push(published_market);
            continue;
        }

        let tx = publish_event_market_tx(
            &state.env,
            &event.event_id,
            &market.question_id,
            unix_timestamp_u64(market.end_time, "market end_time")?,
            &market.oracle_address,
        )
        .await
        .map_err(|error| map_admin_chain_error("event market publish", error))?;

        let published_market = crud::update_market_publication_status(
            &state.db,
            market.id,
            PUBLICATION_STATUS_PUBLISHED,
            Some(&tx.condition_id),
        )
        .await?;
        published_markets.push(published_market);
    }

    Ok(EventMarketsResponse::from_records(
        &event,
        &published_markets,
    ))
}

pub async fn pause_market(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketTradingStatusResponse, AuthError> {
    set_market_trading_status(state, market_id, "paused").await
}

pub async fn unpause_market(
    state: &AppState,
    market_id: Uuid,
) -> Result<MarketTradingStatusResponse, AuthError> {
    set_market_trading_status(state, market_id, "active").await
}

pub async fn propose_market_resolution(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: ProposeMarketResolutionRequest,
) -> Result<MarketResolutionWorkflowResponse, AuthError> {
    let market = crud::get_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;
    let now = Utc::now();

    ensure_market_not_resolved(&market)?;
    ensure_market_has_ended(&market, now)?;

    let existing_resolution =
        crud::get_market_resolution_by_market_id(&state.db, market_id).await?;
    match existing_resolution
        .as_ref()
        .map(|value| value.status.as_str())
    {
        Some("proposed") => {
            return Err(AuthError::bad_request(
                "market already has an active resolution proposal",
            ));
        }
        Some("finalized") | Some("emergency_resolved") => {
            return Err(AuthError::bad_request("market already resolved"));
        }
        _ => {}
    }

    let winning_outcome = validate_winning_outcome(&market, payload.resolution.winning_outcome)?;
    let notes = normalize_optional_text(payload.resolution.notes.as_deref());
    let dispute_window_seconds = if market.publication_status == PUBLICATION_STATUS_PUBLISHED {
        let condition_id = required_condition_id(&market)?;
        propose_resolution_tx(
            &state.env,
            condition_id,
            winning_outcome as u64,
            &market.oracle_address,
        )
        .await
        .map_err(|error| map_admin_chain_error("market resolution proposal", error))?
        .dispute_window_seconds
    } else {
        DEFAULT_RESOLUTION_DISPUTE_WINDOW_SECONDS
    };

    let resolution = NewMarketResolutionRecord {
        market_id,
        status: "proposed".to_owned(),
        proposed_winning_outcome: winning_outcome,
        final_winning_outcome: None,
        payout_vector_hash: derive_payout_vector_hash(market.outcome_count, winning_outcome)?,
        proposed_by_user_id: authenticated_user.user_id,
        proposed_at: now,
        dispute_deadline: now + Duration::seconds(dispute_window_seconds),
        notes,
        disputed_by_user_id: None,
        disputed_at: None,
        dispute_reason: None,
        finalized_by_user_id: None,
        finalized_at: None,
        emergency_resolved_by_user_id: None,
        emergency_resolved_at: None,
    };

    let (updated_market, resolution_record) =
        crud::upsert_market_resolution_with_trading_status(&state.db, &resolution, "paused")
            .await?;

    let event = load_market_event(state, updated_market.event_db_id).await?;

    Ok(MarketResolutionWorkflowResponse::from_records(
        &event,
        &updated_market,
        &resolution_record,
    ))
}

pub async fn dispute_market_resolution(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: DisputeMarketResolutionRequest,
) -> Result<MarketResolutionWorkflowResponse, AuthError> {
    let market = crud::get_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;
    ensure_market_not_resolved(&market)?;

    let resolution = crud::get_market_resolution_by_market_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::bad_request("market has no active resolution proposal"))?;

    if resolution.status != "proposed" {
        return Err(AuthError::bad_request(
            "only proposed resolutions can be disputed",
        ));
    }

    let now = Utc::now();
    if now >= resolution.dispute_deadline {
        return Err(AuthError::bad_request(
            "resolution dispute window has already elapsed",
        ));
    }

    let reason = normalize_required_text(&payload.resolution.reason, "dispute reason")?;
    if market.publication_status == PUBLICATION_STATUS_PUBLISHED {
        dispute_resolution_tx(&state.env, required_condition_id(&market)?)
            .await
            .map_err(|error| map_admin_chain_error("market resolution dispute", error))?;
    }

    let (updated_market, resolution_record) = crud::dispute_market_resolution(
        &state.db,
        market_id,
        authenticated_user.user_id,
        now,
        &reason,
    )
    .await?;

    let event = load_market_event(state, updated_market.event_db_id).await?;

    Ok(MarketResolutionWorkflowResponse::from_records(
        &event,
        &updated_market,
        &resolution_record,
    ))
}

pub async fn finalize_market_resolution(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
) -> Result<MarketResolutionWorkflowResponse, AuthError> {
    let market = crud::get_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;
    ensure_market_not_resolved(&market)?;

    let resolution = crud::get_market_resolution_by_market_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::bad_request("market has no active resolution proposal"))?;

    if resolution.status == "disputed" {
        return Err(AuthError::bad_request(
            "disputed resolutions cannot be finalized",
        ));
    }
    if resolution.status == "finalized" || resolution.status == "emergency_resolved" {
        return Err(AuthError::bad_request("market already resolved"));
    }
    if resolution.status != "proposed" {
        return Err(AuthError::bad_request(
            "only proposed resolutions can be finalized",
        ));
    }

    let now = Utc::now();
    if now < resolution.dispute_deadline {
        return Err(AuthError::bad_request(
            "resolution dispute window is still active",
        ));
    }

    if market.publication_status == PUBLICATION_STATUS_PUBLISHED {
        finalize_resolution_tx(
            &state.env,
            required_condition_id(&market)?,
            &market.oracle_address,
            resolution.proposed_winning_outcome as u64,
        )
            .await
            .map_err(|error| map_admin_chain_error("market resolution finalization", error))?;
    }

    let (updated_market, resolution_record) =
        crud::finalize_market_resolution(&state.db, market_id, authenticated_user.user_id, now)
            .await?;

    let event = load_market_event(state, updated_market.event_db_id).await?;

    Ok(MarketResolutionWorkflowResponse::from_records(
        &event,
        &updated_market,
        &resolution_record,
    ))
}

pub async fn emergency_market_resolution(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    market_id: Uuid,
    payload: EmergencyMarketResolutionRequest,
) -> Result<MarketResolutionWorkflowResponse, AuthError> {
    let market = crud::get_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;
    let now = Utc::now();

    ensure_market_not_resolved(&market)?;
    ensure_market_has_ended(&market, now)?;

    let existing_resolution =
        crud::get_market_resolution_by_market_id(&state.db, market_id).await?;
    match existing_resolution
        .as_ref()
        .map(|value| value.status.as_str())
    {
        Some("finalized") | Some("emergency_resolved") => {
            return Err(AuthError::bad_request("market already resolved"));
        }
        _ => {}
    }

    let winning_outcome = validate_winning_outcome(&market, payload.resolution.winning_outcome)?;
    let reason = normalize_required_text(&payload.resolution.reason, "emergency reason")?;
    if market.publication_status == PUBLICATION_STATUS_PUBLISHED {
        emergency_resolve_market(
            &state.env,
            required_condition_id(&market)?,
            &market.oracle_address,
            winning_outcome as u64,
        )
        .await
        .map_err(|error| map_admin_chain_error("emergency market resolution", error))?;
    }

    let resolution = NewMarketResolutionRecord {
        market_id,
        status: "emergency_resolved".to_owned(),
        proposed_winning_outcome: winning_outcome,
        final_winning_outcome: Some(winning_outcome),
        payout_vector_hash: derive_payout_vector_hash(market.outcome_count, winning_outcome)?,
        proposed_by_user_id: authenticated_user.user_id,
        proposed_at: now,
        dispute_deadline: now,
        notes: Some(reason),
        disputed_by_user_id: None,
        disputed_at: None,
        dispute_reason: None,
        finalized_by_user_id: None,
        finalized_at: None,
        emergency_resolved_by_user_id: Some(authenticated_user.user_id),
        emergency_resolved_at: Some(now),
    };

    let (updated_market, resolution_record) =
        crud::upsert_market_resolution_with_trading_status(&state.db, &resolution, "resolved")
            .await?;

    let event = load_market_event(state, updated_market.event_db_id).await?;

    Ok(MarketResolutionWorkflowResponse::from_records(
        &event,
        &updated_market,
        &resolution_record,
    ))
}

pub async fn register_event_neg_risk(
    state: &AppState,
    authenticated_user: AuthenticatedUser,
    event_id: Uuid,
    payload: RegisterNegRiskEventRequest,
) -> Result<NegRiskRegistrationResponse, AuthError> {
    let event = load_market_event(state, event_id).await?;

    if !event.neg_risk {
        return Err(AuthError::bad_request("event is not marked as neg risk"));
    }

    if event.publication_status != PUBLICATION_STATUS_PUBLISHED {
        return Err(AuthError::bad_request(
            "neg risk registration requires a published event",
        ));
    }

    if crud::count_markets_for_event(&state.db, event.id).await? < 2 {
        return Err(AuthError::bad_request(
            "neg risk registration requires at least two event markets",
        ));
    }

    if crud::get_event_neg_risk_config_by_event_id(&state.db, event.id)
        .await?
        .is_some()
    {
        return Err(AuthError::conflict("neg risk event already registered"));
    }

    let (other_market_id, other_condition_id) = match payload.neg_risk.other_market_id {
        Some(other_market_id) => {
            let other_market = crud::get_market_by_id(&state.db, other_market_id)
                .await?
                .ok_or_else(|| AuthError::not_found("other market not found"))?;

            if other_market.event_db_id != event.id {
                return Err(AuthError::bad_request(
                    "other market must belong to the same event",
                ));
            }

            let other_condition_id = other_market.condition_id.clone().ok_or_else(|| {
                AuthError::bad_request("other market is not linked to an on-chain condition yet")
            })?;

            (Some(other_market.id), Some(other_condition_id))
        }
        None => (None, None),
    };

    let tx = register_neg_risk_event(&state.env, &event.event_id, other_condition_id.as_deref())
        .await
        .map_err(|error| map_admin_chain_error("neg risk registration", error))?;

    let config = NewMarketEventNegRiskConfigRecord {
        event_id: event.id,
        registered: true,
        has_other: other_market_id.is_some(),
        other_market_id,
        other_condition_id,
        registered_by_user_id: authenticated_user.user_id,
        registered_at: Utc::now(),
    };

    let config = crud::create_event_neg_risk_config(&state.db, &config).await?;

    Ok(NegRiskRegistrationResponse::from_records(
        &event,
        &config,
        Some(tx.tx_hash),
    ))
}

fn build_market_record(
    event_db_id: Uuid,
    market: &CreateEventMarketRequest,
    index: usize,
    now: chrono::DateTime<Utc>,
    publication_status: &str,
) -> Result<NewMarketRecord, AuthError> {
    let label = normalize_required_text(&market.label, "market label")?;
    let slug = normalize_slug(&market.slug, "market slug")?;
    let question = normalize_required_text(&market.question, "market question")?;
    let oracle_address = normalize_wallet_address(&market.oracle_address)?;

    if market.end_time <= now {
        return Err(AuthError::bad_request(
            "market end_time must be in the future",
        ));
    }

    let outcomes = normalize_outcomes(&market.outcomes)?;
    let sort_order = market.sort_order.unwrap_or((index + 1) as i32);
    if sort_order <= 0 {
        return Err(AuthError::bad_request(
            "market sort_order must be greater than zero",
        ));
    }

    Ok(NewMarketRecord {
        id: Uuid::new_v4(),
        event_db_id,
        slug: slug.clone(),
        label,
        question,
        question_id: derive_bytes32_id("question", &slug),
        condition_id: None,
        market_type: "binary".to_owned(),
        outcome_count: outcomes.len() as i32,
        outcomes,
        end_time: market.end_time,
        sort_order,
        publication_status: publication_status.to_owned(),
        trading_status: "active".to_owned(),
        metadata_hash: None,
        oracle_address,
    })
}

fn build_price_ladder_markets(
    event: &MarketEventRecord,
    template: CreatePriceLadderTemplateRequest,
) -> Result<Vec<CreateEventMarketRequest>, AuthError> {
    let underlying = normalize_required_text(&template.underlying, "underlying")?;
    let deadline_label = normalize_required_text(&template.deadline_label, "deadline_label")?;
    let oracle_address = normalize_wallet_address(&template.oracle_address)?;
    let unit_symbol = normalize_required_text(&template.unit_symbol, "unit_symbol")?;
    let up_thresholds = normalize_price_thresholds(&template.up_thresholds, "up_thresholds")?;
    let down_thresholds = normalize_price_thresholds(&template.down_thresholds, "down_thresholds")?;

    if up_thresholds.is_empty() && down_thresholds.is_empty() {
        return Err(AuthError::bad_request(
            "at least one up_threshold or down_threshold is required",
        ));
    }

    let mut markets = Vec::with_capacity(up_thresholds.len() + down_thresholds.len());
    let mut seen_slugs = BTreeSet::new();
    let mut sort_order = 1_i32;

    for threshold in up_thresholds {
        let market = build_price_ladder_market(
            event,
            &underlying,
            &deadline_label,
            &unit_symbol,
            &oracle_address,
            template.end_time,
            "up",
            &threshold,
            sort_order,
        )?;

        if !seen_slugs.insert(market.slug.clone()) {
            return Err(AuthError::conflict(
                "generated ladder market slug already exists",
            ));
        }

        markets.push(market);
        sort_order += 1;
    }

    for threshold in down_thresholds {
        let market = build_price_ladder_market(
            event,
            &underlying,
            &deadline_label,
            &unit_symbol,
            &oracle_address,
            template.end_time,
            "down",
            &threshold,
            sort_order,
        )?;

        if !seen_slugs.insert(market.slug.clone()) {
            return Err(AuthError::conflict(
                "generated ladder market slug already exists",
            ));
        }

        markets.push(market);
        sort_order += 1;
    }

    Ok(markets)
}

fn build_price_ladder_market(
    event: &MarketEventRecord,
    underlying: &str,
    deadline_label: &str,
    unit_symbol: &str,
    oracle_address: &str,
    end_time: chrono::DateTime<Utc>,
    direction: &str,
    threshold: &str,
    sort_order: i32,
) -> Result<CreateEventMarketRequest, AuthError> {
    let direction_symbol = if direction == "up" { "↑" } else { "↓" };
    let comparator_text = if direction == "up" {
        "or higher"
    } else {
        "or lower"
    };
    let threshold_label = format!("{unit_symbol}{threshold}");
    let threshold_slug = slugify_threshold_value(threshold);

    Ok(CreateEventMarketRequest {
        label: format!("{direction_symbol} {threshold_label}"),
        slug: format!("{}-{}-{}", event.slug, direction, threshold_slug),
        question: format!(
            "Will {underlying} hit {threshold_label} {comparator_text} by {deadline_label}?"
        ),
        end_time,
        outcomes: vec!["Yes".to_owned(), "No".to_owned()],
        sort_order: Some(sort_order),
        oracle_address: oracle_address.to_owned(),
    })
}

fn build_updated_market_record(
    existing_market: MarketRecord,
    patch: crate::module::market::schema::UpdateMarketFieldsRequest,
) -> Result<MarketRecord, AuthError> {
    let now = Utc::now();

    let slug = match patch.slug.as_deref() {
        Some(value) => normalize_slug(value, "market slug")?,
        None => existing_market.slug.clone(),
    };
    let label = match patch.label.as_deref() {
        Some(value) => normalize_required_text(value, "market label")?,
        None => existing_market.label.clone(),
    };
    let question = match patch.question.as_deref() {
        Some(value) => normalize_required_text(value, "market question")?,
        None => existing_market.question.clone(),
    };
    let end_time = match patch.end_time {
        Some(value) => {
            if value <= now {
                return Err(AuthError::bad_request(
                    "market end_time must be in the future",
                ));
            }
            value
        }
        None => existing_market.end_time,
    };
    let outcomes = match patch.outcomes {
        Some(value) => normalize_outcomes(&value)?,
        None => existing_market.outcomes.clone(),
    };
    let sort_order = match patch.sort_order {
        Some(value) if value > 0 => value,
        Some(_) => {
            return Err(AuthError::bad_request(
                "market sort_order must be greater than zero",
            ));
        }
        None => existing_market.sort_order,
    };
    let oracle_address = match patch.oracle_address.as_deref() {
        Some(value) => normalize_wallet_address(value)?,
        None => existing_market.oracle_address.clone(),
    };

    Ok(MarketRecord {
        slug: slug.clone(),
        label,
        question,
        question_id: derive_bytes32_id("question", &slug),
        outcome_count: outcomes.len() as i32,
        outcomes,
        end_time,
        sort_order,
        oracle_address,
        ..existing_market
    })
}

async fn sync_standalone_event_wrapper(
    state: &AppState,
    event: MarketEventRecord,
    market: &MarketRecord,
) -> Result<MarketEventRecord, AuthError> {
    let event_slug = market.slug.clone();
    let group_key = format!("{event_slug}-group");
    let series_key = format!("{event_slug}-series");

    crud::update_market_event_for_standalone(
        &state.db,
        &event,
        &market.label,
        &event_slug,
        &group_key,
        &series_key,
        &derive_bytes32_id("event", &event_slug),
        &derive_bytes32_id("group", &group_key),
        &derive_bytes32_id("series", &series_key),
        &market.oracle_address,
    )
    .await
}

async fn set_market_trading_status(
    state: &AppState,
    market_id: Uuid,
    trading_status: &str,
) -> Result<MarketTradingStatusResponse, AuthError> {
    let market = crud::get_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))?;

    if market.trading_status == "resolved" {
        return Err(AuthError::bad_request(
            "resolved markets cannot be paused or unpaused",
        ));
    }

    if matches!(
        crud::get_market_resolution_by_market_id(&state.db, market_id)
            .await?
            .as_ref()
            .map(|value| value.status.as_str()),
        Some("proposed") | Some("disputed")
    ) {
        return Err(AuthError::bad_request(
            "markets with an active resolution workflow cannot be paused or unpaused",
        ));
    }

    let updated_market = if market.trading_status == trading_status {
        market
    } else {
        if market.publication_status == PUBLICATION_STATUS_PUBLISHED {
            let condition_id = required_condition_id(&market)?;
            let tx_result = if trading_status == "paused" {
                pause_market_tx(&state.env, condition_id).await
            } else {
                unpause_market_tx(&state.env, condition_id).await
            };

            tx_result
                .map_err(|error| map_admin_chain_error("market trading status update", error))?;
        }

        crud::update_market_trading_status(&state.db, market_id, trading_status).await?
    };

    let event = crud::get_market_event_by_id(&state.db, updated_market.event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))?;

    Ok(MarketTradingStatusResponse::from_records(
        &event,
        &updated_market,
    ))
}

async fn load_market_event(
    state: &AppState,
    event_db_id: Uuid,
) -> Result<MarketEventRecord, AuthError> {
    crud::get_market_event_by_id(&state.db, event_db_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))
}

async fn build_market_liquidity_response(
    state: &AppState,
    market: &MarketRecord,
) -> Result<MarketLiquidityResponse, AuthError> {
    let condition_id = required_condition_id(market)?;
    let liquidity = match get_market_liquidity_on_chain(&state.env, condition_id).await {
        Ok(liquidity) => liquidity,
        Err(error) => {
            tracing::warn!(
                market_id = %market.id,
                condition_id,
                ?error,
                "falling back to zero liquidity because on-chain market state is unavailable"
            );
            crate::service::stellar::MarketLiquidityReadResult {
                yes_available: "0".to_owned(),
                no_available: "0".to_owned(),
                idle_yes_total: "0".to_owned(),
                idle_no_total: "0".to_owned(),
                posted_yes_total: "0".to_owned(),
                posted_no_total: "0".to_owned(),
                claimable_collateral_total: "0".to_owned(),
            }
        }
    };

    Ok(MarketLiquidityResponse::new(
        market,
        vec![
            MarketLiquidityOutcomeResponse {
                outcome_index: 0,
                outcome_label: market
                    .outcomes
                    .first()
                    .cloned()
                    .unwrap_or_else(|| "Yes".to_owned()),
                available: liquidity.yes_available,
            },
            MarketLiquidityOutcomeResponse {
                outcome_index: 1,
                outcome_label: market
                    .outcomes
                    .get(1)
                    .cloned()
                    .unwrap_or_else(|| "No".to_owned()),
                available: liquidity.no_available,
            },
        ],
        PoolLiquidityResponse {
            idle_yes_total: liquidity.idle_yes_total,
            idle_no_total: liquidity.idle_no_total,
            posted_yes_total: liquidity.posted_yes_total,
            posted_no_total: liquidity.posted_no_total,
            claimable_collateral_total: liquidity.claimable_collateral_total,
        },
    ))
}

async fn build_market_detail_response(
    state: &AppState,
    market: MarketRecord,
) -> Result<MarketDetailResponse, AuthError> {
    // Keep public read paths to one DB connection at a time so cache misses
    // do not exhaust the shared pool and starve auth requests.
    let event = load_public_event(state, market.event_db_id).await?;
    let sibling_markets =
        crud::list_public_markets_for_event(&state.db, market.event_db_id).await?;
    let resolution = crud::get_market_resolution_by_market_id(&state.db, market.id).await?;
    let prices_by_condition = load_current_prices_by_market_refs_db_only(
        state,
        sibling_markets
            .iter()
            .filter_map(|value| {
                value
                    .condition_id
                    .clone()
                    .map(|condition_id| (value.id, condition_id))
            })
            .collect(),
    )
    .await?;
    let market_response = build_market_response_with_current_prices(&market, &prices_by_condition);
    let sibling_market_responses =
        build_market_responses_with_current_prices(&sibling_markets, &prices_by_condition);

    Ok(MarketDetailResponse {
        event: crate::module::market::schema::EventResponse::from(&event),
        on_chain: crate::module::market::schema::EventOnChainResponse::from(&event),
        market: market_response,
        resolution: resolution
            .as_ref()
            .map(crate::module::market::schema::MarketResolutionStateResponse::from),
        sibling_markets: sibling_market_responses,
    })
}

fn build_market_responses_with_current_prices(
    markets: &[MarketRecord],
    prices_by_condition: &HashMap<String, MarketCurrentPricesResponse>,
) -> Vec<MarketResponse> {
    markets
        .iter()
        .map(|market| build_market_response_with_current_prices(market, prices_by_condition))
        .collect()
}

fn build_event_market_responses(
    markets: &[MarketRecord],
    snapshots_by_condition: &HashMap<String, MarketPriceSnapshotRecord>,
    stats_by_market_id: &HashMap<Uuid, MarketTradeStatsRecord>,
) -> Result<Vec<MarketResponse>, AuthError> {
    markets
        .iter()
        .map(|market| {
            build_event_market_response(market, snapshots_by_condition, stats_by_market_id)
        })
        .collect()
}

#[derive(Clone, Copy)]
enum SnapshotRefreshPolicy {
    DbOnly,
    BestEffortRefresh,
}

async fn build_public_market_cards_by_event_with_current_prices(
    state: &AppState,
    markets: &[PublicMarketSummaryRecord],
) -> Result<HashMap<Uuid, Vec<PublicMarketCardResponse>>, AuthError> {
    let market_refs = markets
        .iter()
        .filter_map(|market| {
            market
                .condition_id
                .clone()
                .map(|condition_id| (market.market_id, condition_id))
        })
        .collect();
    let market_ids = markets.iter().map(|market| market.market_id).collect();
    let (snapshots_by_condition, stats_by_market_id) = tokio::try_join!(
        load_market_price_snapshots_by_market_refs_db_only(state, market_refs),
        load_market_trade_stats_by_market_id(state, market_ids),
    )?;
    let mut grouped = HashMap::<Uuid, Vec<PublicMarketCardResponse>>::new();

    for market in markets {
        grouped
            .entry(market.event_id)
            .or_default()
            .push(build_public_market_card_response(
                market,
                &snapshots_by_condition,
                &stats_by_market_id,
            )?);
    }

    Ok(grouped)
}

async fn build_public_market_cards_with_current_prices(
    state: &AppState,
    markets: &[PublicMarketSummaryRecord],
) -> Result<Vec<PublicMarketCardResponse>, AuthError> {
    let market_refs = markets
        .iter()
        .filter_map(|market| {
            market
                .condition_id
                .clone()
                .map(|condition_id| (market.market_id, condition_id))
        })
        .collect();
    let market_ids = markets.iter().map(|market| market.market_id).collect();
    let (snapshots_by_condition, stats_by_market_id) = tokio::try_join!(
        load_market_price_snapshots_by_market_refs_db_only(state, market_refs),
        load_market_trade_stats_by_market_id(state, market_ids),
    )?;

    markets
        .iter()
        .map(|market| {
            build_public_market_card_response(market, &snapshots_by_condition, &stats_by_market_id)
        })
        .collect()
}

async fn build_public_market_cards_from_market_records_with_current_prices(
    state: &AppState,
    event: &MarketEventRecord,
    markets: &[MarketRecord],
) -> Result<Vec<PublicMarketCardResponse>, AuthError> {
    let prices_by_condition = load_current_prices_by_market_refs_db_only(
        state,
        markets
            .iter()
            .filter_map(|market| {
                market
                    .condition_id
                    .clone()
                    .map(|condition_id| (market.id, condition_id))
            })
            .collect(),
    )
    .await?;

    Ok(markets
        .iter()
        .map(|market| {
            let current_prices = market
                .condition_id
                .as_ref()
                .and_then(|condition_id| prices_by_condition.get(condition_id))
                .cloned()
                .or_else(|| market.condition_id.as_ref().map(|_| fallback_current_prices()));

            PublicMarketCardResponse {
                current_prices,
                ..PublicMarketCardResponse::from_market_and_event(event, market)
            }
        })
        .collect())
}

fn build_market_response_with_current_prices(
    market: &MarketRecord,
    prices_by_condition: &HashMap<String, MarketCurrentPricesResponse>,
) -> MarketResponse {
    let current_prices = market
        .condition_id
        .as_ref()
        .and_then(|condition_id| prices_by_condition.get(condition_id))
        .cloned()
        .or_else(|| market.condition_id.as_ref().map(|_| fallback_current_prices()));

    MarketResponse {
        current_prices,
        ..MarketResponse::from(market)
    }
}

fn build_event_market_response(
    market: &MarketRecord,
    snapshots_by_condition: &HashMap<String, MarketPriceSnapshotRecord>,
    stats_by_market_id: &HashMap<Uuid, MarketTradeStatsRecord>,
) -> Result<MarketResponse, AuthError> {
    let snapshot = market
        .condition_id
        .as_ref()
        .and_then(|condition_id| snapshots_by_condition.get(condition_id));
    let trade_stats = stats_by_market_id.get(&market.id);
    let current_prices = match snapshot {
        Some(snapshot) => Some(market_current_prices_from_snapshot(snapshot)?),
        None => market
            .condition_id
            .as_ref()
            .map(|_| fallback_current_prices_from_trade_stats(trade_stats))
            .transpose()?,
    };
    let quote_summary = match snapshot {
        Some(snapshot) => Some(build_market_quote_summary_from_snapshot(snapshot)?),
        None => current_prices.as_ref().map(|current_prices| {
            build_market_quote_summary_from_prices(
                current_prices,
                "price_snapshot_fallback",
                Utc::now(),
            )
        }),
    };
    let stats = Some(build_market_stats_response(trade_stats));
    let last_trade_yes_bps = current_prices
        .as_ref()
        .map(|prices| derive_last_trade_yes_bps(trade_stats, prices.yes_bps))
        .transpose()?;

    Ok(MarketResponse {
        current_prices,
        stats,
        quote_summary,
        last_trade_yes_bps,
        ..MarketResponse::from(market)
    })
}

fn build_public_market_card_response(
    market: &PublicMarketSummaryRecord,
    snapshots_by_condition: &HashMap<String, MarketPriceSnapshotRecord>,
    stats_by_market_id: &HashMap<Uuid, MarketTradeStatsRecord>,
) -> Result<PublicMarketCardResponse, AuthError> {
    let snapshot = market
        .condition_id
        .as_ref()
        .and_then(|condition_id| snapshots_by_condition.get(condition_id));
    let trade_stats = stats_by_market_id.get(&market.market_id);
    let current_prices = match snapshot {
        Some(snapshot) => Some(market_current_prices_from_snapshot(snapshot)?),
        None => market
            .condition_id
            .as_ref()
            .map(|_| fallback_current_prices_from_trade_stats(trade_stats))
            .transpose()?,
    };
    let quote_summary = match snapshot {
        Some(snapshot) => Some(build_market_quote_summary_from_snapshot(snapshot)?),
        None => current_prices.as_ref().map(|current_prices| {
            build_market_quote_summary_from_prices(
                current_prices,
                "price_snapshot_fallback",
                Utc::now(),
            )
        }),
    };
    let stats = Some(build_market_stats_response(trade_stats));
    let last_trade_yes_bps = current_prices
        .as_ref()
        .map(|prices| derive_last_trade_yes_bps(trade_stats, prices.yes_bps))
        .transpose()?;

    Ok(PublicMarketCardResponse {
        current_prices,
        stats,
        quote_summary,
        last_trade_yes_bps,
        ..PublicMarketCardResponse::from(market)
    })
}

#[allow(dead_code)]
async fn load_current_prices_by_market_refs(
    state: &AppState,
    market_refs: Vec<(Uuid, String)>,
) -> Result<HashMap<String, MarketCurrentPricesResponse>, AuthError> {
    load_current_prices_by_market_refs_with_policy(
        state,
        market_refs,
        SnapshotRefreshPolicy::BestEffortRefresh,
    )
    .await
}

async fn load_current_prices_by_market_refs_db_only(
    state: &AppState,
    market_refs: Vec<(Uuid, String)>,
) -> Result<HashMap<String, MarketCurrentPricesResponse>, AuthError> {
    load_current_prices_by_market_refs_with_policy(
        state,
        market_refs,
        SnapshotRefreshPolicy::DbOnly,
    )
    .await
}

async fn load_current_prices_by_market_refs_with_policy(
    state: &AppState,
    market_refs: Vec<(Uuid, String)>,
    refresh_policy: SnapshotRefreshPolicy,
) -> Result<HashMap<String, MarketCurrentPricesResponse>, AuthError> {
    let snapshots =
        load_market_price_snapshots_by_market_refs_with_policy(state, market_refs, refresh_policy)
            .await?;

    Ok(snapshots
        .into_values()
        .filter_map(|snapshot| {
            let yes_bps = u32::try_from(snapshot.yes_bps).ok()?;
            let no_bps = u32::try_from(snapshot.no_bps).ok()?;

            Some((
                snapshot.condition_id,
                MarketCurrentPricesResponse { yes_bps, no_bps },
            ))
        })
        .collect())
}

async fn load_market_price_snapshots_by_market_refs(
    state: &AppState,
    market_refs: Vec<(Uuid, String)>,
) -> Result<HashMap<String, MarketPriceSnapshotRecord>, AuthError> {
    load_market_price_snapshots_by_market_refs_with_policy(
        state,
        market_refs,
        SnapshotRefreshPolicy::BestEffortRefresh,
    )
    .await
}

async fn load_market_price_snapshots_by_market_refs_db_only(
    state: &AppState,
    market_refs: Vec<(Uuid, String)>,
) -> Result<HashMap<String, MarketPriceSnapshotRecord>, AuthError> {
    load_market_price_snapshots_by_market_refs_with_policy(
        state,
        market_refs,
        SnapshotRefreshPolicy::DbOnly,
    )
    .await
}

async fn load_market_price_snapshots_by_market_refs_with_policy(
    state: &AppState,
    market_refs: Vec<(Uuid, String)>,
    refresh_policy: SnapshotRefreshPolicy,
) -> Result<HashMap<String, MarketPriceSnapshotRecord>, AuthError> {
    if market_refs.is_empty() {
        return Ok(HashMap::new());
    }

    let market_ids_by_condition =
        market_refs
            .into_iter()
            .fold(HashMap::new(), |mut acc, (market_id, condition_id)| {
                acc.entry(condition_id).or_insert(market_id);
                acc
            });
    let condition_ids = market_ids_by_condition.keys().cloned().collect::<Vec<_>>();
    let mut snapshots_by_condition =
        crud::list_market_price_snapshots_by_condition_ids(&state.db, &condition_ids)
            .await?
            .into_iter()
            .map(|snapshot| (snapshot.condition_id.clone(), snapshot))
            .collect::<HashMap<_, _>>();

    if matches!(refresh_policy, SnapshotRefreshPolicy::DbOnly) {
        return Ok(snapshots_by_condition);
    }

    let refresh_condition_ids = condition_ids
        .iter()
        .filter(|condition_id| {
            snapshot_refresh_needed(state, snapshots_by_condition.get(condition_id.as_str()))
        })
        .cloned()
        .collect::<Vec<_>>();

    if refresh_condition_ids.is_empty() {
        return Ok(snapshots_by_condition);
    }

    let refreshed_prices =
        match get_market_prices_batch_best_effort(&state.env, &refresh_condition_ids).await {
            Ok(prices) => prices,
            Err(error) => {
                tracing::warn!(error = %error, "unable to refresh stale market price snapshots");
                return Ok(snapshots_by_condition);
            }
        };

    for condition_id in refresh_condition_ids {
        let Some(prices) = refreshed_prices.get(&condition_id) else {
            continue;
        };
        let Some(market_id) = market_ids_by_condition.get(&condition_id).copied() else {
            continue;
        };
        if prices.yes_bps + prices.no_bps != 10_000 {
            tracing::warn!(
                market_id = %market_id,
                condition_id = %condition_id,
                yes_bps = prices.yes_bps,
                no_bps = prices.no_bps,
                "skipping invalid refreshed market price snapshot"
            );
            continue;
        }
        let snapshot = persist_market_price_snapshot(
            state,
            market_id,
            &condition_id,
            prices.yes_bps,
            prices.no_bps,
        )
        .await?;
        snapshots_by_condition.insert(condition_id, snapshot);
    }

    Ok(snapshots_by_condition)
}

async fn load_market_trade_stats_by_market_id(
    state: &AppState,
    market_ids: Vec<Uuid>,
) -> Result<HashMap<Uuid, MarketTradeStatsRecord>, AuthError> {
    let stats = crud::list_market_trade_stats_by_market_ids(&state.db, &market_ids).await?;

    Ok(stats
        .into_iter()
        .map(|record| (record.market_id, record))
        .collect())
}

async fn persist_market_price_snapshot(
    state: &AppState,
    market_id: Uuid,
    condition_id: &str,
    yes_bps: u32,
    no_bps: u32,
) -> Result<MarketPriceSnapshotRecord, AuthError> {
    let yes_bps =
        i32::try_from(yes_bps).map_err(|error| AuthError::internal("invalid YES price", error))?;
    let no_bps =
        i32::try_from(no_bps).map_err(|error| AuthError::internal("invalid NO price", error))?;

    if yes_bps + no_bps != 10_000 {
        return Err(AuthError::internal(
            "invalid price snapshot sum",
            anyhow!("price snapshot must sum to 10000"),
        ));
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

async fn load_or_fetch_current_market_snapshot(
    state: &AppState,
    market: &MarketRecord,
) -> Result<Option<MarketPriceSnapshotRecord>, AuthError> {
    let Some(condition_id) = market.condition_id.clone() else {
        return Ok(None);
    };
    let mut snapshots =
        load_market_price_snapshots_by_market_refs(state, vec![(market.id, condition_id.clone())])
            .await?;
    Ok(snapshots.remove(&condition_id))
}

fn snapshot_refresh_needed(state: &AppState, snapshot: Option<&MarketPriceSnapshotRecord>) -> bool {
    let Some(snapshot) = snapshot else {
        return true;
    };
    let max_age_secs = i64::try_from(state.env.market_price_sync_interval_secs).unwrap_or(i64::MAX);
    if max_age_secs == 0 {
        return true;
    }

    Utc::now()
        .signed_duration_since(snapshot.synced_at)
        .ge(&Duration::seconds(max_age_secs))
}

fn market_current_prices_from_snapshot(
    snapshot: &MarketPriceSnapshotRecord,
) -> Result<MarketCurrentPricesResponse, AuthError> {
    let yes_bps = u32::try_from(snapshot.yes_bps)
        .map_err(|error| AuthError::internal("invalid YES snapshot price", error))?;
    let no_bps = u32::try_from(snapshot.no_bps)
        .map_err(|error| AuthError::internal("invalid NO snapshot price", error))?;

    Ok(MarketCurrentPricesResponse { yes_bps, no_bps })
}

fn fallback_current_prices() -> MarketCurrentPricesResponse {
    MarketCurrentPricesResponse {
        yes_bps: DEFAULT_BINARY_MARKET_YES_BPS,
        no_bps: MARKET_PRICE_BPS_SCALE - DEFAULT_BINARY_MARKET_YES_BPS,
    }
}

fn fallback_current_prices_from_trade_stats(
    stats: Option<&MarketTradeStatsRecord>,
) -> Result<MarketCurrentPricesResponse, AuthError> {
    let Some(last_trade_yes_bps) = stats.and_then(|record| record.last_trade_yes_bps) else {
        return Ok(fallback_current_prices());
    };
    let yes_bps = u32::try_from(last_trade_yes_bps)
        .map_err(|error| AuthError::internal("invalid last trade fallback price", error))?;
    if yes_bps > MARKET_PRICE_BPS_SCALE {
        return Err(AuthError::internal(
            "invalid last trade fallback price",
            anyhow!("last trade fallback price exceeds basis-point scale"),
        ));
    }

    Ok(MarketCurrentPricesResponse {
        yes_bps,
        no_bps: MARKET_PRICE_BPS_SCALE - yes_bps,
    })
}

fn build_market_quote_summary_from_snapshot(
    snapshot: &MarketPriceSnapshotRecord,
) -> Result<MarketQuoteSummaryResponse, AuthError> {
    let current_prices = market_current_prices_from_snapshot(snapshot)?;

    Ok(build_market_quote_summary_from_prices(
        &current_prices,
        "price_snapshot",
        snapshot.synced_at,
    ))
}

fn build_market_quote_summary_from_prices(
    current_prices: &MarketCurrentPricesResponse,
    source: &str,
    as_of: chrono::DateTime<Utc>,
) -> MarketQuoteSummaryResponse {
    MarketQuoteSummaryResponse {
        buy_yes_bps: current_prices.yes_bps,
        buy_no_bps: current_prices.no_bps,
        as_of,
        source: source.to_owned(),
    }
}

fn build_market_stats_response(stats: Option<&MarketTradeStatsRecord>) -> MarketStatsResponse {
    MarketStatsResponse {
        volume_usd: format_usd_cents(stats.map_or(0, |record| record.volume_usd_cents)),
    }
}

fn derive_last_trade_yes_bps(
    stats: Option<&MarketTradeStatsRecord>,
    fallback_yes_bps: u32,
) -> Result<u32, AuthError> {
    match stats.and_then(|record| record.last_trade_yes_bps) {
        Some(value) => u32::try_from(value)
            .map_err(|error| AuthError::internal("invalid last trade price", error)),
        None => Ok(fallback_yes_bps),
    }
}

fn market_price_history_fetch_limit(limit: i64) -> i64 {
    (limit.saturating_mul(PRICE_HISTORY_LOOKBACK_MULTIPLIER))
        .clamp(limit, MAX_PRICE_HISTORY_SNAPSHOT_FETCH)
}

fn build_market_price_history_points(
    market: &MarketRecord,
    snapshots: &[MarketPriceHistorySnapshotRecord],
    interval: &str,
    limit: i64,
) -> Result<Vec<MarketPriceHistoryPointResponse>, AuthError> {
    if snapshots.is_empty() {
        return Ok(Vec::new());
    }

    let bucket_size_secs = history_interval_seconds(interval)?;
    let mut latest_by_bucket = HashMap::<i64, &MarketPriceHistorySnapshotRecord>::new();

    for snapshot in snapshots {
        let bucket = bucket_start_timestamp(snapshot.captured_at.timestamp(), bucket_size_secs);
        latest_by_bucket
            .entry(bucket)
            .and_modify(|existing| {
                if snapshot.captured_at > existing.captured_at {
                    *existing = snapshot;
                }
            })
            .or_insert(snapshot);
    }

    let mut buckets = latest_by_bucket.into_iter().collect::<Vec<_>>();
    buckets.sort_by_key(|(bucket, _)| *bucket);

    if buckets.len() > limit as usize {
        buckets.drain(0..(buckets.len() - limit as usize));
    }

    let mut points = Vec::with_capacity(buckets.len() * 2);
    for (bucket, snapshot) in buckets {
        let timestamp = bucket_timestamp(bucket)?;
        points.extend(build_history_points_from_snapshot(
            market,
            &timestamp,
            snapshot.yes_bps,
            snapshot.no_bps,
        )?);
    }

    Ok(points)
}

fn build_history_points_from_snapshot(
    market: &MarketRecord,
    timestamp: &chrono::DateTime<Utc>,
    yes_bps_raw: i32,
    no_bps_raw: i32,
) -> Result<Vec<MarketPriceHistoryPointResponse>, AuthError> {
    let yes_bps = u32::try_from(yes_bps_raw)
        .map_err(|error| AuthError::internal("invalid YES history price", error))?;
    let no_bps = u32::try_from(no_bps_raw)
        .map_err(|error| AuthError::internal("invalid NO history price", error))?;

    Ok(vec![
        MarketPriceHistoryPointResponse {
            timestamp: timestamp.to_owned(),
            outcome_index: 0,
            outcome_label: outcome_label(market, 0),
            price_bps: yes_bps,
            price: bps_to_price(yes_bps),
        },
        MarketPriceHistoryPointResponse {
            timestamp: timestamp.to_owned(),
            outcome_index: 1,
            outcome_label: outcome_label(market, 1),
            price_bps: no_bps,
            price: bps_to_price(no_bps),
        },
    ])
}

fn build_market_price_history_points_from_fills(
    market: &MarketRecord,
    fills: &[MarketOrderFillRecord],
    interval: &str,
    limit: i64,
) -> Result<Vec<MarketPriceHistoryPointResponse>, AuthError> {
    if fills.is_empty() {
        return Ok(Vec::new());
    }

    let bucket_size_secs = history_interval_seconds(interval)?;
    let mut latest_by_bucket = HashMap::<i64, &MarketOrderFillRecord>::new();

    for fill in fills {
        let bucket = bucket_start_timestamp(fill.created_at.timestamp(), bucket_size_secs);
        latest_by_bucket
            .entry(bucket)
            .and_modify(|existing| {
                if fill.created_at > existing.created_at {
                    *existing = fill;
                }
            })
            .or_insert(fill);
    }

    let mut buckets = latest_by_bucket.into_iter().collect::<Vec<_>>();
    buckets.sort_by_key(|(bucket, _)| *bucket);

    if buckets.len() > limit as usize {
        buckets.drain(0..(buckets.len() - limit as usize));
    }

    let mut points = Vec::with_capacity(buckets.len() * 2);
    for (bucket, fill) in buckets {
        let timestamp = bucket_timestamp(bucket)?;
        points.push(MarketPriceHistoryPointResponse {
            timestamp,
            outcome_index: 0,
            outcome_label: outcome_label(market, 0),
            price_bps: u32::try_from(fill.yes_price_bps)
                .map_err(|error| AuthError::internal("invalid YES fill price", error))?,
            price: bps_to_price(
                u32::try_from(fill.yes_price_bps)
                    .map_err(|error| AuthError::internal("invalid YES fill price", error))?,
            ),
        });
        points.push(MarketPriceHistoryPointResponse {
            timestamp,
            outcome_index: 1,
            outcome_label: outcome_label(market, 1),
            price_bps: u32::try_from(fill.no_price_bps)
                .map_err(|error| AuthError::internal("invalid NO fill price", error))?,
            price: bps_to_price(
                u32::try_from(fill.no_price_bps)
                    .map_err(|error| AuthError::internal("invalid NO fill price", error))?,
            ),
        });
    }

    Ok(points)
}

struct ClobOrderbookSnapshot {
    bids: Vec<OrderbookLevelResponse>,
    asks: Vec<OrderbookLevelResponse>,
    spread_bps: u32,
}

fn build_clob_orderbook(
    market: &MarketRecord,
    orders: &[MarketOrderRecord],
) -> Result<ClobOrderbookSnapshot, AuthError> {
    let mut bids_by_level = HashMap::<(i32, u32), U256>::new();
    let mut asks_by_level = HashMap::<(i32, u32), U256>::new();

    for order in orders {
        let remaining = parse_u256_decimal(&order.remaining_amount, "remaining_amount")?;
        if remaining.is_zero() {
            continue;
        }
        let price_bps = u32::try_from(order.price_bps)
            .map_err(|error| AuthError::internal("invalid orderbook price", error))?;
        let key = (order.outcome_index, price_bps);
        let levels = if order.side == "buy" {
            &mut bids_by_level
        } else if order.side == "sell" {
            &mut asks_by_level
        } else {
            continue;
        };
        let entry = levels.entry(key).or_insert_with(U256::zero);
        *entry += remaining;
    }

    let mut bids = bids_by_level
        .into_iter()
        .map(|((outcome_index, price_bps), shares)| {
            build_orderbook_level(market, outcome_index, price_bps, shares)
        })
        .collect::<Result<Vec<_>, _>>()?;
    bids.sort_by(|left, right| {
        right
            .price_bps
            .cmp(&left.price_bps)
            .then_with(|| left.outcome_index.cmp(&right.outcome_index))
    });

    let mut asks = asks_by_level
        .into_iter()
        .map(|((outcome_index, price_bps), shares)| {
            build_orderbook_level(market, outcome_index, price_bps, shares)
        })
        .collect::<Result<Vec<_>, _>>()?;
    asks.sort_by(|left, right| {
        left.price_bps
            .cmp(&right.price_bps)
            .then_with(|| left.outcome_index.cmp(&right.outcome_index))
    });

    let spread_bps = match (best_ask_price_bps(&asks, 0), best_bid_price_bps(&bids, 0)) {
        (Some(ask), Some(bid)) if ask >= bid => ask - bid,
        _ => 0,
    };

    Ok(ClobOrderbookSnapshot {
        bids,
        asks,
        spread_bps,
    })
}

fn best_bid_price_bps(levels: &[OrderbookLevelResponse], outcome_index: i32) -> Option<u32> {
    levels
        .iter()
        .filter(|level| level.outcome_index == outcome_index)
        .map(|level| level.price_bps)
        .max()
}

fn best_ask_price_bps(levels: &[OrderbookLevelResponse], outcome_index: i32) -> Option<u32> {
    levels
        .iter()
        .filter(|level| level.outcome_index == outcome_index)
        .map(|level| level.price_bps)
        .min()
}

async fn current_last_trade_yes_bps_from_fills(
    state: &AppState,
    market_id: Uuid,
) -> Result<Option<u32>, AuthError> {
    let fills = order_crud::list_market_order_fills_by_market_id(&state.db, market_id, 1).await?;
    let Some(fill) = fills.first() else {
        return Ok(None);
    };

    Ok(Some(u32::try_from(fill.yes_price_bps).map_err(
        |error| AuthError::internal("invalid YES fill price", error),
    )?))
}

fn build_market_trade_fill_response(
    fill: &MarketOrderFillRecord,
) -> Result<MarketTradeFillResponse, AuthError> {
    let yes_price_bps = u32::try_from(fill.yes_price_bps)
        .map_err(|error| AuthError::internal("invalid YES fill price", error))?;
    let no_price_bps = u32::try_from(fill.no_price_bps)
        .map_err(|error| AuthError::internal("invalid NO fill price", error))?;

    Ok(MarketTradeFillResponse {
        id: fill.id,
        match_type: fill.match_type.clone(),
        outcome_index: fill.outcome_index,
        fill_token_amount: format_decimal_str(&fill.fill_amount, USDC_DECIMALS),
        collateral_amount: format_decimal_str(&fill.collateral_amount, USDC_DECIMALS),
        yes_price_bps,
        no_price_bps,
        yes_price: bps_to_price(yes_price_bps),
        no_price: bps_to_price(no_price_bps),
        tx_hash: fill.tx_hash.clone(),
        executed_at: fill.created_at,
    })
}

fn build_synthetic_pool_asks(
    market: &MarketRecord,
    current_prices: &MarketCurrentPricesResponse,
    yes_available_raw: &str,
    no_available_raw: &str,
) -> Result<Vec<OrderbookLevelResponse>, AuthError> {
    let yes_available = parse_u256_decimal(yes_available_raw, "yes_available")?;
    let no_available = parse_u256_decimal(no_available_raw, "no_available")?;
    let mut asks = Vec::new();

    if !yes_available.is_zero() {
        asks.push(build_orderbook_level(
            market,
            0,
            current_prices.yes_bps,
            yes_available,
        )?);
    }

    if !no_available.is_zero() {
        asks.push(build_orderbook_level(
            market,
            1,
            current_prices.no_bps,
            no_available,
        )?);
    }

    Ok(asks)
}

fn build_synthetic_pool_bids(
    market: &MarketRecord,
    current_prices: &MarketCurrentPricesResponse,
    claimable_collateral_raw: &str,
) -> Result<Vec<OrderbookLevelResponse>, AuthError> {
    let claimable_collateral =
        parse_u256_decimal(claimable_collateral_raw, "claimable_collateral_total")?;
    if claimable_collateral.is_zero() {
        return Ok(Vec::new());
    }

    let (yes_budget, no_budget) =
        split_shared_collateral_budget(claimable_collateral, current_prices);
    let yes_shares = quote_sell_capacity(yes_budget, current_prices.yes_bps)?;
    let no_shares = quote_sell_capacity(no_budget, current_prices.no_bps)?;
    let mut bids = Vec::new();

    if !yes_shares.is_zero() {
        bids.push(build_orderbook_level(
            market,
            0,
            current_prices.yes_bps,
            yes_shares,
        )?);
    }

    if !no_shares.is_zero() {
        bids.push(build_orderbook_level(
            market,
            1,
            current_prices.no_bps,
            no_shares,
        )?);
    }

    Ok(bids)
}

fn build_orderbook_level(
    market: &MarketRecord,
    outcome_index: i32,
    price_bps: u32,
    shares_raw: U256,
) -> Result<OrderbookLevelResponse, AuthError> {
    let notional_raw = shares_raw
        .checked_mul(U256::from(price_bps))
        .ok_or_else(|| AuthError::internal("orderbook notional overflow", "uint256 overflow"))?
        / U256::from(MARKET_PRICE_BPS_SCALE);
    let shares = format_decimal_str(&shares_raw.to_string(), USDC_DECIMALS);
    let quantity = parse_decimal_f64(&shares)
        .map_err(|error| AuthError::internal("invalid share quantity", error))?;

    Ok(OrderbookLevelResponse {
        outcome_index,
        outcome_label: outcome_label(market, outcome_index),
        price_bps,
        price: bps_to_price(price_bps),
        quantity,
        shares,
        notional_usd: format_decimal_str(&notional_raw.to_string(), USDC_DECIMALS),
    })
}

fn split_shared_collateral_budget(
    claimable_collateral: U256,
    current_prices: &MarketCurrentPricesResponse,
) -> (U256, U256) {
    match (current_prices.yes_bps == 0, current_prices.no_bps == 0) {
        (true, false) => (U256::zero(), claimable_collateral),
        (false, true) => (claimable_collateral, U256::zero()),
        _ => {
            let yes_budget = claimable_collateral / U256::from(2_u64);
            (yes_budget, claimable_collateral - yes_budget)
        }
    }
}

fn quote_sell_capacity(collateral_raw: U256, price_bps: u32) -> Result<U256, AuthError> {
    if price_bps == 0 {
        return Ok(U256::zero());
    }

    collateral_raw
        .checked_mul(U256::from(MARKET_PRICE_BPS_SCALE))
        .ok_or_else(|| {
            AuthError::internal(
                "quote capacity overflow",
                anyhow!("quote capacity overflow"),
            )
        })
        .map(|value| value / U256::from(price_bps))
}

fn history_interval_seconds(interval: &str) -> Result<i64, AuthError> {
    match interval {
        "5m" => Ok(5 * 60),
        "15m" => Ok(15 * 60),
        "1h" => Ok(60 * 60),
        "4h" => Ok(4 * 60 * 60),
        "1d" => Ok(24 * 60 * 60),
        _ => Err(AuthError::bad_request(
            "interval must be one of 5m, 15m, 1h, 4h, or 1d",
        )),
    }
}

fn bucket_start_timestamp(timestamp_secs: i64, bucket_size_secs: i64) -> i64 {
    timestamp_secs - timestamp_secs.rem_euclid(bucket_size_secs)
}

fn bucket_timestamp(timestamp_secs: i64) -> Result<chrono::DateTime<Utc>, AuthError> {
    chrono::DateTime::<Utc>::from_timestamp(timestamp_secs, 0)
        .ok_or_else(|| AuthError::bad_request("invalid history bucket timestamp"))
}

fn outcome_label(market: &MarketRecord, outcome_index: i32) -> String {
    market
        .outcomes
        .get(outcome_index.max(0) as usize)
        .cloned()
        .unwrap_or_else(|| {
            if outcome_index == 0 {
                "Yes".to_owned()
            } else {
                "No".to_owned()
            }
        })
}

fn parse_u256_decimal(raw: &str, field_name: &str) -> Result<U256, AuthError> {
    U256::from_dec_str(raw).map_err(|error| {
        AuthError::internal("invalid on-chain amount", format!("{field_name}: {error}"))
    })
}

fn format_usd_cents(value: i64) -> String {
    let whole = value / 100;
    let fractional = value.rem_euclid(100);
    format!("{whole}.{fractional:02}")
}

fn format_decimal_str(raw: &str, decimals: usize) -> String {
    if raw.is_empty() {
        return "0".to_owned();
    }

    let raw = raw.trim_start_matches('0');
    if raw.is_empty() {
        return "0".to_owned();
    }

    if raw.len() <= decimals {
        let fractional = format!("{raw:0>width$}", width = decimals);
        return format_fractional("0", &fractional);
    }

    let split_index = raw.len() - decimals;
    let whole = &raw[..split_index];
    let fractional = &raw[split_index..];
    format_fractional(whole, fractional)
}

fn format_fractional(whole: &str, fractional: &str) -> String {
    let trimmed_fractional = fractional.trim_end_matches('0');
    if trimmed_fractional.is_empty() {
        whole.to_owned()
    } else {
        format!("{whole}.{trimmed_fractional}")
    }
}

fn parse_decimal_f64(raw: &str) -> Result<f64, std::num::ParseFloatError> {
    raw.parse::<f64>()
}

fn bps_to_price(price_bps: u32) -> f64 {
    f64::from(price_bps) / f64::from(MARKET_PRICE_BPS_SCALE)
}

pub async fn load_public_market_context(
    state: &AppState,
    market_id: Uuid,
) -> Result<PublicMarketContext, AuthError> {
    let market = load_public_market(state, market_id).await?;
    let event = load_public_event(state, market.event_db_id).await?;

    Ok(PublicMarketContext { market, event })
}

async fn load_public_market(state: &AppState, market_id: Uuid) -> Result<MarketRecord, AuthError> {
    crud::get_public_market_by_id(&state.db, market_id)
        .await?
        .ok_or_else(|| AuthError::not_found("market not found"))
}

async fn load_public_event(
    state: &AppState,
    event_id: Uuid,
) -> Result<MarketEventRecord, AuthError> {
    crud::get_public_market_event_by_id(&state.db, event_id)
        .await?
        .ok_or_else(|| AuthError::not_found("event not found"))
}

fn ensure_market_not_resolved(market: &MarketRecord) -> Result<(), AuthError> {
    if market.trading_status == "resolved" {
        return Err(AuthError::bad_request("market already resolved"));
    }

    Ok(())
}

fn ensure_market_is_bootstrappable(market: &MarketRecord) -> Result<(), AuthError> {
    if market.publication_status != PUBLICATION_STATUS_PUBLISHED {
        return Err(AuthError::bad_request(
            "market must be published before liquidity can be bootstrapped",
        ));
    }

    if market.market_type != "binary" || market.outcome_count != 2 {
        return Err(AuthError::bad_request(
            "only published binary markets can be bootstrapped today",
        ));
    }

    ensure_market_not_resolved(market)
}

fn required_condition_id(market: &MarketRecord) -> Result<&str, AuthError> {
    market
        .condition_id
        .as_deref()
        .ok_or_else(|| AuthError::bad_request("published market is missing condition_id"))
}

fn normalize_binary_prices(yes_bps: u32, no_bps: u32) -> Result<(u32, u32), AuthError> {
    if yes_bps == 0 || yes_bps >= 10_000 {
        return Err(AuthError::bad_request(
            "prices.yes_bps must be greater than 0 and less than 10000",
        ));
    }

    if no_bps == 0 || no_bps >= 10_000 {
        return Err(AuthError::bad_request(
            "prices.no_bps must be greater than 0 and less than 10000",
        ));
    }

    if yes_bps + no_bps != 10_000 {
        return Err(AuthError::bad_request(
            "yes_bps and no_bps must sum to 10000",
        ));
    }

    Ok((yes_bps, no_bps))
}

fn normalize_u256_decimal(
    raw: &str,
    field_name: &str,
    allow_zero: bool,
) -> Result<String, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }

    let parsed = U256::from_dec_str(value).map_err(|_| {
        AuthError::bad_request(format!("{field_name} must be a base-10 integer string"))
    })?;

    if parsed.is_zero() && !allow_zero {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be greater than zero"
        )));
    }

    Ok(parsed.to_string())
}

fn sum_u128_decimal_strings(left: &str, right: &str, field_name: &str) -> Result<u128, AuthError> {
    let left = left
        .parse::<u128>()
        .map_err(|_| AuthError::bad_request(format!("{field_name} is not a valid u128 amount")))?;
    let right = right
        .parse::<u128>()
        .map_err(|_| AuthError::bad_request(format!("{field_name} is not a valid u128 amount")))?;
    left.checked_add(right)
        .ok_or_else(|| AuthError::bad_request(format!("{field_name} overflowed u128")))
}

fn map_admin_chain_error(action: &'static str, error: anyhow::Error) -> AuthError {
    let message = format!("{error:#}");
    tracing::error!(%message, action, "admin chain action failed");

    if message.contains("already exists")
        || message.contains("already registered")
        || message.contains("already resolved")
        || message.contains("missing condition_id")
    {
        return AuthError::conflict(format!("{action} failed: {message}"));
    }

    AuthError::bad_request(format!("{action} failed: {message}"))
}

fn unix_timestamp_u64(value: chrono::DateTime<Utc>, field_name: &str) -> Result<u64, AuthError> {
    u64::try_from(value.timestamp())
        .map_err(|_| AuthError::bad_request(format!("{field_name} must be a valid unix timestamp")))
}

fn normalize_limit(value: Option<i64>, default: i64) -> Result<i64, AuthError> {
    let limit = value.unwrap_or(default);

    if limit <= 0 {
        return Err(AuthError::bad_request("limit must be greater than zero"));
    }

    Ok(limit.min(MAX_LIST_LIMIT))
}

fn normalize_offset(value: Option<i64>) -> Result<i64, AuthError> {
    let offset = value.unwrap_or(0);

    if offset < 0 {
        return Err(AuthError::bad_request("offset must be zero or greater"));
    }

    Ok(offset)
}

fn normalize_market_search_query(
    raw: Option<&str>,
) -> Result<NormalizedMarketSearchQuery, AuthError> {
    let raw =
        normalize_optional_text(raw).ok_or_else(|| AuthError::bad_request("q is required"))?;

    if raw.chars().count() < 2 {
        return Err(AuthError::bad_request(
            "q must be at least 2 characters long",
        ));
    }

    if raw.chars().count() > 120 {
        return Err(AuthError::bad_request("q must be 120 characters or fewer"));
    }

    let tokens = tokenize_market_search_query(&raw);
    if tokens.is_empty() {
        return Err(AuthError::bad_request(
            "q must contain at least one letter or number",
        ));
    }

    Ok(NormalizedMarketSearchQuery {
        raw,
        tsquery: tokens
            .into_iter()
            .map(|token| format!("{token}:*"))
            .collect::<Vec<_>>()
            .join(" & "),
    })
}

fn normalize_optional_trading_status(value: Option<&str>) -> Result<Option<String>, AuthError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() {
        return Ok(None);
    }

    match normalized.as_str() {
        "active" | "paused" | "resolved" => Ok(Some(normalized)),
        _ => Err(AuthError::bad_request(
            "trading_status must be active, paused, or resolved",
        )),
    }
}

fn normalize_optional_publication_status(value: Option<&str>) -> Result<Option<String>, AuthError> {
    let Some(value) = value else {
        return Ok(None);
    };

    let normalized = value.trim().to_ascii_lowercase();
    if normalized.is_empty() || normalized == "all" {
        return Ok(None);
    }

    match normalized.as_str() {
        "draft" | "published" => Ok(Some(normalized)),
        _ => Err(AuthError::bad_request(
            "publication_status must be draft, published, or all",
        )),
    }
}

fn normalize_history_interval(value: Option<&str>) -> Result<String, AuthError> {
    let normalized = value
        .map(|value| value.trim().to_ascii_lowercase())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "1h".to_owned());

    match normalized.as_str() {
        "5m" | "15m" | "1h" | "4h" | "1d" => Ok(normalized),
        _ => Err(AuthError::bad_request(
            "interval must be one of 5m, 15m, 1h, 4h, or 1d",
        )),
    }
}

fn ensure_market_has_ended(
    market: &MarketRecord,
    now: chrono::DateTime<Utc>,
) -> Result<(), AuthError> {
    if market.end_time > now {
        return Err(AuthError::bad_request("market has not ended yet"));
    }

    Ok(())
}

fn validate_winning_outcome(market: &MarketRecord, winning_outcome: i32) -> Result<i32, AuthError> {
    if winning_outcome < 0 || winning_outcome >= market.outcome_count {
        return Err(AuthError::bad_request("winning_outcome is out of range"));
    }

    Ok(winning_outcome)
}

fn derive_payout_vector_hash(
    outcome_count: i32,
    winning_outcome: i32,
) -> Result<String, AuthError> {
    let outcome_count = usize::try_from(outcome_count)
        .map_err(|_| AuthError::bad_request("invalid market outcome count"))?;
    let winning_outcome = usize::try_from(winning_outcome)
        .map_err(|_| AuthError::bad_request("winning_outcome is out of range"))?;

    if winning_outcome >= outcome_count {
        return Err(AuthError::bad_request("winning_outcome is out of range"));
    }

    let mut payouts = Vec::with_capacity(outcome_count);
    for index in 0..outcome_count {
        let value = if index == winning_outcome {
            U256::one()
        } else {
            U256::zero()
        };
        payouts.push(Token::Uint(value));
    }

    let hash = keccak256(encode(&[Token::Array(payouts)]));
    Ok(format!("0x{}", hex::encode(hash)))
}

fn is_standalone_event_wrapper(event: &MarketEventRecord, market: &MarketRecord) -> bool {
    event.slug == market.slug
        && event.title == market.label
        && event.group_key == format!("{}-group", market.slug)
        && event.series_key == format!("{}-series", market.slug)
}

fn normalize_outcomes(raw: &[String]) -> Result<Vec<String>, AuthError> {
    let outcomes = normalize_text_list(raw);

    if outcomes.len() != 2 {
        return Err(AuthError::bad_request(
            "only binary markets with exactly two outcomes are supported today",
        ));
    }

    if outcomes[0] == outcomes[1] {
        return Err(AuthError::bad_request(
            "market outcomes must be distinct values",
        ));
    }

    Ok(outcomes)
}

fn normalize_price_thresholds(raw: &[String], field_name: &str) -> Result<Vec<String>, AuthError> {
    raw.iter()
        .map(|value| normalize_price_threshold(value, field_name))
        .collect()
}

fn normalize_price_threshold(raw: &str, field_name: &str) -> Result<String, AuthError> {
    let normalized = raw.trim().replace(',', "");

    if normalized.is_empty() {
        return Err(AuthError::bad_request(format!(
            "{field_name} entries must be non-empty",
        )));
    }

    let mut dot_count = 0;
    for character in normalized.chars() {
        if character == '.' {
            dot_count += 1;
            if dot_count > 1 {
                return Err(AuthError::bad_request(format!(
                    "{field_name} entries must be numeric values",
                )));
            }
            continue;
        }

        if !character.is_ascii_digit() {
            return Err(AuthError::bad_request(format!(
                "{field_name} entries must be numeric values",
            )));
        }
    }

    Ok(normalized)
}

fn slugify_threshold_value(value: &str) -> String {
    value
        .chars()
        .map(|character| if character == '.' { '-' } else { character })
        .collect()
}

fn normalize_slug_list(values: &[String], field_name: &str) -> Result<Vec<String>, AuthError> {
    values
        .iter()
        .map(|value| normalize_slug(value, field_name))
        .collect()
}

fn normalize_text_list(values: &[String]) -> Vec<String> {
    values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            (!trimmed.is_empty()).then(|| trimmed.to_owned())
        })
        .collect()
}

fn normalize_slug(raw: &str, field_name: &str) -> Result<String, AuthError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }

    let mut slug = String::with_capacity(trimmed.len());
    let mut previous_was_hyphen = false;

    for character in trimmed.chars() {
        if character.is_ascii_alphanumeric() {
            slug.push(character.to_ascii_lowercase());
            previous_was_hyphen = false;
            continue;
        }

        if !previous_was_hyphen {
            slug.push('-');
            previous_was_hyphen = true;
        }
    }

    let normalized = slug.trim_matches('-').to_owned();

    if normalized.is_empty() {
        return Err(AuthError::bad_request(format!(
            "{field_name} must contain letters or numbers",
        )));
    }

    Ok(normalized)
}

fn normalize_bytes32(raw: &str, field_name: &str) -> Result<String, AuthError> {
    let trimmed = raw.trim().trim_matches('"').to_ascii_lowercase();
    let hex = trimmed.strip_prefix("0x").unwrap_or(&trimmed);

    if hex.len() != 64 {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be a 32-byte hex string",
        )));
    }

    if !hex.chars().all(|ch| ch.is_ascii_hexdigit()) {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be a 32-byte hex string",
        )));
    }

    Ok(format!("0x{hex}"))
}

fn normalize_optional_slug(raw: Option<&str>) -> Result<Option<String>, AuthError> {
    raw.map(|value| normalize_slug(value, "subcategory slug"))
        .transpose()
}

fn normalize_required_text(raw: &str, field_name: &str) -> Result<String, AuthError> {
    let trimmed = raw.trim();

    if trimmed.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }

    Ok(trimmed.to_owned())
}

fn normalize_optional_text(raw: Option<&str>) -> Option<String> {
    raw.and_then(|value| {
        let trimmed = value.trim();
        (!trimmed.is_empty()).then(|| trimmed.to_owned())
    })
}

fn tokenize_market_search_query(raw: &str) -> Vec<String> {
    raw.split(|ch: char| !ch.is_alphanumeric())
        .filter_map(|value| {
            let token = value.trim().to_lowercase();
            (token.chars().count() >= 2).then_some(token)
        })
        .collect()
}

fn derive_bytes32_id(namespace: &str, value: &str) -> String {
    let hash = keccak256(format!("sabi:{namespace}:{value}"));
    format!("0x{}", hex::encode(hash))
}

#[cfg(test)]
mod tests {
    use super::{
        MarketEventRecord, build_price_ladder_markets, derive_bytes32_id,
        normalize_market_search_query, normalize_slug, tokenize_market_search_query,
    };
    use crate::module::market::schema::CreatePriceLadderTemplateRequest;
    use chrono::{TimeZone, Utc};
    use uuid::Uuid;

    #[test]
    fn derives_stable_bytes32_ids() {
        let first = derive_bytes32_id("event", "us-iran-ceasefire-by");
        let second = derive_bytes32_id("event", "us-iran-ceasefire-by");

        assert_eq!(first, second);
        assert_eq!(first.len(), 66);
    }

    #[test]
    fn normalizes_hyphenated_slugs() {
        let slug = normalize_slug("  US-Iran-Ceasefire-By  ", "slug").unwrap();

        assert_eq!(slug, "us-iran-ceasefire-by");
    }

    #[test]
    fn slugifies_capitalized_and_spaced_values() {
        assert_eq!(
            normalize_slug("Finance", "category slug").unwrap(),
            "finance"
        );
        assert_eq!(
            normalize_slug("Finance, Monthly", "tag slug").unwrap(),
            "finance-monthly"
        );
        assert_eq!(
            normalize_slug("What will WTI Crude Oil (WTI) hit in April 2026?", "slug").unwrap(),
            "what-will-wti-crude-oil-wti-hit-in-april-2026"
        );
    }

    #[test]
    fn tokenizes_market_search_query_into_prefix_terms() {
        let terms = tokenize_market_search_query("  Fed 25-bps?  ");

        assert_eq!(terms, vec!["fed", "25", "bps"]);
    }

    #[test]
    fn normalizes_market_search_query_into_tsquery() {
        let search = normalize_market_search_query(Some("  Trump Fed  ")).unwrap();

        assert_eq!(search.raw, "Trump Fed");
        assert_eq!(search.tsquery, "trump:* & fed:*");
    }

    #[test]
    fn rejects_market_search_query_without_terms() {
        assert!(normalize_market_search_query(Some(" % _ - ")).is_err());
    }

    #[test]
    fn builds_price_ladder_markets_for_one_event() {
        let event = MarketEventRecord {
            id: Uuid::new_v4(),
            title: "What will WTI Crude Oil (WTI) hit in April 2026?".to_owned(),
            slug: "what-will-wti-crude-oil-hit-in-april-2026".to_owned(),
            category_slug: "economy".to_owned(),
            subcategory_slug: Some("commodities".to_owned()),
            tag_slugs: vec!["oil".to_owned()],
            image_url: None,
            summary_text: None,
            rules_text: "rules".to_owned(),
            context_text: None,
            additional_context: None,
            resolution_sources: vec![],
            resolution_timezone: "America/New_York".to_owned(),
            starts_at: None,
            sort_at: None,
            featured: false,
            breaking: false,
            searchable: true,
            visible: true,
            hide_resolved_by_default: true,
            group_key: "wti".to_owned(),
            series_key: "april-2026".to_owned(),
            event_id: derive_bytes32_id("event", "what-will-wti-crude-oil-hit-in-april-2026"),
            group_id: derive_bytes32_id("group", "wti"),
            series_id: derive_bytes32_id("series", "april-2026"),
            neg_risk: false,
            oracle_address: None,
            publication_status: "draft".to_owned(),
            published_tx_hash: None,
            created_by_user_id: Uuid::new_v4(),
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        let payload = CreatePriceLadderTemplateRequest {
            underlying: "WTI Crude Oil (WTI)".to_owned(),
            deadline_label: "May 1, 2026".to_owned(),
            end_time: Utc.with_ymd_and_hms(2026, 5, 2, 3, 59, 59).unwrap(),
            oracle_address: "0x6D21167d874C842386e8c484519B5ddBBaB87b43".to_owned(),
            unit_symbol: "$".to_owned(),
            up_thresholds: vec!["120".to_owned(), "130".to_owned()],
            down_thresholds: vec!["80".to_owned()],
        };

        let markets = build_price_ladder_markets(&event, payload).unwrap();

        assert_eq!(markets.len(), 3);
        assert_eq!(markets[0].label, "↑ $120");
        assert_eq!(
            markets[0].slug,
            "what-will-wti-crude-oil-hit-in-april-2026-up-120"
        );
        assert_eq!(
            markets[0].question,
            "Will WTI Crude Oil (WTI) hit $120 or higher by May 1, 2026?"
        );
        assert_eq!(markets[2].label, "↓ $80");
    }
}
