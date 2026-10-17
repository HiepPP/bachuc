//! Links in terminal output, as the Electron terminal finds them: `http` and `https` URLs, and
//! file paths with an optional line (`terminal-local-link-parsing.ts`), plus the rule that picks
//! the file a path link opens (Q-112).
//!
//! Rows that wrap are joined before links are found, as the Electron provider joins wrapped
//! buffer lines, so a path cut by the row end opens whole.

use bachuc_client::DirectorySuggestion;
use std::ops::Range;

/// What a link opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkTarget {
    Url(String),
    File { path: String, line: Option<u32> },
}

/// A link in a row: the byte range it covers, with its line suffix, and what it opens.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Link {
    pub range: Range<usize>,
    pub target: LinkTarget,
}

/// `ExcludedPathCharactersClause` of the Electron parser: these characters end a path.
fn ends_path(c: char) -> bool {
    c.is_whitespace()
        || matches!(
            c,
            '\0' | '<'
                | '>'
                | '?'
                | '!'
                | '`'
                | '&'
                | '*'
                | '('
                | ')'
                | '\''
                | '"'
                | ':'
                | ';'
                | '\\'
        )
}

/// Characters a URL does not end with, as text around it often has them.
fn url_tail(c: char) -> bool {
    matches!(c, '.' | ',' | ';' | ':' | '!' | '?')
}

fn digits(text: &str) -> (Option<u32>, usize) {
    let len = text.bytes().take_while(u8::is_ascii_digit).count();
    (text[..len].parse().ok(), len)
}

/// The line of a suffix at the start of `rest` and the suffix length: `:12`, `:12:4`, `:12.4`,
/// `(12)`, `(12,4)`, or the Python `", line 12`.
fn line_suffix(rest: &str) -> Option<(u32, usize)> {
    if let Some(after) = rest.strip_prefix(':') {
        let (line, len) = digits(after);
        let mut used = 1 + len;
        let tail = &after[len..];
        if let Some(col) = tail.strip_prefix([':', '.'])
            && digits(col).1 > 0
        {
            used += 1 + digits(col).1;
        }
        return line.map(|line| (line, used));
    }
    if let Some(after) = rest.strip_prefix('(') {
        let (line, len) = digits(after);
        let mut tail = &after[len..];
        if let Some(col) = tail.strip_prefix([',', ':']) {
            let col = col.strip_prefix(' ').unwrap_or(col);
            tail = &col[digits(col).1..];
        }
        let close = tail.strip_prefix(')')?;
        return line.map(|line| (line, rest.len() - close.len()));
    }
    let after = rest.strip_prefix(['"', '\'']).unwrap_or(rest);
    for lead in [", line ", ", lines ", ": line ", " line ", " on line "] {
        if let Some(number) = after.strip_prefix(lead) {
            let (line, len) = digits(number);
            return line.map(|line| (line, rest.len() - number.len() + len));
        }
    }
    None
}

/// The URL that starts at byte `start` of `text` and ends at or before `end`.
fn url_at(text: &str, start: usize, end: usize) -> Option<Link> {
    let mut url = &text[start..end];
    if let Some(stop) = url.find(['<', '>', '"', '\'', '`']) {
        url = &url[..stop];
    }
    while let Some(last) = url.chars().last() {
        let unbalanced = |open: char| url.matches(open).count() < url.matches(last).count();
        let cut =
            url_tail(last) || (last == ')' && unbalanced('(')) || (last == ']' && unbalanced('['));
        if !cut {
            break;
        }
        url = &url[..url.len() - last.len_utf8()];
    }
    let host = url.split_once("://")?.1;
    if host.is_empty() {
        return None;
    }
    Some(Link {
        range: start..start + url.len(),
        target: LinkTarget::Url(url.to_string()),
    })
}

