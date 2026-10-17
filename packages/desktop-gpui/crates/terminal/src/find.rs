//! Find in the terminal, as `TerminalFind` in `packages/app/src/terminal/find/index.tsx` with the
//! xterm search of `terminal-emulator-runtime.ts`: a search without case over the history and the
//! screen, the status text of the bar, and the highlight spans of a drawn line.

use crate::emulator::Emulator;
use gpui_kit::HighlightStyle;
use std::ops::Range;

/// `FIND_HIGHLIGHT_LIMIT` of `terminal-emulator-runtime.ts`. A search stops at this many matches,
/// and the count shows a "+".
pub const MATCH_LIMIT: usize = 20_000;

/// One match: the buffer line (0 is the oldest history line) and the byte range in its text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Match {
    pub line: usize,
    pub range: Range<usize>,
}

/// The byte ranges of `needle` in `haystack`, compared without case, not overlapping.
pub fn match_ranges(haystack: &str, needle: &str) -> Vec<Range<usize>> {
    let needle: Vec<char> = needle.chars().flat_map(char::to_lowercase).collect();
    if needle.is_empty() {
        return Vec::new();
    }
    let chars: Vec<(usize, char)> = haystack.char_indices().collect();
    let mut ranges = Vec::new();
    let mut start = 0;
    'outer: while start < chars.len() {
        let mut matched = 0;
        let mut index = start;
        while matched < needle.len() {
            let Some(&(_, c)) = chars.get(index) else {
                break 'outer;
            };
            // A character whose lowercase is several characters must match them all.
            for expected in c.to_lowercase() {
                if needle.get(matched) != Some(&expected) {
                    start += 1;
                    continue 'outer;
                }
                matched += 1;
            }
            index += 1;
        }
        let begin = chars[start].0;
        let end = chars
            .get(index)
            .map_or(haystack.len(), |&(offset, _)| offset);
        ranges.push(begin..end);
        start = index;
    }
    ranges
}

/// Every match of `query` in `lines`, in buffer order, and whether the search stopped at
/// [`MATCH_LIMIT`].
pub fn find_matches(lines: &[String], query: &str) -> (Vec<Match>, bool) {
    let mut matches = Vec::new();
    for (line, text) in lines.iter().enumerate() {
        for range in match_ranges(text, query) {
            if matches.len() == MATCH_LIMIT {
                return (matches, true);
            }
            matches.push(Match { line, range });
        }
    }
    (matches, false)
}

/// The search of the find bar. The query outlives the bar, as in `TerminalFind`, which keeps it
/// when the bar closes and searches it again when the bar opens.
#[derive(Default)]
pub struct Search {
    pub query: String,
    pub matches: Vec<Match>,
    /// The search stopped at [`MATCH_LIMIT`].
    pub limited: bool,
    pub current: Option<usize>,
    /// Output arrived after the search, so the next step searches again.
    pub stale: bool,
}

impl Search {
    /// Searches `lines` for the query again and keeps the current match in range.
    pub fn run(&mut self, lines: &[String]) {
        let (matches, limited) = find_matches(lines, &self.query);
        self.current = self
            .current
            .filter(|_| !matches.is_empty())
            .map(|current| current.min(matches.len() - 1));
        self.matches = matches;
        self.limited = limited;
        self.stale = false;
    }

    /// Drops the matches when the bar closes and keeps the query.
    pub fn close(&mut self) {
        self.matches = Vec::new();
        self.limited = false;
        self.current = None;
        self.stale = false;
    }
}

/// The buffer lines, oldest first: the history, then the screen. The emulator reads only the
/// rows on screen, so this scrolls through the history and puts the view back where it was.
pub fn buffer_lines(emulator: &mut Emulator) -> Vec<String> {
    let rows = emulator.rows();
    let history = emulator.history_size();
    let saved = emulator.display_offset();
    let mut lines = Vec::with_capacity(history + rows);
    emulator.scroll((history - saved) as i32);
    loop {
        let top = history - emulator.display_offset();
        for row in lines.len() - top..rows {
            lines.push(emulator.row_text(row));
        }
        let offset = emulator.display_offset();
        if offset == 0 {
            break;
        }
        emulator.scroll(-(offset.min(rows) as i32));
    }
    emulator.scroll(saved as i32);
    lines
}

/// The buffer line on screen row 0.
pub fn top_line(history: usize, offset: usize) -> usize {
    history.saturating_sub(offset)
}

/// The first match at or below `line`, else the first match.
pub fn first_from(matches: &[Match], line: usize) -> Option<usize> {
    if matches.is_empty() {
        return None;
    }
    Some(matches.iter().position(|m| m.line >= line).unwrap_or(0))
}

