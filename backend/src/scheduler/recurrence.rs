//! Recurrence patterns and next trigger time calculations.

use crate::common::AppError;
use chrono::{
    DateTime, Datelike, Duration, NaiveDate, NaiveDateTime, NaiveTime, TimeZone, Utc, Weekday,
};
use chrono_tz::Tz;
use std::collections::HashSet;
use std::str::FromStr;

/// Supported recurrence patterns for alarms.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum RecurrencePattern {
    Daily,
    Weekdays,
    Weekends,
    CustomDays(HashSet<Weekday>),
}

impl RecurrencePattern {
    pub fn contains(&self, weekday: Weekday) -> bool {
        match self {
            RecurrencePattern::Daily => true,
            RecurrencePattern::Weekdays => matches!(
                weekday,
                Weekday::Mon | Weekday::Tue | Weekday::Wed | Weekday::Thu | Weekday::Fri
            ),
            RecurrencePattern::Weekends => matches!(weekday, Weekday::Sat | Weekday::Sun),
            RecurrencePattern::CustomDays(days) => days.contains(&weekday),
        }
    }
}

/// Parse recurrence string into a pattern or validate it.
/// Returns None if recurrence is empty (one-shot).
pub fn parse_recurrence(raw: Option<&str>) -> Result<Option<RecurrencePattern>, AppError> {
    let s = match raw {
        Some(s) => s.trim(),
        None => return Ok(None),
    };

    if s.is_empty() {
        return Ok(None);
    }

    if s.eq_ignore_ascii_case("daily") {
        return Ok(Some(RecurrencePattern::Daily));
    }

    if s.eq_ignore_ascii_case("weekdays") {
        return Ok(Some(RecurrencePattern::Weekdays));
    }

    if s.eq_ignore_ascii_case("weekends") {
        return Ok(Some(RecurrencePattern::Weekends));
    }

    // Comma-separated days
    let mut days = HashSet::new();
    for token in s.split(',') {
        let t = token.trim().to_ascii_lowercase();
        if t.is_empty() {
            continue;
        }
        let weekday = match t.as_str() {
            "mon" | "monday" => Weekday::Mon,
            "tue" | "tuesday" => Weekday::Tue,
            "wed" | "wednesday" => Weekday::Wed,
            "thu" | "thursday" => Weekday::Thu,
            "fri" | "friday" => Weekday::Fri,
            "sat" | "saturday" => Weekday::Sat,
            "sun" | "sunday" => Weekday::Sun,
            other => {
                return Err(AppError::Config(format!(
                    "Invalid recurrence day: '{other}'. Expected daily, weekdays, weekends, or comma-separated days (mon..sun)."
                )));
            }
        };
        days.insert(weekday);
    }

    if days.is_empty() {
        return Err(AppError::Config(
            "Recurrence specification does not contain any valid days.".to_string(),
        ));
    }

    Ok(Some(RecurrencePattern::CustomDays(days)))
}

/// Parse and validate HH:MM time string.
pub fn parse_time_hh_mm(time_str: &str) -> Result<NaiveTime, AppError> {
    let trimmed = time_str.trim();
    NaiveTime::parse_from_str(trimmed, "%H:%M").map_err(|_| {
        AppError::Config(format!(
            "Invalid time format: '{time_str}'. Expected 'HH:MM' (24-hour clock, e.g. '07:30')."
        ))
    })
}

/// Parse timezone name (IANA or UTC).
pub fn parse_timezone(tz_str: &str) -> Result<Tz, AppError> {
    let trimmed = tz_str.trim();
    if trimmed.is_empty() || trimmed.eq_ignore_ascii_case("utc") {
        return Ok(chrono_tz::UTC);
    }

    Tz::from_str(trimmed).map_err(|_| {
        AppError::Config(format!(
            "Invalid timezone: '{tz_str}'. Must be a valid IANA timezone name (e.g. 'Europe/Moscow', 'America/New_York', 'UTC')."
        ))
    })
}

