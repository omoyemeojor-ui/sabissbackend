use std::{fmt::Display, str::FromStr};

use anyhow::{Result, anyhow};
use ethers_core::types::H256;

pub(super) fn parse_bytes32(value: &str) -> Result<H256> {
    H256::from_str(value).map_err(|error| anyhow!("invalid bytes32 value: {error}"))
}

pub(super) fn conditional_tokens_approval_query_reverted(error: &impl Display) -> bool {
    let message = error.to_string();
    message.contains("execution reverted") || message.contains("Contract call reverted")
}

#[cfg(test)]
mod tests {
    use super::conditional_tokens_approval_query_reverted;

    #[test]
    fn detects_reverted_conditional_tokens_approval_queries() {
        assert!(conditional_tokens_approval_query_reverted(
            &"Contract call reverted with data: 0x",
        ));
        assert!(conditional_tokens_approval_query_reverted(
            &"execution reverted: custom error",
        ));
        assert!(!conditional_tokens_approval_query_reverted(
            &"failed to resolve host",
        ));
    }
}
