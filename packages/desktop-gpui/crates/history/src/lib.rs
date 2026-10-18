//! `bachuc-history`: the History screen of the GPUI client, as the Electron Sessions screen
//! (`packages/app/src/screens/sessions-screen.tsx`) and its rows in `components/agent-list.tsx`.
//!
//! `HistoryScreen` emits `HistoryEvent::OpenAgent` on a row click; the app opens the agent. The
//! rules below (sort, date groups, match highlight, time ago, body and footer state) hold no GPUI
//! state and have their own tests.

mod screen;

use std::ops::Range;
use std::time::Duration;

use bachuc_client::HistoryEntry;
use chrono::{DateTime, Datelike as _, FixedOffset, TimeZone, Utc};

pub use screen::{HistoryEvent, HistoryScreen};

/// `SEARCH_DEBOUNCE_MS`: long enough that a typed word is one request.
pub const SEARCH_DEBOUNCE: Duration = Duration::from_millis(200);

pub const TITLE: &str = "History";
pub const SEARCH_PLACEHOLDER: &str = "Search history";
pub const CLEAR_SEARCH: &str = "Clear search";
pub const EMPTY_TEXT: &str = "No sessions yet";
pub const NO_MATCHES_TEXT: &str = "No sessions match";
pub const TOO_MANY_MATCHES_TEXT: &str = "Too many matches — narrow your search";
pub const LOAD_ERROR_TEXT: &str = "Unable to load sessions";
pub const TRY_AGAIN: &str = "Try again";
pub const LOAD_MORE: &str = "Load more";
pub const LOADING_MORE: &str = "Loading...";
pub const FALLBACK_TITLE: &str = "New session";
pub const ARCHIVED_BADGE: &str = "Archived";

/// `agentList.badges.pending`.
pub fn pending_badge(count: usize) -> String {
    format!("{count} pending")
}

/// The date groups of `agent-list.tsx`, in display order.
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum DateSection {
    Today,
    Yesterday,
    ThisWeek,
    ThisMonth,
    Older,
}

impl DateSection {
    pub fn label(self) -> &'static str {
        match self {
            DateSection::Today => "Today",
            DateSection::Yesterday => "Yesterday",
            DateSection::ThisWeek => "This week",
            DateSection::ThisMonth => "This month",
            DateSection::Older => "Older",
        }
    }
}

/// `deriveDateSectionKey`, on local day numbers: 7 days or less is this week, 30 or less this
/// month. A later day than today (a clock skew) counts as today.
pub fn date_section(activity_day: i64, today: i64) -> DateSection {
    match today - activity_day {
        ..=0 => DateSection::Today,
        1 => DateSection::Yesterday,
        2..=7 => DateSection::ThisWeek,
        8..=30 => DateSection::ThisMonth,
        _ => DateSection::Older,
    }
}

/// The last activity time of an entry; `None` when it is missing or unreadable.
pub fn activity_time(entry: &HistoryEntry) -> Option<DateTime<FixedOffset>> {
    DateTime::parse_from_rfc3339(entry.last_activity_at.as_deref()?).ok()
}

/// The day number of `at` in `tz`, for [`date_section`].
pub fn day_number<Tz: TimeZone>(at: &DateTime<FixedOffset>, tz: &Tz) -> i64 {
    i64::from(at.with_timezone(tz).date_naive().num_days_from_ce())
}

/// `sortByLatestActivity`: newest first. The sort is stable, and an entry with no time goes last.
pub fn sort_newest_first(entries: &mut [HistoryEntry]) {
    entries.sort_by_cached_key(|entry| {
        std::cmp::Reverse(activity_time(entry).map(|at| at.timestamp_millis()))
    });
}

/// Appends a page, skips agents the list already has, and sorts the result.
pub fn merge_page(entries: &mut Vec<HistoryEntry>, page: Vec<HistoryEntry>) {
    for entry in page {
        if !entries.iter().any(|known| known.agent_id == entry.agent_id) {
            entries.push(entry);
        }
    }
    sort_newest_first(entries);
}

/// One list row: a date group header, or the entry at an index.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum HistoryRow {
    Header(DateSection),
    Entry(usize),
}

