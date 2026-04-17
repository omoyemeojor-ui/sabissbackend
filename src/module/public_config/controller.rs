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
        chain_id: state.env.monad_chain_id,
        contracts: PublicContractsAddressesResponse {
            conditional_tokens: state.env.monad_conditional_tokens_address.clone(),
            usdc: state.env.monad_usdc_address.clone(),
            market_factory: state.env.monad_market_factory_address.clone(),
            liquidity_manager: state.env.monad_liquidity_manager_address.clone(),
            pool_exchange: state.env.monad_exchange_address.clone(),
            orderbook_exchange: state.env.monad_orderbook_exchange_address.clone(),
            redemption: state.env.monad_redemption_address.clone(),
            neg_risk_adapter: state.env.monad_neg_risk_adapter_address.clone(),
        },
    })
}
