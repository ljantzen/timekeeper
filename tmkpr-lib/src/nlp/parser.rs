use chrono::{DateTime, Local, TimeZone, Timelike, Utc};
use interim::{parse_date_string, Dialect};
use serde::{Deserialize, Serialize};

use crate::error::{TmkprError, TmkprResult};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "lowercase")]
pub enum TimeFormat {
    #[default]
    #[serde(rename = "24h")]
    H24,
    #[serde(rename = "12h")]
    H12,
}

/// Parse a human-readable date/time string relative to `now` (UTC).
///
/// Supported: "2 hours ago", "yesterday", "last monday at 9am",
/// "today at 14:30", "2025-04-01", "9am", ISO 8601, and more.
/// In H24 mode, bare "HH:MM" and "H:MM" are also accepted.
pub fn parse_datetime(
    input: &str,
    now: DateTime<Utc>,
    format: TimeFormat,
) -> TmkprResult<DateTime<Utc>> {
    let local_now = now.with_timezone(&Local);

    if format == TimeFormat::H24 && is_bare_time(input) {
        return parse_bare_time_24h(input, &local_now)
            .map(|dt| dt.with_timezone(&Utc))
            .ok_or_else(|| TmkprError::DateParse {
                input: input.to_string(),
                reason: "invalid 24h time (expected H:MM or HH:MM with hours 0–23)".to_string(),
            });
    }

    let dialect = match format {
        TimeFormat::H24 => Dialect::Uk,
        TimeFormat::H12 => Dialect::Us,
    };

    parse_date_string(input, local_now, dialect)
        .map(|dt| dt.with_timezone(&Utc))
        .map_err(|e| TmkprError::DateParse {
            input: input.to_string(),
            reason: e.to_string(),
        })
}

pub fn parse_datetime_now(input: &str, format: TimeFormat) -> TmkprResult<DateTime<Utc>> {
    parse_datetime(input, Utc::now(), format)
}

/// Parse a pair of start/end datetime strings with smart defaults:
/// - If both lack dates, assume today
/// - If one lacks a date, use the same date as the other
/// - If end < start and on same date, assume end is next day (midnight crossing)
/// - If end < start and on different dates, return error
pub fn parse_datetime_pair(
    start_str: &str,
    end_str: &str,
    now: DateTime<Utc>,
    format: TimeFormat,
) -> TmkprResult<(DateTime<Utc>, DateTime<Utc>)> {
    let has_date = |s: &str| {
        s.contains('-') || s.contains('/') ||
        s.to_lowercase().contains("today") ||
        s.to_lowercase().contains("yesterday") ||
        s.to_lowercase().contains("ago") ||
        s.to_lowercase().contains("week")
    };

    let start_has_date = has_date(start_str);
    let end_has_date = has_date(end_str);

    let start_dt = parse_datetime(start_str, now, format)?;
    let mut end_dt = parse_datetime(end_str, now, format)?;

    match (start_has_date, end_has_date) {
        (false, false) => {
            // Both are times only. If end < start, assume end is next day (midnight crossing).
            if end_dt < start_dt {
                end_dt += chrono::Duration::days(1);
            }
        }
        (true, false) => {
            // Start has date, end doesn't. Use start's date for end.
            let start_date = start_dt.with_timezone(&Local).date_naive();
            let end_time = end_dt.with_timezone(&Local).time();
            let new_end = start_date
                .and_hms_opt(end_time.hour(), end_time.minute(), end_time.second())
                .and_then(|naive| Local.from_local_datetime(&naive).single())
                .ok_or_else(|| TmkprError::DateParse {
                    input: end_str.to_string(),
                    reason: "failed to combine date and time".to_string(),
                })?;
            end_dt = new_end.with_timezone(&Utc);
        }
        (false, true) => {
            // End has date, start doesn't. Use end's date for start.
            let end_date = end_dt.with_timezone(&Local).date_naive();
            let start_time = start_dt.with_timezone(&Local).time();
            let new_start = end_date
                .and_hms_opt(start_time.hour(), start_time.minute(), start_time.second())
                .and_then(|naive| Local.from_local_datetime(&naive).single())
                .ok_or_else(|| TmkprError::DateParse {
                    input: start_str.to_string(),
                    reason: "failed to combine date and time".to_string(),
                })?;
            let start_dt_new = new_start.with_timezone(&Utc);

            // If start > end now, end must be on the next day
            if start_dt_new > end_dt {
                end_dt += chrono::Duration::days(1);
            }
            return Ok((start_dt_new, end_dt));
        }
        (true, true) => {
            // Both have dates. Check if end < start.
            if end_dt < start_dt {
                // Check if they're on the same day
                let start_date = start_dt.with_timezone(&Local).date_naive();
                let end_date = end_dt.with_timezone(&Local).date_naive();
                if start_date == end_date {
                    // Assume end is next day
                    end_dt += chrono::Duration::days(1);
                } else {
                    // Different dates and end is before start - error
                    return Err(TmkprError::DateParse {
                        input: format!("{} to {}", start_str, end_str),
                        reason: "end time is before start time".to_string(),
                    });
                }
            }
        }
    }

    if end_dt <= start_dt {
        return Err(TmkprError::DateParse {
            input: format!("{} to {}", start_str, end_str),
            reason: "end time must be after start time".to_string(),
        });
    }

    Ok((start_dt, end_dt))
}