/// Convert local naive datetime to UTC with timezone transition tolerance.
fn resolve_local_to_utc(tz: Tz, naive: NaiveDateTime) -> Result<DateTime<Utc>, AppError> {
    match tz.from_local_datetime(&naive) {
        chrono::LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
        chrono::LocalResult::Ambiguous(early, _) => Ok(early.with_timezone(&Utc)),
        chrono::LocalResult::None => {
            // Spring forward gap: advance 1 hour into valid local time
            let shifted = naive + Duration::hours(1);
            match tz.from_local_datetime(&shifted) {
                chrono::LocalResult::Single(dt) => Ok(dt.with_timezone(&Utc)),
                chrono::LocalResult::Ambiguous(early, _) => Ok(early.with_timezone(&Utc)),
                chrono::LocalResult::None => Err(AppError::Config(format!(
                    "Unable to resolve local time {} in timezone {}",
                    naive, tz
                ))),
            }
        }
    }
}

/// Calculate the next trigger time in UTC for an alarm, given the current point in time `after`.
pub fn calculate_next_alarm_trigger(
    time_str: &str,
    recurrence_str: Option<&str>,
    tz_str: &str,
    after: DateTime<Utc>,
) -> Result<DateTime<Utc>, AppError> {
    let target_time = parse_time_hh_mm(time_str)?;
    let tz = parse_timezone(tz_str)?;
    let pattern_opt = parse_recurrence(recurrence_str)?;

    let local_now = after.with_timezone(&tz);
    let today = local_now.date_naive();

    match pattern_opt {
        None => {
            // One-shot alarm
            let today_cand = today.and_time(target_time);
            let today_utc = resolve_local_to_utc(tz, today_cand)?;

            if today_utc > after {
                Ok(today_utc)
            } else {
                let tomorrow = today + Duration::days(1);
                let tomorrow_cand = tomorrow.and_time(target_time);
                resolve_local_to_utc(tz, tomorrow_cand)
            }
        }
        Some(pattern) => {
            // Recurring alarm: search days 0..=7
            for day_offset in 0..=7 {
                let cand_date: NaiveDate = today + Duration::days(day_offset);
                if !pattern.contains(cand_date.weekday()) {
                    continue;
                }

                let cand_local = cand_date.and_time(target_time);
                let cand_utc = resolve_local_to_utc(tz, cand_local)?;

                if cand_utc > after {
                    return Ok(cand_utc);
                }
            }

            // Fallback safety (e.g. if exactly on boundary)
            let cand_date = today + Duration::days(7);
            let cand_local = cand_date.and_time(target_time);
            resolve_local_to_utc(tz, cand_local)
        }
    }
}

