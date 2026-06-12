//! Shared `?from_date=&to_date=` (YYYY-MM-DD) parsing into inclusive UTC bounds.

use chrono::{DateTime, NaiveDate, TimeZone, Utc};

use crate::domain::shared::error::DomainError;

/// Inclusive date-range filter as `(from, to_exclusive)` UTC instants. Either
/// bound is `None` when unbounded.
pub type DateBounds = (Option<DateTime<Utc>>, Option<DateTime<Utc>>);

/// Parse `from_date`/`to_date` (YYYY-MM-DD) into `(from, to_exclusive)` UTC
/// bounds: `from` at 00:00:00 of `from_date`, and `to_exclusive` at 00:00:00 of
/// the day *after* `to_date` so the range includes all of `to_date`. Empty or
/// missing inputs are `None` (unbounded); malformed input is a `Validation` error.
pub fn parse_bounds(
    from_date: Option<&str>,
    to_date: Option<&str>,
) -> Result<DateBounds, DomainError> {
    Ok((day_start(from_date)?, next_day_start(to_date)?))
}

fn parse(s: &str) -> Result<NaiveDate, DomainError> {
    NaiveDate::parse_from_str(s, "%Y-%m-%d")
        .map_err(|_| DomainError::Validation(format!("invalid date '{s}'; expected YYYY-MM-DD")))
}

fn clean(s: Option<&str>) -> Option<&str> {
    s.map(str::trim).filter(|s| !s.is_empty())
}

fn day_start(s: Option<&str>) -> Result<Option<DateTime<Utc>>, DomainError> {
    match clean(s) {
        None => Ok(None),
        Some(s) => {
            let dt = parse(s)?.and_hms_opt(0, 0, 0).expect("midnight is valid");
            Ok(Some(Utc.from_utc_datetime(&dt)))
        }
    }
}

fn next_day_start(s: Option<&str>) -> Result<Option<DateTime<Utc>>, DomainError> {
    match clean(s) {
        None => Ok(None),
        Some(s) => {
            let next = parse(s)?
                .succ_opt()
                .ok_or_else(|| DomainError::Validation(format!("date '{s}' out of range")))?;
            let dt = next.and_hms_opt(0, 0, 0).expect("midnight is valid");
            Ok(Some(Utc.from_utc_datetime(&dt)))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_an_inclusive_range() {
        let (from, to) = parse_bounds(Some("2026-06-01"), Some("2026-06-30")).unwrap();
        assert_eq!(from.unwrap().to_rfc3339(), "2026-06-01T00:00:00+00:00");
        // `to` is the exclusive start of the next day, so 2026-06-30 is included.
        assert_eq!(to.unwrap().to_rfc3339(), "2026-07-01T00:00:00+00:00");
    }

    #[test]
    fn empty_is_unbounded_and_bad_input_is_validation() {
        assert_eq!(parse_bounds(None, Some("   ")).unwrap(), (None, None));
        assert!(parse_bounds(Some("not-a-date"), None).is_err());
    }
}
