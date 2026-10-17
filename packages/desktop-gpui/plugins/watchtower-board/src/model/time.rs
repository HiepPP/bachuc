//! The JS number and date rules that the Electron client leans on, by hand: `Math.round` rounds
//! halves up, and dates parse by regex instead of a date library.

use regex::Regex;
use std::sync::LazyLock;
use std::time::{SystemTime, UNIX_EPOCH};

/// `Math.round`: halves round up, also below zero (`-2.5` gives `-2`).
pub(crate) fn round_half_up(value: f64) -> i64 {
    (value + 0.5).floor() as i64
}

/// Milliseconds since the Unix epoch.
pub(crate) fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

static ISO: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([0-9]{4})-([0-9]{2})-([0-9]{2})(?:[T ]([0-9]{2}):([0-9]{2})(?::([0-9]{2})(?:\.([0-9]{1,9}))?)?)?(Z|[+-][0-9]{2}:?[0-9]{2})?$",
    )
    .expect("valid regex")
});

/// An ISO 8601 time such as `2026-10-09T10:00:00.000Z`, as epoch milliseconds, the way
/// `Date.parse` reads the daemon's `updatedAt`. `None` when unreadable. A time without a zone
/// counts as UTC.
pub(crate) fn parse_iso_ms(text: &str) -> Option<i64> {
    let parts = ISO.captures(text.trim())?;
    let number = |index: usize| -> i64 {
        parts
            .get(index)
            .map_or(0, |part| part.as_str().parse().unwrap_or(0))
    };
    let (year, month, day) = (number(1), number(2), number(3));
    let (hour, minute, second) = (number(4), number(5), number(6));
    if !(1..=12).contains(&month)
        || !(1..=31).contains(&day)
        || hour > 23
        || minute > 59
        || second > 59
    {
        return None;
    }
    // The first three fraction digits are the milliseconds.
    let millis = parts.get(7).map_or(0, |fraction| {
        let digits: String = fraction
            .as_str()
            .chars()
            .chain("00".chars())
            .take(3)
            .collect();
        digits.parse().unwrap_or(0)
    });
    let offset_minutes = match parts.get(8).map(|zone| zone.as_str()) {
        None | Some("Z") => 0,
        Some(zone) => {
            let sign = if zone.starts_with('-') { -1 } else { 1 };
            let digits: String = zone[1..].chars().filter(char::is_ascii_digit).collect();
            let hours: i64 = digits[..2].parse().ok()?;
            let minutes: i64 = digits[2..].parse().ok()?;
            sign * (hours * 60 + minutes)
        }
    };
    let days = days_from_civil(year, month, day);
    let seconds = days * 86_400 + hour * 3_600 + minute * 60 + second - offset_minutes * 60;
    Some(seconds * 1_000 + millis)
}

/// Days since 1970-01-01 of a proleptic Gregorian date (Howard Hinnant's algorithm).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let year_of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let day_of_year = (153 * month_index + 2) / 5 + day - 1;
    let day_of_era = year_of_era * 365 + year_of_era / 4 - year_of_era / 100 + day_of_year;
    era * 146_097 + day_of_era - 719_468
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn halves_round_up_as_math_round_does() {
        assert_eq!(round_half_up(2.5), 3);
        assert_eq!(round_half_up(2.4), 2);
        assert_eq!(round_half_up(-2.5), -2);
        assert_eq!(round_half_up(-2.6), -3);
        assert_eq!(round_half_up(18.0), 18);
    }

    #[test]
    fn iso_times_parse_to_epoch_milliseconds() {
        assert_eq!(parse_iso_ms("1970-01-01T00:00:00.000Z"), Some(0));
        assert_eq!(
            parse_iso_ms("2026-10-14T12:00:00.000Z"),
            Some(1_791_979_200_000)
        );
        assert_eq!(
            parse_iso_ms("2026-10-14T11:50:00.000Z"),
            Some(1_791_979_200_000 - 10 * 60_000)
        );
        assert_eq!(
            parse_iso_ms("2026-10-14T14:00:00+02:00"),
            Some(1_791_979_200_000)
        );
        assert_eq!(
            parse_iso_ms("2026-10-14T12:00:00.5Z"),
            Some(1_791_979_200_500)
        );
        assert_eq!(parse_iso_ms("2026-10-14"), Some(1_791_936_000_000));
    }

    #[test]
    fn unreadable_times_are_none() {
        assert_eq!(parse_iso_ms("not a date"), None);
        assert_eq!(parse_iso_ms(""), None);
        assert_eq!(parse_iso_ms("2026-13-01T00:00:00Z"), None);
        assert_eq!(parse_iso_ms("2026-10-14T25:00:00Z"), None);
    }
}