/// The rows of `flatItems`: each non-empty group in order, its header first, its entries in list
/// order. `days[i]` is the local day of `entries[i]`; an entry with no day goes to Older.
pub fn group_rows(days: &[Option<i64>], today: i64) -> Vec<HistoryRow> {
    let sections: Vec<DateSection> = days
        .iter()
        .map(|day| day.map_or(DateSection::Older, |day| date_section(day, today)))
        .collect();
    let mut rows = Vec::with_capacity(days.len() + 5);
    for section in [
        DateSection::Today,
        DateSection::Yesterday,
        DateSection::ThisWeek,
        DateSection::ThisMonth,
        DateSection::Older,
    ] {
        let mut members = (0..sections.len())
            .filter(|&i| sections[i] == section)
            .peekable();
        if members.peek().is_some() {
            rows.push(HistoryRow::Header(section));
            rows.extend(members.map(HistoryRow::Entry));
        }
    }
    rows
}

/// `formatTimeAgo`: `just now`, `12s ago`, `5m ago`, `2h ago`, `3d ago`, then `Oct 9` in `tz`.
pub fn time_ago<Tz: TimeZone>(at: &DateTime<FixedOffset>, now: &DateTime<Utc>, tz: &Tz) -> String {
    let seconds = (now.timestamp_millis() - at.timestamp_millis()).div_euclid(1000);
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
        at.with_timezone(tz)
            .naive_local()
            .format("%b %-d")
            .to_string()
    }
}

/// The search the screen sends: the debounced input trimmed, or `None` when it is empty or the
/// host cannot search.
pub fn effective_search(input: &str, supported: bool) -> Option<String> {
    let search = input.trim();
    (supported && !search.is_empty()).then(|| search.to_string())
}

/// What the screen body shows, as `sessions-screen.tsx` picks it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Body {
    Loading,
    Error,
    /// The empty text, and whether the Clear search button shows.
    Empty {
        text: &'static str,
        clear_search: bool,
    },
    List,
}

/// The body for the load state. A failed load with rows keeps the rows.
pub fn body(initial_load: bool, failed: bool, entry_count: usize, searching: bool) -> Body {
    if initial_load {
        Body::Loading
    } else if failed && entry_count == 0 {
        Body::Error
    } else if entry_count == 0 {
        Body::Empty {
            text: if searching {
                NO_MATCHES_TEXT
            } else {
                EMPTY_TEXT
            },
            clear_search: searching,
        }
    } else {
        Body::List
    }
}

/// The list footer.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Footer {
    None,
    TooManyMatches,
    LoadMore { loading: bool },
}

/// `listFooterComponent`: a truncated search shows the hint instead of Load more.
pub fn footer(searching: bool, search_truncated: bool, has_more: bool, loading: bool) -> Footer {
    if searching && search_truncated {
        Footer::TooManyMatches
    } else if has_more {
        Footer::LoadMore { loading }
    } else {
        Footer::None
    }
}

/// `findHighlightRanges` with `text-match.ts`: each query token is scored against the text
/// (substring, then subsequence within a word, then a typo match), and the ranges where it landed
/// are merged. The ranges are byte ranges of `text`.
pub fn highlight_ranges(query: &str, text: &str) -> Vec<Range<usize>> {
    let chars: Vec<(usize, char)> = text.char_indices().collect();
    let lower: Vec<char> = chars.iter().map(|&(_, c)| lower_char(c)).collect();
    let mut ranges: Vec<Range<usize>> = query
        .split_whitespace()
        .filter_map(|token| {
            let token: Vec<char> = token.chars().map(lower_char).collect();
            let score = score_match(&token, &lower, fuzzy_policy(token.len()))?;
            Some(match_ranges(&token, &lower, score))
        })
        .flatten()
        .collect();
    ranges.sort_by_key(|range| range.start);
    let mut merged: Vec<Range<usize>> = Vec::new();
    for range in ranges {
        match merged.last_mut() {
            Some(last) if range.start <= last.end => last.end = last.end.max(range.end),
            _ => merged.push(range),
        }
    }
    let byte = |index: usize| chars.get(index).map_or(text.len(), |&(at, _)| at);
    merged
        .into_iter()
        .map(|range| byte(range.start)..byte(range.end))
        .collect()
}

/// One char per char, so char indices of the lowered text match the source.
fn lower_char(c: char) -> char {
    c.to_lowercase().next().unwrap_or(c)
}

fn is_word_char(c: char) -> bool {
    c.is_ascii_lowercase() || c.is_ascii_digit()
}

const TIER_EXACT: u8 = 0;
const TIER_WHOLE_WORD: u8 = 1;
const TIER_PREFIX: u8 = 2;
const TIER_WORD_START: u8 = 3;
const TIER_SUBSTRING: u8 = 4;
const TIER_SUBSEQUENCE: u8 = 5;
const TIER_FUZZY: u8 = 6;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
struct Score {
    tier: u8,
    offset: usize,
    spread: usize,
}

#[derive(Clone, Copy)]
struct FuzzyPolicy {
    max_edits: usize,
    transpositions_only: bool,
}

