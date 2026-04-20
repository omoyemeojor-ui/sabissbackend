use axum::{Json, extract::State};

use crate::{
    app::AppState,
    module::public_config::schema::{
        PublicContractsAddressesResponse, PublicContractsConfigResponse,
    },
};

pub async fn contracts_config(
    State(state): State<AppState>,
) -> Json<PublicContractsConfigResponse> {
    Json(PublicContractsConfigResponse {
        chain_id: match state.env.network.as_str() {
            "mainnet" => 1,
            _ => 10143,
        },
        network: state.env.network.clone(),
        rpc_url: state.env.rpc_url.clone(),
        horizon_url: state.env.horizon_url.clone(),
        contracts: PublicContractsAddressesResponse {
            conditional_tokens: state.env.sabi_ctf_id.clone(),
            usdc: state.env.mock_usdc_id.clone(),
            market_factory: state.env.sabi_market_id.clone(),
            liquidity_manager: state.env.sabi_liquidity_manager_id.clone(),
            pool_exchange: state.env.sabi_exchange_id.clone(),
            orderbook_exchange: state.env.sabi_exchange_id.clone(),
            redemption: state.env.sabi_ctf_id.clone(),
            neg_risk_adapter: state.env.sabi_neg_risk_id.clone(),
            wallet_factory: state.env.sabi_wallet_factory_id.clone(),
            wallet_web_auth: state.env.sep45_web_auth_contract_id.clone(),
        },
    })
}
