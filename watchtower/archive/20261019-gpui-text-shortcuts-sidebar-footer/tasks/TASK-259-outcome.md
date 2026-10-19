# TASK-259 Outcome

## Status

DONE.

## Changed

- `packages/desktop-gpui/crates/client/src/model.rs`: `WorkspaceNode` gains `pull_request`, `labels`, `status_entered_at` (and `Default`). New `PullRequestState`, `WorkspacePullRequest`, `WorkspaceLabel`, `CheckSummary`, `CheckSummaryState`, `select_check_summary` (port of `selectCheckSummary`), and `parse_iso_ms`.
- `packages/desktop-gpui/crates/client/src/core.rs`: `upsert_workspace` parses the PR, label keys, and `statusEnteredAt` into `WorkspaceMeta`. A `LabelCatalog` (`workspace.label.list.request` on `Ready` when `features.workspaceLabels`; `workspace.label.update` pushes) is resolved in `rebuild_projects`.
- `packages/desktop-gpui/crates/client/src/lib.rs`: exports the new types and `select_check_summary`.
- `packages/desktop-gpui/crates/client/tests/sync.rs`: 8 new tests and a 1,000-update timing test.
- Outside the Files list, one line each (`..Default::default()` in a `WorkspaceNode` literal, so the new fields still compile): `crates/client/tests/rpcs.rs`, `crates/sidebar/tests/model.rs`, `crates/sidebar/tests/plugin.rs`, `crates/app/src/shell.rs`, `crates/app/src/state.rs`, `crates/app/src/new_workspace.rs`, `plugins/watchtower-board/src/model/page.rs`.

## Contract

- `WorkspaceNode.pull_request: Option<WorkspacePullRequest>`: `number: u64`, `title: String`, `url: String`, `state: PullRequestState` (`Open`, `Merged`, `Closed`), `is_merged: bool`, `is_draft: bool`, `forge: String` (workspace `forge`, `github` when absent), `checks: Option<CheckSummary>`. `None` without a URL or a number (the number comes from the URL like `parsePullRequestNumber`, else the wire `number`).
- `WorkspaceNode.labels: Vec<WorkspaceLabel>` (`name`, `color` strings): the catalog definition (its name and color) of each workspace label, matched by the whitespace-collapsed lower-case key. A name missing from the catalog is dropped. A host without `features.workspaceLabels` gives an empty list.
- `WorkspaceNode.status_entered_at: Option<i64>`: ms since the epoch; `None` when absent, null, or unreadable.
- `CheckSummary { state: CheckSummaryState (Passed, Failed, Running), completed: u32, total: u32 }`.
- Everything is also public through `bachuc_client::{...}`.

## Verified

- `./cargo.sh test -p bachuc-client` -> all suites pass (sync 22, rpcs 65, others 13+15+8).
- New tests: full payload fills PR, checks, labels, `status_entered_at`; payload without them gives `None` and `[]`; invalid `statusEnteredAt` (text, month 13, null) gives `None`; checks without `checksStatus` parse; a label missing from the catalog is dropped and shows after a `workspace.label.update`; an out-of-order update asks for a new snapshot; a remove drops a label; `select_check_summary` matches the 11 cases of `check-summary.test.ts`; PR state, number, and forge cases; a host without labels.
- `./cargo.sh clippy -p bachuc-client --all-targets` -> no warning in this crate (two older warnings in `protocol/build.rs`).
- `./cargo.sh fmt --check -p bachuc-client` -> exit 0; `-p bachuc-app -p bachuc-sidebar -p bachuc-plugin-watchtower-board` -> no diff.
- `./cargo.sh check --tests -p bachuc-sidebar -p bachuc-app -p bachuc-plugin-watchtower-board` -> compiles.

## Anti-goal

| Checkpoint                          | Value                   | Source                                                       | Time             |
| ----------------------------------- | ----------------------- | ------------------------------------------------------------ | ---------------- |
| before the change (payload ignored) | 21.9, 21.7, 22.7 ms     | `one_thousand_workspace_updates_stay_cheap`, debug, load 4.3 | 2026-10-19 11:0x |
| after the change, first build       | 41.4 to 41.8 ms (1.9x)  | same test                                                    | 2026-10-19 11:2x |
| after caching label keys at upsert  | 28.8 to 30.3 ms (1.35x) | same test                                                    | 2026-10-19 11:3x |

The test asserts under 45 ms (2x the 22 ms baseline). The loop calls `handle` and `take_events` for each of 1,000 updates of 20 workspaces. Parsing runs in `upsert_workspace` only; `rebuild_projects` clones the parsed values and looks labels up by a stored key.

## Lessons

- Adding a field to `WorkspaceNode` breaks every struct literal in other crates' tests; `Default` plus `..Default::default()` fixes it with one line each.
- Compute the label key once at upsert. Doing it in `rebuild_projects` (per workspace, per update) cost 11 ms of the 41 ms.
- The 45 ms bound is tight in a debug build (30 ms measured); a high load average can flake it.