/// The view offset that shows buffer line `line`: the same offset when the line is on screen,
/// else the one that puts the line in the middle of the screen.
pub fn reveal_offset(line: usize, history: usize, rows: usize, offset: usize) -> usize {
    let top = top_line(history, offset);
    if (top..top + rows).contains(&line) {
        return offset;
    }
    history.saturating_sub(line.saturating_sub(rows / 2))
}

/// The next match index, wrapping.
pub fn step(current: usize, count: usize, forward: bool) -> usize {
    if count == 0 {
        0
    } else if forward {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    }
}

/// The count text of `TerminalFind`: empty with no query, "No matches", "3 of 12", or
/// "12 matches" with no current match. A search that hit the limit shows "20000+".
pub fn status(query: &str, count: usize, current: Option<usize>, limited: bool) -> String {
    let total = format!("{count}{}", if limited { "+" } else { "" });
    if query.is_empty() {
        String::new()
    } else if count == 0 {
        "No matches".into()
    } else if let Some(current) = current {
        format!("{} of {total}", current + 1)
    } else {
        format!("{total} matches")
    }
}

/// The current match as a buffer line and its place among that line's matches, so a drawn line
/// can find it among its own matches.
pub fn current_place(matches: &[Match], current: Option<usize>) -> Option<(usize, usize)> {
    let found = matches.get(current?)?;
    let first = matches[..current?]
        .iter()
        .rposition(|m| m.line != found.line)
        .map_or(0, |index| index + 1);
    Some((found.line, current? - first))
}

