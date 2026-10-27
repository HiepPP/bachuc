//! File links in assistant text: a port of `packages/app/src/assistant-file-links/parse.ts`.
//!
//! A Markdown link, or an inline code span that looks like a file (`src/app.ts:12`), opens the
//! file. `linkify_code_spans` turns such code spans into Markdown links, so the text view's link
//! click handler sees both kinds.

/// A file a link points at. `raw` is the text the link was parsed from.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileTarget {
    pub raw: String,
    pub path: String,
    pub line_start: Option<u32>,
    pub line_end: Option<u32>,
}

/// What a link is, as `classifyAssistantFileLink` sorts it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkKind {
    External(String),
    DirectFile(FileTarget),
    /// A bare file name under the workspace root. The app asks the daemon for the file; this
    /// client has no suffix lookup, so it opens the name under the root.
    AmbiguousFile(FileTarget),
}

const ASSISTANT_FILE_EXTENSIONS: &[&str] = &[
    "astro", "bash", "c", "cc", "cjs", "cpp", "cs", "css", "cts", "cxx", "env", "fish", "go",
    "gql", "gradle", "graphql", "h", "hpp", "htm", "html", "ini", "java", "js", "json", "jsonc",
    "jsx", "kt", "kts", "less", "lock", "lua", "md", "mdx", "mjs", "mts", "php", "proto", "py",
    "rb", "rs", "sass", "scss", "sh", "sql", "svelte", "swift", "toml", "ts", "tsx", "txt", "vue",
    "xml", "yaml", "yml", "zsh",
];

type Lines = (Option<u32>, Option<u32>);

fn strip_quotes(value: &str) -> &str {
    let quote = |c: char| matches!(c, '\'' | '"' | '`');
    let value = value.strip_prefix(quote).unwrap_or(value);
    value.strip_suffix(quote).unwrap_or(value)
}

fn normalize_path_token(value: &str) -> Option<String> {
    let trimmed = strip_quotes(value.trim());
    (!trimmed.is_empty()).then(|| trimmed.replace('\\', "/"))
}

fn normalize_path_input(value: Option<&str>) -> Option<String> {
    let trimmed = strip_quotes(value?.trim());
    if trimmed.is_empty() {
        return None;
    }
    let mut out = String::with_capacity(trimmed.len());
    for c in trimmed.replace('\\', "/").chars() {
        if c == '/' && out.ends_with('/') {
            continue;
        }
        out.push(c);
    }
    Some(out)
}

pub fn is_absolute_path(value: &str) -> bool {
    value.starts_with('/') || value.starts_with("\\\\") || is_windows_drive_path(value)
}

fn is_windows_drive_path(value: &str) -> bool {
    let bytes = value.as_bytes();
    bytes.len() >= 3
        && bytes[0].is_ascii_alphabetic()
        && bytes[1] == b':'
        && matches!(bytes[2], b'/' | b'\\')
}

fn digits(value: &str) -> Option<u32> {
    (!value.is_empty() && value.bytes().all(|b| b.is_ascii_digit()))
        .then(|| value.parse().ok())
        .flatten()
}

/// `L12`, `L12C4`, `L12-L20`, `L12C4-20C2`, case-insensitive. Returns the two line numbers.
fn match_line_fragment(fragment: &str) -> Option<(u32, Option<u32>)> {
    let rest = fragment.strip_prefix(['L', 'l'])?;
    let (start, rest) = split_digits(rest)?;
    let rest = skip_column(rest, ['C', 'c']);
    if rest.is_empty() {
        return Some((start, None));
    }
    let rest = rest.strip_prefix('-')?;
    let rest = rest.strip_prefix(['L', 'l']).unwrap_or(rest);
    let (end, rest) = split_digits(rest)?;
    skip_column(rest, ['C', 'c'])
        .is_empty()
        .then_some((start, Some(end)))
}

fn split_digits(value: &str) -> Option<(u32, &str)> {
    let count = value.bytes().take_while(u8::is_ascii_digit).count();
    Some((digits(&value[..count])?, &value[count..]))
}

/// Skips `<marker><digits>` when it is there.
fn skip_column(value: &str, marker: [char; 2]) -> &str {
    match value.strip_prefix(marker) {
        Some(rest) => match split_digits(rest) {
            Some((_, after)) => after,
            None => value,
        },
        None => value,
    }
}