/// `fuzzyPolicyForToken`.
fn fuzzy_policy(len: usize) -> Option<FuzzyPolicy> {
    let policy = |max_edits, transpositions_only| FuzzyPolicy {
        max_edits,
        transpositions_only,
    };
    match len {
        0..=3 => None,
        4 => Some(policy(1, true)),
        5..=7 => Some(policy(1, false)),
        _ => Some(policy(2, false)),
    }
}

fn score_match(q: &[char], t: &[char], fuzzy: Option<FuzzyPolicy>) -> Option<Score> {
    if q.is_empty() || q == t {
        return Some(Score {
            tier: TIER_EXACT,
            offset: 0,
            spread: 0,
        });
    }
    score_substring(q, t)
        .or_else(|| score_subsequence(q, t))
        .or_else(|| fuzzy.and_then(|policy| score_fuzzy(q, t, policy)))
}

fn score_substring(q: &[char], t: &[char]) -> Option<Score> {
    if q.len() > t.len() {
        return None;
    }
    (0..=t.len() - q.len())
        .filter(|&found| t[found..found + q.len()] == *q)
        .map(|found| {
            let starts = found == 0 || !is_word_char(t[found - 1]);
            let ends = t.get(found + q.len()).is_none_or(|&c| !is_word_char(c));
            let tier = if starts && ends {
                TIER_WHOLE_WORD
            } else if found == 0 {
                TIER_PREFIX
            } else if starts {
                TIER_WORD_START
            } else {
                TIER_SUBSTRING
            };
            Score {
                tier,
                offset: found,
                spread: 0,
            }
        })
        .min()
}

fn score_subsequence(q: &[char], t: &[char]) -> Option<Score> {
    let (mut matched, mut first, mut last) = (0, None, 0);
    for (index, &c) in t.iter().enumerate() {
        if matched == q.len() {
            break;
        }
        if c.is_whitespace() {
            (matched, first) = (0, None);
            continue;
        }
        if c != q[matched] {
            continue;
        }
        first.get_or_insert(index);
        last = index;
        matched += 1;
    }
    let first = first.filter(|_| matched == q.len())?;
    Some(Score {
        tier: TIER_SUBSEQUENCE,
        offset: first,
        spread: last - first + 1,
    })
}

fn score_fuzzy(q: &[char], t: &[char], policy: FuzzyPolicy) -> Option<Score> {
    if policy.max_edits == 0 || q.len() <= policy.max_edits {
        return None;
    }
    let mut best: Option<Score> = None;
    let mut index = 0;
    while index < t.len() {
        if !is_word_char(t[index]) {
            index += 1;
            continue;
        }
        let start = index;
        while index < t.len() && is_word_char(t[index]) {
            index += 1;
        }
        let word = &t[start..index];
        let candidates = [
            word,
            &word[..q.len().min(word.len())],
            &word[..(q.len() + policy.max_edits).min(word.len())],
        ];
        for candidate in candidates {
            let distance = if policy.transpositions_only {
                is_adjacent_transposition(q, candidate).then_some(1)
            } else {
                bounded_edit_distance(q, candidate, policy.max_edits)
            };
            if let Some(spread) = distance {
                let score = Score {
                    tier: TIER_FUZZY,
                    offset: start,
                    spread,
                };
                best = Some(best.map_or(score, |best| best.min(score)));
            }
        }
    }
    best
}

/// Damerau-Levenshtein distance, abandoned once a whole row is over `budget`.
fn bounded_edit_distance(q: &[char], word: &[char], budget: usize) -> Option<usize> {
    if q.len().abs_diff(word.len()) > budget {
        return None;
    }
    let mut two_back: Vec<usize> = Vec::new();
    let mut previous: Vec<usize> = (0..=word.len()).collect();
    for qi in 1..=q.len() {
        let mut current = vec![qi];
        let mut row_best = qi;
        for wi in 1..=word.len() {
            let substitution = usize::from(q[qi - 1] != word[wi - 1]);
            let mut cost = (current[wi - 1] + 1)
                .min(previous[wi] + 1)
                .min(previous[wi - 1] + substitution);
            if qi > 1 && wi > 1 && q[qi - 1] == word[wi - 2] && q[qi - 2] == word[wi - 1] {
                cost = cost.min(two_back[wi - 2] + 1);
            }
            current.push(cost);
            row_best = row_best.min(cost);
        }
        if row_best > budget {
            return None;
        }
        two_back = std::mem::replace(&mut previous, current);
    }
    let distance = previous[word.len()];
    (distance <= budget).then_some(distance)
}

