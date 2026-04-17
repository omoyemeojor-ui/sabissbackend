use axum::{
    Json,
    extract::{Extension, Path, Query, State},
    http::StatusCode,
};
use uuid::Uuid;

use crate::{
    app::AppState,
    module::{
        auth::error::AuthError,
        market::schema::{
            AdminEventMarketsQuery, AdminListEventsQuery, BootstrapEventLiquidityRequest,
            BootstrapMarketLiquidityRequest, CategoriesResponse, CategoryDetailResponse,
            CategoryMarketsQuery, ConfigureMarketAutoCreateSeriesRequest,
            ConfigureMarketAutoResolveRequest, CreateEventMarketLadderRequest,
            CreateEventMarketsRequest, CreateEventMarketsResponse, CreateEventRequest,
            CreateEventResponse, CreateMarketRequest, CreateMarketResponse,
            DisputeMarketResolutionRequest, EmergencyMarketResolutionRequest, EventDetailResponse,
            EventLiquidityBootstrapResponse, EventListResponse, EventMarketsQuery,
            EventMarketsResponse, ListEventsQuery, ListMarketsQuery, MarketActivityResponse,
            MarketAutoCreateSeriesResponse, MarketAutoResolveConfigResponse,
            MarketDetailResponse, MarketLiquidityBootstrapResponse, MarketLiquidityResponse,
            MarketListResponse, MarketOrderbookResponse, MarketOutcomesResponse,
            MarketPriceHistoryQuery, MarketPriceHistoryResponse, MarketPricesResponse,
            MarketQuoteResponse, MarketResolutionReadResponse, MarketResolutionWorkflowResponse,
            MarketTradesResponse, MarketTradingStatusResponse, MarketsHomeQuery,
            MarketsHomeResponse, NegRiskRegistrationResponse, ProposeMarketResolutionRequest,
            RegisterNegRiskEventRequest, RelatedMarketsResponse, SearchMarketsQuery,
            SetMarketPricesRequest, TagsResponse, UpdateMarketRequest, UpdateMarketResponse,
            AdminEventDetailResponse, AdminEventListResponse, AdminEventMarketsResponse,
        },
    },
    service::{
        jwt::AuthenticatedUser,
        market::{
            admin_get_event_by_id, admin_get_event_markets, admin_list_events,
            bootstrap_event_liquidity, bootstrap_market_liquidity, create_event,
            create_event_market_ladder, create_event_markets, create_market,
            dispute_market_resolution, emergency_market_resolution, finalize_market_resolution,
            get_category_by_slug, get_event_by_id, get_event_markets, get_market_activity,
            get_market_by_condition_id, get_market_by_id, get_market_by_slug,
            get_market_liquidity, get_market_orderbook, get_market_outcomes,
            get_market_price_history, get_market_quote, get_market_resolution,
            get_market_trades, get_markets_home, get_related_markets, list_categories,
            list_events, list_markets, list_tags, pause_market, propose_market_resolution,
            publish_existing_event, publish_existing_event_markets, register_event_neg_risk,
            search_markets, set_market_prices, unpause_market, update_market,
        },
        market_auto_create::configure_market_auto_create_series,
        market_auto_resolution::configure_market_auto_resolve_coinbase,
    },
};

pub async fn markets_home(
    State(state): State<AppState>,
    Query(query): Query<MarketsHomeQuery>,
) -> Result<Json<MarketsHomeResponse>, AuthError> {
    Ok(Json(get_markets_home(&state, query).await?))
}

pub async fn markets_index(
    State(state): State<AppState>,
    Query(query): Query<ListMarketsQuery>,
) -> Result<Json<MarketListResponse>, AuthError> {
    Ok(Json(list_markets(&state, query).await?))
}

pub async fn markets_search(
    State(state): State<AppState>,
    Query(query): Query<SearchMarketsQuery>,
) -> Result<Json<MarketListResponse>, AuthError> {
    Ok(Json(search_markets(&state, query).await?))
}