/// The file path that starts in the word at `start..end` of `row`.
fn path_at(row: &str, start: usize, end: usize) -> Option<Link> {
    let mut begin = start
        + row[start..end]
            .find(|c: char| !matches!(c, '\'' | '"' | '(' | '[' | '{' | '<' | '`'))
            .unwrap_or(end - start);
    if row[begin..end].starts_with("file://") {
        begin += "file://".len();
    }
    // Diff headers name `a/path` and `b/path`.
    let diff_side = row[begin..end].starts_with("a/") || row[begin..end].starts_with("b/");
    if (begin == 4 && (row.starts_with("--- a/") || row.starts_with("+++ b/")))
        || (diff_side && row.starts_with("diff --git"))
    {
        begin += 2;
    }
    let mut stop = begin + row[begin..end].find(ends_path).unwrap_or(end - begin);
    let line = line_suffix(&row[stop..]);
    if line.is_none() {
        let trimmed = row[begin..stop].trim_end_matches(['.', ',', '[', ']']);
        stop = begin + trimmed.len();
    }
    let path = &row[begin..stop];
    if path.is_empty() || !(path.contains('/') || (line.is_some() && path.contains('.'))) {
        return None;
    }
    let close = line.map_or(stop, |(_, len)| stop + len);
    Some(Link {
        range: begin..close,
        target: LinkTarget::File {
            path: path.to_string(),
            // `lineStart ? { lineStart } : {}`: line 0 is no line.
            line: line.map(|(line, _)| line).filter(|line| *line > 0),
        },
    })
}

/// The URLs and file paths of a row, left to right.
pub fn detect_links(row: &str) -> Vec<Link> {
    let mut links: Vec<Link> = Vec::new();
    let mut words = Vec::new();
    let mut start = None;
    for (index, c) in row.char_indices().chain([(row.len(), ' ')]) {
        match (c.is_whitespace(), start) {
            (false, None) => start = Some(index),
            (true, Some(from)) => {
                words.push(from..index);
                start = None;
            }
            _ => {}
        }
    }
    for word in words {
        if links.last().is_some_and(|link| link.range.end > word.start) {
            continue;
        }
        let token = &row[word.clone()];
        let scheme = [token.find("https://"), token.find("http://")]
            .into_iter()
            .flatten()
            .min();
        let link = match scheme {
            Some(at) => url_at(row, word.start + at, word.end),
            None => path_at(row, word.start, word.end),
        };
        links.extend(link);
    }
    links
}

/// The cells a character takes: 2 for East Asian wide characters, 0 for combining marks.
fn cell_width(c: char) -> usize {
    match c as u32 {
        0x0300..=0x036F
        | 0x1AB0..=0x1AFF
        | 0x1DC0..=0x1DFF
        | 0x200B..=0x200F
        | 0x20D0..=0x20FF
        | 0xFE00..=0xFE0F
        | 0xFE20..=0xFE2F => 0,
        0x1100..=0x115F
        | 0x2E80..=0x303E
        | 0x3041..=0x33FF
        | 0x3400..=0x4DBF
        | 0x4E00..=0x9FFF
        | 0xA000..=0xA4CF
        | 0xAC00..=0xD7A3
        | 0xF900..=0xFAFF
        | 0xFE30..=0xFE4F
        | 0xFF00..=0xFF60
        | 0xFFE0..=0xFFE6
        | 0x1F300..=0x1F64F
        | 0x1F900..=0x1F9FF
        | 0x20000..=0x3FFFD => 2,
        _ => 1,
    }
}

/// The byte of `row` drawn in cell `col`. A drawn row skips the spacer cell after a wide
/// character, so a cell is not a character index.
pub fn byte_at_column(row: &str, col: usize) -> Option<usize> {
    let mut at = 0;
    for (index, c) in row.char_indices() {
        let width = cell_width(c);
        if width == 0 {
            continue;
        }
        if col < at + width {
            return Some(index);
        }
        at += width;
    }
    None
}

/// `MAX_LINK_LENGTH` of the Electron provider: a wrapped line is joined up to about this many
/// bytes before and after the row.
const MAX_LINK_LENGTH: usize = 500;

/// A link in a line that may wrap over several rows: what it opens, and the byte range it covers
/// in each row it touches, as (row, range).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowLink {
    pub target: LinkTarget,
    pub spans: Vec<(usize, Range<usize>)>,
}

