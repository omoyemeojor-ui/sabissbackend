use chrono::{DateTime, Utc};
use ethers_core::types::U256;
use uuid::Uuid;

use crate::module::{auth::error::AuthError, market::schema::MarketQuoteResponse};

const TOKEN_DECIMALS: usize = 6;
const BPS_SCALE: u64 = 10_000;
const USDC_BASE_UNITS_PER_CENT: u64 = 10_000;
const MIN_TRADE_USDC_BASE_UNITS: u64 = 500_000;
const MAX_TRADE_USDC_BASE_UNITS: u64 = 10_000_000_000;

pub fn parse_trade_amount(raw: &str, field_name: &str) -> Result<U256, AuthError> {
    let value = raw.trim();
    if value.is_empty() {
        return Err(AuthError::bad_request(format!("{field_name} is required")));
    }

    let parsed = U256::from_dec_str(value).map_err(|_| {
        AuthError::bad_request(format!(
            "{field_name} must be a base-10 integer string in 6-decimal base units; for example 5.25 = 5250000"
        ))
    })?;
    if parsed.is_zero() {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be greater than zero"
        )));
    }

    Ok(parsed)
}

pub fn validate_trade_value_bounds(usdc_amount: U256, field_name: &str) -> Result<(), AuthError> {
    let minimum = U256::from(MIN_TRADE_USDC_BASE_UNITS);
    let maximum = U256::from(MAX_TRADE_USDC_BASE_UNITS);

    if usdc_amount < minimum || usdc_amount > maximum {
        return Err(AuthError::bad_request(format!(
            "{field_name} must be between {minimum} and {maximum} base units ({} to {} USDC). The trade API expects 6-decimal base units, for example 100 USDC = 100000000",
            format_amount(&minimum),
            format_amount(&maximum),
        )));
    }

    Ok(())
}

pub fn quote_token_amount(usdc_amount: U256, price_bps: u32) -> Result<U256, AuthError> {
    if price_bps == 0 {
        return Err(AuthError::bad_request("market price is not set"));
    }

    Ok((usdc_amount * U256::from(BPS_SCALE)) / U256::from(price_bps))
}

pub fn quote_usdc_amount(token_amount: U256, price_bps: u32) -> U256 {
    (token_amount * U256::from(price_bps)) / U256::from(BPS_SCALE)
}

pub fn format_amount(value: &U256) -> String {
    format_decimal_str(&value.to_string())
}

pub fn bps_to_price(value: u32) -> f64 {
    f64::from(value) / f64::from(BPS_SCALE as u32)
}

pub fn volume_usd_cents(usdc_amount: &U256) -> Result<i64, AuthError> {
    let cents = (*usdc_amount / U256::from(USDC_BASE_UNITS_PER_CENT)).to_string();
    cents
        .parse::<i64>()
        .map_err(|error| AuthError::internal("trade volume exceeds i64 range", error))
}

pub fn last_trade_yes_bps(outcome_index: i32, price_bps: u32) -> Result<i32, AuthError> {
    match outcome_index {
        0 => i32::try_from(price_bps)
            .map_err(|error| AuthError::internal("invalid YES trade price", error)),
        1 => i32::try_from(BPS_SCALE as u32 - price_bps)
            .map_err(|error| AuthError::internal("invalid NO trade price", error)),
        _ => Err(AuthError::bad_request("trade.outcome_index must be 0 or 1")),
    }
}

pub fn build_market_quote(
    market_id: Uuid,
    condition_id: &str,
    yes_bps: u32,
    no_bps: u32,
    last_trade_yes_bps: u32,
    as_of: DateTime<Utc>,
) -> MarketQuoteResponse {
    MarketQuoteResponse {
        market_id,
        condition_id: Some(condition_id.to_owned()),
        source: "price_snapshot".to_owned(),
        as_of,
        buy_yes_bps: yes_bps,
        buy_no_bps: no_bps,
        sell_yes_bps: yes_bps,
        sell_no_bps: no_bps,
        last_trade_yes_bps,
        spread_bps: 0,
    }
}

fn format_decimal_str(raw: &str) -> String {
    if raw.is_empty() {
        return "0".to_owned();
    }

    let raw = raw.trim_start_matches('0');
    if raw.is_empty() {
        return "0".to_owned();
    }

    if raw.len() <= TOKEN_DECIMALS {
        return format_fractional("0", &format!("{raw:0>6}"));
    }

    let split_index = raw.len() - TOKEN_DECIMALS;
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

#[cfg(test)]
mod tests {
    use super::{parse_trade_amount, validate_trade_value_bounds};
    use ethers_core::types::U256;

    #[test]
    fn parse_trade_amount_rejects_decimal_display_strings() {
        let error = parse_trade_amount("5.25", "trade.usdc_amount").unwrap_err();
        assert_eq!(
            error.to_string(),
            "400 Bad Request: trade.usdc_amount must be a base-10 integer string in 6-decimal base units; for example 5.25 = 5250000"
        );
    }

    #[test]
    fn validate_trade_value_bounds_accepts_contract_limits() {
        assert!(validate_trade_value_bounds(U256::from(500_000_u64), "trade.usdc_amount").is_ok());
        assert!(
            validate_trade_value_bounds(U256::from(10_000_000_000_u64), "trade.usdc_amount")
                .is_ok()
        );
    }

    #[test]
    fn validate_trade_value_bounds_rejects_display_amounts() {
        let error =
            validate_trade_value_bounds(U256::from(100_u64), "trade.usdc_amount").unwrap_err();
        assert_eq!(
            error.to_string(),
            "400 Bad Request: trade.usdc_amount must be between 500000 and 10000000000 base units (0.5 to 10000 USDC). The trade API expects 6-decimal base units, for example 100 USDC = 100000000"
        );
    }
}
