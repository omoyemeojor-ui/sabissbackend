use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::module::{
    market::{
        schema::{EventOnChainResponse, EventResponse, MarketResponse},
        trade_schema::PreparedWalletCallResponse,
    },
    order::model::MarketOrderRecord,
};

#[derive(Debug, Deserialize)]
pub struct CreateOrderRequest {
    pub order: CreateOrderFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct CancelOrderRequest {
    pub order: CancelOrderFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct AdminFillDirectOrdersRequest {
    pub fill: AdminFillDirectOrdersFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct AdminFillComplementaryBuyOrdersRequest {
    pub fill: AdminFillComplementaryBuyOrdersFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct AdminFillComplementarySellOrdersRequest {
    pub fill: AdminFillComplementarySellOrdersFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct AdminMatchOrdersRequest {
    pub matching: AdminMatchOrdersFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct CreateOrderFieldsRequest {
    pub market_id: Uuid,
    pub outcome_index: i32,
    pub side: String,
    pub price_bps: u32,
    pub token_amount: String,
    pub expiry_epoch_seconds: Option<i64>,
    pub salt: String,
    pub signature: String,
}

#[derive(Debug, Deserialize)]
pub struct CancelOrderFieldsRequest {
    pub order_id: Uuid,
}

#[derive(Debug, Deserialize)]
pub struct AdminFillDirectOrdersFieldsRequest {
    pub buy_order_id: Uuid,
    pub sell_order_id: Uuid,
    pub fill_token_amount: String,
    pub settlement_price_bps: u32,
}

#[derive(Debug, Deserialize)]
pub struct AdminFillComplementaryBuyOrdersFieldsRequest {
    pub yes_buy_order_id: Uuid,
    pub no_buy_order_id: Uuid,
    pub fill_token_amount: String,
    pub yes_settlement_price_bps: u32,
}

#[derive(Debug, Deserialize)]
pub struct AdminFillComplementarySellOrdersFieldsRequest {
    pub yes_sell_order_id: Uuid,
    pub no_sell_order_id: Uuid,
    pub fill_token_amount: String,
    pub yes_settlement_price_bps: u32,
}

#[derive(Debug, Deserialize)]
pub struct AdminMatchOrdersFieldsRequest {
    pub market_id: Option<Uuid>,
    pub max_fills_per_market: Option<u32>,
}

#[derive(Debug, Serialize)]
pub struct OrderResponse {
    pub id: Uuid,
    pub status: String,
    pub order_hash: String,
    pub order_digest: String,
    pub side: String,
    pub outcome_index: i32,
    pub outcome_label: String,
    pub price_bps: u32,
    pub price: f64,
    pub token_amount: String,
    pub filled_token_amount: String,
    pub remaining_token_amount: String,
    pub quoted_usdc_amount: String,
    pub expiry_epoch_seconds: Option<i64>,
    pub expires_at: Option<DateTime<Utc>>,
    pub salt: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct OrderItemResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub order: OrderResponse,
}

#[derive(Debug, Serialize)]
pub struct CreateOrderResponse {
    pub wallet_address: String,
    pub account_kind: String,
    pub order: OrderItemResponse,
}

#[derive(Debug, Serialize)]
pub struct CancelOrderResponse {
    pub wallet_address: String,
    pub account_kind: String,
    pub cancellation_scope: String,
    pub cancellation_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_transactions: Option<Vec<PreparedWalletCallResponse>>,
    pub order: OrderItemResponse,
}

#[derive(Debug, Serialize)]
pub struct MyOrdersResponse {
    pub wallet_address: String,
    pub account_kind: String,
    pub orders: Vec<OrderItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct PositionOutcomeResponse {
    pub outcome_index: i32,
    pub outcome_label: String,
    pub token_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub estimated_value_usdc: Option<String>,
}

#[derive(Debug, Serialize)]
pub struct PositionItemResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub outcomes: Vec<PositionOutcomeResponse>,
    pub total_token_amount: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_estimated_value_usdc: Option<String>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MyPositionsResponse {
    pub wallet_address: String,
    pub account_kind: String,
    pub positions: Vec<PositionItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct PortfolioSummaryResponse {
    pub cash_balance: String,
    pub portfolio_balance: String,
    pub total_balance: String,
    pub total_buy_amount: String,
    pub total_sell_amount: String,
}

#[derive(Debug, Serialize)]
pub struct PortfolioMarketSummaryResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub buy_amount: String,
    pub sell_amount: String,
    pub portfolio_balance: String,
    pub positions: Vec<PositionOutcomeResponse>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_traded_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Serialize)]
pub struct PortfolioTradeHistoryItemResponse {
    pub id: String,
    pub execution_source: String,
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub action: String,
    pub outcome_index: i32,
    pub outcome_label: String,
    pub usdc_amount: String,
    pub token_amount: String,
    pub price_bps: u32,
    pub price: f64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct MyPortfolioResponse {
    pub wallet_address: String,
    pub account_kind: String,
    pub summary: PortfolioSummaryResponse,
    pub markets: Vec<PortfolioMarketSummaryResponse>,
    pub history: Vec<PortfolioTradeHistoryItemResponse>,
}

#[derive(Debug, Serialize)]
pub struct OrderFillOrderStateResponse {
    pub order_id: Uuid,
    pub side: String,
    pub outcome_index: i32,
    pub status: String,
    pub filled_token_amount: String,
    pub remaining_token_amount: String,
}

#[derive(Debug, Serialize)]
pub struct AdminOrderFillResponse {
    pub market_id: Uuid,
    pub condition_id: String,
    pub match_type: String,
    pub tx_hash: String,
    pub fill_token_amount: String,
    pub collateral_amount: String,
    pub yes_price_bps: u32,
    pub no_price_bps: u32,
    pub orders: Vec<OrderFillOrderStateResponse>,
    pub executed_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct AdminMatchedMarketResponse {
    pub market_id: Uuid,
    pub executed_fills: usize,
    pub fills: Vec<AdminOrderFillResponse>,
}

#[derive(Debug, Serialize)]
pub struct AdminMatchOrdersResponse {
    pub scope: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub market_id: Option<Uuid>,
    pub markets_scanned: usize,
    pub markets_matched: usize,
    pub executed_fills: usize,
    pub max_fills_per_market: u32,
    pub markets: Vec<AdminMatchedMarketResponse>,
    pub completed_at: DateTime<Utc>,
}

impl OrderResponse {
    pub fn from_record(
        record: &MarketOrderRecord,
        outcome_label: String,
        price_bps: u32,
        price: f64,
        token_amount: String,
        filled_token_amount: String,
        remaining_token_amount: String,
        quoted_usdc_amount: String,
        expires_at: Option<DateTime<Utc>>,
    ) -> Self {
        Self {
            id: record.id,
            status: record.status.clone(),
            order_hash: record.order_hash.clone(),
            order_digest: record.order_digest.clone(),
            side: record.side.clone(),
            outcome_index: record.outcome_index,
            outcome_label,
            price_bps,
            price,
            token_amount,
            filled_token_amount,
            remaining_token_amount,
            quoted_usdc_amount,
            expiry_epoch_seconds: record.expiry_epoch_seconds,
            expires_at,
            salt: record.salt.clone(),
            created_at: record.created_at,
            updated_at: record.updated_at,
            cancelled_at: record.cancelled_at,
        }
    }
}
