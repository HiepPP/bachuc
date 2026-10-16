//! Search and row layout of the command center, ported from `command-center/results.ts` and
//! `protocol/src/search/text-match.ts`.
//!
//! Matching is per token: every token of the query must land in a field, and the tokens add up
//! to the score. A token matches as the whole text, a whole word, a prefix, the start of a word,
//! a substring, or characters in order inside one word, best first. A match on the visible title
//! always outranks a match that needs the group name. Typo tolerance is not ported: the command
//! center turns it off too.

use std::cmp::Ordering;

/// Height of an item row.
pub const ROW_HEIGHT: f32 = 36.;
/// Height of a section title row at the top of the list.
pub const TITLED_SECTION_HEIGHT: f32 = 32.;
/// Height of a section title row below a divider.
pub const DIVIDED_SECTION_HEIGHT: f32 = 49.;
/// Height of a divider without a title.
pub const DIVIDER_HEIGHT: f32 = 17.;

const TIER_EXACT: u32 = 0;
const TIER_WHOLE_WORD: u32 = 1;
const TIER_PREFIX: u32 = 2;
const TIER_WORD_START: u32 = 3;
const TIER_SUBSTRING: u32 = 4;
const TIER_SUBSEQUENCE: u32 = 5;

/// Every token matched the visible title.
const VISIBLE_FIELD_RANK: u8 = 0;
/// At least one token needed the group name.
const GROUP_FIELD_RANK: u8 = 1;
/// At least one token needed a hidden keyword. Below every title and group match.
const KEYWORD_FIELD_RANK: u8 = 2;

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct Match {
    tier: u32,
    offset: usize,
    spread: usize,
}

impl Match {
    fn key(&self) -> (u32, usize, usize) {
        (self.tier, self.offset, self.spread)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
struct Score {
    field_rank: u8,
    found: Match,
}

impl Score {
    fn key(&self) -> (u8, (u32, usize, usize)) {
        (self.field_rank, self.found.key())
    }
}

fn is_boundary(ch: Option<char>) -> bool {
    ch.is_none_or(|ch| !ch.is_ascii_alphanumeric())
}

fn score_substring(query: &str, text: &str) -> Option<Match> {
    let mut best: Option<Match> = None;
    for (found, _) in text.char_indices() {
        if !text[found..].starts_with(query) {
            continue;
        }
        let before = text[..found].chars().next_back();
        let after = text[found + query.len()..].chars().next();
        let starts_at_boundary = found == 0 || is_boundary(before);
        let ends_at_boundary = is_boundary(after);
        let tier = if starts_at_boundary && ends_at_boundary {
            TIER_WHOLE_WORD
        } else if found == 0 {
            TIER_PREFIX
        } else if starts_at_boundary {
            TIER_WORD_START
        } else {
            TIER_SUBSTRING
        };
        let better = best.is_none_or(|b| tier < b.tier || (tier == b.tier && found < b.offset));
        if better {
            best = Some(Match {
                tier,
                offset: found,
                spread: 0,
            });
        }
    }
    best
}

fn score_subsequence(query: &str, text: &str) -> Option<Match> {
    let query: Vec<char> = query.chars().collect();
    let mut matched = 0;
    let mut first: Option<usize> = None;
    let mut last = 0;
    for (index, ch) in text.char_indices() {
        if matched == query.len() {
            break;
        }
        if ch.is_whitespace() {
            matched = 0;
            first = None;
            continue;
        }
        if ch != query[matched] {
            continue;
        }
        first.get_or_insert(index);
        last = index;
        matched += 1;
    }
    if matched != query.len() {
        return None;
    }
    let first = first?;
    Some(Match {
        tier: TIER_SUBSEQUENCE,
        offset: first,
        spread: last - first + 1,
    })
}

/// Both strings are already lowercase.
fn score_match(token: &str, text: &str) -> Option<Match> {
    if text == token {
        return Some(Match {
            tier: TIER_EXACT,
            ..Match::default()
        });
    }
    score_substring(token, text).or_else(|| score_subsequence(token, text))
}

/// Every token must match some field; the per-token bests add up.
fn score_fields(tokens: &[&str], fields: &[&str]) -> Option<Match> {
    let mut total = Match::default();
    for token in tokens {
        let best = fields
            .iter()
            .filter_map(|field| score_match(token, field))
            .min_by_key(Match::key)?;
        total.tier += best.tier;
        total.offset += best.offset;
        total.spread += if best.spread == 0 {
            token.len()
        } else {
            best.spread
        };
    }
    Some(total)
}

/// One searchable item: its lowercase title, the index of its group, and its hidden keywords.
struct Entry {
    title: String,
    group: usize,
    keywords: Vec<String>,
}

/// The searchable form of the items. Lowercasing happens once here, not per keystroke.
pub struct SearchIndex {
    entries: Vec<Entry>,
    /// Group names in the order of first appearance. The empty name has no section title.
    groups: Vec<String>,
    groups_lower: Vec<String>,
}

/// One row of the list.
#[derive(Clone, Debug, PartialEq)]
pub enum Row {
    /// A section title and divider. `title` is None for a group without a name.
    Section {
        title: Option<String>,
        divider: bool,
        height: f32,
    },
    /// An item, by its index in the list given to `SearchIndex::new`.
    Item { item: usize },
}

impl Row {
    pub fn height(&self) -> f32 {
        match self {
            Row::Section { height, .. } => *height,
            Row::Item { .. } => ROW_HEIGHT,
        }
    }
}

/// What a query shows.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Layout {
    pub rows: Vec<Row>,
    /// The items in display order: the order the arrow keys walk.
    pub order: Vec<usize>,
    /// For each position in `order`, its index in `rows`.
    pub row_of: Vec<usize>,
}

impl SearchIndex {
    /// Builds the index from `(title, group)` pairs.
    pub fn new<'a>(items: impl IntoIterator<Item = (&'a str, &'a str)>) -> SearchIndex {
        Self::with_keywords(
            items
                .into_iter()
                .map(|(title, group)| (title, group, Vec::new())),
        )
    }

