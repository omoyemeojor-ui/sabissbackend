use std::{fmt::Display, str::FromStr};

use anyhow::{Result, anyhow};
use ethers_core::{
    types::{Address, H256},
    utils::keccak256,
};

pub(super) fn parse_bytes32(value: &str) -> Result<H256> {
    H256::from_str(value).map_err(|error| anyhow!("invalid bytes32 value: {error}"))
}

pub(super) fn parse_address(value: &str) -> Result<Address> {
    Address::from_str(value).map_err(|error| anyhow!("invalid address value: {error}"))
}

pub(super) fn market_maker_role() -> H256 {
    H256::from(keccak256("MARKET_MAKER_ROLE"))
}

pub(super) fn conditional_tokens_approval_query_reverted(error: &impl Display) -> bool {
    let message = error.to_string();
    message.contains("execution reverted") || message.contains("Contract call reverted")
}
