//! The Commits footer model of the Changes tab (`commits-section.tsx`): the commits of the
//! branch that are not on the base, each with its files once it is opened.

use bachuc_client::{CommitList, CommitSummary};
use std::collections::HashSet;

/// What the footer shows once it is open.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommitsState {
    Loading,
    /// `loadError`. The daemon's message goes to the log.
    Failed,
    /// No commit ahead of the base: `noneAhead` with the base label.
    Empty {
        base: String,
    },
    Ready(Vec<CommitSummary>),
}

impl CommitsState {
    pub fn from_result(result: anyhow::Result<CommitList>) -> CommitsState {
        match result {
            Err(_) => CommitsState::Failed,
            Ok(list) => {
                let base = base_label(list.base_ref.as_deref());
                let commits: Vec<CommitSummary> = list
                    .commits
                    .into_iter()
                    .filter(|commit| commit.is_on_base != Some(true))
                    .collect();
                if commits.is_empty() {
                    CommitsState::Empty { base }
                } else {
                    CommitsState::Ready(commits)
                }
            }
        }
    }

    /// The count after the title; none until the list arrives.
    pub fn count(&self) -> Option<usize> {
        match self {
            CommitsState::Ready(commits) => Some(commits.len()),
            CommitsState::Empty { .. } => Some(0),
            _ => None,
        }
    }
}

/// `normalizeBranchOptionName`, with `base` when the daemon names no base.
pub fn base_label(base_ref: Option<&str>) -> String {
    let name = base_ref.map(str::trim).unwrap_or_default();
    let name = name
        .strip_prefix("refs/heads/")
        .or_else(|| name.strip_prefix("refs/remotes/"))
        .unwrap_or(name);
    let name = name.strip_prefix("origin/").unwrap_or(name);
    if name.is_empty() || name == "HEAD" {
        "base".to_string()
    } else {
        name.to_string()
    }
}

/// One row of the open footer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum CommitRow {
    Commit { index: usize },
    File { commit: usize, file: usize },
}

/// The commit rows, each opened commit followed by its files.
pub fn commit_rows(commits: &[CommitSummary], open: &HashSet<String>) -> Vec<CommitRow> {
    let mut rows = Vec::new();
    for (index, commit) in commits.iter().enumerate() {
        rows.push(CommitRow::Commit { index });
        if open.contains(&commit.sha) {
            rows.extend((0..commit.files.len()).map(|file| CommitRow::File {
                commit: index,
                file,
            }));
        }
    }
    rows
}

/// Seconds since the Unix epoch of an ISO 8601 time such as `2026-10-16T01:00:00Z` or
/// `2026-10-16T08:00:00+07:00`, the forms git and the daemon send.
pub fn parse_iso8601(text: &str) -> Option<i64> {
    let (date, rest) = text.trim().split_once('T')?;
    let mut parts = date.splitn(3, '-');
    let year: i64 = parts.next()?.parse().ok()?;
    let month: i64 = parts.next()?.parse().ok()?;
    let day: i64 = parts.next()?.parse().ok()?;
    let (time, offset) = if let Some(time) = rest.strip_suffix('Z') {
        (time, 0)
    } else if let Some(at) = rest.rfind(['+', '-']) {
        let (time, zone) = rest.split_at(at);
        let sign = if zone.starts_with('-') { -1 } else { 1 };
        let zone = &zone[1..];
        let (hours, minutes) = zone.split_once(':').unwrap_or((zone, "0"));
        let hours: i64 = hours.parse().ok()?;
        let minutes: i64 = minutes.parse().ok()?;
        (time, sign * (hours * 3600 + minutes * 60))
    } else {
        (rest, 0)
    };
    let mut clock = time.split(':');
    let hour: i64 = clock.next()?.parse().ok()?;
    let minute: i64 = clock.next()?.parse().ok()?;
    let second: i64 = clock
        .next()
        .map(|second| second.split('.').next().unwrap_or("0"))
        .unwrap_or("0")
        .parse()
        .ok()?;
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) {
        return None;
    }
    Some(days_from_civil(year, month, day) * 86_400 + hour * 3600 + minute * 60 + second - offset)
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

/// `formatTimeAgo`: `just now`, `12s ago`, `5m ago`, `2h ago`, `3d ago`, then `Oct 9`. The date
/// is in UTC. An unreadable time gives an empty label.
pub fn time_ago(author_date: &str, now: i64) -> String {
    let Some(at) = parse_iso8601(author_date) else {
        return String::new();
    };
    let seconds = now - at;
    let (minutes, hours, days) = (
        seconds.div_euclid(60),
        seconds.div_euclid(3600),
        seconds.div_euclid(86_400),
    );
    if seconds < 10 {
        "just now".to_string()
    } else if minutes < 1 {
        format!("{seconds}s ago")
    } else if hours < 1 {
        format!("{minutes}m ago")
    } else if days < 1 {
        format!("{hours}h ago")
    } else if days < 7 {
        format!("{days}d ago")
    } else {
        const MONTHS: [&str; 12] = [
            "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
        ];
        let (month, day) = civil_month_day(at.div_euclid(86_400));
        format!("{} {day}", MONTHS[month - 1])
    }
}

/// The month (1-12) and day of a day count since 1970-01-01.
fn civil_month_day(days: i64) -> (usize, i64) {
    let z = days + 719_468;
    let era = z.div_euclid(146_097);
    let day_of_era = z - era * 146_097;
    let year_of_era =
        (day_of_era - day_of_era / 1460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let month_index = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * month_index + 2) / 5 + 1;
    let month = if month_index < 10 {
        month_index + 3
    } else {
        month_index - 9
    };
    (month as usize, day)
}
