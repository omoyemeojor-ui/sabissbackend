use chrono::{DateTime, Utc};
use serde::{Deserialize, Deserializer, Serialize};
use uuid::Uuid;

use crate::module::market::model::{
    CategorySummaryRecord, MarketAutoCreateSeriesRecord, MarketAutoResolutionConfigRecord,
    MarketEventNegRiskConfigRecord, MarketEventRecord, MarketRecord, MarketResolutionRecord,
    PublicEventSummaryRecord, PublicMarketSummaryRecord, TagSummaryRecord,
};

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MarketsHomeQuery {
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ListMarketsQuery {
    pub category_slug: Option<String>,
    pub subcategory_slug: Option<String>,
    pub tag_slug: Option<String>,
    pub q: Option<String>,
    pub featured: Option<bool>,
    pub breaking: Option<bool>,
    pub trading_status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct SearchMarketsQuery {
    pub q: Option<String>,
    pub category_slug: Option<String>,
    pub subcategory_slug: Option<String>,
    pub tag_slug: Option<String>,
    pub trading_status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct MarketPriceHistoryQuery {
    pub interval: Option<String>,
    pub limit: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct ListEventsQuery {
    pub category_slug: Option<String>,
    pub subcategory_slug: Option<String>,
    pub tag_slug: Option<String>,
    pub featured: Option<bool>,
    pub breaking: Option<bool>,
    pub include_markets: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AdminListEventsQuery {
    pub publication_status: Option<String>,
    pub category_slug: Option<String>,
    pub subcategory_slug: Option<String>,
    pub tag_slug: Option<String>,
    pub featured: Option<bool>,
    pub breaking: Option<bool>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct AdminEventMarketsQuery {
    pub publication_status: Option<String>,
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct EventMarketsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Clone, Deserialize, Serialize, Default)]
pub struct CategoryMarketsQuery {
    pub limit: Option<i64>,
    pub offset: Option<i64>,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventRequest {
    pub event: CreateEventMetadataRequest,
    pub chain: CreateEventChainRequest,
    #[serde(default)]
    pub publish: CreateEventPublishRequest,
}

#[derive(Debug, Deserialize)]
pub struct CreateMarketRequest {
    pub market: CreateStandaloneMarketMetadataRequest,
    pub chain: CreateStandaloneMarketChainRequest,
    #[serde(default)]
    pub publish: CreateEventPublishRequest,
}

#[derive(Debug, Deserialize)]
pub struct UpdateMarketRequest {
    pub market: UpdateMarketFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct ProposeMarketResolutionRequest {
    pub resolution: ProposeMarketResolutionFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct DisputeMarketResolutionRequest {
    pub resolution: DisputeMarketResolutionFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct EmergencyMarketResolutionRequest {
    pub resolution: EmergencyMarketResolutionFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct RegisterNegRiskEventRequest {
    pub neg_risk: RegisterNegRiskEventFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct SetMarketPricesRequest {
    pub prices: SetMarketPricesFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapMarketLiquidityRequest {
    pub liquidity: BootstrapMarketLiquidityFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapEventLiquidityRequest {
    pub liquidity: BootstrapEventLiquidityFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureMarketAutoResolveRequest {
    pub auto_resolve: ConfigureCoinbaseAutoResolveFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureMarketAutoCreateSeriesRequest {
    pub series: ConfigureCoinbaseAutoCreateSeriesFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventMetadataRequest {
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    #[serde(default)]
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub rules: String,
    pub context: Option<String>,
    pub additional_context: Option<String>,
    #[serde(default)]
    pub resolution_sources: Vec<String>,
    #[serde(default = "default_resolution_timezone")]
    pub resolution_timezone: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub sort_at: Option<DateTime<Utc>>,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub breaking: bool,
    #[serde(default = "default_true")]
    pub searchable: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub hide_resolved_by_default: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateStandaloneMarketMetadataRequest {
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    #[serde(default)]
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub rules: String,
    pub context: Option<String>,
    pub additional_context: Option<String>,
    #[serde(default)]
    pub resolution_sources: Vec<String>,
    #[serde(default = "default_resolution_timezone")]
    pub resolution_timezone: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub sort_at: Option<DateTime<Utc>>,
    pub end_time: DateTime<Utc>,
    #[serde(default = "default_binary_outcomes")]
    pub outcomes: Vec<String>,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub breaking: bool,
    #[serde(default = "default_true")]
    pub searchable: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub hide_resolved_by_default: bool,
}

#[derive(Debug, Deserialize, Default)]
pub struct UpdateMarketFieldsRequest {
    pub slug: Option<String>,
    pub label: Option<String>,
    pub question: Option<String>,
    pub end_time: Option<DateTime<Utc>>,
    pub outcomes: Option<Vec<String>>,
    pub sort_order: Option<i32>,
    pub oracle_address: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct ProposeMarketResolutionFieldsRequest {
    pub winning_outcome: i32,
    pub notes: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct DisputeMarketResolutionFieldsRequest {
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct EmergencyMarketResolutionFieldsRequest {
    pub winning_outcome: i32,
    pub reason: String,
}

#[derive(Debug, Deserialize)]
pub struct RegisterNegRiskEventFieldsRequest {
    pub other_market_id: Option<Uuid>,
}

#[derive(Debug, Deserialize)]
pub struct SetMarketPricesFieldsRequest {
    pub yes_bps: u32,
    pub no_bps: u32,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapMarketLiquidityFieldsRequest {
    pub yes_bps: u32,
    pub no_bps: u32,
    pub inventory_usdc_amount: String,
    pub exit_collateral_usdc_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapEventLiquidityFieldsRequest {
    pub markets: Vec<BootstrapEventLiquidityMarketRequest>,
}

#[derive(Debug, Deserialize)]
pub struct BootstrapEventLiquidityMarketRequest {
    pub market_id: Uuid,
    pub yes_bps: u32,
    pub no_bps: u32,
    pub inventory_usdc_amount: String,
    pub exit_collateral_usdc_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureCoinbaseAutoResolveFieldsRequest {
    pub product_id: String,
    pub start_time: Option<DateTime<Utc>>,
    #[serde(default = "default_zero_i32")]
    pub up_outcome_index: i32,
    #[serde(default = "default_one_i32")]
    pub down_outcome_index: i32,
    #[serde(default = "default_zero_i32")]
    pub tie_outcome_index: i32,
}

#[derive(Debug, Deserialize)]
pub struct ConfigureCoinbaseAutoCreateSeriesFieldsRequest {
    pub product_id: String,
    pub title_prefix: String,
    pub slug_prefix: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    #[serde(default)]
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub rules: String,
    pub context: Option<String>,
    pub additional_context: Option<String>,
    #[serde(default)]
    pub resolution_sources: Vec<String>,
    #[serde(default = "default_resolution_timezone")]
    pub resolution_timezone: String,
    pub start_time: DateTime<Utc>,
    pub cadence_seconds: i32,
    pub market_duration_seconds: i32,
    pub oracle_address: String,
    #[serde(default = "default_binary_outcomes")]
    pub outcomes: Vec<String>,
    #[serde(default = "default_zero_i32")]
    pub up_outcome_index: i32,
    #[serde(default = "default_one_i32")]
    pub down_outcome_index: i32,
    #[serde(default = "default_zero_i32")]
    pub tie_outcome_index: i32,
    #[serde(default)]
    pub featured: bool,
    #[serde(default)]
    pub breaking: bool,
    #[serde(default = "default_true")]
    pub searchable: bool,
    #[serde(default = "default_true")]
    pub visible: bool,
    #[serde(default)]
    pub hide_resolved_by_default: bool,
    #[serde(default = "default_true")]
    pub active: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventChainRequest {
    #[serde(default)]
    pub neg_risk: bool,
    pub group_key: String,
    pub series_key: String,
}

#[derive(Debug, Deserialize)]
pub struct CreateStandaloneMarketChainRequest {
    pub oracle_address: String,
    #[serde(default)]
    pub neg_risk: bool,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventMarketsRequest {
    pub markets: Vec<CreateEventMarketRequest>,
    #[serde(default)]
    pub publish: CreateEventPublishRequest,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventMarketLadderRequest {
    pub template: CreatePriceLadderTemplateRequest,
    #[serde(default)]
    pub publish: CreateEventPublishRequest,
}

#[derive(Debug, Deserialize)]
pub struct CreateEventMarketRequest {
    pub label: String,
    pub slug: String,
    pub question: String,
    pub end_time: DateTime<Utc>,
    #[serde(default = "default_binary_outcomes")]
    pub outcomes: Vec<String>,
    pub sort_order: Option<i32>,
    pub oracle_address: String,
}

#[derive(Debug, Deserialize)]
pub struct CreatePriceLadderTemplateRequest {
    pub underlying: String,
    pub deadline_label: String,
    pub end_time: DateTime<Utc>,
    pub oracle_address: String,
    #[serde(default = "default_price_ladder_unit_symbol")]
    pub unit_symbol: String,
    #[serde(default, deserialize_with = "deserialize_string_list_or_csv")]
    pub up_thresholds: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_list_or_csv")]
    pub down_thresholds: Vec<String>,
}

#[derive(Debug, Deserialize, Default)]
pub struct CreateEventPublishRequest {
    #[serde(default)]
    pub mode: CreateEventPublishMode,
}

#[derive(Debug, Deserialize, Serialize, Clone, Copy, Default)]
#[serde(rename_all = "snake_case")]
pub enum CreateEventPublishMode {
    #[default]
    Draft,
    Prepare,
    Publish,
}

#[derive(Debug, Serialize)]
pub struct CreateEventResponse {
    pub id: Uuid,
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct CreateEventMarketsResponse {
    pub event_id: Uuid,
    pub event_slug: String,
    pub markets: Vec<MarketResponse>,
}

#[derive(Debug, Serialize)]
pub struct CreateMarketResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct UpdateMarketResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketTradingStatusResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketResolutionWorkflowResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub resolution: MarketResolutionStateResponse,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketAutoResolveConfigResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub auto_resolve: MarketAutoResolveConfigStateResponse,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketAutoCreateSeriesResponse {
    pub series: MarketAutoCreateSeriesStateResponse,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct NegRiskRegistrationResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub neg_risk: NegRiskRegistrationStateResponse,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketPricesResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub prices: MarketPricesStateResponse,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketLiquidityBootstrapResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub bootstrap: MarketLiquidityBootstrapStateResponse,
    pub liquidity: MarketLiquidityResponse,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct EventLiquidityBootstrapResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub results: Vec<EventLiquidityBootstrapItemResponse>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketsHomeResponse {
    pub featured: Vec<PublicMarketCardResponse>,
    pub breaking: Vec<PublicMarketCardResponse>,
    pub newest: Vec<PublicMarketCardResponse>,
}

#[derive(Debug, Serialize)]
pub struct MarketListResponse {
    pub markets: Vec<PublicMarketCardResponse>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct MarketDetailResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub resolution: Option<MarketResolutionStateResponse>,
    pub sibling_markets: Vec<MarketResponse>,
}

#[derive(Debug, Serialize)]
pub struct MarketOutcomesResponse {
    pub market_id: Uuid,
    pub condition_id: Option<String>,
    pub market_type: String,
    pub outcomes: Vec<MarketOutcomeResponse>,
}

#[derive(Debug, Serialize)]
pub struct MarketActivityResponse {
    pub market_id: Uuid,
    pub source: String,
    pub items: Vec<MarketActivityItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct MarketPriceHistoryResponse {
    pub market_id: Uuid,
    pub condition_id: Option<String>,
    pub source: String,
    pub interval: String,
    pub history: Vec<CompactMarketPriceHistoryPointResponse>,
    pub points: Vec<MarketPriceHistoryPointResponse>,
}

#[derive(Debug, Serialize)]
pub struct MarketOrderbookResponse {
    pub market_id: Uuid,
    pub condition_id: Option<String>,
    pub source: String,
    pub as_of: DateTime<Utc>,
    pub spread_bps: u32,
    pub last_trade_yes_bps: u32,
    pub bids: Vec<OrderbookLevelResponse>,
    pub asks: Vec<OrderbookLevelResponse>,
}

#[derive(Debug, Serialize)]
pub struct MarketLiquidityResponse {
    pub market_id: Uuid,
    pub condition_id: Option<String>,
    pub source: String,
    pub exchange_outcomes: Vec<MarketLiquidityOutcomeResponse>,
    pub pool: PoolLiquidityResponse,
}

#[derive(Debug, Serialize)]
pub struct MarketResolutionReadResponse {
    pub market_id: Uuid,
    pub resolution: Option<MarketResolutionStateResponse>,
}

#[derive(Debug, Serialize)]
pub struct MarketTradesResponse {
    pub market_id: Uuid,
    pub condition_id: Option<String>,
    pub source: String,
    pub trades: Vec<MarketTradeFillResponse>,
}

#[derive(Debug, Serialize)]
pub struct RelatedMarketsResponse {
    pub market_id: Uuid,
    pub related: Vec<PublicMarketCardResponse>,
}

#[derive(Debug, Serialize)]
pub struct EventListResponse {
    pub events: Vec<PublicEventCardResponse>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct EventDetailResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub markets_count: i64,
}

#[derive(Debug, Serialize)]
pub struct EventMarketsResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub markets: Vec<MarketResponse>,
}

#[derive(Debug, Serialize)]
pub struct AdminEventListResponse {
    pub events: Vec<AdminEventCardResponse>,
    pub limit: i64,
    pub offset: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminEventDetailResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub markets_count: i64,
}

#[derive(Debug, Serialize)]
pub struct AdminEventMarketsResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub markets: Vec<MarketResponse>,
}

#[derive(Debug, Serialize)]
pub struct CategoriesResponse {
    pub categories: Vec<CategorySummaryResponse>,
}

#[derive(Debug, Serialize)]
pub struct CategoryDetailResponse {
    pub category: CategorySummaryResponse,
    pub markets: Vec<PublicMarketCardResponse>,
}

#[derive(Debug, Serialize)]
pub struct TagsResponse {
    pub tags: Vec<TagSummaryResponse>,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventResponse {
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub rules: String,
    pub context: Option<String>,
    pub additional_context: Option<String>,
    pub resolution_sources: Vec<String>,
    pub resolution_timezone: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub sort_at: Option<DateTime<Utc>>,
    pub featured: bool,
    pub breaking: bool,
    pub searchable: bool,
    pub visible: bool,
    pub hide_resolved_by_default: bool,
    pub publication_status: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct EventOnChainResponse {
    pub event_id: String,
    pub group_id: String,
    pub series_id: String,
    pub neg_risk: bool,
    pub tx_hash: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketResponse {
    pub id: Uuid,
    pub slug: String,
    pub label: String,
    pub question: String,
    pub question_id: String,
    pub condition_id: Option<String>,
    pub market_type: String,
    pub outcomes: Vec<String>,
    pub end_time: DateTime<Utc>,
    pub sort_order: i32,
    pub publication_status: String,
    pub trading_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_prices: Option<MarketCurrentPricesResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<MarketStatsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_summary: Option<MarketQuoteSummaryResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_trade_yes_bps: Option<u32>,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketCurrentPricesResponse {
    pub yes_bps: u32,
    pub no_bps: u32,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketStatsResponse {
    pub volume_usd: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct MarketQuoteSummaryResponse {
    pub buy_yes_bps: u32,
    pub buy_no_bps: u32,
    pub as_of: DateTime<Utc>,
    pub source: String,
}

#[derive(Debug, Serialize)]
pub struct MarketQuoteResponse {
    pub market_id: Uuid,
    pub condition_id: Option<String>,
    pub source: String,
    pub as_of: DateTime<Utc>,
    pub buy_yes_bps: u32,
    pub buy_no_bps: u32,
    pub sell_yes_bps: u32,
    pub sell_no_bps: u32,
    pub last_trade_yes_bps: u32,
    pub spread_bps: u32,
}

#[derive(Debug, Serialize)]
pub struct MarketResolutionStateResponse {
    pub status: String,
    pub proposed_winning_outcome: i32,
    pub final_winning_outcome: Option<i32>,
    pub payout_vector_hash: String,
    pub proposed_by_user_id: Uuid,
    pub proposed_at: DateTime<Utc>,
    pub dispute_deadline: DateTime<Utc>,
    pub notes: Option<String>,
    pub disputed_by_user_id: Option<Uuid>,
    pub disputed_at: Option<DateTime<Utc>>,
    pub dispute_reason: Option<String>,
    pub finalized_by_user_id: Option<Uuid>,
    pub finalized_at: Option<DateTime<Utc>>,
    pub emergency_resolved_by_user_id: Option<Uuid>,
    pub emergency_resolved_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct MarketAutoResolveConfigStateResponse {
    pub provider: String,
    pub product_id: String,
    pub start_time: DateTime<Utc>,
    pub start_price: Option<String>,
    pub start_price_captured_at: Option<DateTime<Utc>>,
    pub end_price: Option<String>,
    pub end_price_captured_at: Option<DateTime<Utc>>,
    pub up_outcome_index: i32,
    pub down_outcome_index: i32,
    pub tie_outcome_index: i32,
    pub last_error: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MarketAutoCreateSeriesStateResponse {
    pub id: Uuid,
    pub provider: String,
    pub product_id: String,
    pub title_prefix: String,
    pub slug_prefix: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub rules: String,
    pub context: Option<String>,
    pub additional_context: Option<String>,
    pub resolution_sources: Vec<String>,
    pub resolution_timezone: String,
    pub start_time: DateTime<Utc>,
    pub cadence_seconds: i32,
    pub market_duration_seconds: i32,
    pub oracle_address: String,
    pub outcomes: Vec<String>,
    pub up_outcome_index: i32,
    pub down_outcome_index: i32,
    pub tie_outcome_index: i32,
    pub featured: bool,
    pub breaking: bool,
    pub searchable: bool,
    pub visible: bool,
    pub hide_resolved_by_default: bool,
    pub active: bool,
    pub last_created_slot_start: Option<DateTime<Utc>>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct NegRiskRegistrationStateResponse {
    pub registered: bool,
    pub has_other: bool,
    pub other_market_id: Option<Uuid>,
    pub other_condition_id: Option<String>,
    pub tx_hash: Option<String>,
    pub registered_by_user_id: Uuid,
    pub registered_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketPricesStateResponse {
    pub yes_bps: u32,
    pub no_bps: u32,
    pub tx_hashes: MarketPriceTxHashesResponse,
}

#[derive(Debug, Serialize)]
pub struct MarketPriceTxHashesResponse {
    pub yes_price: String,
    pub no_price: String,
}

#[derive(Debug, Serialize)]
pub struct MarketLiquidityBootstrapStateResponse {
    pub yes_bps: u32,
    pub no_bps: u32,
    pub inventory_usdc_amount: String,
    pub exit_collateral_usdc_amount: String,
    pub tx_hashes: MarketLiquidityBootstrapTxHashesResponse,
}

#[derive(Debug, Serialize)]
pub struct MarketLiquidityBootstrapTxHashesResponse {
    pub yes_price: String,
    pub no_price: String,
    pub split_and_add_liquidity: String,
    pub deposit_collateral: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct EventLiquidityBootstrapItemResponse {
    pub market: MarketResponse,
    pub bootstrap: MarketLiquidityBootstrapStateResponse,
    pub liquidity: MarketLiquidityResponse,
}

#[derive(Debug, Serialize)]
pub struct PublicMarketCardResponse {
    pub id: Uuid,
    pub slug: String,
    pub label: String,
    pub question: String,
    pub question_id: String,
    pub condition_id: Option<String>,
    pub market_type: String,
    pub outcomes: Vec<String>,
    pub end_time: DateTime<Utc>,
    pub sort_order: i32,
    pub trading_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub current_prices: Option<MarketCurrentPricesResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub stats: Option<MarketStatsResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub quote_summary: Option<MarketQuoteSummaryResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_trade_yes_bps: Option<u32>,
    pub event: PublicEventTeaserResponse,
}

#[derive(Debug, Serialize)]
pub struct PublicEventTeaserResponse {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub featured: bool,
    pub breaking: bool,
    pub neg_risk: bool,
}

#[derive(Debug, Serialize)]
pub struct MarketOutcomeResponse {
    pub index: i32,
    pub label: String,
    pub is_winning: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct MarketActivityItemResponse {
    pub activity_type: String,
    pub occurred_at: DateTime<Utc>,
    pub actor_user_id: Option<Uuid>,
    pub details: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct MarketTradeFillResponse {
    pub id: Uuid,
    pub match_type: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub outcome_index: Option<i32>,
    pub fill_token_amount: String,
    pub collateral_amount: String,
    pub yes_price_bps: u32,
    pub no_price_bps: u32,
    pub yes_price: f64,
    pub no_price: f64,
    pub tx_hash: String,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MarketPriceHistoryPointResponse {
    pub timestamp: DateTime<Utc>,
    pub outcome_index: i32,
    pub outcome_label: String,
    pub price_bps: u32,
    pub price: f64,
}

#[derive(Debug, Serialize)]
pub struct CompactMarketPriceHistoryPointResponse {
    pub t: i64,
    pub p: f64,
}

#[derive(Debug, Serialize)]
pub struct OrderbookLevelResponse {
    pub outcome_index: i32,
    pub outcome_label: String,
    pub price_bps: u32,
    pub price: f64,
    pub quantity: f64,
    pub shares: String,
    pub notional_usd: String,
}

#[derive(Debug, Serialize)]
pub struct MarketLiquidityOutcomeResponse {
    pub outcome_index: i32,
    pub outcome_label: String,
    pub available: String,
}

#[derive(Debug, Serialize)]
pub struct PoolLiquidityResponse {
    pub idle_yes_total: String,
    pub idle_no_total: String,
    pub posted_yes_total: String,
    pub posted_no_total: String,
    pub claimable_collateral_total: String,
}

#[derive(Debug, Serialize)]
pub struct PublicEventCardResponse {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub featured: bool,
    pub breaking: bool,
    pub neg_risk: bool,
    pub starts_at: Option<DateTime<Utc>>,
    pub sort_at: Option<DateTime<Utc>>,
    pub market_count: i64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub markets: Option<Vec<PublicMarketCardResponse>>,
}

#[derive(Debug, Serialize)]
pub struct AdminEventCardResponse {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary: Option<String>,
    pub featured: bool,
    pub breaking: bool,
    pub neg_risk: bool,
    pub publication_status: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub sort_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub market_count: i64,
}

#[derive(Debug, Serialize)]
pub struct CategorySummaryResponse {
    pub slug: String,
    pub label: String,
    pub event_count: i64,
    pub market_count: i64,
    pub featured_event_count: i64,
    pub breaking_event_count: i64,
}

#[derive(Debug, Serialize)]
pub struct TagSummaryResponse {
    pub slug: String,
    pub label: String,
    pub event_count: i64,
    pub market_count: i64,
}

impl CreateEventResponse {
    pub fn from_record(event: MarketEventRecord) -> Self {
        Self {
            id: event.id,
            created_at: event.created_at,
            event: EventResponse::from(&event),
            on_chain: EventOnChainResponse::from(&event),
        }
    }
}

impl CreateEventMarketsResponse {
    pub fn from_records(event: &MarketEventRecord, markets: &[MarketRecord]) -> Self {
        Self {
            event_id: event.id,
            event_slug: event.slug.clone(),
            markets: markets.iter().map(MarketResponse::from).collect(),
        }
    }
}

impl CreateMarketResponse {
    pub fn from_records(event: &MarketEventRecord, market: &MarketRecord) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            market: MarketResponse::from(market),
            created_at: market.created_at,
        }
    }
}

impl UpdateMarketResponse {
    pub fn from_records(event: &MarketEventRecord, market: &MarketRecord) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            market: MarketResponse::from(market),
            updated_at: market.updated_at,
        }
    }
}

impl MarketTradingStatusResponse {
    pub fn from_records(event: &MarketEventRecord, market: &MarketRecord) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            market: MarketResponse::from(market),
            updated_at: market.updated_at,
        }
    }
}

impl MarketResolutionWorkflowResponse {
    pub fn from_records(
        event: &MarketEventRecord,
        market: &MarketRecord,
        resolution: &MarketResolutionRecord,
    ) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            market: MarketResponse::from(market),
            resolution: MarketResolutionStateResponse::from(resolution),
            updated_at: resolution.updated_at,
        }
    }
}

impl MarketAutoResolveConfigResponse {
    pub fn from_records(
        event: &MarketEventRecord,
        market: &MarketRecord,
        config: &MarketAutoResolutionConfigRecord,
    ) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            market: MarketResponse::from(market),
            auto_resolve: MarketAutoResolveConfigStateResponse::from(config),
            updated_at: config.updated_at,
        }
    }
}

impl MarketAutoCreateSeriesResponse {
    pub fn from_record(series: &MarketAutoCreateSeriesRecord) -> Self {
        Self {
            series: MarketAutoCreateSeriesStateResponse::from(series),
            updated_at: series.updated_at,
        }
    }
}

impl NegRiskRegistrationResponse {
    pub fn from_records(
        event: &MarketEventRecord,
        neg_risk: &MarketEventNegRiskConfigRecord,
        tx_hash: Option<String>,
    ) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            neg_risk: NegRiskRegistrationStateResponse::from_record(neg_risk, tx_hash),
            updated_at: neg_risk.updated_at,
        }
    }
}

impl MarketPricesResponse {
    pub fn from_records(
        event: &MarketEventRecord,
        market: &MarketRecord,
        prices: MarketPricesStateResponse,
    ) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            market: MarketResponse::from(market),
            prices,
            updated_at: Utc::now(),
        }
    }
}

impl MarketLiquidityBootstrapResponse {
    pub fn from_records(
        event: &MarketEventRecord,
        market: &MarketRecord,
        bootstrap: MarketLiquidityBootstrapStateResponse,
        liquidity: MarketLiquidityResponse,
    ) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            market: MarketResponse::from(market),
            bootstrap,
            liquidity,
            updated_at: Utc::now(),
        }
    }
}

impl EventLiquidityBootstrapResponse {
    pub fn from_records(
        event: &MarketEventRecord,
        results: Vec<EventLiquidityBootstrapItemResponse>,
    ) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            results,
            updated_at: Utc::now(),
        }
    }
}

impl MarketsHomeResponse {
    pub fn new(
        featured: Vec<PublicMarketCardResponse>,
        breaking: Vec<PublicMarketCardResponse>,
        newest: Vec<PublicMarketCardResponse>,
    ) -> Self {
        Self {
            featured,
            breaking,
            newest,
        }
    }
}

impl MarketListResponse {
    pub fn new(markets: Vec<PublicMarketCardResponse>, limit: i64, offset: i64) -> Self {
        Self {
            markets,
            limit,
            offset,
        }
    }
}

impl MarketDetailResponse {
    pub fn from_records(
        event: &MarketEventRecord,
        market: &MarketRecord,
        resolution: Option<&MarketResolutionRecord>,
        sibling_markets: &[MarketRecord],
    ) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            market: MarketResponse::from(market),
            resolution: resolution.map(MarketResolutionStateResponse::from),
            sibling_markets: sibling_markets.iter().map(MarketResponse::from).collect(),
        }
    }
}

impl MarketOutcomesResponse {
    pub fn from_records(
        market: &MarketRecord,
        resolution: Option<&MarketResolutionRecord>,
    ) -> Self {
        let final_winning_outcome = resolution.and_then(|value| value.final_winning_outcome);

        let outcomes = market
            .outcomes
            .iter()
            .enumerate()
            .map(|(index, label)| MarketOutcomeResponse {
                index: index as i32,
                label: label.clone(),
                is_winning: final_winning_outcome.map(|winning| winning == index as i32),
            })
            .collect();

        Self {
            market_id: market.id,
            condition_id: market.condition_id.clone(),
            market_type: market.market_type.clone(),
            outcomes,
        }
    }
}

impl MarketActivityResponse {
    pub fn new(market_id: Uuid, items: Vec<MarketActivityItemResponse>) -> Self {
        Self {
            market_id,
            source: "lifecycle_only".to_owned(),
            items,
        }
    }
}

impl MarketPriceHistoryResponse {
    pub fn from_points(
        market_id: Uuid,
        condition_id: Option<String>,
        source: impl Into<String>,
        interval: impl Into<String>,
        points: Vec<MarketPriceHistoryPointResponse>,
    ) -> Self {
        let history = points
            .iter()
            .filter(|point| point.outcome_index == 0)
            .map(|point| CompactMarketPriceHistoryPointResponse {
                t: point.timestamp.timestamp(),
                p: point.price,
            })
            .collect();

        Self {
            market_id,
            condition_id,
            source: source.into(),
            interval: interval.into(),
            history,
            points,
        }
    }

    pub fn empty(market: &MarketRecord, interval: String) -> Self {
        Self {
            market_id: market.id,
            condition_id: market.condition_id.clone(),
            source: "not_indexed_yet".to_owned(),
            interval,
            history: Vec::new(),
            points: Vec::new(),
        }
    }
}

impl MarketOrderbookResponse {
    pub fn empty(market: &MarketRecord) -> Self {
        Self {
            market_id: market.id,
            condition_id: market.condition_id.clone(),
            source: "not_indexed_yet".to_owned(),
            as_of: Utc::now(),
            spread_bps: 0,
            last_trade_yes_bps: 0,
            bids: Vec::new(),
            asks: Vec::new(),
        }
    }
}

impl MarketLiquidityResponse {
    pub fn new(
        market: &MarketRecord,
        exchange_outcomes: Vec<MarketLiquidityOutcomeResponse>,
        pool: PoolLiquidityResponse,
    ) -> Self {
        Self {
            market_id: market.id,
            condition_id: market.condition_id.clone(),
            source: "on_chain".to_owned(),
            exchange_outcomes,
            pool,
        }
    }
}

impl MarketResolutionReadResponse {
    pub fn new(market_id: Uuid, resolution: Option<&MarketResolutionRecord>) -> Self {
        Self {
            market_id,
            resolution: resolution.map(MarketResolutionStateResponse::from),
        }
    }
}

impl RelatedMarketsResponse {
    pub fn new(market_id: Uuid, related: Vec<PublicMarketCardResponse>) -> Self {
        Self { market_id, related }
    }
}

impl EventListResponse {
    pub fn new(events: Vec<PublicEventSummaryRecord>, limit: i64, offset: i64) -> Self {
        Self {
            events: events.iter().map(PublicEventCardResponse::from).collect(),
            limit,
            offset,
        }
    }
}

impl EventDetailResponse {
    pub fn from_records(event: &MarketEventRecord, markets_count: i64) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            markets_count,
        }
    }
}

impl EventMarketsResponse {
    pub fn from_records(event: &MarketEventRecord, markets: &[MarketRecord]) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            markets: markets.iter().map(MarketResponse::from).collect(),
        }
    }
}

impl AdminEventListResponse {
    pub fn new(events: Vec<PublicEventSummaryRecord>, limit: i64, offset: i64) -> Self {
        Self {
            events: events.iter().map(AdminEventCardResponse::from).collect(),
            limit,
            offset,
        }
    }
}

impl AdminEventDetailResponse {
    pub fn from_records(event: &MarketEventRecord, markets_count: i64) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            markets_count,
        }
    }
}

impl AdminEventMarketsResponse {
    pub fn from_records(event: &MarketEventRecord, markets: &[MarketRecord]) -> Self {
        Self {
            event: EventResponse::from(event),
            on_chain: EventOnChainResponse::from(event),
            markets: markets.iter().map(MarketResponse::from).collect(),
        }
    }
}

impl CategoriesResponse {
    pub fn new(categories: Vec<CategorySummaryRecord>) -> Self {
        Self {
            categories: categories
                .iter()
                .map(CategorySummaryResponse::from)
                .collect(),
        }
    }
}

impl CategoryDetailResponse {
    pub fn new(category: &CategorySummaryRecord, markets: Vec<PublicMarketCardResponse>) -> Self {
        Self {
            category: CategorySummaryResponse::from(category),
            markets,
        }
    }
}

impl TagsResponse {
    pub fn new(tags: Vec<TagSummaryRecord>) -> Self {
        Self {
            tags: tags.iter().map(TagSummaryResponse::from).collect(),
        }
    }
}

impl From<&MarketEventRecord> for EventResponse {
    fn from(value: &MarketEventRecord) -> Self {
        Self {
            title: value.title.clone(),
            slug: value.slug.clone(),
            category_slug: value.category_slug.clone(),
            subcategory_slug: value.subcategory_slug.clone(),
            tag_slugs: value.tag_slugs.clone(),
            image_url: value.image_url.clone(),
            summary: value.summary_text.clone(),
            rules: value.rules_text.clone(),
            context: value.context_text.clone(),
            additional_context: value.additional_context.clone(),
            resolution_sources: value.resolution_sources.clone(),
            resolution_timezone: value.resolution_timezone.clone(),
            starts_at: value.starts_at,
            sort_at: value.sort_at,
            featured: value.featured,
            breaking: value.breaking,
            searchable: value.searchable,
            visible: value.visible,
            hide_resolved_by_default: value.hide_resolved_by_default,
            publication_status: value.publication_status.clone(),
        }
    }
}

impl From<&MarketEventRecord> for EventOnChainResponse {
    fn from(value: &MarketEventRecord) -> Self {
        Self {
            event_id: value.event_id.clone(),
            group_id: value.group_id.clone(),
            series_id: value.series_id.clone(),
            neg_risk: value.neg_risk,
            tx_hash: value.published_tx_hash.clone(),
        }
    }
}

impl From<&MarketRecord> for MarketResponse {
    fn from(value: &MarketRecord) -> Self {
        Self {
            id: value.id,
            slug: value.slug.clone(),
            label: value.label.clone(),
            question: value.question.clone(),
            question_id: value.question_id.clone(),
            condition_id: value.condition_id.clone(),
            market_type: value.market_type.clone(),
            outcomes: value.outcomes.clone(),
            end_time: value.end_time,
            sort_order: value.sort_order,
            publication_status: value.publication_status.clone(),
            trading_status: value.trading_status.clone(),
            current_prices: None,
            stats: None,
            quote_summary: None,
            last_trade_yes_bps: None,
        }
    }
}

impl From<&MarketResolutionRecord> for MarketResolutionStateResponse {
    fn from(value: &MarketResolutionRecord) -> Self {
        Self {
            status: value.status.clone(),
            proposed_winning_outcome: value.proposed_winning_outcome,
            final_winning_outcome: value.final_winning_outcome,
            payout_vector_hash: value.payout_vector_hash.clone(),
            proposed_by_user_id: value.proposed_by_user_id,
            proposed_at: value.proposed_at,
            dispute_deadline: value.dispute_deadline,
            notes: value.notes.clone(),
            disputed_by_user_id: value.disputed_by_user_id,
            disputed_at: value.disputed_at,
            dispute_reason: value.dispute_reason.clone(),
            finalized_by_user_id: value.finalized_by_user_id,
            finalized_at: value.finalized_at,
            emergency_resolved_by_user_id: value.emergency_resolved_by_user_id,
            emergency_resolved_at: value.emergency_resolved_at,
        }
    }
}

impl From<&MarketAutoResolutionConfigRecord> for MarketAutoResolveConfigStateResponse {
    fn from(value: &MarketAutoResolutionConfigRecord) -> Self {
        Self {
            provider: value.provider.clone(),
            product_id: value.product_id.clone(),
            start_time: value.start_time,
            start_price: value.start_price.clone(),
            start_price_captured_at: value.start_price_captured_at,
            end_price: value.end_price.clone(),
            end_price_captured_at: value.end_price_captured_at,
            up_outcome_index: value.up_outcome_index,
            down_outcome_index: value.down_outcome_index,
            tie_outcome_index: value.tie_outcome_index,
            last_error: value.last_error.clone(),
        }
    }
}

impl From<&MarketAutoCreateSeriesRecord> for MarketAutoCreateSeriesStateResponse {
    fn from(value: &MarketAutoCreateSeriesRecord) -> Self {
        Self {
            id: value.id,
            provider: value.provider.clone(),
            product_id: value.product_id.clone(),
            title_prefix: value.title_prefix.clone(),
            slug_prefix: value.slug_prefix.clone(),
            category_slug: value.category_slug.clone(),
            subcategory_slug: value.subcategory_slug.clone(),
            tag_slugs: value.tag_slugs.clone(),
            image_url: value.image_url.clone(),
            summary: value.summary_text.clone(),
            rules: value.rules_text.clone(),
            context: value.context_text.clone(),
            additional_context: value.additional_context.clone(),
            resolution_sources: value.resolution_sources.clone(),
            resolution_timezone: value.resolution_timezone.clone(),
            start_time: value.start_time,
            cadence_seconds: value.cadence_seconds,
            market_duration_seconds: value.market_duration_seconds,
            oracle_address: value.oracle_address.clone(),
            outcomes: value.outcomes.clone(),
            up_outcome_index: value.up_outcome_index,
            down_outcome_index: value.down_outcome_index,
            tie_outcome_index: value.tie_outcome_index,
            featured: value.featured,
            breaking: value.breaking,
            searchable: value.searchable,
            visible: value.visible,
            hide_resolved_by_default: value.hide_resolved_by_default,
            active: value.active,
            last_created_slot_start: value.last_created_slot_start,
            created_by_user_id: value.created_by_user_id,
            created_at: value.created_at,
        }
    }
}

impl NegRiskRegistrationStateResponse {
    fn from_record(value: &MarketEventNegRiskConfigRecord, tx_hash: Option<String>) -> Self {
        Self {
            registered: value.registered,
            has_other: value.has_other,
            other_market_id: value.other_market_id,
            other_condition_id: value.other_condition_id.clone(),
            tx_hash,
            registered_by_user_id: value.registered_by_user_id,
            registered_at: value.registered_at,
        }
    }
}

impl MarketPricesStateResponse {
    pub fn new(
        yes_bps: u32,
        no_bps: u32,
        yes_price_tx_hash: String,
        no_price_tx_hash: String,
    ) -> Self {
        Self {
            yes_bps,
            no_bps,
            tx_hashes: MarketPriceTxHashesResponse {
                yes_price: yes_price_tx_hash,
                no_price: no_price_tx_hash,
            },
        }
    }
}

impl MarketLiquidityBootstrapStateResponse {
    pub fn new(
        yes_bps: u32,
        no_bps: u32,
        inventory_usdc_amount: String,
        exit_collateral_usdc_amount: String,
        yes_price_tx_hash: String,
        no_price_tx_hash: String,
        split_and_add_liquidity_tx_hash: String,
        deposit_collateral_tx_hash: Option<String>,
    ) -> Self {
        Self {
            yes_bps,
            no_bps,
            inventory_usdc_amount,
            exit_collateral_usdc_amount,
            tx_hashes: MarketLiquidityBootstrapTxHashesResponse {
                yes_price: yes_price_tx_hash,
                no_price: no_price_tx_hash,
                split_and_add_liquidity: split_and_add_liquidity_tx_hash,
                deposit_collateral: deposit_collateral_tx_hash,
            },
        }
    }
}

impl EventLiquidityBootstrapItemResponse {
    pub fn new(
        market: &MarketRecord,
        bootstrap: MarketLiquidityBootstrapStateResponse,
        liquidity: MarketLiquidityResponse,
    ) -> Self {
        Self {
            market: MarketResponse::from(market),
            bootstrap,
            liquidity,
        }
    }
}

impl From<&PublicMarketSummaryRecord> for PublicMarketCardResponse {
    fn from(value: &PublicMarketSummaryRecord) -> Self {
        Self {
            id: value.market_id,
            slug: value.market_slug.clone(),
            label: value.label.clone(),
            question: value.question.clone(),
            question_id: value.question_id.clone(),
            condition_id: value.condition_id.clone(),
            market_type: value.market_type.clone(),
            outcomes: value.outcomes.clone(),
            end_time: value.end_time,
            sort_order: value.sort_order,
            trading_status: value.trading_status.clone(),
            current_prices: None,
            stats: None,
            quote_summary: None,
            last_trade_yes_bps: None,
            event: PublicEventTeaserResponse::from(value),
        }
    }
}

impl From<&PublicMarketSummaryRecord> for PublicEventTeaserResponse {
    fn from(value: &PublicMarketSummaryRecord) -> Self {
        Self {
            id: value.event_id,
            title: value.event_title.clone(),
            slug: value.event_slug.clone(),
            category_slug: value.category_slug.clone(),
            subcategory_slug: value.subcategory_slug.clone(),
            tag_slugs: value.tag_slugs.clone(),
            image_url: value.image_url.clone(),
            summary: value.summary_text.clone(),
            featured: value.featured,
            breaking: value.breaking,
            neg_risk: value.neg_risk,
        }
    }
}

impl PublicMarketCardResponse {
    pub fn from_market_and_event(event: &MarketEventRecord, market: &MarketRecord) -> Self {
        Self {
            id: market.id,
            slug: market.slug.clone(),
            label: market.label.clone(),
            question: market.question.clone(),
            question_id: market.question_id.clone(),
            condition_id: market.condition_id.clone(),
            market_type: market.market_type.clone(),
            outcomes: market.outcomes.clone(),
            end_time: market.end_time,
            sort_order: market.sort_order,
            trading_status: market.trading_status.clone(),
            current_prices: None,
            stats: None,
            quote_summary: None,
            last_trade_yes_bps: None,
            event: PublicEventTeaserResponse {
                id: event.id,
                title: event.title.clone(),
                slug: event.slug.clone(),
                category_slug: event.category_slug.clone(),
                subcategory_slug: event.subcategory_slug.clone(),
                tag_slugs: event.tag_slugs.clone(),
                image_url: event.image_url.clone(),
                summary: event.summary_text.clone(),
                featured: event.featured,
                breaking: event.breaking,
                neg_risk: event.neg_risk,
            },
        }
    }
}

impl From<&PublicEventSummaryRecord> for PublicEventCardResponse {
    fn from(value: &PublicEventSummaryRecord) -> Self {
        Self {
            id: value.event_id,
            title: value.event_title.clone(),
            slug: value.event_slug.clone(),
            category_slug: value.category_slug.clone(),
            subcategory_slug: value.subcategory_slug.clone(),
            tag_slugs: value.tag_slugs.clone(),
            image_url: value.image_url.clone(),
            summary: value.summary_text.clone(),
            featured: value.featured,
            breaking: value.breaking,
            neg_risk: value.neg_risk,
            starts_at: value.starts_at,
            sort_at: value.sort_at,
            market_count: value.market_count,
            markets: None,
        }
    }
}

impl From<&PublicEventSummaryRecord> for AdminEventCardResponse {
    fn from(value: &PublicEventSummaryRecord) -> Self {
        Self {
            id: value.event_id,
            title: value.event_title.clone(),
            slug: value.event_slug.clone(),
            category_slug: value.category_slug.clone(),
            subcategory_slug: value.subcategory_slug.clone(),
            tag_slugs: value.tag_slugs.clone(),
            image_url: value.image_url.clone(),
            summary: value.summary_text.clone(),
            featured: value.featured,
            breaking: value.breaking,
            neg_risk: value.neg_risk,
            publication_status: value.publication_status.clone(),
            starts_at: value.starts_at,
            sort_at: value.sort_at,
            created_at: value.created_at,
            market_count: value.market_count,
        }
    }
}

impl From<&CategorySummaryRecord> for CategorySummaryResponse {
    fn from(value: &CategorySummaryRecord) -> Self {
        Self {
            slug: value.slug.clone(),
            label: humanize_slug(&value.slug),
            event_count: value.event_count,
            market_count: value.market_count,
            featured_event_count: value.featured_event_count,
            breaking_event_count: value.breaking_event_count,
        }
    }
}

impl From<&TagSummaryRecord> for TagSummaryResponse {
    fn from(value: &TagSummaryRecord) -> Self {
        Self {
            slug: value.slug.clone(),
            label: humanize_slug(&value.slug),
            event_count: value.event_count,
            market_count: value.market_count,
        }
    }
}

fn default_binary_outcomes() -> Vec<String> {
    vec!["Yes".to_owned(), "No".to_owned()]
}

fn default_price_ladder_unit_symbol() -> String {
    "$".to_owned()
}

fn default_resolution_timezone() -> String {
    "America/New_York".to_owned()
}

fn default_zero_i32() -> i32 {
    0
}

fn default_one_i32() -> i32 {
    1
}

fn default_true() -> bool {
    true
}

fn deserialize_string_list_or_csv<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: Deserializer<'de>,
{
    #[derive(Deserialize)]
    #[serde(untagged)]
    enum StringListOrCsv {
        List(Vec<String>),
        String(String),
    }

    let parsed = StringListOrCsv::deserialize(deserializer)?;

    Ok(match parsed {
        StringListOrCsv::List(values) => values,
        StringListOrCsv::String(value) => value
            .split(|character| matches!(character, ',' | '\n' | '\r'))
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned)
            .collect(),
    })
}

fn humanize_slug(slug: &str) -> String {
    slug.split('-')
        .filter(|segment| !segment.is_empty())
        .map(|segment| {
            let mut chars = segment.chars();
            match chars.next() {
                Some(first) => {
                    let mut value = first.to_ascii_uppercase().to_string();
                    value.push_str(chars.as_str());
                    value
                }
                None => String::new(),
            }
        })
        .collect::<Vec<_>>()
        .join(" ")
}

#[cfg(test)]
mod tests {
    use super::CreateEventMarketLadderRequest;
    use serde_json::json;

    #[test]
    fn accepts_price_ladder_threshold_arrays() {
        let payload: CreateEventMarketLadderRequest = serde_json::from_value(json!({
            "template": {
                "underlying": "WTI",
                "deadline_label": "April 30, 2026",
                "end_time": "2026-05-01T03:59:59Z",
                "oracle_address": "0x6D21167d874C842386e8c484519B5ddBBaB87b43",
                "unit_symbol": "$",
                "up_thresholds": ["90", "100"],
                "down_thresholds": ["80", "70"]
            }
        }))
        .unwrap();

        assert_eq!(payload.template.up_thresholds, vec!["90", "100"]);
        assert_eq!(payload.template.down_thresholds, vec!["80", "70"]);
    }

    #[test]
    fn accepts_price_ladder_threshold_csv_strings() {
        let payload: CreateEventMarketLadderRequest = serde_json::from_value(json!({
            "template": {
                "underlying": "WTI",
                "deadline_label": "April 30, 2026",
                "end_time": "2026-05-01T03:59:59Z",
                "oracle_address": "0x6D21167d874C842386e8c484519B5ddBBaB87b43",
                "unit_symbol": "$",
                "up_thresholds": "90, 100, 110",
                "down_thresholds": "80\n70\n60"
            }
        }))
        .unwrap();

        assert_eq!(payload.template.up_thresholds, vec!["90", "100", "110"]);
        assert_eq!(payload.template.down_thresholds, vec!["80", "70", "60"]);
    }
}
