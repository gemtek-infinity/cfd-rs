//! Grace period parsing (HIS-059).

use std::time::Duration;

use cfdrs_shared::{ConfigError, Result};

/// Default grace period matching Go `--grace-period` default (30s).
pub const DEFAULT_GRACE_PERIOD: Duration = Duration::from_secs(30);

/// Maximum grace period matching Go `connection.MaxGracePeriod` (3m).
pub const MAX_GRACE_PERIOD: Duration = Duration::from_secs(3 * 60);

/// Parse a Go-style duration string for `--grace-period`.
pub fn parse_grace_period(value: Option<&str>) -> Result<Duration> {
    let Some(raw_value) = value else {
        return Ok(DEFAULT_GRACE_PERIOD);
    };

    let trimmed = raw_value.trim();

    if trimmed.is_empty() {
        return Ok(DEFAULT_GRACE_PERIOD);
    }

    let duration = parse_go_duration(trimmed)?;

    if duration > MAX_GRACE_PERIOD {
        return Err(ConfigError::invariant(format!(
            "grace-period must be equal or less than {:?}",
            MAX_GRACE_PERIOD
        )));
    }

    Ok(duration)
}

fn parse_go_duration(raw: &str) -> Result<Duration> {
    if raw == "0" {
        return Ok(Duration::ZERO);
    }

    let mut total_nanos = 0f64;
    let mut rest = raw;

    while !rest.is_empty() {
        let number_end = rest
            .find(|ch: char| !(ch.is_ascii_digit() || ch == '.'))
            .ok_or_else(|| ConfigError::invariant(format!("invalid grace-period duration `{raw}`")))?;

        let number_text = &rest[..number_end];

        if number_text.is_empty() {
            return Err(ConfigError::invariant(format!(
                "invalid grace-period duration `{raw}`"
            )));
        }

        let value = number_text
            .parse::<f64>()
            .map_err(|_| ConfigError::invariant(format!("invalid grace-period duration `{raw}`")))?;

        rest = &rest[number_end..];

        let (unit_nanos, next_rest) = parse_go_duration_unit(rest, raw)?;
        total_nanos += value * unit_nanos;
        rest = next_rest;
    }

    if !total_nanos.is_finite() || total_nanos.is_sign_negative() {
        return Err(ConfigError::invariant(format!(
            "invalid grace-period duration `{raw}`"
        )));
    }

    if total_nanos > u64::MAX as f64 {
        return Err(ConfigError::invariant(format!(
            "invalid grace-period duration `{raw}`"
        )));
    }

    Ok(Duration::from_nanos(total_nanos.round() as u64))
}

fn parse_go_duration_unit<'a>(raw: &'a str, full_value: &str) -> Result<(f64, &'a str)> {
    for (unit, nanos) in [
        ("ms", 1_000_000f64),
        ("us", 1_000f64),
        ("ns", 1f64),
        ("h", 3_600_000_000_000f64),
        ("m", 60_000_000_000f64),
        ("s", 1_000_000_000f64),
    ] {
        if let Some(next_rest) = raw.strip_prefix(unit) {
            return Ok((nanos, next_rest));
        }
    }

    Err(ConfigError::invariant(format!(
        "invalid grace-period duration `{full_value}`"
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn default_grace_period_is_30s() {
        assert_eq!(DEFAULT_GRACE_PERIOD, Duration::from_secs(30));
    }

    #[test]
    fn max_grace_period_is_three_minutes() {
        assert_eq!(MAX_GRACE_PERIOD, Duration::from_secs(180));
    }

    #[test]
    fn parse_grace_period_defaults_when_unset() {
        let duration = parse_grace_period(None).expect("default grace period should parse");
        assert_eq!(duration, Duration::from_secs(30));
    }

    #[test]
    fn parse_grace_period_accepts_go_style_sequences() {
        let duration = parse_grace_period(Some("1m30s")).expect("sequence duration should parse");
        assert_eq!(duration, Duration::from_secs(90));

        let milliseconds = parse_grace_period(Some("250ms")).expect("millisecond duration should parse");
        assert_eq!(milliseconds, Duration::from_millis(250));
    }

    #[test]
    fn parse_grace_period_rejects_values_above_max() {
        let error = parse_grace_period(Some("181s")).expect_err("duration above max should fail");
        assert!(
            error
                .to_string()
                .contains("grace-period must be equal or less than")
        );
    }

    #[test]
    fn parse_grace_period_empty_string_returns_default() {
        let duration = parse_grace_period(Some("")).expect("empty should return default");
        assert_eq!(duration, DEFAULT_GRACE_PERIOD);
    }

    #[test]
    fn parse_grace_period_whitespace_returns_default() {
        let duration = parse_grace_period(Some("   ")).expect("whitespace should return default");
        assert_eq!(duration, DEFAULT_GRACE_PERIOD);
    }

    #[test]
    fn parse_grace_period_accepts_pure_seconds() {
        let duration = parse_grace_period(Some("45s")).expect("45s should parse");
        assert_eq!(duration, Duration::from_secs(45));
    }

    #[test]
    fn parse_grace_period_accepts_zero() {
        let duration = parse_grace_period(Some("0")).expect("zero should parse");
        assert_eq!(duration, Duration::ZERO);
    }

    #[test]
    fn parse_grace_period_boundary_at_max() {
        let duration = parse_grace_period(Some("3m")).expect("3m should parse");
        assert_eq!(duration, Duration::from_secs(180));
    }

    #[test]
    fn parse_grace_period_rejects_invalid_unit() {
        assert!(parse_grace_period(Some("10x")).is_err());
    }

    #[test]
    fn parse_grace_period_rejects_bare_number() {
        assert!(parse_grace_period(Some("30")).is_err());
    }

    #[test]
    fn parse_grace_period_accepts_hours() {
        let duration = parse_grace_period(Some("0h1m")).expect("0h1m should parse");
        assert_eq!(duration, Duration::from_secs(60));
    }
}
