use chrono::{DateTime, Utc};
use sqlx::FromRow;
use uuid::Uuid;

#[derive(Debug, Clone, FromRow)]
pub struct MarketEventRecord {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary_text: Option<String>,
    pub rules_text: String,
    pub context_text: Option<String>,
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
    pub group_key: String,
    pub series_key: String,
    pub event_id: String,
    pub group_id: String,
    pub series_id: String,
    pub neg_risk: bool,
    pub oracle_address: Option<String>,
    pub publication_status: String,
    pub published_tx_hash: Option<String>,
    pub created_by_user_id: Uuid,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketRecord {
    pub id: Uuid,
    pub event_db_id: Uuid,
    pub slug: String,
    pub label: String,
    pub question: String,
    pub question_id: String,
    pub condition_id: Option<String>,
    pub market_type: String,
    pub outcome_count: i32,
    pub outcomes: Vec<String>,
    pub end_time: DateTime<Utc>,
    pub sort_order: i32,
    pub publication_status: String,
    pub trading_status: String,
    pub metadata_hash: Option<String>,
    pub oracle_address: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketResolutionRecord {
    pub market_id: Uuid,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketAutoResolutionConfigRecord {
    pub market_id: Uuid,
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
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketAutoCreateSeriesRecord {
    pub id: Uuid,
    pub provider: String,
    pub product_id: String,
    pub title_prefix: String,
    pub slug_prefix: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary_text: Option<String>,
    pub rules_text: String,
    pub context_text: Option<String>,
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
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PendingMarketAutoResolutionRecord {
    pub market_id: Uuid,
    pub condition_id: String,
    pub question: String,
    pub outcomes: Vec<String>,
    pub end_time: DateTime<Utc>,
    pub publication_status: String,
    pub trading_status: String,
    pub oracle_address: String,
    pub resolution_status: Option<String>,
    pub dispute_deadline: Option<DateTime<Utc>>,
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

#[derive(Debug, Clone, FromRow)]
pub struct MarketEventNegRiskConfigRecord {
    pub event_id: Uuid,
    pub registered: bool,
    pub has_other: bool,
    pub other_market_id: Option<Uuid>,
    pub other_condition_id: Option<String>,
    pub registered_by_user_id: Uuid,
    pub registered_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct PublicMarketSummaryRecord {
    pub market_id: Uuid,
    pub market_slug: String,
    pub label: String,
    pub question: String,
    pub question_id: String,
    pub condition_id: Option<String>,
    pub market_type: String,
    pub outcomes: Vec<String>,
    pub end_time: DateTime<Utc>,
    pub sort_order: i32,
    pub trading_status: String,
    pub market_created_at: DateTime<Utc>,
    pub event_id: Uuid,
    pub event_slug: String,
    pub event_title: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary_text: Option<String>,
    pub featured: bool,
    pub breaking: bool,
    pub neg_risk: bool,
}

#[derive(Debug, Clone, FromRow)]
pub struct PublicEventSummaryRecord {
    pub event_id: Uuid,
    pub event_slug: String,
    pub event_title: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary_text: Option<String>,
    pub featured: bool,
    pub breaking: bool,
    pub neg_risk: bool,
    pub publication_status: String,
    pub starts_at: Option<DateTime<Utc>>,
    pub sort_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub market_count: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketConditionRecord {
    pub market_id: Uuid,
    pub condition_id: String,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketPriceSnapshotRecord {
    pub market_id: Uuid,
    pub condition_id: String,
    pub yes_bps: i32,
    pub no_bps: i32,
    pub synced_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketTradeStatsRecord {
    pub market_id: Uuid,
    pub volume_usd_cents: i64,
    pub last_trade_yes_bps: Option<i32>,
    pub last_trade_at: Option<DateTime<Utc>>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct MarketPriceHistorySnapshotRecord {
    pub id: i64,
    pub market_id: Uuid,
    pub condition_id: String,
    pub yes_bps: i32,
    pub no_bps: i32,
    pub captured_at: DateTime<Utc>,
    pub created_at: DateTime<Utc>,
}

#[derive(Debug, Clone, FromRow)]
pub struct CategorySummaryRecord {
    pub slug: String,
    pub event_count: i64,
    pub market_count: i64,
    pub featured_event_count: i64,
    pub breaking_event_count: i64,
}

#[derive(Debug, Clone, FromRow)]
pub struct TagSummaryRecord {
    pub slug: String,
    pub event_count: i64,
    pub market_count: i64,
}

#[derive(Debug, Clone)]
pub struct NewMarketEventRecord {
    pub id: Uuid,
    pub title: String,
    pub slug: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary_text: Option<String>,
    pub rules_text: String,
    pub context_text: Option<String>,
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
    pub group_key: String,
    pub series_key: String,
    pub event_id: String,
    pub group_id: String,
    pub series_id: String,
    pub neg_risk: bool,
    pub oracle_address: Option<String>,
    pub publication_status: String,
    pub published_tx_hash: Option<String>,
    pub created_by_user_id: Uuid,
}

#[derive(Debug, Clone)]
pub struct NewMarketRecord {
    pub id: Uuid,
    pub event_db_id: Uuid,
    pub slug: String,
    pub label: String,
    pub question: String,
    pub question_id: String,
    pub condition_id: Option<String>,
    pub market_type: String,
    pub outcome_count: i32,
    pub outcomes: Vec<String>,
    pub end_time: DateTime<Utc>,
    pub sort_order: i32,
    pub publication_status: String,
    pub trading_status: String,
    pub metadata_hash: Option<String>,
    pub oracle_address: String,
}

#[derive(Debug, Clone)]
pub struct NewMarketResolutionRecord {
    pub market_id: Uuid,
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

#[derive(Debug, Clone)]
pub struct NewMarketAutoResolutionConfigRecord {
    pub market_id: Uuid,
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

#[derive(Debug, Clone)]
pub struct NewMarketAutoCreateSeriesRecord {
    pub id: Uuid,
    pub provider: String,
    pub product_id: String,
    pub title_prefix: String,
    pub slug_prefix: String,
    pub category_slug: String,
    pub subcategory_slug: Option<String>,
    pub tag_slugs: Vec<String>,
    pub image_url: Option<String>,
    pub summary_text: Option<String>,
    pub rules_text: String,
    pub context_text: Option<String>,
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
}

#[derive(Debug, Clone)]
pub struct NewMarketEventNegRiskConfigRecord {
    pub event_id: Uuid,
    pub registered: bool,
    pub has_other: bool,
    pub other_market_id: Option<Uuid>,
    pub other_condition_id: Option<String>,
    pub registered_by_user_id: Uuid,
    pub registered_at: DateTime<Utc>,
}