/// `parseLineFragment`: None for an invalid range, no lines for an empty or unknown fragment.
fn parse_line_fragment(value: &str) -> Option<Lines> {
    let fragment = value.strip_prefix('#').unwrap_or(value);
    if fragment.is_empty() {
        return Some((None, None));
    }
    match match_line_fragment(fragment) {
        Some((start, end)) => {
            if start == 0 || end == Some(0) || end.is_some_and(|end| end < start) {
                None
            } else {
                Some((Some(start), end))
            }
        }
        None => Some((None, None)),
    }
}

/// `<digits>[<sep><digits>][-<digits>[<sep><digits>]]` filling the whole of `value`.
fn match_line_suffix(value: &str, separator: char) -> Option<(u32, Option<u32>)> {
    let (start, rest) = split_digits(value)?;
    let rest = skip_separated(rest, separator);
    if rest.is_empty() {
        return Some((start, None));
    }
    let (end, rest) = split_digits(rest.strip_prefix('-')?)?;
    skip_separated(rest, separator)
        .is_empty()
        .then_some((start, Some(end)))
}

fn skip_separated(value: &str, separator: char) -> &str {
    match value.strip_prefix(separator).and_then(split_digits) {
        Some((_, after)) => after,
        None => value,
    }
}

/// The three suffix forms of `parseInlinePathToken`, each with the shortest base path.
fn match_inline_suffix(value: &str) -> Option<(&str, u32, Option<u32>)> {
    // `path:12`, `path:12:4`, `path:12-20`.
    for (index, _) in value.match_indices(':') {
        if index == 0 || value[..index].contains('\n') {
            continue;
        }
        if let Some((start, end)) = match_line_suffix(&value[index + 1..], ':') {
            return Some((&value[..index], start, end));
        }
    }
    // `path(12)`, `path(12,4)`, `path(12-20)`.
    if let Some(inner_end) = value.strip_suffix(')') {
        for (index, _) in inner_end.match_indices('(') {
            if index == 0 {
                continue;
            }
            if let Some((start, end)) = match_line_suffix(&inner_end[index + 1..], ',') {
                return Some((&value[..index], start, end));
            }
        }
    }
    // `path line 12`, `path lines 12-20`.
    for (index, c) in value.char_indices() {
        if index == 0 || !c.is_whitespace() {
            continue;
        }
        let rest = value[index..].trim_start();
        if !rest
            .get(..4)
            .is_some_and(|word| word.eq_ignore_ascii_case("line"))
        {
            continue;
        }
        let mut rest = &rest[4..];
        rest = rest.strip_prefix(['s', 'S']).unwrap_or(rest);
        let trimmed = rest.trim_start();
        if trimmed.len() == rest.len() {
            continue;
        }
        let Some((start, after)) = split_digits(trimmed) else {
            continue;
        };
        let end = match after.strip_prefix('-') {
            Some(range) => match digits(range) {
                Some(end) => Some(end),
                None => continue,
            },
            None if after.is_empty() => None,
            None => continue,
        };
        return Some((&value[..index], start, end));
    }
    None
}

/// `parseInlinePathToken`: VS Code-style `path:line`, `path:line:column`, `path:start-end`,
/// `path(line,column)`, and `path lines start-end`. A plain path gives None.
pub fn parse_inline_path_token(value: &str) -> Option<FileTarget> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    let (base, start, end) = match_inline_suffix(trimmed)?;
    let base = base.trim();
    if base.is_empty() || base.contains("://") {
        return None;
    }
    let path = normalize_path_token(base)?;
    if start == 0 || end == Some(0) || end.is_some_and(|end| end < start) {
        return None;
    }
    Some(FileTarget {
        raw: value.to_string(),
        path,
        line_start: Some(start),
        line_end: end,
    })
}

/// The file and lines a code fence cites in its meta, the text after the language: one token,
/// `path:12` or `path:12-20`. Other meta, such as `title="x"`, gives None.
pub fn code_citation(meta: &str) -> Option<FileTarget> {
    let meta = meta.trim();
    if meta.is_empty() || meta.contains(|c: char| c.is_whitespace() || "=\"'()".contains(c)) {
        return None;
    }
    // One colon, so `path:12:4` (a column) is not a citation.
    let (_, lines) = meta
        .split_once(':')
        .filter(|(_, lines)| !lines.contains(':'))?;
    let (start, end) = lines.split_once('-').unwrap_or((lines, "1"));
    let is_number = |text: &str| !text.is_empty() && text.bytes().all(|b| b.is_ascii_digit());
    if !is_number(start) || !is_number(end) {
        return None;
    }
    parse_inline_path_token(meta)
}

