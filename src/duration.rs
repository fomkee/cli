use crate::error::CliError;

/// Parse whole seconds or a duration with a single s/m/h/d suffix.
pub fn seconds(value: &str) -> Result<u64, CliError> {
    let (number, multiplier) = if let Some(number) = value.strip_suffix('s') {
        (number, 1)
    } else if let Some(number) = value.strip_suffix('m') {
        (number, 60)
    } else if let Some(number) = value.strip_suffix('h') {
        (number, 3600)
    } else if let Some(number) = value.strip_suffix('d') {
        (number, 86400)
    } else {
        (value, 1)
    };
    number
        .parse::<u64>()
        .ok()
        .and_then(|number| number.checked_mul(multiplier))
        .ok_or_else(|| {
            CliError::InvalidInput(
                "duration must be whole seconds or use s/m/h/d, for example 30s, 1m, or 24h".into(),
            )
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_converts_minutes_to_seconds() {
        assert_eq!(seconds("5m").unwrap(), 300);
    }
    #[test]
    fn test_accepts_whole_seconds() {
        assert_eq!(seconds("45").unwrap(), 45);
    }
    #[test]
    fn test_converts_days_to_seconds() {
        assert_eq!(seconds("1d").unwrap(), 86400);
    }
    #[test]
    fn test_rejects_overflow_without_panicking() {
        assert!(seconds("18446744073709551615d").is_err());
    }
    #[test]
    fn test_rejects_fractional_duration() {
        assert!(seconds("1.5m").is_err());
    }
    #[test]
    fn test_leaves_zero_interval_validation_to_the_api() {
        assert_eq!(seconds("0s").unwrap(), 0);
    }
}
