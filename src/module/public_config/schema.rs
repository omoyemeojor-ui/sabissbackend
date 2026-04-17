use serde::Serialize;

#[derive(Debug, Serialize)]
pub struct PublicContractsConfigResponse {
    pub chain_id: i64,
    pub contracts: PublicContractsAddressesResponse,
}

#[derive(Debug, Serialize)]
pub struct PublicContractsAddressesResponse {
    pub conditional_tokens: String,
    pub usdc: String,
    pub market_factory: String,
    pub liquidity_manager: String,
    pub pool_exchange: String,
    pub orderbook_exchange: String,
    pub redemption: String,
    pub neg_risk_adapter: String,
}