    /// Builds the index from `(title, group, keywords)`. A keyword is never drawn: a row that
    /// matches only through a keyword ranks below every title or group match.
    pub fn with_keywords<'a>(
        items: impl IntoIterator<Item = (&'a str, &'a str, Vec<&'a str>)>,
    ) -> SearchIndex {
        let mut groups: Vec<String> = Vec::new();
        let entries = items
            .into_iter()
            .map(|(title, group, keywords)| {
                let group = match groups.iter().position(|name| name == group) {
                    Some(index) => index,
                    None => {
                        groups.push(group.to_string());
                        groups.len() - 1
                    }
                };
                Entry {
                    title: title.to_lowercase(),
                    group,
                    keywords: keywords.into_iter().map(str::to_lowercase).collect(),
                }
            })
            .collect();
        let groups_lower = groups.iter().map(|name| name.to_lowercase()).collect();
        SearchIndex {
            entries,
            groups,
            groups_lower,
        }
    }

    pub fn len(&self) -> usize {
        self.entries.len()
    }

    pub fn is_empty(&self) -> bool {
        self.entries.is_empty()
    }

    fn score(&self, entry: &Entry, tokens: &[&str]) -> Option<Score> {
        if let Some(found) = score_fields(tokens, &[&entry.title]) {
            return Some(Score {
                field_rank: VISIBLE_FIELD_RANK,
                found,
            });
        }
        let group = &self.groups_lower[entry.group];
        if !group.is_empty() {
            if let Some(found) = score_fields(tokens, &[&entry.title, group]) {
                return Some(Score {
                    field_rank: GROUP_FIELD_RANK,
                    found,
                });
            }
        }
        if entry.keywords.is_empty() {
            return None;
        }
        let mut fields: Vec<&str> = vec![&entry.title];
        if !group.is_empty() {
            fields.push(group);
        }
        fields.extend(entry.keywords.iter().map(String::as_str));
        score_fields(tokens, &fields).map(|found| Score {
            field_rank: KEYWORD_FIELD_RANK,
            found,
        })
    }

