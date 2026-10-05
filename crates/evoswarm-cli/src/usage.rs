//! `evoswarm usage` — daily token/cost summary over the gateway usage log (e4-2, AC2).
//!
//! A thin presentation layer over [`evoswarm_gateway::UsageStore`]: parse `--since`
//! (YYYY-MM-DD, UTC midnight inclusive; defaults to 7 days back), aggregate per day and
//! print totals plus estimated cost. Formatting lives in [`format_usage_report`] so it is
//! unit-testable without spawning the binary.

use std::path::Path;

use evoswarm_gateway::usage::{DailyUsage, UsagePricing, UsageStore};

/// Seconds per UTC day (the granularity of the summary).
pub const SECS_PER_DAY: u64 = 86_400;

/// Errors running the usage report.
#[derive(Debug, thiserror::Error)]
pub enum UsageCliError {
    #[error("--since must be a UTC date in YYYY-MM-DD form, got {0:?}")]
    BadSince(String),
    #[error("usage database error: {0}")]
    Store(#[from] evoswarm_gateway::UsageError),
}

/// Parses a `YYYY-MM-DD` date into its UTC-midnight unix timestamp. Hand-rolled (no chrono
/// dependency): days-from-civil is exact for the proleptic Gregorian calendar.
pub fn parse_since_date(date: &str) -> Result<u64, UsageCliError> {
    let bad = || UsageCliError::BadSince(date.to_string());
    let mut it = date.split('-');
    let (y, m, d) = match (
        it.next().and_then(|s| s.parse::<i64>().ok()),
        it.next().and_then(|s| s.parse::<u32>().ok()),
        it.next(),
    ) {
        (Some(y), Some(m), Some(d)) if it.next().is_none() && d.len() == 2 => {
            (y, m, d.parse::<u32>().map_err(|_| bad())?)
        }
        _ => return Err(bad()),
    };
    if !(1..=12).contains(&m) || !(1..=31).contains(&d) {
        return Err(bad());
    }
    // Howard Hinnant's days_from_civil, valid for the full i64 year range.
    let y_shift = if m <= 2 { y - 1 } else { y };
    let era = if y_shift >= 0 { y_shift } else { y_shift - 399 } / 400;
    let yoe = y_shift - era * 400;
    let mp = (m as i64 + 9) % 12; // Mar=0..Feb=11
    let doy = (153 * mp + 2) / 5 + d as i64 - 1;
    let doe = yoe * 365 + yoe / 4 - yoe / 100 + doy;
    let days = era * 146_097 + doe - 719_468;
    Ok((days * SECS_PER_DAY as i64) as u64)
}

/// Formats the per-day report plus a totals line. Empty input yields an explicit
/// "no usage recorded" line rather than a blank report.
pub fn format_usage_report(days: &[DailyUsage], pricing: &UsagePricing) -> String {
    if days.is_empty() {
        return "no usage recorded since the given date\n".to_string();
    }
    let mut out = String::new();
    out.push_str("day         requests  in_tokens  out_tokens  cached  cost_usd\n");
    let (mut r, mut i, mut o, mut c, mut cost) = (0u64, 0u64, 0u64, 0u64, 0.0f64);
    for d in days {
        out.push_str(&format!(
            "{:<11} {:>8}  {:>9}  {:>10}  {:>6}  {:>8.4}\n",
            d.day, d.requests, d.input_tokens, d.output_tokens, d.cached_tokens, d.cost_usd
        ));
        r += d.requests;
        i += d.input_tokens;
        o += d.output_tokens;
        c += d.cached_tokens;
        cost += d.cost_usd;
    }
    out.push_str(&format!(
        "TOTAL       {:>8}  {:>9}  {:>10}  {:>6}  {:>8.4}\n",
        r, i, o, c, cost
    ));
    let _ = pricing; // pricing already applied in `daily_summary`; kept for symmetry.
    out
}

/// Default `--since` horizon when the flag is omitted: the last 7 days.
pub fn default_since_unix(now_unix: u64) -> u64 {
    now_unix.saturating_sub(7 * SECS_PER_DAY)
}

/// Runs the report end-to-end: open the store at `db_path`, aggregate from `since_unix`
/// and return the printable report.
pub fn run_usage_report(
    db_path: &Path,
    since_unix: u64,
    pricing: &UsagePricing,
) -> Result<String, UsageCliError> {
    let store = UsageStore::open(db_path)?;
    let days = store.daily_summary(since_unix, pricing)?;
    Ok(format_usage_report(&days, pricing))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_known_dates() {
        // 2025-07-03T00:00:00Z = 1751500800 (verified against `date -d 2025-07-03 +%s`).
        assert_eq!(parse_since_date("2025-07-03").unwrap(), 1_751_500_800);
        assert_eq!(parse_since_date("1970-01-01").unwrap(), 0);
        assert_eq!(parse_since_date("2026-10-05").unwrap(), 1_791_158_400);
    }

    #[test]
    fn rejects_malformed_dates() {
        for bad in [
            "2025-7-3",
            "not-a-date",
            "2025-13-01",
            "2025-00-10",
            "2025-07-03-04",
        ] {
            assert!(
                matches!(parse_since_date(bad), Err(UsageCliError::BadSince(_))),
                "{bad:?} must be rejected"
            );
        }
    }

    #[test]
    fn report_includes_totals_and_empty_case() {
        let pricing = UsagePricing::default();
        assert_eq!(
            format_usage_report(&[], &pricing),
            "no usage recorded since the given date\n"
        );
        let days = vec![DailyUsage {
            day: "2025-07-03".into(),
            requests: 2,
            input_tokens: 300,
            output_tokens: 110,
            cached_tokens: 10,
            cost_usd: 0.002553,
        }];
        let out = format_usage_report(&days, &pricing);
        assert!(out.contains("2025-07-03"), "day row present");
        assert!(out.contains("TOTAL"), "totals line present");
        assert!(out.contains("300") && out.contains("110"), "token totals");
    }

    #[test]
    fn default_since_is_seven_days() {
        assert_eq!(default_since_unix(1_000_000), 1_000_000 - 7 * SECS_PER_DAY);
        assert_eq!(default_since_unix(0), 0, "saturates at the epoch");
    }
}