/// True when the two differ only by one swap of neighbouring characters.
fn is_adjacent_transposition(q: &[char], word: &[char]) -> bool {
    if q.len() != word.len() {
        return false;
    }
    let mut index = 0;
    while index < q.len() && q[index] == word[index] {
        index += 1;
    }
    if index + 1 >= q.len() {
        return false;
    }
    q[index] == word[index + 1]
        && q[index + 1] == word[index]
        && q[index + 2..] == word[index + 2..]
}

/// `matchRanges`, in char indices: a substring is one span, a subsequence the characters it
/// walked, and a typo match the whole word.
fn match_ranges(q: &[char], t: &[char], score: Score) -> Vec<Range<usize>> {
    match score.tier {
        _ if q.is_empty() => Vec::new(),
        TIER_EXACT => std::iter::once(0..t.len()).collect(),
        TIER_FUZZY => {
            let end = (score.offset..t.len())
                .find(|&i| !is_word_char(t[i]))
                .unwrap_or(t.len());
            std::iter::once(score.offset..end.max(score.offset + 1)).collect()
        }
        TIER_SUBSEQUENCE => {
            let mut ranges: Vec<Range<usize>> = Vec::new();
            let mut matched = 0;
            for (index, &c) in t.iter().enumerate().skip(score.offset) {
                if matched == q.len() {
                    break;
                }
                if c != q[matched] {
                    continue;
                }
                matched += 1;
                match ranges.last_mut() {
                    Some(last) if last.end == index => last.end += 1,
                    _ => ranges.push(index..index + 1),
                }
            }
            ranges
        }
        _ => std::iter::once(score.offset..score.offset + q.len()).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        Body, DateSection, Footer, HistoryRow, LOAD_ERROR_TEXT, activity_time, body, date_section,
        day_number, effective_search, footer, group_rows, highlight_ranges, merge_page,
        sort_newest_first, time_ago,
    };
    use bachuc_client::HistoryEntry;
    use chrono::{DateTime, FixedOffset, Utc};

    fn entry(id: &str, at: Option<&str>) -> HistoryEntry {
        HistoryEntry {
            agent_id: id.to_string(),
            title: id.to_string(),
            last_activity_at: at.map(str::to_string),
            ..HistoryEntry::default()
        }
    }

    fn ids(entries: &[HistoryEntry]) -> Vec<&str> {
        entries.iter().map(|e| e.agent_id.as_str()).collect()
    }

    fn marked<'a>(query: &str, text: &'a str) -> Vec<&'a str> {
        highlight_ranges(query, text)
            .into_iter()
            .map(|range| &text[range])
            .collect()
    }

    #[test]
    fn date_sections_follow_the_electron_day_limits() {
        let today = 20_000;
        assert_eq!(date_section(today + 1, today), DateSection::Today);
        assert_eq!(date_section(today, today), DateSection::Today);
        assert_eq!(date_section(today - 1, today), DateSection::Yesterday);
        assert_eq!(date_section(today - 2, today), DateSection::ThisWeek);
        assert_eq!(date_section(today - 7, today), DateSection::ThisWeek);
        assert_eq!(date_section(today - 8, today), DateSection::ThisMonth);
        assert_eq!(date_section(today - 30, today), DateSection::ThisMonth);
        assert_eq!(date_section(today - 31, today), DateSection::Older);
    }

    #[test]
    fn day_numbers_use_the_local_date() {
        let at = DateTime::parse_from_rfc3339("2026-10-17T23:30:00Z").unwrap();
        let utc = FixedOffset::east_opt(0).unwrap();
        let hanoi = FixedOffset::east_opt(7 * 3600).unwrap();
        assert_eq!(day_number(&at, &hanoi), day_number(&at, &utc) + 1);
    }

    #[test]
    fn groups_keep_order_and_skip_empty_sections() {
        let today = 100;
        let days = [Some(100), Some(99), Some(60), None, Some(100), Some(95)];
        assert_eq!(
            group_rows(&days, today),
            vec![
                HistoryRow::Header(DateSection::Today),
                HistoryRow::Entry(0),
                HistoryRow::Entry(4),
                HistoryRow::Header(DateSection::Yesterday),
                HistoryRow::Entry(1),
                HistoryRow::Header(DateSection::ThisWeek),
                HistoryRow::Entry(5),
                HistoryRow::Header(DateSection::Older),
                HistoryRow::Entry(2),
                HistoryRow::Entry(3),
            ]
        );
        assert!(group_rows(&[], today).is_empty());
    }

    #[test]
    fn sort_puts_the_newest_first_and_missing_times_last() {
        let mut entries = vec![
            entry("old", Some("2026-10-01T10:00:00Z")),
            entry("none", None),
            entry("new", Some("2026-10-17T10:00:00+07:00")),
            entry("bad", Some("yesterday")),
            entry("mid", Some("2026-10-10T10:00:00.123Z")),
        ];
        sort_newest_first(&mut entries);
        assert_eq!(ids(&entries), ["new", "mid", "old", "none", "bad"]);
    }

    #[test]
    fn merge_page_appends_new_agents_only() {
        let mut entries = vec![entry("a", Some("2026-10-17T10:00:00Z"))];
        merge_page(
            &mut entries,
            vec![
                entry("a", Some("2026-10-17T10:00:00Z")),
                entry("b", Some("2026-10-18T10:00:00Z")),
            ],
        );
        assert_eq!(ids(&entries), ["b", "a"]);
    }

    #[test]
    fn time_ago_matches_format_time_ago() {
        let now: DateTime<Utc> = "2026-10-18T12:00:00Z".parse().unwrap();
        let utc = FixedOffset::east_opt(0).unwrap();
        let ago = |iso: &str| time_ago(&DateTime::parse_from_rfc3339(iso).unwrap(), &now, &utc);
        assert_eq!(ago("2026-10-18T11:59:55Z"), "just now");
        assert_eq!(ago("2026-10-18T12:00:30Z"), "just now");
        assert_eq!(ago("2026-10-18T11:59:30Z"), "30s ago");
        assert_eq!(ago("2026-10-18T11:55:00Z"), "5m ago");
        assert_eq!(ago("2026-10-18T09:00:00Z"), "3h ago");
        assert_eq!(ago("2026-10-15T12:00:00Z"), "3d ago");
        assert_eq!(ago("2026-10-09T08:00:00Z"), "Oct 9");
    }

    #[test]
    fn activity_time_reads_the_snapshot_time() {
        assert!(activity_time(&entry("a", Some("2026-10-17T10:00:00.000Z"))).is_some());
        assert!(activity_time(&entry("a", None)).is_none());
    }

    #[test]
    fn highlight_marks_substring_matches_of_each_token() {
        assert_eq!(marked("fix", "Fix the sidebar"), ["Fix"]);
        assert_eq!(marked("side BAR", "Fix the sidebar"), ["sidebar"]);
        assert_eq!(marked("the fix", "Fix the sidebar"), ["Fix", "the"]);
        assert!(marked("", "Fix the sidebar").is_empty());
        assert!(marked("zzz", "Fix the sidebar").is_empty());
    }

    #[test]
    fn highlight_marks_a_subsequence_within_one_word() {
        assert_eq!(marked("pasbab", "Run paseo-babysit"), ["pas", "bab"]);
        assert!(marked("labdes", "Label as Design").is_empty());
    }

    #[test]
    fn highlight_marks_the_whole_word_for_a_typo() {
        assert_eq!(marked("sidbear", "Fix the sidebar now"), ["sidebar"]);
        assert_eq!(marked("mian", "Merge main"), ["main"]);
        assert!(marked("mail", "Merge main").is_empty());
    }

    #[test]
    fn highlight_ranges_are_byte_ranges() {
        assert_eq!(marked("caf", "Đổi café"), ["caf"]);
        assert_eq!(marked("đổi", "Đổi café"), ["Đổi"]);
    }

    #[test]
    fn body_picks_the_empty_no_match_and_error_texts() {
        assert_eq!(
            body(false, false, 0, false),
            Body::Empty {
                text: "No sessions yet",
                clear_search: false
            }
        );
        assert_eq!(
            body(false, false, 0, true),
            Body::Empty {
                text: "No sessions match",
                clear_search: true
            }
        );
        assert_eq!(body(false, true, 0, false), Body::Error);
        assert_eq!(LOAD_ERROR_TEXT, "Unable to load sessions");
        assert_eq!(body(false, true, 3, false), Body::List);
        assert_eq!(body(true, true, 0, true), Body::Loading);
        assert_eq!(body(false, false, 2, true), Body::List);
    }

    #[test]
    fn footer_shows_the_truncated_hint_before_load_more() {
        assert_eq!(footer(true, true, true, false), Footer::TooManyMatches);
        assert_eq!(
            footer(false, true, true, true),
            Footer::LoadMore { loading: true }
        );
        assert_eq!(footer(true, false, false, false), Footer::None);
    }

    #[test]
    fn effective_search_trims_and_needs_support() {
        assert_eq!(effective_search("  fix  ", true).as_deref(), Some("fix"));
        assert_eq!(effective_search("   ", true), None);
        assert_eq!(effective_search("fix", false), None);
    }
}
