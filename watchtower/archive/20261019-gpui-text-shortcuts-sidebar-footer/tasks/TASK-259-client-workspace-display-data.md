# TASK-259 Client parses workspace display data

Group: C (writes [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) and [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs); shared files with TASK-260)
Class: risky

## Brief

Goal: the GPUI client holds the workspace data that Electron's sidebar Show items and Status grouping need.

Change: `upsert_workspace` reads only name, directory, branch, dirty, pinnedAt, diffStat, and scripts -> it also reads the pull request, its checks, workspace labels, and `statusEnteredAt`.

Boundaries:

- Electron sources: `createSidebarWorkspaceEntry` in [packages/app/src/hooks/sidebar-workspaces-view-model.ts](packages/app/src/hooks/sidebar-workspaces-view-model.ts) (lines 146-185). Wire shapes are in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts):
  - `githubRuntime.pullRequest` (line 4072): `number`, `url`, `title`, `state`, `isMerged`, `isDraft`, `checks[]`, `checksStatus`.
  - `forge` is on the workspace, not on the pull request.
  - `labels` is an optional `string[]` on the workspace (line 4146).
  - `statusEnteredAt` is an ISO string or null (line 4153). Convert it to ms. An invalid string gives `None`.
- `CheckSummary` holds what Electron's `selectCheckSummary` returns. Port that function as a pure Rust function with tests.
- Label colors come from the host label catalog, like `useWorkspaceLabelDefinitions` ([packages/app/src/components/workspace-labels/index.ts](packages/app/src/components/workspace-labels/index.ts) line 107). The client has no catalog today. Add `workspace.label.list.request` / `workspace.label.list.response` on connect and apply `workspace.label.update` pushes (messages.ts lines 1040, 4384, 4393). Key the catalog by Electron's `workspaceLabelKey(name)`.
- A label name missing from the catalog is dropped, as in Electron.
- Keep the data on `WorkspaceNode` ([packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) line 81). Parse only in `upsert_workspace` ([packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) lines 3360-3405), never in a render path.
- All new fields are optional. A host that does not send them gives `None` or an empty list.
- Non-goal: no sidebar drawing. TASK-261 and TASK-262 draw the data.
- Class risky: `upsert_workspace` runs on every workspace update.

Files:

- [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs) (new fields and small types)
- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (parse the fields; label catalog if needed)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (public getters if needed)
- [packages/desktop-gpui/crates/client/tests/sync.rs](packages/desktop-gpui/crates/client/tests/sync.rs) (tests)

Interfaces:

- Produces: `WorkspaceNode.pull_request: Option<WorkspacePullRequest>` (number, title, url, state, is_merged, is_draft, forge, `checks: Option<CheckSummary>`) for TASK-261 and TASK-262
- Produces: `WorkspaceNode.labels: Vec<WorkspaceLabel>` (name, color) for TASK-261 and TASK-262
- Produces: `WorkspaceNode.status_entered_at: Option<i64>` (ms since epoch) for TASK-261 and TASK-262

Expected result:

- After a workspace update with a PR, labels, and `statusEnteredAt`, the client snapshot holds all three.

Anti-goal: workspace update cost stays flat; a client test that applies 1,000 workspace updates runs in under 2x its time before this change; read from `cargo test` timing at start and end of the TASK.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-client` -> new tests pass: a full workspace payload fills the PR, checks, labels, and `status_entered_at`; a payload without them gives `None` and an empty list; an invalid `statusEnteredAt` gives `None`; checks present with `checksStatus` absent still parse; a label missing from the catalog is dropped and appears after a `workspace.label.update` adds it; `CheckSummary` matches `selectCheckSummary` for each `checksStatus`.
- The 1,000-update timing test -> under 2x the time recorded before the change.
