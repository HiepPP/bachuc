# TASK-205 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/client/src/model.rs`: new types `HistoryEntry`, `HistoryPage`, `ForkCursor`, `ForkBoundary`, and `ForkContext`.
- `packages/desktop-gpui/crates/client/src/core.rs`: the `archived` map, `Pending::History` and `Pending::ForkContext`, the history, unarchive, and fork context calls, and the feature checks. The `AgentSummary` build moved to `agent_summary`, so live and archived upserts share it.
- `packages/desktop-gpui/crates/client/src/lib.rs`: the exports and one `Client` wrapper per call.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: 6 new tests.

## Contract

- Every "Produces:" line is implemented with its exact name and signature. `ClientCore` has the same methods without `cx`.
- An `agent_update` upsert with `archivedAt` set removes the agent from `agents()` and its extras, as before. It also stores the `AgentSummary` in the archived map. An upsert with `archivedAt` null removes the map entry, and the agent goes back to `agents()`. `remove_agent` does not touch the map.
- History rows with `archivedAt` set fill the archived map. A row for an agent that is still in `agents()` does not go into the map, so one agent is never in both. Rows with `archivedAt` null do not change the map.
- `fetch_agent_history` sends `sort: [{key: "updated_at", direction: "desc"}]` and `page: {limit: 200, cursor?}`, with no `filter`. It sends `search` only when the search is not empty and `features.agentHistorySearch` is true. The search is not trimmed.
- `HistoryEntry.last_activity_at` is the snapshot `updatedAt`, the value the app's `agent-snapshots.ts:118` uses. `branch` is `project.checkout.currentBranch`. `title` falls back to the first 8 characters of the id, the same rule as `AgentSummary`.
- `fork_context` fails with "Update the host to use this." and sends no request without `features.agentForkContext`. That is the app's `forkUnavailable` text. `boundaryCursor` goes out only with `features.agentForkContextCursor`. A daemon `error` gives `Err(error)`. A null `attachment` gives `Err("Nothing to fork")`. `attachment` is kept as raw JSON. `title` is `attachment.title`.
- `unarchive_agent` calls `refresh_agent`. The map entry is cleared by the `agent_update` upsert that the daemon sends after the unarchive, not by the reply.

## Verified

- Baseline `./cargo.sh test -p bachuc-client`: 88 passed, 5 ignored.
- After the change, `./cargo.sh test -p bachuc-client`: 94 passed, 5 ignored. That is 88 existing plus 6 new tests, and no existing test changed (the `rpcs.rs` diff only adds lines, plus the import list).
- `./cargo.sh build -p bachuc-app` exited 0.
- `./cargo.sh fmt --check -p bachuc-client` exited 0.
- `./cargo.sh metadata --format-version 1` exited 0.
- `clippy -p bachuc-client --tests` gave no client warnings. The 2 warnings come from the existing `bachuc-protocol` build script.
- No UI checks apply to this TASK.

## Lessons

- An archived agent loses its `extras` (modes, queue, labels, updated time). After unarchive, the next upsert fills them again. A view of an archived agent must read `archived_agent`, not the `agent_*` lookups.
- The archived map has no stale-snapshot check. An older archived snapshot that arrives after the unarchive upsert would move the agent out of `agents()` again.
- `ForkCursor.seq` is a `u64` built with `json!`, so a test can compare `boundaryCursor` with `json!({"epoch": .., "seq": 5})`. The f64 rule of TASK-204 applies only to generated protocol types.