/// Returns true when input is purely digits and colons in H:MM or H:MM:SS shape.
fn is_bare_time(input: &str) -> bool {
    let mut parts = input.splitn(3, ':');
    let h = parts.next().unwrap_or("");
    let m = parts.next().unwrap_or("");
    let s = parts.next().unwrap_or("0");
    !h.is_empty()
        && !m.is_empty()
        && !s.is_empty()
        && h.chars().all(|c| c.is_ascii_digit())
        && m.chars().all(|c| c.is_ascii_digit())
        && s.chars().all(|c| c.is_ascii_digit())
}

/// Handles bare `H:MM`, `HH:MM`, `H:MM:SS`, `HH:MM:SS` as today's time (24h).
/// Returns None for anything that doesn't match that exact shape.
fn parse_bare_time_24h(input: &str, now: &DateTime<Local>) -> Option<DateTime<Local>> {
    let mut parts = input.splitn(3, ':');
    let h_str = parts.next()?;
    let m_str = parts.next()?;
    let s_str = parts.next().unwrap_or("0");

    if !h_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !m_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if !s_str.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    if h_str.is_empty() || m_str.is_empty() || s_str.is_empty() {
        return None;
    }

    let h: u32 = h_str.parse().ok()?;
    let m: u32 = m_str.parse().ok()?;
    let s: u32 = s_str.parse().ok()?;

    if h > 23 || m > 59 || s > 59 {
        return None;
    }

    now.date_naive()
        .and_hms_opt(h, m, s)
        .and_then(|naive| Local.from_local_datetime(&naive).single())
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::{TimeZone, Timelike};

    fn fixed_now() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2025, 4, 7, 12, 0, 0).unwrap()
    }

    #[test]
    fn parses_iso_date() {
        let dt = parse_datetime("2025-04-01", fixed_now(), TimeFormat::H24).unwrap();
        let local_date = dt.with_timezone(&Local).date_naive().to_string();
        assert_eq!(local_date, "2025-04-01");
    }

    #[test]
    fn parses_hours_ago() {
        let dt = parse_datetime("2 hours ago", fixed_now(), TimeFormat::H24).unwrap();
        let expected = fixed_now() - chrono::Duration::hours(2);
        assert_eq!(dt, expected);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse_datetime("not a date at all !@#", fixed_now(), TimeFormat::H24).is_err());
    }

    #[test]
    fn parses_24h_time_hhmm() {
        let now = fixed_now();
        let dt = parse_datetime("14:50", now, TimeFormat::H24).unwrap();
        let local = dt.with_timezone(&Local);
        assert_eq!(local.hour(), 14);
        assert_eq!(local.minute(), 50);
    }

    #[test]
    fn parses_24h_time_leading_zero() {
        let now = fixed_now();
        let dt = parse_datetime("07:51", now, TimeFormat::H24).unwrap();
        let local = dt.with_timezone(&Local);
        assert_eq!(local.hour(), 7);
        assert_eq!(local.minute(), 51);
    }

    #[test]
    fn parses_24h_time_with_seconds() {
        let now = fixed_now();
        let dt = parse_datetime("14:30:45", now, TimeFormat::H24).unwrap();
        let local = dt.with_timezone(&Local);
        assert_eq!(local.hour(), 14);
        assert_eq!(local.minute(), 30);
        assert_eq!(local.second(), 45);
    }

    #[test]
    fn rejects_invalid_24h_hour() {
        assert!(parse_datetime("25:00", fixed_now(), TimeFormat::H24).is_err());
    }

    #[test]
    fn rejects_invalid_24h_minute() {
        assert!(parse_datetime("10:61", fixed_now(), TimeFormat::H24).is_err());
    }

    #[test]
    fn parses_12h_am_pm() {
        let dt = parse_datetime("9am", fixed_now(), TimeFormat::H12).unwrap();
        assert_eq!(dt.with_timezone(&Local).hour(), 9);
        let dt2 = parse_datetime("2pm", fixed_now(), TimeFormat::H12).unwrap();
        assert_eq!(dt2.with_timezone(&Local).hour(), 14);
    }

    // --- Pair parsing tests ---

    #[test]
    fn pair_both_times_only_assumes_today() {
        let now = fixed_now();
        let (start, end) = parse_datetime_pair("09:00", "17:30", now, TimeFormat::H24).unwrap();
        assert_eq!(start.with_timezone(&Local).hour(), 9);
        assert_eq!(end.with_timezone(&Local).hour(), 17);
        assert!(end > start);
    }

    #[test]
    fn pair_start_has_date_end_doesnt_uses_start_date() {
        let now = fixed_now();
        let (start, end) = parse_datetime_pair("2025-05-10 09:00", "17:30", now, TimeFormat::H24).unwrap();
        let start_date = start.with_timezone(&Local).date_naive();
        let end_date = end.with_timezone(&Local).date_naive();
        assert_eq!(start_date, end_date);
        assert_eq!(end_date.to_string(), "2025-05-10");
    }

    #[test]
    fn pair_end_has_date_start_doesnt_uses_end_date() {
        let now = fixed_now();
        let (start, end) = parse_datetime_pair("09:00", "2025-05-10 17:30", now, TimeFormat::H24).unwrap();
        let start_date = start.with_timezone(&Local).date_naive();
        let end_date = end.with_timezone(&Local).date_naive();
        assert_eq!(start_date, end_date);
        assert_eq!(start_date.to_string(), "2025-05-10");
    }

    #[test]
    fn pair_same_date_midnight_crossing() {
        let now = fixed_now();
        let (start, end) = parse_datetime_pair("2025-05-10 23:00", "2025-05-10 01:00", now, TimeFormat::H24).unwrap();
        let start_date = start.with_timezone(&Local).date_naive();
        let end_date = end.with_timezone(&Local).date_naive();
        assert_eq!(start_date.to_string(), "2025-05-10");
        assert_eq!(end_date.to_string(), "2025-05-11");
    }

    #[test]
    fn pair_end_before_start_same_date_errors() {
        let now = fixed_now();
        let result = parse_datetime_pair("2025-05-10 17:00", "2025-05-10 09:00", now, TimeFormat::H24);
        assert!(result.is_ok()); // Should auto-adjust to next day
    }

    #[test]
    fn pair_end_before_start_different_dates_errors() {
        let now = fixed_now();
        let result = parse_datetime_pair("2025-05-10 17:00", "2025-05-09 17:00", now, TimeFormat::H24);
        assert!(result.is_err());
    }

    #[test]
    fn pair_times_with_no_date_and_end_earlier_assumes_next_day() {
        let now = fixed_now();
        let (start, end) = parse_datetime_pair("22:00", "02:00", now, TimeFormat::H24).unwrap();
        let start_date = start.with_timezone(&Local).date_naive();
        let end_date = end.with_timezone(&Local).date_naive();
        assert!(end_date > start_date);
    }
}
