use anyhow::Result;
use ethers_core::abi::{Abi, AbiParser};

pub fn exchange_write_abi() -> Result<Abi> {
    AbiParser::default()
        .parse(&[
            "function buyOutcome(bytes32 conditionId, uint256 outcomeIndex, uint256 usdcAmount) returns (uint256)",
            "function sellOutcome(bytes32 conditionId, uint256 outcomeIndex, uint256 tokenAmount) returns (uint256)",
        ])
        .map_err(Into::into)
}

pub fn erc20_abi() -> Result<Abi> {
    AbiParser::default()
        .parse(&[
            "function allowance(address owner, address spender) view returns (uint256)",
            "function approve(address spender, uint256 amount) returns (bool)",
        ])
        .map_err(Into::into)
}

pub fn conditional_tokens_abi() -> Result<Abi> {
    AbiParser::default()
        .parse(&[
            "function isApprovedForAll(address account, address operator) view returns (bool)",
            "function setApprovalForAll(address operator, bool approved)",
            "function splitPosition(address collateralToken, bytes32 parentCollectionId, bytes32 conditionId, uint256[] partition, uint256 amount)",
            "function mergePositions(address collateralToken, bytes32 parentCollectionId, bytes32 conditionId, uint256[] partition, uint256 amount)",
        ])
        .map_err(Into::into)
}
