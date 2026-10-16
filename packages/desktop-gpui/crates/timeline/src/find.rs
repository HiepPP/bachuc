//! Find in the agent pane: case-insensitive matches in the loaded messages and the status text
//! of `packages/app/src/pane-find/index.tsx` as the chat find words it.

use std::ops::Range;

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
            let mut lower = c.to_lowercase();
            // A character whose lowercase is several characters must match them all.
            for expected in lower.by_ref() {
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

/// The match count text: empty with no query, "3 of 12", or "No matches".
pub fn status(query: &str, count: usize, current: usize) -> String {
    if query.trim().is_empty() {
        String::new()
    } else if count == 0 {
        "No matches".into()
    } else {
        format!("{} of {count}", current + 1)
    }
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
