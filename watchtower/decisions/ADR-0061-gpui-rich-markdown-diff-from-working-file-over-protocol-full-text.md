# ADR-0061 GPUI rich markdown diff from the working file over full text in the diff protocol

Status: accepted
Date: 2026-10-22
Supersedes: -
Scope: `packages/desktop-gpui/crates/explorer/src/diff_view.rs`, `packages/desktop-gpui/crates/explorer/src/markdown_diff.rs`

## Context

- A rich markdown diff needs the whole old and new file. The daemon diff has only hunks with a few context lines: `CheckoutDiffCompareSchema` in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) has `mode`, `baseRef`, and `ignoreWhitespace`, and no full-file option.
- The one-file diff (`Source::File` in [packages/desktop-gpui/crates/explorer/src/diff_view.rs](packages/desktop-gpui/crates/explorer/src/diff_view.rs)) is the uncommitted diff, so its new side is the working file. `Client::read_file` already reads it.
- A protocol change needs a daemon restart on release, which only the owner does ([CLAUDE.md](CLAUDE.md)).

## Decision

- The GPUI client reads the working file, rebuilds the old side by reversing the patch, and compares the two texts block by block.
- The rich diff exists only for the one-file uncommitted diff. Commit diffs and the all-files view get no rich diff.
- Hiep chose this on 2026-10-22 in the watchtower interview of plan `20261022-gpui-diff-wrap-markdown-render`.

## Consequences

- No protocol, server, or Electron change.
- A file that changed after the diff loaded gives a patch that does not match; the view must fall back to the line diff.
- A commit diff of a markdown file cannot show a rich diff.

## Options Considered

- Add optional full old and new text to the diff payload: rejected. It changes the protocol and server and needs a release daemon restart, while the one-file case works without it.
- Render each hunk's old and new parts as markdown: rejected. Hunks have about 3 context lines, so a table or a code fence cut by a hunk renders wrong.

## Revisit If

- Hiep asks for a rich diff of commit diffs or of the all-files view.
- The fallback notice shows in more than 1 of 10 opens of a markdown diff in normal use.
