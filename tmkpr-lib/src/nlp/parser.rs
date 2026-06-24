use chrono::{DateTime, Local, TimeZone, Utc};
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

/// Parse a human-readable duration string into a `chrono::Duration`.
///
/// Accepts `HH:MM`, `HH:MM:SS`, unit-suffixed strings like `1h30m`, `90m`,
/// `45s`, and bare integers (treated as seconds).
pub fn parse_duration(s: &str) -> TmkprResult<chrono::Duration> {
    let s = s.trim();
    if s.is_empty() {
        return Err(TmkprError::DateParse {
            input: s.to_string(),
            reason: "empty duration".to_string(),
        });
    }
    let parts: Vec<&str> = s.splitn(3, ':').collect();
    if parts.len() >= 2
        && parts
            .iter()
            .all(|p| p.chars().all(|c| c.is_ascii_digit() || c == '.'))
    {
        let h: i64 = parts[0].parse().map_err(|_| TmkprError::DateParse {
            input: s.to_string(),
            reason: "invalid hours in HH:MM duration".to_string(),
        })?;
        let m: i64 = parts[1].parse().map_err(|_| TmkprError::DateParse {
            input: s.to_string(),
            reason: "invalid minutes in HH:MM duration".to_string(),
        })?;
        let sec: i64 = if parts.len() == 3 {
            parts[2]
                .split('.')
                .next()
                .unwrap_or("0")
                .parse()
                .map_err(|_| TmkprError::DateParse {
                    input: s.to_string(),
                    reason: "invalid seconds in HH:MM:SS duration".to_string(),
                })?
        } else {
            0
        };
        return Ok(chrono::Duration::seconds(h * 3600 + m * 60 + sec));
    }
    let lower = s
        .to_lowercase()
        .replace("hours", "h")
        .replace("hour", "h")
        .replace("minutes", "m")
        .replace("minute", "m")
        .replace("mins", "m")
        .replace("min", "m")
        .replace("seconds", "s")
        .replace("second", "s")
        .replace("secs", "s")
        .replace("sec", "s");
    let mut total_secs: i64 = 0;
    let mut buf = String::new();
    let mut found_unit = false;
    for ch in lower.chars() {
        match ch {
            '0'..='9' | '.' => buf.push(ch),
            'h' => {
                let v: f64 = buf.trim().parse().map_err(|_| TmkprError::DateParse {
                    input: s.to_string(),
                    reason: "invalid hours value".to_string(),
                })?;
                total_secs += (v * 3600.0) as i64;
                buf.clear();
                found_unit = true;
            }
            'm' => {
                let v: f64 = buf.trim().parse().map_err(|_| TmkprError::DateParse {
                    input: s.to_string(),
                    reason: "invalid minutes value".to_string(),
                })?;
                total_secs += (v * 60.0) as i64;
                buf.clear();
                found_unit = true;
            }
            's' => {
                let v: f64 = buf.trim().parse().map_err(|_| TmkprError::DateParse {
                    input: s.to_string(),
                    reason: "invalid seconds value".to_string(),
                })?;
                total_secs += v as i64;
                buf.clear();
                found_unit = true;
            }
            ' ' | '_' => {}
            _ => {
                return Err(TmkprError::DateParse {
                    input: s.to_string(),
                    reason: format!("unexpected character '{ch}'"),
                })
            }
        }
    }
    if found_unit {
        return Ok(chrono::Duration::seconds(total_secs));
    }
    if let Ok(secs) = s.parse::<f64>() {
        return Ok(chrono::Duration::seconds(secs as i64));
    }
    Err(TmkprError::DateParse {
        input: s.to_string(),
        reason: "cannot parse duration".to_string(),
    })
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

    #[test]
    fn duration_hms() {
        assert_eq!(parse_duration("1:30:00").unwrap().num_seconds(), 5400);
    }

    #[test]
    fn duration_hm() {
        assert_eq!(parse_duration("1:30").unwrap().num_seconds(), 5400);
    }

    #[test]
    fn duration_natural_h_m() {
        assert_eq!(parse_duration("1h30m").unwrap().num_seconds(), 5400);
        assert_eq!(parse_duration("1h 30m").unwrap().num_seconds(), 5400);
    }

    #[test]
    fn duration_minutes_only() {
        assert_eq!(parse_duration("45m").unwrap().num_seconds(), 2700);
        assert_eq!(parse_duration("45min").unwrap().num_seconds(), 2700);
    }

    #[test]
    fn duration_empty_errors() {
        assert!(parse_duration("").is_err());
    }

    #[test]
    fn duration_garbage_errors() {
        assert!(parse_duration("xyz").is_err());
    }

    #[test]
    fn duration_bare_seconds_integer() {
        assert_eq!(parse_duration("3600").unwrap().num_seconds(), 3600);
    }

    #[test]
    fn duration_seconds_unit() {
        assert_eq!(parse_duration("30s").unwrap().num_seconds(), 30);
    }

    #[test]
    fn duration_seconds_unit_spelled_out() {
        assert_eq!(parse_duration("30sec").unwrap().num_seconds(), 30);
    }
}
