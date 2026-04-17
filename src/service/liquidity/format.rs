use ethers_core::types::U256;

use crate::{
    module::{
        auth::error::AuthError,
        liquidity::schema::{
            LiquidityPositionResponse, LiquidityTotalsResponse,
            MyEventLiquidityPositionTotalsResponse,
        },
    },
    service::liquidity::chain_read::{LiquidityPositionReadResult, LiquidityTotalsReadResult},
};

const USDC_DECIMALS: usize = 6;

#[derive(Debug, Default)]
pub struct PositionAccumulator {
    pub active_markets: i64,
    pub posted_yes_amount: U256,
    pub posted_no_amount: U256,
    pub idle_yes_amount: U256,
    pub idle_no_amount: U256,
    pub collateral_amount: U256,
    pub claimable_collateral_amount: U256,
}

impl PositionAccumulator {
    pub fn add(&mut self, position: &LiquidityPositionReadResult) {
        if has_position_balance(position) {
            self.active_markets += 1;
        }

        self.posted_yes_amount += position.posted_yes_amount;
        self.posted_no_amount += position.posted_no_amount;
        self.idle_yes_amount += position.idle_yes_amount;
        self.idle_no_amount += position.idle_no_amount;
        self.collateral_amount += position.collateral_amount;
        self.claimable_collateral_amount += position.claimable_collateral_amount;
    }

    pub fn into_response(self) -> MyEventLiquidityPositionTotalsResponse {
        MyEventLiquidityPositionTotalsResponse {
            active_markets: self.active_markets,
            posted_yes_amount: format_amount(&self.posted_yes_amount),
            posted_no_amount: format_amount(&self.posted_no_amount),
            idle_yes_amount: format_amount(&self.idle_yes_amount),
            idle_no_amount: format_amount(&self.idle_no_amount),
            collateral_amount: format_amount(&self.collateral_amount),
            claimable_collateral_amount: format_amount(&self.claimable_collateral_amount),
        }
    }
}

pub fn liquidity_totals_response(value: &LiquidityTotalsReadResult) -> LiquidityTotalsResponse {
    LiquidityTotalsResponse {
        idle_yes_total: format_amount(&value.idle_yes_total),
        idle_no_total: format_amount(&value.idle_no_total),
        posted_yes_total: format_amount(&value.posted_yes_total),
        posted_no_total: format_amount(&value.posted_no_total),
        claimable_collateral_total: format_amount(&value.claimable_collateral_total),
    }
}

pub fn liquidity_position_response(
    value: &LiquidityPositionReadResult,
) -> LiquidityPositionResponse {
    LiquidityPositionResponse {
        posted_yes_amount: format_amount(&value.posted_yes_amount),
        posted_no_amount: format_amount(&value.posted_no_amount),
        idle_yes_amount: format_amount(&value.idle_yes_amount),
        idle_no_amount: format_amount(&value.idle_no_amount),
        collateral_amount: format_amount(&value.collateral_amount),
        claimable_collateral_amount: format_amount(&value.claimable_collateral_amount),
        updated_at: value.updated_at,
        active: value.active,
    }
}

pub fn parse_amount(raw: &str, field_name: &str, allow_zero: bool) -> Result<U256, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }

    let parsed = U256::from_dec_str(value).map_err(|_| {
        AuthError::bad_request(format!("{field_name} must be a base-10 integer string"))
    })?;

    if parsed.is_zero() && !allow_zero {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be greater than zero"
        )));
    }

    Ok(parsed)
}

pub fn has_position_balance(position: &LiquidityPositionReadResult) -> bool {
    position.active
        || !position.posted_yes_amount.is_zero()
        || !position.posted_no_amount.is_zero()
        || !position.idle_yes_amount.is_zero()
        || !position.idle_no_amount.is_zero()
        || !position.collateral_amount.is_zero()
        || !position.claimable_collateral_amount.is_zero()
}

fn format_amount(value: &U256) -> String {
    format_decimal_str(&value.to_string())
}

fn format_decimal_str(raw: &str) -> String {
    if raw.is_empty() {
        return "0".to_owned();
    }

    let raw = raw.trim_start_matches('0');
    if raw.is_empty() {
        return "0".to_owned();
    }

    if raw.len() <= USDC_DECIMALS {
        return format_fractional("0", &format!("{raw:0>6}"));
    }

    let split_index = raw.len() - USDC_DECIMALS;
    format_fractional(&raw[..split_index], &raw[split_index..])
}

fn format_fractional(whole: &str, fractional: &str) -> String {
    let trimmed_fractional = fractional.trim_end_matches('0');
    if trimmed_fractional.is_empty() {
        whole.to_owned()
    } else {
        format!("{whole}.{trimmed_fractional}")
    }
}
