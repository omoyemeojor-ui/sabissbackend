use anyhow::Result;
use ethers_core::abi::{Abi, AbiParser};

pub fn liquidity_manager_read_abi() -> Result<Abi> {
    AbiParser::default()
        .parse(&[
            "function getMarketLiquidity(bytes32 conditionId) view returns (uint256 idleYesTotal, uint256 idleNoTotal, uint256 postedYesTotal, uint256 postedNoTotal, uint256 claimableCollateralTotal)",
            "function getEventLiquidity(bytes32 eventId) view returns (uint256 idleYesTotal, uint256 idleNoTotal, uint256 postedYesTotal, uint256 postedNoTotal, uint256 claimableCollateralTotal)",
            "function getLiquidityPosition(bytes32 conditionId, address provider) view returns (uint256 yesAmount, uint256 noAmount, uint256 idleYesAmount, uint256 idleNoAmount, uint256 collateralAmount, uint256 claimableCollateralAmount, uint256 timestamp, bool active)",
        ])
        .map_err(Into::into)
}

pub fn liquidity_manager_write_abi() -> Result<Abi> {
    AbiParser::default()
        .parse(&[
            "function hasRole(bytes32 role, address account) view returns (bool)",
            "function depositInventory(bytes32 conditionId, uint256 yesAmount, uint256 noAmount)",
            "function depositCollateral(bytes32 conditionId, uint256 amount)",
            "function removeLiquidity(bytes32 conditionId, uint256 yesAmount, uint256 noAmount)",
            "function withdrawInventory(bytes32 conditionId, uint256 yesAmount, uint256 noAmount, address recipient)",
            "function withdrawCollateral(bytes32 conditionId, uint256 amount, address recipient)",
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
        ])
        .map_err(Into::into)
}

pub fn simple_account_abi() -> Result<Abi> {
    AbiParser::default()
        .parse(&[
            "function execute(address dest, uint256 value, bytes func)",
            "function executeBatch(address[] dest, bytes[] func)",
        ])
        .map_err(Into::into)
}

pub fn simple_account_factory_abi() -> Result<Abi> {
    AbiParser::default()
        .parse(&[
            "function createAccount(address owner, uint256 salt) returns (address)",
            "function getAddress(address owner, uint256 salt) view returns (address)",
        ])
        .map_err(Into::into)
}

pub fn entry_point_abi() -> Result<Abi> {
    AbiParser::default()
        .parse(&["function getNonce(address sender, uint192 key) view returns (uint256)"])
        .map_err(Into::into)
}
