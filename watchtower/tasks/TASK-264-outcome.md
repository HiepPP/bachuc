# TASK-264 outcome

Status: DONE

Changed:

- `packages/desktop-gpui/crates/client/src/model.rs`: `BackgroundProcess` and `parse_all`.
- `packages/desktop-gpui/crates/client/src/core.rs`: `AgentExtra.background_processes`, snapshot parse after the older-snapshot check, `background_processes`, `supports_background_stop`, `stop_background_process`.
- `packages/desktop-gpui/crates/client/src/lib.rs`: export and the three `Client` wrappers.
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: five tests.

Contract:

- `bachuc_client::BackgroundProcess` has the exact fields of the spec, `Clone, Debug, PartialEq, Eq`.
- `Client::background_processes(&self, agent_id: &str) -> Vec<BackgroundProcess>`.
- `Client::supports_background_stop(&self) -> bool` reads `features.agentBackgroundProcesses == true`.
- `Client::stop_background_process(&mut self, agent_id, process_id, cx)` sends `agent.background_process.stop.request` through `Pending::Action`, so a refused stop logs a warning.

Verified:

- `./cargo.sh test -p bachuc-client` (group-level, after both TASKs): PASS. 135 passed, 0 failed, 5 ignored.
- Upsert with two processes, `command: null` and `exitCode: 0`: PASS (`an_agent_snapshot_keeps_its_background_processes`).
- Upsert with no `backgroundProcesses` after one with a process: PASS (`an_upsert_without_processes_clears_the_list`).
- Unparsable `startedAt` drops the process; unparsable `endedAt` gives `None`: PASS (`a_process_with_an_unreadable_start_or_end_is_dropped_or_kept_open`).
- `server_info` with and without `agentBackgroundProcesses`: PASS (`background_stop_follows_the_host_feature`).
- Stop request shape and a `"gone"` refusal leaving the core running: PASS (`stop_background_process_sends_the_request_and_survives_a_refusal`).
- `./cargo.sh build`: PASS, exit 0.
- `./cargo.sh fmt --check -p bachuc-client`: PASS, exit 0.
- Anti-goal: before changes `./cargo.sh test -p bachuc-client` had 123 tests: 122 passed and `one_thousand_workspace_updates_stay_cheap` failed at 45.09 ms against a 45 ms bound under builder load; it passed alone, so it was a pre-existing load flake. At completion all 123 old tests pass (135 tests total).

Anti-goal: no existing `bachuc-client` test fails; the workspace builds, so no struct literal broke. `AgentExtra` is private and built only with `or_default()`.

Lessons:

- The background-process `kind` and `status` are plain strings on purpose; do not turn them into enums.
- `BackgroundProcess::parse_all` runs after the `updatedAt` check in `upsert_agent`, so an old snapshot cannot replace newer processes.