pub async fn market_show(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketDetailResponse>, AuthError> {
    Ok(Json(get_market_by_id(&state, market_id).await?))
}

pub async fn market_show_by_slug(
    State(state): State<AppState>,
    Path(slug): Path<String>,
) -> Result<Json<MarketDetailResponse>, AuthError> {
    Ok(Json(get_market_by_slug(&state, slug).await?))
}

pub async fn market_show_by_condition(
    State(state): State<AppState>,
    Path(condition_id): Path<String>,
) -> Result<Json<MarketDetailResponse>, AuthError> {
    Ok(Json(get_market_by_condition_id(&state, condition_id).await?))
}

pub async fn market_outcomes(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketOutcomesResponse>, AuthError> {
    Ok(Json(get_market_outcomes(&state, market_id).await?))
}

pub async fn market_activity(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketActivityResponse>, AuthError> {
    Ok(Json(get_market_activity(&state, market_id).await?))
}

pub async fn market_quote(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketQuoteResponse>, AuthError> {
    Ok(Json(get_market_quote(&state, market_id).await?))
}

pub async fn market_price_history(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
    Query(query): Query<MarketPriceHistoryQuery>,
) -> Result<Json<MarketPriceHistoryResponse>, AuthError> {
    Ok(Json(get_market_price_history(&state, market_id, query).await?))
}

pub async fn market_orderbook(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketOrderbookResponse>, AuthError> {
    Ok(Json(get_market_orderbook(&state, market_id).await?))
}

pub async fn market_trades(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketTradesResponse>, AuthError> {
    Ok(Json(get_market_trades(&state, market_id).await?))
}

pub async fn market_liquidity(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketLiquidityResponse>, AuthError> {
    Ok(Json(get_market_liquidity(&state, market_id).await?))
}

pub async fn market_resolution(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketResolutionReadResponse>, AuthError> {
    Ok(Json(get_market_resolution(&state, market_id).await?))
}

pub async fn market_related(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<RelatedMarketsResponse>, AuthError> {
    Ok(Json(get_related_markets(&state, market_id).await?))
}

pub async fn events_index(
    State(state): State<AppState>,
    Query(query): Query<ListEventsQuery>,
) -> Result<Json<EventListResponse>, AuthError> {
    Ok(Json(list_events(&state, query).await?))
}

pub async fn event_show(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventDetailResponse>, AuthError> {
    Ok(Json(get_event_by_id(&state, event_id).await?))
}

pub async fn event_markets(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Query(query): Query<EventMarketsQuery>,
) -> Result<Json<EventMarketsResponse>, AuthError> {
    Ok(Json(get_event_markets(&state, event_id, query).await?))
}

pub async fn categories_index(
    State(state): State<AppState>,
) -> Result<Json<CategoriesResponse>, AuthError> {
    Ok(Json(list_categories(&state).await?))
}

pub async fn category_show(
    State(state): State<AppState>,
    Path(slug): Path<String>,
    Query(query): Query<CategoryMarketsQuery>,
) -> Result<Json<CategoryDetailResponse>, AuthError> {
    Ok(Json(get_category_by_slug(&state, slug, query).await?))
}

pub async fn tags_index(State(state): State<AppState>) -> Result<Json<TagsResponse>, AuthError> {
    Ok(Json(list_tags(&state).await?))
}

pub async fn admin_events_index(
    State(state): State<AppState>,
    Query(query): Query<AdminListEventsQuery>,
) -> Result<Json<AdminEventListResponse>, AuthError> {
    Ok(Json(admin_list_events(&state, query).await?))
}

pub async fn admin_event_show(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<AdminEventDetailResponse>, AuthError> {
    Ok(Json(admin_get_event_by_id(&state, event_id).await?))
}

pub async fn admin_event_markets(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Query(query): Query<AdminEventMarketsQuery>,
) -> Result<Json<AdminEventMarketsResponse>, AuthError> {
    Ok(Json(admin_get_event_markets(&state, event_id, query).await?))
}

pub async fn create_event_draft(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateEventRequest>,
) -> Result<(StatusCode, Json<CreateEventResponse>), AuthError> {
    Ok((
        StatusCode::CREATED,
        Json(create_event(&state, authenticated_user, payload).await?),
    ))
}

pub async fn create_market_draft(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<CreateMarketRequest>,
) -> Result<(StatusCode, Json<CreateMarketResponse>), AuthError> {
    Ok((
        StatusCode::CREATED,
        Json(create_market(&state, authenticated_user, payload).await?),
    ))
}

pub async fn create_event_markets_draft(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<CreateEventMarketsRequest>,
) -> Result<(StatusCode, Json<CreateEventMarketsResponse>), AuthError> {
    Ok((
        StatusCode::CREATED,
        Json(create_event_markets(&state, event_id, payload).await?),
    ))
}

pub async fn create_event_market_ladder_draft(
    State(state): State<AppState>,
    Extension(_authenticated_user): Extension<AuthenticatedUser>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<CreateEventMarketLadderRequest>,
) -> Result<(StatusCode, Json<CreateEventMarketsResponse>), AuthError> {
    Ok((
        StatusCode::CREATED,
        Json(create_event_market_ladder(&state, event_id, payload).await?),
    ))
}

pub async fn update_market_draft(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<UpdateMarketRequest>,
) -> Result<Json<UpdateMarketResponse>, AuthError> {
    Ok(Json(update_market(&state, market_id, payload).await?))
}

pub async fn set_market_prices_draft(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<SetMarketPricesRequest>,
) -> Result<Json<MarketPricesResponse>, AuthError> {
    Ok(Json(set_market_prices(&state, market_id, payload).await?))
}

pub async fn configure_market_auto_resolve_coinbase_draft(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<ConfigureMarketAutoResolveRequest>,
) -> Result<Json<MarketAutoResolveConfigResponse>, AuthError> {
    Ok(Json(
        configure_market_auto_resolve_coinbase(&state, market_id, payload).await?,
    ))
}

pub async fn configure_market_auto_create_series_draft(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Json(payload): Json<ConfigureMarketAutoCreateSeriesRequest>,
) -> Result<Json<MarketAutoCreateSeriesResponse>, AuthError> {
    Ok(Json(
        configure_market_auto_create_series(&state, authenticated_user, payload).await?,
    ))
}

pub async fn bootstrap_market_liquidity_draft(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<BootstrapMarketLiquidityRequest>,
) -> Result<Json<MarketLiquidityBootstrapResponse>, AuthError> {
    Ok(Json(bootstrap_market_liquidity(&state, market_id, payload).await?))
}

pub async fn bootstrap_event_liquidity_draft(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<BootstrapEventLiquidityRequest>,
) -> Result<Json<EventLiquidityBootstrapResponse>, AuthError> {
    Ok(Json(bootstrap_event_liquidity(&state, event_id, payload).await?))
}

pub async fn publish_event_shell(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventDetailResponse>, AuthError> {
    Ok(Json(publish_existing_event(&state, event_id).await?))
}

pub async fn publish_event_markets_batch(
    State(state): State<AppState>,
    Path(event_id): Path<Uuid>,
) -> Result<Json<EventMarketsResponse>, AuthError> {
    Ok(Json(publish_existing_event_markets(&state, event_id).await?))
}

pub async fn pause_market_draft(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketTradingStatusResponse>, AuthError> {
    Ok(Json(pause_market(&state, market_id).await?))
}

pub async fn unpause_market_draft(
    State(state): State<AppState>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketTradingStatusResponse>, AuthError> {
    Ok(Json(unpause_market(&state, market_id).await?))
}

pub async fn propose_market_resolution_draft(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<ProposeMarketResolutionRequest>,
) -> Result<Json<MarketResolutionWorkflowResponse>, AuthError> {
    Ok(Json(
        propose_market_resolution(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn dispute_market_resolution_draft(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<DisputeMarketResolutionRequest>,
) -> Result<Json<MarketResolutionWorkflowResponse>, AuthError> {
    Ok(Json(
        dispute_market_resolution(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn finalize_market_resolution_draft(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
) -> Result<Json<MarketResolutionWorkflowResponse>, AuthError> {
    Ok(Json(
        finalize_market_resolution(&state, authenticated_user, market_id).await?,
    ))
}

pub async fn emergency_market_resolution_draft(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(market_id): Path<Uuid>,
    Json(payload): Json<EmergencyMarketResolutionRequest>,
) -> Result<Json<MarketResolutionWorkflowResponse>, AuthError> {
    Ok(Json(
        emergency_market_resolution(&state, authenticated_user, market_id, payload).await?,
    ))
}

pub async fn register_event_neg_risk_draft(
    State(state): State<AppState>,
    Extension(authenticated_user): Extension<AuthenticatedUser>,
    Path(event_id): Path<Uuid>,
    Json(payload): Json<RegisterNegRiskEventRequest>,
) -> Result<Json<NegRiskRegistrationResponse>, AuthError> {
    Ok(Json(
        register_event_neg_risk(&state, authenticated_user, event_id, payload).await?,
    ))
}