fn percent_decode(value: &str) -> String {
    let bytes = value.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut index = 0;
    while index < bytes.len() {
        let hex = |b: u8| (b as char).to_digit(16);
        if bytes[index] == b'%'
            && index + 2 < bytes.len()
            && let (Some(high), Some(low)) = (hex(bytes[index + 1]), hex(bytes[index + 2]))
        {
            out.push((high * 16 + low) as u8);
            index += 3;
            continue;
        }
        out.push(bytes[index]);
        index += 1;
    }
    // A malformed escape leaves the text as it was, as `safeDecodeURIComponent` does.
    String::from_utf8(out).unwrap_or_else(|_| value.to_string())
}

/// Splits `path?query#hash` into the path and the hash (with its `#`).
fn split_url_path(value: &str) -> (&str, &str) {
    let (before_hash, hash) = match value.find('#') {
        Some(index) => (&value[..index], &value[index..]),
        None => (value, ""),
    };
    let path = before_hash.split('?').next().unwrap_or("");
    (path, hash)
}

/// `parseFileProtocolUrl`: `file:///path#L12`.
pub fn parse_file_protocol_url(value: &str) -> Option<FileTarget> {
    let trimmed = value.trim();
    let rest = trimmed
        .get(..7)
        .filter(|scheme| scheme.eq_ignore_ascii_case("file://"))
        .map(|_| &trimmed[7..])?;
    let (path, hash) = split_url_path(rest);
    // `file://host/path` keeps only the path.
    let path = &path[path.find('/')?..];
    let decoded = percent_decode(path).replace('\\', "/");
    if decoded.is_empty() {
        return None;
    }
    let decoded = match decoded.strip_prefix('/') {
        Some(rest) if is_windows_drive_path(rest) => rest.to_string(),
        _ => decoded,
    };
    let (line_start, line_end) = parse_line_fragment(hash)?;
    Some(FileTarget {
        raw: value.to_string(),
        path: decoded,
        line_start,
        line_end,
    })
}

fn is_external_href(value: &str) -> bool {
    if value.contains("://") {
        return !value.to_lowercase().starts_with("file://");
    }
    if parse_inline_path_token(value).is_some() {
        return false;
    }
    let scheme_end = value.find(':');
    let has_scheme = scheme_end.is_some_and(|end| {
        let scheme = &value[..end];
        scheme.starts_with(|c: char| c.is_ascii_alphabetic())
            && scheme
                .chars()
                .all(|c| c.is_ascii_alphanumeric() || matches!(c, '+' | '.' | '-'))
    });
    has_scheme && !is_windows_drive_path(value)
}

fn is_home_relative_path(value: &str) -> bool {
    value == "~" || value.starts_with("~/") || value.starts_with("~\\")
}

fn is_domain_like_segment(segment: &str) -> bool {
    let parts: Vec<&str> = segment.split('.').collect();
    parts.len() >= 2
        && parts.iter().all(|part| {
            !part.is_empty() && part.chars().all(|c| c.is_ascii_alphanumeric() || c == '-')
        })
}

fn is_plausible_file_name(name: &str) -> bool {
    if name.starts_with('.') && name.len() > 1 {
        return true;
    }
    let Some(dot) = name.rfind('.') else {
        return false;
    };
    let extension = name[dot + 1..].to_lowercase();
    ASSISTANT_FILE_EXTENSIONS.contains(&extension.as_str())
}

fn is_plausible_local_path(value: &str) -> bool {
    let Some(normalized) = normalize_path_token(value) else {
        return false;
    };
    if is_absolute_path(&normalized) {
        return true;
    }
    if ["./", "../", "~/"]
        .iter()
        .any(|prefix| normalized.starts_with(prefix))
    {
        return true;
    }
    let segments: Vec<&str> = normalized.split('/').filter(|s| !s.is_empty()).collect();
    match segments.as_slice() {
        [] => false,
        [only] => is_plausible_file_name(only),
        [first, .., last] => !is_domain_like_segment(first) && is_plausible_file_name(last),
    }
}

