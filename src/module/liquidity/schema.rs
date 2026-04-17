use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use crate::module::market::schema::{EventOnChainResponse, EventResponse, MarketResponse};

#[derive(Debug, Deserialize)]
pub struct DepositInventoryRequest {
    pub liquidity: InventoryAmountsRequest,
}

#[derive(Debug, Deserialize)]
pub struct DepositCollateralRequest {
    pub liquidity: CollateralAmountRequest,
}

#[derive(Debug, Deserialize)]
pub struct RemoveLiquidityRequest {
    pub liquidity: InventoryAmountsRequest,
}

#[derive(Debug, Deserialize)]
pub struct WithdrawInventoryRequest {
    pub liquidity: WithdrawInventoryFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct WithdrawCollateralRequest {
    pub liquidity: WithdrawCollateralFieldsRequest,
}

#[derive(Debug, Deserialize)]
pub struct InventoryAmountsRequest {
    pub yes_amount: String,
    pub no_amount: String,
}

#[derive(Debug, Deserialize)]
pub struct CollateralAmountRequest {
    pub amount: String,
}

#[derive(Debug, Deserialize)]
pub struct WithdrawInventoryFieldsRequest {
    pub yes_amount: String,
    pub no_amount: String,
    pub recipient: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct WithdrawCollateralFieldsRequest {
    pub amount: String,
    pub recipient: Option<String>,
}

#[derive(Debug, Serialize, Clone)]
pub struct LiquidityTotalsResponse {
    pub idle_yes_total: String,
    pub idle_no_total: String,
    pub posted_yes_total: String,
    pub posted_no_total: String,
    pub claimable_collateral_total: String,
}

#[derive(Debug, Serialize, Clone)]
pub struct LiquidityPositionResponse {
    pub posted_yes_amount: String,
    pub posted_no_amount: String,
    pub idle_yes_amount: String,
    pub idle_no_amount: String,
    pub collateral_amount: String,
    pub claimable_collateral_amount: String,
    pub updated_at: Option<DateTime<Utc>>,
    pub active: bool,
}

#[derive(Debug, Serialize)]
pub struct EventLiquidityResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub markets_count: i64,
    pub liquidity: LiquidityTotalsResponse,
}

#[derive(Debug, Serialize)]
pub struct MyMarketLiquidityResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub wallet_address: String,
    pub position: LiquidityPositionResponse,
    pub market_liquidity: LiquidityTotalsResponse,
}

#[derive(Debug, Serialize)]
pub struct MyEventLiquidityPositionTotalsResponse {
    pub active_markets: i64,
    pub posted_yes_amount: String,
    pub posted_no_amount: String,
    pub idle_yes_amount: String,
    pub idle_no_amount: String,
    pub collateral_amount: String,
    pub claimable_collateral_amount: String,
}

#[derive(Debug, Serialize)]
pub struct MyEventLiquidityMarketResponse {
    pub market: MarketResponse,
    pub position: LiquidityPositionResponse,
    pub market_liquidity: LiquidityTotalsResponse,
}

#[derive(Debug, Serialize)]
pub struct MyEventLiquidityResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub wallet_address: String,
    pub event_liquidity: LiquidityTotalsResponse,
    pub position_totals: MyEventLiquidityPositionTotalsResponse,
    pub markets: Vec<MyEventLiquidityMarketResponse>,
}

#[derive(Debug, Serialize)]
pub struct LiquidityWriteResponse {
    pub event: EventResponse,
    pub on_chain: EventOnChainResponse,
    pub market: MarketResponse,
    pub wallet_address: String,
    pub action: String,
    pub tx_hash: String,
    pub position: LiquidityPositionResponse,
    pub market_liquidity: LiquidityTotalsResponse,
    pub updated_at: DateTime<Utc>,
}

pub fn empty_liquidity_totals() -> LiquidityTotalsResponse {
    LiquidityTotalsResponse {
        idle_yes_total: "0".to_owned(),
        idle_no_total: "0".to_owned(),
        posted_yes_total: "0".to_owned(),
        posted_no_total: "0".to_owned(),
        claimable_collateral_total: "0".to_owned(),
    }
}

pub fn empty_liquidity_position() -> LiquidityPositionResponse {
    LiquidityPositionResponse {
        posted_yes_amount: "0".to_owned(),
        posted_no_amount: "0".to_owned(),
        idle_yes_amount: "0".to_owned(),
        idle_no_amount: "0".to_owned(),
        collateral_amount: "0".to_owned(),
        claimable_collateral_amount: "0".to_owned(),
        updated_at: None,
        active: false,
    }
}
