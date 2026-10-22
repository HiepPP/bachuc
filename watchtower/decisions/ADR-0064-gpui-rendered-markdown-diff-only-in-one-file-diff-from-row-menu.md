# ADR-0064 GPUI rendered markdown diff only in the one-file diff, opened from the Changes row menu

Status: accepted
Date: 2026-10-22
Supersedes: -
Scope: `packages/desktop-gpui/crates/explorer/src/diff_view.rs`, `packages/desktop-gpui/crates/explorer/src/explorer.rs`, `packages/desktop-gpui/crates/explorer/src/file_menu.rs`

## Context

- The rich diff reads the working file and reverses the patch ([ADR-0061](watchtower/decisions/ADR-0061-gpui-rich-markdown-diff-from-working-file-over-protocol-full-text.md)), so it only fits the uncommitted compare of one file.
- A click on a Changes row has opened the all-files diff since `57d2d6b47` (2026-10-16). Nothing emitted `ExplorerEvent::OpenDiff`, so the one-file diff had no entry (Q-186).

## Decision

- The Rendered/Lines toggle shows only in the one-file uncommitted diff (`Source::File`) of a markdown path.
- The one-file diff opens from the "Open file diff" item of the right-click menu of an uncommitted changed file. A row click keeps opening the all-files diff.
- Hiep chose the one-file scope and the menu entry on 2026-10-22 (watchtower interviews for TASK-291 and TASK-297).

## Consequences

- The all-files view and commit diffs stay line-only.
- The entry is a menu item, which needs the pointer: a UI checker cannot open it, so its screen checks are owner checks until the row menu gets a key or AX path.

## Options Considered

- Rendered toggle in the all-files view or in commit diffs: rejected. A commit's new side is not the working file, and the all-files list is one shared row model.
- A row click opens the one-file diff: rejected. It changes how Hiep uses the click now.
- An "Open file diff" button in each all-files file header: rejected. More wiring in `diff_view.rs` and `shell.rs`.

## Revisit If

- The diff protocol starts sending full old and new text, so commit diffs can render too.
- The row menu gets a keyboard path, or Hiep wants a visible button.