/// `parseLocalPathParts`: a relative or home path with its lines.
fn parse_local_path_parts(value: &str) -> Option<(String, Lines)> {
    let normalized = normalize_path_token(value)?;
    if normalized.contains('?') {
        return None;
    }
    let (before_hash, hash) = match normalized.find('#') {
        Some(index) => (&normalized[..index], &normalized[index..]),
        None => (normalized.as_str(), ""),
    };
    let fragment_lines = parse_line_fragment(hash)?;
    if let Some(target) = parse_inline_path_token(before_hash) {
        if !is_plausible_local_path(&target.path) {
            return None;
        }
        return Some((target.path, (target.line_start, target.line_end)));
    }
    if before_hash.is_empty() || before_hash.contains(':') || !is_plausible_local_path(before_hash)
    {
        return None;
    }
    Some((before_hash.to_string(), fragment_lines))
}

fn resolve_relative_under_root(path: &str, root: &str) -> Option<String> {
    let normalized = normalize_path_token(path)?;
    if is_absolute_path(&normalized) {
        return None;
    }
    let root = match root.trim_end_matches('/') {
        "" => "/",
        root => root,
    };
    let mut resolved: Vec<&str> = Vec::new();
    for segment in normalized.split('/') {
        match segment {
            "" | "." => {}
            ".." => {
                resolved.pop()?;
            }
            segment => resolved.push(segment),
        }
    }
    if resolved.is_empty() {
        return Some(root.to_string());
    }
    Some(if root == "/" {
        format!("/{}", resolved.join("/"))
    } else {
        format!("{root}/{}", resolved.join("/"))
    })
}

fn parse_workspace_relative(value: &str, root: Option<&str>) -> Option<FileTarget> {
    let (path, (line_start, line_end)) = parse_local_path_parts(value)?;
    if is_absolute_path(&path) {
        return None;
    }
    let path = if is_home_relative_path(&path) {
        path
    } else {
        resolve_relative_under_root(&path, &normalize_path_input(root)?)?
    };
    Some(FileTarget {
        raw: value.to_string(),
        path,
        line_start,
        line_end,
    })
}

fn compare_form(value: &str) -> String {
    if value.as_bytes().get(1) == Some(&b':') {
        value.to_lowercase()
    } else {
        value.to_string()
    }
}

fn is_allowed_absolute_path(path: &str, root: Option<&str>) -> bool {
    let Some(root) = normalize_path_input(root) else {
        return true;
    };
    let root = compare_form(match root.trim_end_matches('/') {
        "" => "/",
        root => root,
    });
    let path = compare_form(path);
    let prefix = if root == "/" {
        "/".to_string()
    } else {
        format!("{root}/")
    };
    path == root || path.starts_with(&prefix)
}

/// `parseAssistantFileLink`: the file a link text or href points at, resolved under `root`.
pub fn parse_file_link(value: &str, root: Option<&str>) -> Option<FileTarget> {
    if let Some(target) = parse_file_protocol_url(value) {
        return Some(target);
    }
    let trimmed = value.trim();
    if trimmed.is_empty() || is_external_href(trimmed) {
        return None;
    }
    if let Some(target) = parse_inline_path_token(trimmed)
        && let Some(path) = normalize_path_token(&target.path)
        && is_absolute_path(&path)
    {
        return Some(FileTarget { path, ..target });
    }
    if is_windows_drive_path(trimmed) {
        let (path, hash) = match trimmed.find('#') {
            Some(index) => (&trimmed[..index], &trimmed[index..]),
            None => (trimmed, ""),
        };
        if !path.contains('?') && !hash.contains('?') && hash != "#" {
            let path = normalize_path_token(path)?;
            let (line_start, line_end) = parse_line_fragment(hash)?;
            return Some(FileTarget {
                raw: value.to_string(),
                path,
                line_start,
                line_end,
            });
        }
    }
    if let Some(target) = parse_workspace_relative(trimmed, root) {
        return Some(target);
    }
    if !is_absolute_path(trimmed) {
        return None;
    }
    let (path, hash) = split_url_path(trimmed);
    let path = normalize_path_token(&percent_decode(path))?;
    if !is_absolute_path(&path) {
        return None;
    }
    let (line_start, line_end) = parse_line_fragment(hash)?;
    Some(FileTarget {
        raw: value.to_string(),
        path,
        line_start,
        line_end,
    })
}