/// Parse and validate reminder trigger time (ISO-8601).
pub fn parse_reminder_trigger_time(
    trigger_time_str: &str,
    default_tz_str: &str,
) -> Result<DateTime<Utc>, AppError> {
    let trimmed = trigger_time_str.trim();

    // 1. Try RFC3339 / ISO-8601 with timezone or Z
    if let Ok(dt) = DateTime::parse_from_rfc3339(trimmed) {
        return Ok(dt.with_timezone(&Utc));
    }

    // 2. Try NaiveDateTime without timezone (using provided default timezone)
    let formats = [
        "%Y-%m-%dT%H:%M:%S",
        "%Y-%m-%d %H:%M:%S",
        "%Y-%m-%dT%H:%M",
        "%Y-%m-%d %H:%M",
    ];

    for fmt in formats {
        if let Ok(naive) = NaiveDateTime::parse_from_str(trimmed, fmt) {
            let tz = parse_timezone(default_tz_str)?;
            return resolve_local_to_utc(tz, naive);
        }
    }

    Err(AppError::Config(format!(
        "Invalid trigger_time format: '{trigger_time_str}'. Expected ISO-8601 (e.g. '2026-11-01T08:00:00Z')."
    )))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_recurrence() {
        assert_eq!(parse_recurrence(None).unwrap(), None);
        assert_eq!(parse_recurrence(Some("")).unwrap(), None);
        assert_eq!(
            parse_recurrence(Some("daily")).unwrap(),
            Some(RecurrencePattern::Daily)
        );
        assert_eq!(
            parse_recurrence(Some("weekdays")).unwrap(),
            Some(RecurrencePattern::Weekdays)
        );
        assert_eq!(
            parse_recurrence(Some("weekends")).unwrap(),
            Some(RecurrencePattern::Weekends)
        );

        let custom = parse_recurrence(Some("mon, wed, fri")).unwrap().unwrap();
        assert!(custom.contains(Weekday::Mon));
        assert!(custom.contains(Weekday::Wed));
        assert!(custom.contains(Weekday::Fri));
        assert!(!custom.contains(Weekday::Tue));
        assert!(!custom.contains(Weekday::Sun));

        assert!(parse_recurrence(Some("invalid_day")).is_err());
    }

    #[test]
    fn test_calculate_next_alarm_one_shot() {
        // Now: 2026-11-01 07:00:00 UTC (Sunday)
        let now = Utc.with_ymd_and_hms(2026, 11, 1, 7, 0, 0).unwrap();

        // Alarm for 08:00 today (in future)
        let next = calculate_next_alarm_trigger("08:00", None, "UTC", now).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 11, 1, 8, 0, 0).unwrap());

        // Alarm for 06:00 today (already passed -> tomorrow)
        let next_tomorrow = calculate_next_alarm_trigger("06:00", None, "UTC", now).unwrap();
        assert_eq!(
            next_tomorrow,
            Utc.with_ymd_and_hms(2026, 11, 2, 6, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_calculate_next_alarm_weekdays() {
        // Sunday 2026-11-01 07:00:00 UTC
        let now = Utc.with_ymd_and_hms(2026, 11, 1, 7, 0, 0).unwrap();

        // Weekdays alarm at 08:00 -> Next must be Monday Nov 2 at 08:00
        let next = calculate_next_alarm_trigger("08:00", Some("weekdays"), "UTC", now).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 11, 2, 8, 0, 0).unwrap());

        // Friday 2026-11-06 09:00:00 UTC (08:00 passed) -> Next must be Monday Nov 9 at 08:00
        let friday_after = Utc.with_ymd_and_hms(2026, 11, 6, 9, 0, 0).unwrap();
        let next_mon =
            calculate_next_alarm_trigger("08:00", Some("weekdays"), "UTC", friday_after).unwrap();
        assert_eq!(
            next_mon,
            Utc.with_ymd_and_hms(2026, 11, 9, 8, 0, 0).unwrap()
        );
    }

    #[test]
    fn test_calculate_next_alarm_with_timezone() {
        // Moscow is UTC+3
        // Now: 2026-11-01 04:00:00 UTC -> 07:00:00 MSK
        let now = Utc.with_ymd_and_hms(2026, 11, 1, 4, 0, 0).unwrap();

        // Alarm at 07:30 MSK (which is 04:30 UTC today)
        let next = calculate_next_alarm_trigger("07:30", None, "Europe/Moscow", now).unwrap();
        assert_eq!(next, Utc.with_ymd_and_hms(2026, 11, 1, 4, 30, 0).unwrap());

        // Alarm at 06:30 MSK (already passed -> tomorrow 06:30 MSK = 03:30 UTC Nov 2)
        let next_tomorrow =
            calculate_next_alarm_trigger("06:30", None, "Europe/Moscow", now).unwrap();
        assert_eq!(
            next_tomorrow,
            Utc.with_ymd_and_hms(2026, 11, 2, 3, 30, 0).unwrap()
        );
    }

    #[test]
    fn test_parse_reminder_trigger_time() {
        let parsed = parse_reminder_trigger_time("2026-11-01T15:30:00Z", "UTC").unwrap();
        assert_eq!(
            parsed,
            Utc.with_ymd_and_hms(2026, 11, 1, 15, 30, 0).unwrap()
        );

        // Offset format
        let parsed_offset =
            parse_reminder_trigger_time("2026-11-01T18:30:00+03:00", "UTC").unwrap();
        assert_eq!(
            parsed_offset,
            Utc.with_ymd_and_hms(2026, 11, 1, 15, 30, 0).unwrap()
        );
    }
}