/// The runs of a line with `overlays` painted over them, in order. A run that an overlay covers
/// in part is split, and the fields an overlay sets win over the run's.
pub fn paint(
    runs: &[(Range<usize>, HighlightStyle)],
    overlays: &[(Range<usize>, HighlightStyle)],
) -> Vec<(Range<usize>, HighlightStyle)> {
    let mut edges: Vec<usize> = runs
        .iter()
        .chain(overlays)
        .flat_map(|(range, _)| [range.start, range.end])
        .collect();
    edges.sort_unstable();
    edges.dedup();
    let mut out: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    for pair in edges.windows(2) {
        let (start, end) = (pair[0], pair[1]);
        let base = runs
            .iter()
            .find(|(range, _)| range.contains(&start))
            .map(|(_, style)| *style);
        let over = overlays
            .iter()
            .filter(|(range, _)| range.contains(&start))
            .map(|(_, style)| *style)
            .reduce(|below, above| below.highlight(above));
        let style = match (base, over) {
            (None, None) => continue,
            (Some(base), None) => base,
            (base, Some(over)) => base.unwrap_or_default().highlight(over),
        };
        match out.last_mut() {
            Some((range, last)) if range.end == start && *last == style => range.end = end,
            _ => out.push((start..end, style)),
        }
    }
    out
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings its own `test` attribute.
    use super::{
        MATCH_LIMIT, Match, Search, buffer_lines, current_place, find_matches, first_from,
        match_ranges, paint, reveal_offset, status, step,
    };
    use crate::emulator::Emulator;
    use gpui_kit::{HighlightStyle, hsla};

    #[test]
    fn matches_ignore_case_and_do_not_overlap() {
        assert_eq!(
            match_ranges("Hello hello HELLO", "hello"),
            [0..5, 6..11, 12..17]
        );
        assert_eq!(match_ranges("aaaa", "aa"), [0..2, 2..4]);
        assert!(match_ranges("abc", "").is_empty());
        assert_eq!(match_ranges("é café", "CAFÉ"), vec![3..8]);
    }

    #[test]
    fn find_walks_lines_in_order_and_stops_at_the_limit() {
        let lines = vec!["one hello".to_string(), "".into(), "hello hello".into()];
        let (matches, limited) = find_matches(&lines, "hello");
        assert!(!limited);
        let lines_of: Vec<_> = matches.iter().map(|m| (m.line, m.range.clone())).collect();
        assert_eq!(lines_of, [(0, 4..9), (2, 0..5), (2, 6..11)]);

        let many = vec!["x".repeat(MATCH_LIMIT + 5)];
        let (matches, limited) = find_matches(&many, "x");
        assert_eq!((matches.len(), limited), (MATCH_LIMIT, true));
    }

    #[test]
    fn buffer_lines_read_the_history_and_keep_the_view() {
        let mut emulator = Emulator::new(4, 20);
        let text: String = (1..=30).map(|n| format!("line {n}\r\n")).collect();
        emulator.feed(text.as_bytes());
        emulator.scroll(5);
        let before = emulator.display_offset();
        let lines = buffer_lines(&mut emulator);
        assert_eq!(emulator.display_offset(), before);
        assert_eq!(lines.len(), emulator.history_size() + emulator.rows());
        assert_eq!(lines[0], "line 1");
        assert_eq!(lines[29], "line 30");
        assert_eq!(lines[30], "");
        // Each line once, in order.
        for (index, line) in lines.iter().take(30).enumerate() {
            assert_eq!(line, &format!("line {}", index + 1));
        }
    }

    #[test]
    fn a_closed_search_keeps_its_query_and_finds_it_again_on_open() {
        let mut emulator = Emulator::new(3, 20);
        emulator.feed(b"say hello\r\nhello");
        let mut search = Search {
            query: "hello".into(),
            ..Search::default()
        };
        search.run(&buffer_lines(&mut emulator));
        search.current = Some(1);
        search.close();
        assert_eq!(search.query, "hello");
        assert!(search.matches.is_empty() && search.current.is_none());
        // Output that arrives while the bar is closed is found when it opens again.
        emulator.feed(b" hello");
        search.run(&buffer_lines(&mut emulator));
        assert_eq!(search.matches.len(), 3);
        assert_eq!(
            status(
                &search.query,
                search.matches.len(),
                search.current,
                search.limited
            ),
            "3 matches"
        );
    }

    #[test]
    fn buffer_lines_without_history_are_the_screen() {
        let mut emulator = Emulator::new(3, 10);
        emulator.feed(b"a\r\nb");
        assert_eq!(buffer_lines(&mut emulator), ["a", "b", ""]);
    }

    #[test]
    fn the_first_match_is_at_or_below_the_line_and_wraps() {
        let at = |line| Match { line, range: 0..1 };
        let matches = [at(2), at(5), at(9)];
        assert_eq!(first_from(&matches, 0), Some(0));
        assert_eq!(first_from(&matches, 5), Some(1));
        assert_eq!(first_from(&matches, 10), Some(0));
        assert_eq!(first_from(&[], 0), None);
    }

    #[test]
    fn reveal_keeps_a_line_on_screen_and_centers_one_off_it() {
        // 100 history lines, 10 rows: offset 0 shows lines 100..110.
        assert_eq!(reveal_offset(105, 100, 10, 0), 0);
        assert_eq!(reveal_offset(40, 100, 10, 0), 65);
        assert_eq!(reveal_offset(0, 100, 10, 0), 100);
        assert_eq!(reveal_offset(108, 100, 10, 50), 0);
    }

    #[test]
    fn steps_wrap_both_ways() {
        assert_eq!(step(2, 3, true), 0);
        assert_eq!(step(0, 3, false), 2);
        assert_eq!(step(0, 0, true), 0);
    }

    #[test]
    fn status_words_the_count_as_the_app_does() {
        assert_eq!(status("", 0, None, false), "");
        assert_eq!(status("x", 0, None, false), "No matches");
        assert_eq!(status("x", 12, Some(2), false), "3 of 12");
        assert_eq!(status("x", 12, None, false), "12 matches");
        assert_eq!(status("x", 20000, Some(0), true), "1 of 20000+");
    }

    #[test]
    fn the_current_place_counts_within_its_line() {
        let at = |line, start| Match {
            line,
            range: start..start + 1,
        };
        let matches = [at(1, 0), at(3, 0), at(3, 4), at(3, 8)];
        assert_eq!(current_place(&matches, Some(0)), Some((1, 0)));
        assert_eq!(current_place(&matches, Some(3)), Some((3, 2)));
        assert_eq!(current_place(&matches, None), None);
        assert_eq!(current_place(&matches, Some(9)), None);
    }

    #[test]
    fn paint_splits_runs_and_fills_gaps() {
        let red = HighlightStyle {
            color: Some(hsla(0., 1., 0.5, 1.)),
            ..Default::default()
        };
        let mark = HighlightStyle {
            background_color: Some(hsla(0.15, 1., 0.5, 0.35)),
            ..Default::default()
        };
        let runs = [(0..4, red)];
        let painted = paint(&runs, &[(2..6, mark)]);
        let spans: Vec<_> = painted.iter().map(|(range, _)| range.clone()).collect();
        assert_eq!(spans, [0..2, 2..4, 4..6]);
        assert_eq!(painted[0].1, red);
        assert_eq!(painted[1].1.color, red.color);
        assert_eq!(painted[1].1.background_color, mark.background_color);
        assert_eq!(painted[2].1, mark);
        assert_eq!(paint(&runs, &[]), runs);

        // A link underline over a match keeps the match color.
        let underline = HighlightStyle {
            underline: Some(Default::default()),
            ..Default::default()
        };
        let both = paint(&[], &[(0..3, mark), (1..2, underline)]);
        assert_eq!(both[1].0, 1..2);
        assert_eq!(both[1].1.background_color, mark.background_color);
        assert!(both[1].1.underline.is_some());
    }
}