/// `classifyAssistantFileLink`.
pub fn classify(value: &str, root: Option<&str>) -> Option<LinkKind> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if is_external_href(trimmed) {
        return Some(LinkKind::External(value.to_string()));
    }
    if trimmed.chars().any(char::is_whitespace) {
        return None;
    }
    let target = parse_file_link(trimmed, root)?;
    let is_ambiguous = normalize_path_input(root).is_some()
        && is_allowed_absolute_path(&target.path, root)
        && parse_local_path_parts(trimmed)
            .is_some_and(|(path, _)| !is_absolute_path(&path) && !path.contains('/'));
    Some(if is_ambiguous {
        LinkKind::AmbiguousFile(target)
    } else {
        LinkKind::DirectFile(target)
    })
}

/// `isFileLookingAssistantToken`: inline code that reads as a file path.
pub fn is_file_looking_token(value: &str) -> bool {
    let Some(normalized) = normalize_path_token(value) else {
        return false;
    };
    if normalized.chars().any(char::is_whitespace)
        || normalized.contains('?')
        || normalized.contains("://")
    {
        return false;
    }
    let (before_hash, hash) = match normalized.find('#') {
        Some(index) => (&normalized[..index], &normalized[index..]),
        None => (normalized.as_str(), ""),
    };
    if parse_line_fragment(hash).is_none() {
        return false;
    }
    let path = match parse_inline_path_token(before_hash) {
        Some(target) => target.path,
        None if before_hash.is_empty() || before_hash.contains(':') => return false,
        None => before_hash.to_string(),
    };
    is_plausible_local_path(&path)
}

/// Wraps each inline code span that looks like a file in a Markdown link to its own text, so a
/// click reaches the link handler. Fenced code and code already inside a link stay as they are.
pub fn linkify_code_spans(markdown: &str) -> String {
    if !markdown.contains('`') {
        return markdown.to_string();
    }
    let mut out = String::with_capacity(markdown.len() + 32);
    let mut fence: Option<(char, usize)> = None;
    for line in markdown.split_inclusive('\n') {
        let trimmed = line.trim_start();
        let marker = trimmed.chars().next().filter(|c| *c == '`' || *c == '~');
        let run = marker.map_or(0, |m| trimmed.chars().take_while(|c| *c == m).count());
        match (fence, marker) {
            (Some((open, length)), Some(m)) if m == open && run >= length => {
                fence = None;
                out.push_str(line);
                continue;
            }
            (Some(_), _) => {
                out.push_str(line);
                continue;
            }
            (None, Some(m)) if run >= 3 => {
                fence = Some((m, run));
                out.push_str(line);
                continue;
            }
            _ => {}
        }
        linkify_line(line, &mut out);
    }
    out
}

fn linkify_line(line: &str, out: &mut String) {
    let bytes = line.as_bytes();
    let mut index = 0;
    let mut copied = 0;
    while index < bytes.len() {
        if bytes[index] != b'`' {
            index += 1;
            continue;
        }
        let open = bytes[index..].iter().take_while(|b| **b == b'`').count();
        let content_start = index + open;
        // The span closes at the next run of exactly the same length.
        let mut search = content_start;
        let mut close = None;
        while search < bytes.len() {
            if bytes[search] == b'`' {
                let run = bytes[search..].iter().take_while(|b| **b == b'`').count();
                if run == open {
                    close = Some(search);
                    break;
                }
                search += run;
            } else {
                search += 1;
            }
        }
        let Some(close) = close else {
            break;
        };
        let span_end = close + open;
        let content = line[content_start..close].trim();
        let in_link = line[..index].ends_with('[') || line[span_end..].starts_with("](");
        if !in_link && !content.contains(['<', '>']) && is_file_looking_token(content) {
            out.push_str(&line[copied..index]);
            out.push('[');
            out.push_str(&line[index..span_end]);
            out.push_str("](<");
            out.push_str(content);
            out.push_str(">)");
            copied = span_end;
        }
        index = span_end;
    }
    out.push_str(&line[copied..]);
}

/// What a click on a link in a message does.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum LinkClick {
    External(String),
    File(FileTarget),
    Ignored,
}

/// Resolves a clicked link href against the agent's folder.
pub fn resolve_click(href: &str, cwd: &str) -> LinkClick {
    let root = (!cwd.is_empty()).then_some(cwd);
    match classify(href, root) {
        Some(LinkKind::External(url)) => LinkClick::External(url),
        Some(LinkKind::DirectFile(target) | LinkKind::AmbiguousFile(target)) => {
            LinkClick::File(target)
        }
        None => LinkClick::Ignored,
    }
}