    /// The rows for a query. An empty query shows every item in the order given, in sections
    /// in the order of the first item of each group. A query keeps the matching items, sorts
    /// each section by score, and sorts the sections by their best score.
    pub fn layout(&self, query: &str) -> Layout {
        let lowered = query.to_lowercase();
        let tokens: Vec<&str> = lowered.split_whitespace().collect();
        // Items of each group with their score.
        let mut sections: Vec<Vec<(usize, Option<Score>)>> = vec![Vec::new(); self.groups.len()];
        for (index, entry) in self.entries.iter().enumerate() {
            if tokens.is_empty() {
                sections[entry.group].push((index, None));
            } else if let Some(score) = self.score(entry, &tokens) {
                sections[entry.group].push((index, Some(score)));
            }
        }
        let mut groups: Vec<usize> = (0..self.groups.len())
            .filter(|group| !sections[*group].is_empty())
            .collect();
        if !tokens.is_empty() {
            for items in &mut sections {
                items.sort_by(compare);
            }
            // Each section is sorted, so its first item holds its best score. The sort is stable,
            // so equal sections stay in the order of first appearance.
            groups.sort_by(|a, b| {
                let (left, right) = (&sections[*a][0], &sections[*b][0]);
                match (&left.1, &right.1) {
                    (Some(l), Some(r)) => l.key().cmp(&r.key()).then(a.cmp(b)),
                    _ => a.cmp(b),
                }
            });
        }
        let mut layout = Layout::default();
        for (position, group) in groups.into_iter().enumerate() {
            let divider = position > 0;
            let name = &self.groups[group];
            let title = (!name.is_empty()).then(|| name.clone());
            let height = match (title.is_some(), divider) {
                (true, true) => DIVIDED_SECTION_HEIGHT,
                (true, false) => TITLED_SECTION_HEIGHT,
                (false, true) => DIVIDER_HEIGHT,
                (false, false) => 0.,
            };
            if height > 0. {
                layout.rows.push(Row::Section {
                    title,
                    divider,
                    height,
                });
            }
            for (item, _) in &sections[group] {
                layout.row_of.push(layout.rows.len());
                layout.order.push(*item);
                layout.rows.push(Row::Item { item: *item });
            }
        }
        layout
    }
}

/// Order by score, then by the position in the list. Items without a score keep list order.
fn compare(a: &(usize, Option<Score>), b: &(usize, Option<Score>)) -> Ordering {
    let by_score = match (&a.1, &b.1) {
        (Some(left), Some(right)) => left.key().cmp(&right.key()),
        _ => Ordering::Equal,
    };
    by_score.then(a.0.cmp(&b.0))
}

