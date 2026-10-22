//! The model of the rich markdown diff: the old text of a file rebuilt from its new text and its
//! unified diff, and the blocks of two markdown texts marked same, added, or removed.

/// The LCS table holds one cell per old-by-new block pair; above this the middle part is not
/// matched, to cap its memory.
const MAX_BLOCK_PAIRS: usize = 4_000_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BlockKind {
    Same,
    Added,
    Removed,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RichBlock {
    pub kind: BlockKind,
    pub markdown: String,
}

/// Whether a path is a markdown file: `.md` or `.markdown`, in any case.
pub fn is_markdown_path(path: &str) -> bool {
    let path = path.to_ascii_lowercase();
    path.ends_with(".md") || path.ends_with(".markdown")
}

/// The new-side start (as an index into the new lines) and the lines of one hunk.
struct Hunk<'a> {
    start: usize,
    lines: Vec<&'a str>,
}

/// The index in the new text where a hunk `@@ -a,b +c,d @@` begins. A hunk with no new lines sits
/// after line `c`, so its index is `c`; otherwise it is `c - 1`.
fn hunk_start(header: &str) -> Option<usize> {
    let mut parts = header.split_whitespace();
    parts.next().filter(|marker| *marker == "@@")?;
    parts.next()?.strip_prefix('-')?;
    let mut new = parts.next()?.strip_prefix('+')?.split(',');
    let start: usize = new.next()?.parse().ok()?;
    let count: usize = match new.next() {
        Some(count) => count.parse().ok()?,
        None => 1,
    };
    Some(if count == 0 {
        start
    } else {
        start.saturating_sub(1)
    })
}

fn hunks(diff_text: &str) -> Option<Vec<Hunk<'_>>> {
    let mut hunks: Vec<Hunk> = Vec::new();
    for line in diff_text.lines() {
        if line.starts_with("@@") {
            hunks.push(Hunk {
                start: hunk_start(line)?,
                lines: Vec::new(),
            });
        } else if let Some(hunk) = hunks.last_mut() {
            hunk.lines.push(line);
        }
    }
    Some(hunks)
}

fn split_lines(text: &str) -> Vec<&str> {
    if text.is_empty() {
        return Vec::new();
    }
    text.strip_suffix('\n')
        .unwrap_or(text)
        .split('\n')
        .collect()
}

fn same_line(a: &str, b: &str) -> bool {
    a.trim_end_matches('\r') == b.trim_end_matches('\r')
}

/// The text before the change, from the text after it and the unified diff of the change. None
/// when a context or added line of the diff does not match the new text.
pub fn old_text(new_text: &str, diff_text: &str) -> Option<String> {
    let new_lines = split_lines(new_text);
    let mut out: Vec<&str> = Vec::with_capacity(new_lines.len());
    let mut cursor = 0;
    for hunk in hunks(diff_text)? {
        if hunk.start < cursor || hunk.start > new_lines.len() {
            return None;
        }
        out.extend_from_slice(&new_lines[cursor..hunk.start]);
        cursor = hunk.start;
        for line in hunk.lines {
            let (prefix, text) = match line.chars().next() {
                Some(prefix @ ('+' | '-' | ' ')) => (prefix, &line[1..]),
                // An empty line is an empty context line that lost its space.
                _ => (' ', line),
            };
            if text.starts_with("\\ No newline") {
                continue;
            }
            if prefix == '-' {
                out.push(text);
                continue;
            }
            let current = new_lines.get(cursor)?;
            if !same_line(current, text) {
                return None;
            }
            if prefix == ' ' {
                out.push(current);
            }
            cursor += 1;
        }
    }
    out.extend_from_slice(&new_lines[cursor..]);
    let mut text = out.join("\n");
    if !out.is_empty() && new_text.ends_with('\n') {
        text.push('\n');
    }
    Some(text)
}

/// The length of the fence a line opens, or 0 when it opens none.
fn fence_len(line: &str) -> usize {
    let Some(fence) = ["```", "~~~"].into_iter().find(|f| line.starts_with(f)) else {
        return 0;
    };
    let mark = fence.chars().next().unwrap_or('`');
    line.chars().take_while(|c| *c == mark).count()
}