/// The first and last row of the wrapped line that holds `row`. `rows[i].1` tells whether row
/// `i` goes on in row `i + 1`.
fn wrapped_rows(rows: &[(&str, bool)], row: usize) -> (usize, usize) {
    let (mut first, mut before) = (row, 0);
    while first > 0 && rows[first - 1].1 && before < MAX_LINK_LENGTH {
        first -= 1;
        before += rows[first].0.len();
    }
    let (mut last, mut after) = (row, 0);
    while last + 1 < rows.len() && rows[last].1 && after < MAX_LINK_LENGTH {
        last += 1;
        after += rows[last].0.len();
    }
    (first, last)
}

/// The link drawn in cell `col` of row `row`. `rows` holds each drawn row's text and whether it
/// wraps onto the next row.
pub fn link_in_rows(rows: &[(&str, bool)], row: usize, col: usize) -> Option<RowLink> {
    let byte_in_row = byte_at_column(rows.get(row)?.0, col)?;
    let (first, last) = wrapped_rows(rows, row);
    let mut text = String::new();
    let mut starts = Vec::with_capacity(last - first + 1);
    for (row_text, _) in &rows[first..=last] {
        starts.push(text.len());
        text.push_str(row_text);
    }
    let byte = starts[row - first] + byte_in_row;
    let link = detect_links(&text)
        .into_iter()
        .find(|link| link.range.contains(&byte))?;
    let spans = (first..=last)
        .filter_map(|index| {
            let start = starts[index - first];
            let end = start + rows[index].0.len();
            let (from, to) = (link.range.start.max(start), link.range.end.min(end));
            (from < to).then(|| (index, from - start..to - start))
        })
        .collect();
    Some(RowLink {
        target: link.target,
        spans,
    })
}

/// How to ask the daemon whether a link's file exists: the folder to search, the name to search
/// for, and the folder the found path is under when that is not the terminal's folder.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Lookup {
    pub cwd: String,
    pub query: String,
    pub base: Option<String>,
}

/// Drops `.` and `..` parts of an absolute path.
fn normalize(path: &str) -> String {
    let mut parts: Vec<&str> = Vec::new();
    for part in path.split('/') {
        match part {
            "" | "." => {}
            ".." => {
                parts.pop();
            }
            part => parts.push(part),
        }
    }
    format!("/{}", parts.join("/"))
}

/// The lookup for `path` in a terminal that runs in `cwd`. A path under `cwd` is searched as a
/// relative path there; another absolute path is searched by name in its own folder. A `~` path,
/// or a relative path with no `cwd`, has no lookup.
pub fn lookup(cwd: &str, path: &str) -> Option<Lookup> {
    let path = path.strip_prefix("file://").unwrap_or(path);
    let root = cwd.trim_end_matches('/');
    let absolute = if path.starts_with('/') {
        normalize(path)
    } else if path.is_empty() || path.starts_with('~') || root.is_empty() {
        return None;
    } else {
        normalize(&format!("{root}/{path}"))
    };
    if !root.is_empty()
        && let Some(relative) = absolute.strip_prefix(&format!("{root}/"))
    {
        return Some(Lookup {
            cwd: root.to_string(),
            query: relative.to_string(),
            base: None,
        });
    }
    let (parent, name) = absolute.rsplit_once('/')?;
    if name.is_empty() {
        return None;
    }
    let parent = if parent.is_empty() { "/" } else { parent };
    Some(Lookup {
        cwd: parent.to_string(),
        query: name.to_string(),
        base: Some(parent.to_string()),
    })
}

