# TASK-290 Markdown block diff model

Group: C (writes markdown_diff.rs, lib.rs, and tests/models.rs; no other TASK writes them)
Class: risky

## Brief

Goal: Add a pure model that rebuilds the old text of a file from its new text and its unified diff, and splits two markdown texts into blocks marked same, added, or removed.

Change: no markdown diff model -> `markdown_diff` module with `is_markdown_path`, `old_text`, and `block_diff`.

Boundaries:

- Follows [ADR-0061](watchtower/decisions/ADR-0061-gpui-rich-markdown-diff-from-working-file-over-protocol-full-text.md): rebuild the old side from the working file and the patch.
- The diff text is the one `unified_diff` in [packages/desktop-gpui/crates/client/src/files.rs](packages/desktop-gpui/crates/client/src/files.rs) builds: `--- a/<path>`, `+++ b/<path>`, then `@@ -a,b +c,d @@` headers and lines with a `+`, `-`, or space prefix. A server line holding `\ No newline at end of file` arrives with a space prefix (`unified_diff` maps every other line type to a space).
- Planner call: `old_text` skips a hunk line whose text after the prefix starts with `\ No newline`, as `parse` in `diff.rs` skips the marker. It compares lines with a trailing `\r` removed, so a CRLF file still matches.
- A block is a run of non-blank lines. A fenced code block (a line that starts with three backticks or three tildes, up to its closing fence) is one block, blank lines included.
- Two blocks are the same when their text is equal after trimming trailing whitespace of each line.
- Planner call: when the middle part (after a common prefix and suffix are removed) has more than 4_000_000 old-by-new block pairs, mark all middle old blocks Removed and all middle new blocks Added. This caps the LCS table memory.
- Trap: `hunk_starts` in [packages/desktop-gpui/crates/explorer/src/diff.rs](packages/desktop-gpui/crates/explorer/src/diff.rs) is private. Do not change `diff.rs`; parse hunk headers in the new module.
- Non-goal: no front matter handling and no inline (word level) diff inside a block.

How:

- Add `is_markdown_path`, `old_text`, and `block_diff` in a new `markdown_diff.rs`; export it as `pub mod markdown_diff` in `lib.rs`.
- `old_text` walks the new text lines and, per hunk, checks that context and added lines match the new text at the hunk new start, then swaps them for the context and removed lines.
- `block_diff` splits both texts into blocks, trims the common prefix and suffix, then runs an LCS over the middle and emits Removed before Added for a changed run.

Files:

- [packages/desktop-gpui/crates/explorer/src/markdown_diff.rs](packages/desktop-gpui/crates/explorer/src/markdown_diff.rs) (new: the model)
- [packages/desktop-gpui/crates/explorer/src/lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs) (add `pub mod markdown_diff;`)
- [packages/desktop-gpui/crates/explorer/tests/models.rs](packages/desktop-gpui/crates/explorer/tests/models.rs) (add the red tests)

Interfaces:

- Produces: `bachuc_explorer::markdown_diff::is_markdown_path(path: &str) -> bool` for TASK-289, TASK-291
- Produces: `bachuc_explorer::markdown_diff::old_text(new_text: &str, diff_text: &str) -> Option<String>` for TASK-291
- Produces: `bachuc_explorer::markdown_diff::block_diff(old_text: &str, new_text: &str) -> Vec<RichBlock>` for TASK-291
- Produces: `pub struct RichBlock { pub kind: BlockKind, pub markdown: String }` and `pub enum BlockKind { Same, Added, Removed }`, both `Clone, Debug, PartialEq, Eq`, for TASK-291

Expected result:

- WHEN a path ends in `.md` or `.markdown` in any case THE `is_markdown_path` SHALL return true, and false for any other path.
- WHEN the diff matches the new text THE `old_text` SHALL return the text before the change, with the trailing newline state of the new text.
- WHEN the diff has no hunk THE `old_text` SHALL return the new text unchanged.
- IF a context or added line does not match the new text at its place THEN THE `old_text` SHALL return None.
- WHEN a hunk only removes lines THE `old_text` SHALL put the removed lines back at the hunk place.
- IF the middle part has more than 4_000_000 block pairs THEN THE `block_diff` SHALL mark all middle old blocks Removed, then all middle new blocks Added.
- WHEN one paragraph changes between two same paragraphs THE `block_diff` SHALL return Same, Removed, Added, Same.
- WHEN a fenced code block holds a blank line THE `block_diff` SHALL keep the fence as one block.
- WHEN the old text is empty THE `block_diff` SHALL mark every new block Added.