fn closes_fence(line: &str, open_len: usize, mark: char) -> bool {
    let line = line.trim_end();
    line.chars().count() >= open_len && line.chars().all(|c| c == mark)
}

/// The blocks of a markdown text: runs of non-blank lines, with a fenced code block kept whole.
fn blocks(text: &str) -> Vec<String> {
    let mut blocks = Vec::new();
    let mut current: Vec<&str> = Vec::new();
    let mut fence: Option<(usize, char)> = None;
    for line in text.lines() {
        if let Some((open_len, mark)) = fence {
            current.push(line);
            if closes_fence(line, open_len, mark) {
                fence = None;
            }
            continue;
        }
        if line.trim().is_empty() {
            if !current.is_empty() {
                blocks.push(current.join("\n"));
                current.clear();
            }
            continue;
        }
        let open_len = fence_len(line);
        if open_len > 0 {
            fence = line.chars().next().map(|mark| (open_len, mark));
        }
        current.push(line);
    }
    if !current.is_empty() {
        blocks.push(current.join("\n"));
    }
    blocks
}

/// Two blocks are the same when they are equal after trimming the end of each line.
fn block_key(block: &str) -> Vec<&str> {
    block.lines().map(str::trim_end).collect()
}

fn push(out: &mut Vec<RichBlock>, kind: BlockKind, markdown: &str) {
    out.push(RichBlock {
        kind,
        markdown: markdown.to_string(),
    });
}

/// The blocks of both texts: the blocks of the new text marked Same or Added, with the blocks
/// that only the old text holds marked Removed before the Added blocks that replace them.
pub fn block_diff(old_text: &str, new_text: &str) -> Vec<RichBlock> {
    let old = blocks(old_text);
    let new = blocks(new_text);
    let old_keys: Vec<_> = old.iter().map(|b| block_key(b)).collect();
    let new_keys: Vec<_> = new.iter().map(|b| block_key(b)).collect();

    let prefix = old_keys
        .iter()
        .zip(&new_keys)
        .take_while(|(a, b)| a == b)
        .count();
    let suffix = old_keys[prefix..]
        .iter()
        .rev()
        .zip(new_keys[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    let (old_end, new_end) = (old.len() - suffix, new.len() - suffix);
    let (old_mid, new_mid) = (&old_keys[prefix..old_end], &new_keys[prefix..new_end]);

    let mut out = Vec::with_capacity(new.len() + old_mid.len());
    for block in &new[..prefix] {
        push(&mut out, BlockKind::Same, block);
    }
    let (n, m) = (old_mid.len(), new_mid.len());
    if n.saturating_mul(m) > MAX_BLOCK_PAIRS {
        for block in &old[prefix..old_end] {
            push(&mut out, BlockKind::Removed, block);
        }
        for block in &new[prefix..new_end] {
            push(&mut out, BlockKind::Added, block);
        }
    } else {
        // lcs[i * (m + 1) + j] is the longest common run of old_mid[i..] and new_mid[j..].
        let width = m + 1;
        let mut lcs = vec![0u32; (n + 1) * width];
        for i in (0..n).rev() {
            for j in (0..m).rev() {
                lcs[i * width + j] = if old_mid[i] == new_mid[j] {
                    lcs[(i + 1) * width + j + 1] + 1
                } else {
                    lcs[(i + 1) * width + j].max(lcs[i * width + j + 1])
                };
            }
        }
        let (mut i, mut j) = (0, 0);
        while i < n || j < m {
            if i < n && j < m && old_mid[i] == new_mid[j] {
                push(&mut out, BlockKind::Same, &new[prefix + j]);
                i += 1;
                j += 1;
            } else if j == m || (i < n && lcs[(i + 1) * width + j] >= lcs[i * width + j + 1]) {
                push(&mut out, BlockKind::Removed, &old[prefix + i]);
                i += 1;
            } else {
                push(&mut out, BlockKind::Added, &new[prefix + j]);
                j += 1;
            }
        }
    }
    for block in &new[new_end..] {
        push(&mut out, BlockKind::Same, block);
    }
    out
}