/// The file a link opens, from the daemon's suggestions for its lookup (Q-112): the file at
/// exactly that path, else the one file whose path ends with it. With no such file, or 2 or
/// more, the link opens nothing.
pub fn resolve(lookup: &Lookup, found: &[DirectorySuggestion]) -> Option<String> {
    let files: Vec<&str> = found
        .iter()
        .filter(|entry| !entry.is_directory)
        .map(|entry| entry.path.trim_start_matches("./"))
        .collect();
    let suffix = format!("/{}", lookup.query);
    let path = match files.iter().find(|path| **path == lookup.query) {
        Some(exact) => *exact,
        None => {
            let mut ends = files.iter().filter(|path| path.ends_with(&suffix));
            match (ends.next(), ends.next()) {
                (Some(one), None) => *one,
                _ => return None,
            }
        }
    };
    Some(match &lookup.base {
        Some(base) => format!("{}/{path}", base.trim_end_matches('/')),
        None => path.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn file(path: &str, line: Option<u32>) -> LinkTarget {
        LinkTarget::File {
            path: path.into(),
            line,
        }
    }

    fn targets(row: &str) -> Vec<(String, LinkTarget)> {
        detect_links(row)
            .into_iter()
            .map(|link| (row[link.range].to_string(), link.target))
            .collect()
    }

    #[test]
    fn urls_and_paths_with_lines_are_found() {
        assert_eq!(
            targets("echo hello https://example.com src/main.rs:1"),
            [
                (
                    "https://example.com".into(),
                    LinkTarget::Url("https://example.com".into())
                ),
                ("src/main.rs:1".into(), file("src/main.rs", Some(1))),
            ]
        );
    }

    #[test]
    fn urls_drop_trailing_punctuation_and_unbalanced_brackets() {
        let url = |row: &str| match &targets(row)[..] {
            [(_, LinkTarget::Url(url))] => url.clone(),
            other => panic!("{other:?}"),
        };
        assert_eq!(url("see https://a.dev/x."), "https://a.dev/x");
        assert_eq!(url("(https://a.dev/x)"), "https://a.dev/x");
        assert_eq!(
            url("https://en.wikipedia.org/wiki/Rust_(language)"),
            "https://en.wikipedia.org/wiki/Rust_(language)"
        );
        assert_eq!(
            url("\"http://localhost:3000/a?b=1\","),
            "http://localhost:3000/a?b=1"
        );
        assert!(targets("https://").is_empty());
    }

    #[test]
    fn line_suffixes_take_the_first_number() {
        assert_eq!(targets("src/a.rs:12:4")[0].1, file("src/a.rs", Some(12)));
        assert_eq!(targets("main.rs:42")[0].1, file("main.rs", Some(42)));
        assert_eq!(targets("src/a.ts(12,4)")[0].1, file("src/a.ts", Some(12)));
        assert_eq!(
            targets("  File \"app/run.py\", line 7, in main"),
            [("app/run.py\", line 7".into(), file("app/run.py", Some(7)))]
        );
        assert_eq!(targets("src/a.rs:0")[0].1, file("src/a.rs", None));
    }

    #[test]
    fn plain_words_and_times_are_not_links() {
        assert!(targets("hello world 12:30 a:b").is_empty());
        assert!(targets("done.").is_empty());
        assert_eq!(targets("see ./docs/x.md.")[0].1, file("./docs/x.md", None));
        assert_eq!(targets("/tmp/out.log,")[0].1, file("/tmp/out.log", None));
    }

    #[test]
    fn diff_headers_drop_the_side_prefix() {
        assert_eq!(targets("--- a/src/x.rs")[0].1, file("src/x.rs", None));
        assert_eq!(targets("+++ b/src/x.rs")[0].1, file("src/x.rs", None));
        let both = targets("diff --git a/src/x.rs b/src/x.rs");
        assert_eq!(both.len(), 2);
        assert_eq!(both[1].1, file("src/x.rs", None));
    }

    #[test]
    fn a_cell_finds_its_link_across_wide_characters() {
        let link_at = |row: &str, col| link_in_rows(&[(row, false)], 0, col);
        let row = "日本 src/a.rs:3";
        // Each wide character takes two cells, so the path starts at cell 5.
        assert_eq!(link_at(row, 4), None);
        assert_eq!(link_at(row, 5).unwrap().target, file("src/a.rs", Some(3)));
        assert_eq!(link_at(row, 14).unwrap().target, file("src/a.rs", Some(3)));
        assert_eq!(link_at(row, 15), None);
        assert_eq!(byte_at_column("e\u{301}x", 1), Some(3));
    }

    #[test]
    fn a_path_wrapped_over_two_rows_opens_whole_from_either_row() {
        let rows = [
            ("see src/lo", true),
            ("ng.rs:7 ok", false),
            ("next/a.rs ", false),
        ];
        let whole = RowLink {
            target: file("src/long.rs", Some(7)),
            spans: vec![(0, 4..10), (1, 0..7)],
        };
        assert_eq!(link_in_rows(&rows, 0, 5), Some(whole.clone()));
        assert_eq!(link_in_rows(&rows, 1, 2), Some(whole));
        assert_eq!(link_in_rows(&rows, 1, 9), None);
        // A row that did not wrap is not joined to the next one.
        assert_eq!(link_in_rows(&rows, 2, 0).unwrap().spans, [(2, 0..9)]);
    }

    #[test]
    fn a_path_wrapped_over_three_rows_opens_whole_from_the_middle() {
        let rows = [
            ("$ cat /tmp", true),
            ("/very/deep", true),
            ("/dir/x.log", false),
        ];
        let link = link_in_rows(&rows, 1, 4).unwrap();
        assert_eq!(link.target, file("/tmp/very/deep/dir/x.log", None));
        assert_eq!(link.spans, [(0, 6..10), (1, 0..10), (2, 0..10)]);
        assert_eq!(link_in_rows(&rows, 2, 9).unwrap().target, link.target);
    }

    #[test]
    fn a_wrapped_url_opens_whole() {
        let rows = [("https://ex", true), ("ample.com/", true), ("a", false)];
        assert_eq!(
            link_in_rows(&rows, 2, 0).unwrap().target,
            LinkTarget::Url("https://example.com/a".into())
        );
    }

    #[test]
    fn lookups_search_relative_paths_in_the_cwd() {
        let at = |cwd: &str, query: &str, base: Option<&str>| Lookup {
            cwd: cwd.into(),
            query: query.into(),
            base: base.map(Into::into),
        };
        assert_eq!(
            lookup("/w/proj", "src/main.rs"),
            Some(at("/w/proj", "src/main.rs", None))
        );
        assert_eq!(
            lookup("/w/proj/", "./src/../lib.rs"),
            Some(at("/w/proj", "lib.rs", None))
        );
        assert_eq!(
            lookup("/w/proj", "/w/proj/a/b.rs"),
            Some(at("/w/proj", "a/b.rs", None))
        );
        assert_eq!(
            lookup("/w/proj", "file:///etc/hosts"),
            Some(at("/etc", "hosts", Some("/etc")))
        );
        assert_eq!(
            lookup("/w/proj", "../other/x.rs"),
            Some(at("/w/other", "x.rs", Some("/w/other")))
        );
        assert_eq!(lookup("", "src/main.rs"), None);
        assert_eq!(lookup("/w/proj", "~/x.rs"), None);
        assert_eq!(lookup("", "/x.rs"), Some(at("/", "x.rs", Some("/"))));
    }

    fn found(paths: &[&str]) -> Vec<DirectorySuggestion> {
        paths
            .iter()
            .map(|path| DirectorySuggestion {
                path: path.to_string(),
                is_directory: path.ends_with('/'),
            })
            .collect()
    }

    /// Q-112: 0 matches and 2 matches open nothing; an exact path or 1 match opens.
    #[test]
    fn a_link_opens_only_an_exact_or_single_match() {
        let relative = lookup("/w", "src/main.rs").unwrap();
        assert_eq!(resolve(&relative, &found(&[])), None);
        assert_eq!(resolve(&relative, &found(&["src/domain.rs", "src/"])), None);
        assert_eq!(
            resolve(&relative, &found(&["a/src/main.rs"])),
            Some("a/src/main.rs".into())
        );
        assert_eq!(
            resolve(&relative, &found(&["a/src/main.rs", "b/src/main.rs"])),
            None
        );
        assert_eq!(
            resolve(
                &relative,
                &found(&["a/src/main.rs", "src/main.rs", "b/src/main.rs"])
            ),
            Some("src/main.rs".into())
        );
        let outside = lookup("/w", "/etc/hosts").unwrap();
        assert_eq!(
            resolve(&outside, &found(&["hosts", "x/hosts"])),
            Some("/etc/hosts".into())
        );
    }
}
