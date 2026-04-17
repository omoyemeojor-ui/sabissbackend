use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::module::market::schema::{
    EventOnChainResponse, EventResponse, MarketQuoteResponse, MarketResponse,
};

#[derive(Debug, Deserialize)]
pub struct BuyMarketRequest {
    pub trade: BuyMarketFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct SellMarketRequest {
    pub trade: SellMarketFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct SplitMarketRequest {
    pub conversion: SplitMarketFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct MergeMarketRequest {
    pub conversion: MergeMarketFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct BuyMarketFieldsRequest {
    pub outcome_index: i32,
    pub usdc_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct SellMarketFieldsRequest {
    pub outcome_index: i32,
    pub token_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct SplitMarketFieldsRequest {
    pub collateral_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct MergeMarketFieldsRequest {
    pub pair_token_amount: String,
}

#[derive(Debug, Serialize)]
pub struct MarketTradeExecutionResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub wallet_address: String,
    pub account_kind: String,
    pub action: String,
    pub outcome_index: i32,
    pub outcome_label: String,
    pub execution_mode: String,
    pub execution_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_transactions: Option<Vec<PreparedWalletCallResponse>>,
    pub usdc_amount: String,
    pub token_amount: String,
    pub price_bps: u32,
    pub price: f64,
    pub market_quote: MarketQuoteResponse,
    pub requested_at: DateTime<Utc>,
}

#[derive(Debug, Serialize)]
pub struct PreparedWalletCallResponse {
    pub kind: String,
    pub target: String,
    pub data: String,
    pub value: String,
    pub description: String,
}

#[derive(Debug, Serialize)]
pub struct MarketPositionConversionResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub wallet_address: String,
    pub account_kind: String,
    pub action: String,
    pub execution_mode: String,
    pub execution_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prepared_transactions: Option<Vec<PreparedWalletCallResponse>>,
    pub collateral_amount: String,
    pub token_amount: String,
    pub requested_at: DateTime<Utc>,
}