/// Where the arrow keys go: `delta` is 1 for down and -1 for up, and the list wraps. From no
/// selection, down picks the first item and up picks the last, as Electron does.
pub fn step(active: Option<usize>, len: usize, delta: isize) -> Option<usize> {
    if len == 0 {
        return None;
    }
    let start = match active {
        Some(position) => position as isize,
        None if delta < 0 => 0,
        None => -1,
    };
    Some((start + delta).rem_euclid(len as isize) as usize)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn index(items: &[(&str, &str)]) -> SearchIndex {
        SearchIndex::new(items.iter().copied())
    }

    fn titles(layout: &Layout, titles: &[&str]) -> Vec<String> {
        layout
            .order
            .iter()
            .map(|item| titles[*item].to_string())
            .collect()
    }

    #[test]
    fn empty_query_keeps_order_and_sections() {
        let titles_list = ["B", "A", "C"];
        let index = index(&[("B", "One"), ("A", "Two"), ("C", "One")]);
        let layout = index.layout("");
        assert_eq!(titles(&layout, &titles_list), ["B", "C", "A"]);
        assert_eq!(
            layout.rows,
            vec![
                Row::Section {
                    title: Some("One".into()),
                    divider: false,
                    height: 32.
                },
                Row::Item { item: 0 },
                Row::Item { item: 2 },
                Row::Section {
                    title: Some("Two".into()),
                    divider: true,
                    height: 49.
                },
                Row::Item { item: 1 },
            ]
        );
        assert_eq!(layout.row_of, [1, 2, 4]);
    }

    #[test]
    fn ranks_by_match_quality() {
        let list = [
            "Toggle settings",
            "Settings",
            "Reset sessions",
            "Cycle theme",
        ];
        let index = index(&list.map(|title| (title, "Actions")));
        let layout = index.layout("set");
        // Prefix of the whole text, then a word start, then a substring.
        assert_eq!(
            titles(&layout, &list),
            ["Settings", "Toggle settings", "Reset sessions"]
        );
    }

    #[test]
    fn exact_beats_prefix() {
        let list = ["Close pane", "Close"];
        let index = index(&list.map(|title| (title, "")));
        let layout = index.layout("close");
        assert_eq!(titles(&layout, &list), ["Close", "Close pane"]);
    }

    #[test]
    fn tokens_in_any_order_and_subsequence_inside_a_word() {
        let list = ["Toggle left sidebar"];
        let index = index(&list.map(|title| (title, "Layout")));
        assert_eq!(index.layout("side tog").order, [0]);
        assert_eq!(index.layout("tgl").order, [0]);
        // Characters do not join across words.
        assert!(index.layout("tglsde").order.is_empty());
    }

    #[test]
    fn group_match_ranks_below_title_match() {
        let list = ["Alpha tabs", "Zed"];
        let index = index(&[(list[0], "Misc"), (list[1], "Tabs")]);
        let layout = index.layout("tabs");
        assert_eq!(titles(&layout, &list), ["Alpha tabs", "Zed"]);
    }

    fn keyword_index(items: &[(&'static str, &'static str, &[&'static str])]) -> SearchIndex {
        SearchIndex::with_keywords(items.iter().map(|(t, g, k)| (*t, *g, k.to_vec())))
    }

    #[test]
    fn keyword_match_ranks_below_title_and_group_matches() {
        let list = ["Zed keyword", "Zed group", "Zed title diff"];
        let index = keyword_index(&[
            (list[0], "Keyword section", &["diff"]),
            (list[1], "Diff", &[]),
            (list[2], "Title section", &[]),
        ]);
        assert_eq!(
            titles(&index.layout("diff"), &list),
            [list[2], list[1], list[0]]
        );
    }

    #[test]
    fn keyword_ranks_below_a_weak_title_match() {
        // "diff" is only a subsequence of the first title, yet that still beats a keyword hit.
        let list = ["Dxixfxf", "Open", "Compare"];
        let index = keyword_index(&[
            (list[0], "", &[]),
            (list[1], "", &["diff"]),
            (list[2], "", &["diff"]),
        ]);
        assert_eq!(
            titles(&index.layout("diff"), &list),
            [list[0], list[1], list[2]]
        );
    }

    #[test]
    fn keyword_alone_matches_and_equal_scores_keep_index_order() {
        let list = ["B", "A"];
        let index = keyword_index(&[(list[0], "P", &["review"]), (list[1], "P", &["review"])]);
        assert_eq!(titles(&index.layout("review"), &list), ["B", "A"]);
        assert!(index.layout("nothing").order.is_empty());
    }

    #[test]
    fn keywords_do_not_change_the_empty_query() {
        let list = ["B", "A"];
        let index = keyword_index(&[(list[0], "P", &["x"]), (list[1], "P", &[])]);
        assert_eq!(titles(&index.layout(""), &list), ["B", "A"]);
    }

    #[test]
    fn no_match_is_empty() {
        let index = index(&[("Alpha", "Actions")]);
        let layout = index.layout("zzz");
        assert!(layout.rows.is_empty());
        assert!(layout.order.is_empty());
    }

    #[test]
    fn sections_sort_by_best_score() {
        let list = ["Open settings", "Settings"];
        let index = index(&[(list[0], "First"), (list[1], "Second")]);
        let layout = index.layout("settings");
        assert_eq!(titles(&layout, &list), ["Settings", "Open settings"]);
    }

    #[test]
    fn unnamed_groups_have_no_title() {
        let index = index(&[("A", ""), ("B", "Named")]);
        let layout = index.layout("");
        assert_eq!(
            layout.rows[0],
            Row::Item { item: 0 },
            "an unnamed first group has no row"
        );
        assert!(matches!(
            layout.rows[1],
            Row::Section {
                divider: true,
                height: 49.,
                ..
            }
        ));
    }

    #[test]
    fn step_wraps() {
        assert_eq!(step(None, 3, 1), Some(0));
        assert_eq!(step(None, 3, -1), Some(2));
        assert_eq!(step(Some(2), 3, 1), Some(0));
        assert_eq!(step(Some(0), 3, -1), Some(2));
        assert_eq!(step(None, 0, 1), None);
    }

    #[test]
    fn unicode_titles_do_not_panic() {
        let index = index(&[("Đóng tab", "Tab"), ("Mở dự án", "Dự án")]);
        assert_eq!(index.layout("đóng").order, [0]);
        assert_eq!(index.layout("dự").order, [1]);
    }
}