Red tests (put the `use` line in the top import block of the file, as its other imports):

File: [packages/desktop-gpui/crates/explorer/tests/models.rs](packages/desktop-gpui/crates/explorer/tests/models.rs)

````rust
// ---- Markdown block diff ----

use bachuc_explorer::markdown_diff::{BlockKind, RichBlock, block_diff, is_markdown_path, old_text};

fn block(kind: BlockKind, markdown: &str) -> RichBlock {
    RichBlock { kind, markdown: markdown.to_string() }
}

#[test]
fn markdown_paths_are_md_and_markdown_in_any_case() {
    assert!(is_markdown_path("docs/README.md"));
    assert!(is_markdown_path("Notes.MARKDOWN"));
    assert!(!is_markdown_path("src/main.rs"));
    assert!(!is_markdown_path("page.mdx"));
    assert!(!is_markdown_path("md"));
}

#[test]
fn old_text_reverses_a_matching_patch() {
    let new = "# Title\n\nNew line\n\nEnd\n";
    let diff = "--- a/x.md\n+++ b/x.md\n@@ -1,5 +1,5 @@\n # Title\n \n-Old line\n+New line\n \n End\n";
    assert_eq!(old_text(new, diff).as_deref(), Some("# Title\n\nOld line\n\nEnd\n"));
}

#[test]
fn old_text_without_hunks_is_the_new_text() {
    assert_eq!(old_text("a\n", "").as_deref(), Some("a\n"));
}

#[test]
fn old_text_of_a_new_file_is_empty() {
    let diff = "--- /dev/null\n+++ b/x.md\n@@ -0,0 +1,2 @@\n+# A\n+b\n";
    assert_eq!(old_text("# A\nb\n", diff).as_deref(), Some(""));
}

#[test]
fn old_text_rejects_a_patch_that_does_not_match() {
    let diff = "--- a/x.md\n+++ b/x.md\n@@ -1,1 +1,1 @@\n-old\n+new\n";
    assert_eq!(old_text("other\n", diff), None);
}

#[test]
fn old_text_puts_back_a_deletion_only_hunk() {
    let diff = "--- a/x.md\n+++ b/x.md\n@@ -1,3 +1,2 @@\n a\n-b\n c\n";
    assert_eq!(old_text("a\nc\n", diff).as_deref(), Some("a\nb\nc\n"));
}

#[test]
fn a_huge_middle_is_all_removed_then_all_added() {
    let old: String = (0..2001).map(|i| format!("old {i}\n\n")).collect();
    let new: String = (0..2001).map(|i| format!("new {i}\n\n")).collect();
    let blocks = block_diff(&old, &new);
    assert_eq!(blocks.len(), 4002);
    assert!(blocks[..2001].iter().all(|b| b.kind == BlockKind::Removed));
    assert!(blocks[2001..].iter().all(|b| b.kind == BlockKind::Added));
}

#[test]
fn a_changed_paragraph_is_removed_then_added() {
    let old = "# Title\n\nOld line\n\nEnd\n";
    let new = "# Title\n\nNew line\n\nEnd\n";
    assert_eq!(
        block_diff(old, new),
        vec![
            block(BlockKind::Same, "# Title"),
            block(BlockKind::Removed, "Old line"),
            block(BlockKind::Added, "New line"),
            block(BlockKind::Same, "End"),
        ]
    );
}

#[test]
fn a_fence_with_a_blank_line_is_one_block() {
    let new = "Intro\n\n```rust\nfn a() {}\n\nfn b() {}\n```\n";
    assert_eq!(
        block_diff("Intro\n", new),
        vec![
            block(BlockKind::Same, "Intro"),
            block(BlockKind::Added, "```rust\nfn a() {}\n\nfn b() {}\n```"),
        ]
    );
}

#[test]
fn every_block_of_a_new_file_is_added() {
    assert_eq!(
        block_diff("", "# A\n\nb\n"),
        vec![block(BlockKind::Added, "# A"), block(BlockKind::Added, "b")]
    );
}
````

Anti-goal: the explorer test suite keeps every existing test passing (0 new failures); tripwire; read from `./cargo.sh test -p bachuc-explorer` before the change, after `old_text`, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer --test models` -> all tests pass; the passing count is the baseline count plus 10.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer` -> all tests pass.
- `git diff --stat -- packages/desktop-gpui/crates/explorer/src/diff.rs packages/desktop-gpui/crates/explorer/Cargo.toml` -> no change (no new crate, diff parser untouched).
