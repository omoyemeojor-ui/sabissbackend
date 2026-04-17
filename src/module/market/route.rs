use axum::{
    Router, middleware as axum_middleware,
    routing::{get, patch, post},
};

use crate::{
    app::AppState,
    middleware::user::require_auth,
    module::market::{
        controller::{
            admin_event_markets, admin_event_show, admin_events_index,
            bootstrap_event_liquidity_draft, bootstrap_market_liquidity_draft, categories_index,
            category_show, configure_market_auto_create_series_draft,
            configure_market_auto_resolve_coinbase_draft, create_event_draft,
            create_event_market_ladder_draft, create_event_markets_draft, create_market_draft,
            dispute_market_resolution_draft, emergency_market_resolution_draft, event_markets,
            event_show, events_index, finalize_market_resolution_draft, market_activity,
            market_liquidity, market_orderbook, market_outcomes, market_price_history,
            market_quote, market_related, market_resolution, market_show,
            market_show_by_condition, market_show_by_slug, market_trades, markets_home,
            markets_index, markets_search, pause_market_draft, propose_market_resolution_draft,
            publish_event_markets_batch, publish_event_shell, register_event_neg_risk_draft,
            set_market_prices_draft, tags_index, unpause_market_draft, update_market_draft,
        },
        trade_controller::{market_buy, market_merge, market_sell, market_split},
    },
};

pub fn public_router() -> Router<AppState> {
    Router::new()
        .route("/events", get(events_index))
        .route("/events/{event_id}/markets", get(event_markets))
        .route("/events/{event_id}", get(event_show))
        .route("/categories", get(categories_index))
        .route("/categories/{slug}", get(category_show))
        .route("/tags", get(tags_index))
        .route("/markets/home", get(markets_home))
        .route("/markets", get(markets_index))
        .route("/markets/search", get(markets_search))
        .route(
            "/markets/by-condition/{condition_id}",
            get(market_show_by_condition),
        )
        .route("/markets/slug/{slug}", get(market_show_by_slug))
        .route("/markets/{market_id}/liquidity", get(market_liquidity))
        .route("/markets/{market_id}/resolution", get(market_resolution))
        .route("/markets/{market_id}/related", get(market_related))
        .route("/markets/{market_id}/outcomes", get(market_outcomes))
        .route("/markets/{market_id}/activity", get(market_activity))
        .route("/markets/{market_id}/quote", get(market_quote))
        .route(
            "/markets/{market_id}/price-history",
            get(market_price_history),
        )
        .route("/markets/{market_id}/trades", get(market_trades))
        .route("/markets/{market_id}/orderbook", get(market_orderbook))
        .route("/markets/{market_id}", get(market_show))
}

pub fn admin_router() -> Router<AppState> {
    Router::new()
        .route(
            "/market-series/coinbase",
            post(configure_market_auto_create_series_draft),
        )
        .route("/markets", post(create_market_draft))
        .route("/markets/{market_id}", patch(update_market_draft))
        .route("/markets/{market_id}/prices", post(set_market_prices_draft))
        .route(
            "/markets/{market_id}/auto-resolve/coinbase",
            post(configure_market_auto_resolve_coinbase_draft),
        )
        .route(
            "/markets/{market_id}/liquidity/bootstrap",
            post(bootstrap_market_liquidity_draft),
        )
        .route("/markets/{market_id}/pause", post(pause_market_draft))
        .route("/markets/{market_id}/unpause", post(unpause_market_draft))
        .route(
            "/markets/{market_id}/resolution/propose",
            post(propose_market_resolution_draft),
        )
        .route(
            "/markets/{market_id}/resolution/dispute",
            post(dispute_market_resolution_draft),
        )
        .route(
            "/markets/{market_id}/resolution/finalize",
            post(finalize_market_resolution_draft),
        )
        .route(
            "/markets/{market_id}/resolution/emergency",
            post(emergency_market_resolution_draft),
        )
        .route("/events", get(admin_events_index).post(create_event_draft))
        .route("/events/{event_id}", get(admin_event_show))
        .route("/events/{event_id}/publish", post(publish_event_shell))
        .route("/events/{event_id}/markets", get(admin_event_markets))
        .route(
            "/events/{event_id}/markets",
            post(create_event_markets_draft),
        )
        .route(
            "/events/{event_id}/markets/publish",
            post(publish_event_markets_batch),
        )
        .route(
            "/events/{event_id}/markets/ladders",
            post(create_event_market_ladder_draft),
        )
        .route(
            "/events/{event_id}/liquidity/bootstrap",
            post(bootstrap_event_liquidity_draft),
        )
        .route(
            "/events/{event_id}/neg-risk/register",
            post(register_event_neg_risk_draft),
        )
}

pub fn me_router(state: AppState) -> Router<AppState> {
    Router::new()
        .route("/markets/{market_id}/buy", post(market_buy))
        .route("/markets/{market_id}/sell", post(market_sell))
        .route("/markets/{market_id}/split", post(market_split))
        .route("/markets/{market_id}/merge", post(market_merge))
        .route_layer(axum_middleware::from_fn_with_state(state, require_auth))
}
