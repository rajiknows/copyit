use rust_decimal::Decimal;
use thiserror::Error;

#[derive(Error, Debug, PartialEq)]
pub enum ParseError {
    #[error("Failed to parse price: {0}")]
    Price(String),
    #[error("Failed to parse size: {0}")]
    Size(String),
}

/// Parse trade side to human-readable format
pub fn parse_side(side: &str) -> &str {
    match side {
        "A" => "SELL",
        "B" => "BUY",
        _ => "UNKNOWN",
    }
}

/// Parse price from string to Decimal
pub fn parse_price(price: &str) -> Result<Decimal, ParseError> {
    price
        .parse::<Decimal>()
        .map_err(|e| ParseError::Price(e.to_string()))
}

/// Parse size from string to Decimal
pub fn parse_size(size: &str) -> Result<Decimal, ParseError> {
    size.parse::<Decimal>()
        .map_err(|e| ParseError::Size(e.to_string()))
}

/// Calculate trade value (price × size)
pub fn calculate_trade_value(price: &str, size: &str) -> Result<Decimal, ParseError> {
    let p = parse_price(price)?;
    let s = parse_size(size)?;
    Ok(p * s)
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn test_parse_side() {
        assert_eq!(parse_side("A"), "SELL");
        assert_eq!(parse_side("B"), "BUY");
        assert_eq!(parse_side("X"), "UNKNOWN");
        assert_eq!(parse_side(""), "UNKNOWN");
    }

    #[test]
    fn test_parse_price() {
        assert_eq!(parse_price("42000.5").unwrap(), dec!(42000.5));
        assert!(parse_price("invalid").is_err());
        assert_eq!(
            parse_price("invalid").unwrap_err(),
            ParseError::Price("Invalid decimal: invalid".to_string())
        );
        assert!(parse_price("-100.0").is_ok());
    }

    #[test]
    fn test_parse_size() {
        assert_eq!(parse_size("10.5").unwrap(), dec!(10.5));
        assert!(parse_size("invalid").is_err());
        assert_eq!(
            parse_size("invalid").unwrap_err(),
            ParseError::Size("Invalid decimal: invalid".to_string())
        );
        assert!(parse_size("-5.0").is_ok()); // Assuming size can be negative for some reason
    }

    #[test]
    fn test_calculate_trade_value() {
        assert_eq!(calculate_trade_value("100.0", "2.5").unwrap(), dec!(250.0));
        assert_eq!(calculate_trade_value("100.0", "0").unwrap(), dec!(0));
        assert_eq!(calculate_trade_value("0", "2.5").unwrap(), dec!(0));
        assert!(calculate_trade_value("abc", "2.5").is_err());
        assert!(calculate_trade_value("100.0", "xyz").is_err());
    }
}
