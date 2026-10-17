# TASK-196 Archive confirm on a dirty tree

Group: V (writes `crates/client` and `crates/sidebar`; one group because the sidebar consumes the new accessor)
Class: risky

## Brief

Goal: Ask before archiving a workspace whose checkout has uncommitted or untracked changes, even when its diff stat is 0/0 (Q-095, owner, 2026-10-17).

Change: the sidebar asks only when `workspace_diff_stat` has a changed line ([packages/desktop-gpui/crates/sidebar/src/model.rs](packages/desktop-gpui/crates/sidebar/src/model.rs) `archive_prompt_for_stat`, line 564) -> it also asks when the daemon reports the checkout dirty.

Design: the daemon sends `gitRuntime.isDirty` on each workspace ([packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) line 4055). `diffStat` comes from `git diff --shortstat` ([packages/server/src/server/workspace-git-service.ts](packages/server/src/server/workspace-git-service.ts) line 3358), so it misses untracked files; archiving a Paseo-owned worktree removes them.

Boundaries:

- Add-only in the client: keep every existing public item. Read `isDirty` next to `currentBranch` in the workspace update path (`crates/client/src/core.rs` near line 2205).
- `None` or `false` keeps today's rule. `Some(true)` asks with the same dialog, whose message names uncommitted changes.
- The prompt rule stays a pure function with tests; GPUI view code is checked by reading.
- This differs from Electron on purpose (owner, Q-095).

Files:

- [packages/desktop-gpui/crates/client](packages/desktop-gpui/crates/client) (`src`, `tests`)
- [packages/desktop-gpui/crates/sidebar](packages/desktop-gpui/crates/sidebar) (`src`, `tests`)

Interfaces:

- Produces: `Client::workspace_is_dirty(&self, workspace_id: &str) -> Option<bool>` (the workspace `gitRuntime.isDirty`; `None` when absent, null, or the workspace is unknown) for this TASK's sidebar change

Expected result:

- A workspace with a 0/0 diff stat and `isDirty: true` asks before archiving.
- A clean workspace (`isDirty: false`, no stat) archives at once, as today.

Anti-goal: no archive skips the confirm when either signal shows changes; tripwire: a pure test table over stat (none, 0/0, +1) and dirty (none, false, true) asserts the prompt shows in exactly the 5 changed cases; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client` -> pass, with a fake-daemon test for `isDirty` true, false, null, and absent
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-sidebar` -> pass, including the 9-case prompt table and every existing test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-client -p bachuc-sidebar` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
