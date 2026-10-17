# TASK-187 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/plugins/jev-orchestrator/Cargo.toml` (new): crate `bachuc-plugin-jev-orchestrator`; dependencies `anyhow`, `bachuc-client`, `bachuc-icons`, `bachuc-plugin-host`, `bachuc-theme`, `gpui-kit.workspace = true`, `log`, `serde`, `serde_json`, `uuid` (v4). No view crate.
- `packages/desktop-gpui/plugins/jev-orchestrator/src/lib.rs` (new): `plugin()`, id `"jev-orchestrator"`, and the six contributions: panel `jobs`, slash `/delegations`, panel `direct-route`, Command Center item `open-direct-route`, slash `/route`, renderer `job` v1.
- `packages/desktop-gpui/plugins/jev-orchestrator/src/model.rs` (new): RPC types, the pure rules, `JobsState`, `Poller`, `DirectState`, and 36 unit tests.
- `packages/desktop-gpui/plugins/jev-orchestrator/src/jobs_panel.rs` (new): Delegated work view.
- `packages/desktop-gpui/plugins/jev-orchestrator/src/direct_panel.rs` (new): Direct route view.
- `packages/desktop-gpui/plugins/jev-orchestrator/src/job_item.rs` (new): `job` timeline renderer and its decode tests.
- `packages/desktop-gpui/Cargo.lock` (generated): the new package entry.

## Contract

- Produces `bachuc_plugin_jev_orchestrator::plugin() -> Box<dyn NativePlugin>` with id `"jev-orchestrator"`. The crate is not wired into `bachuc-app` (TASK-191).
- Contributions copy `index.client.tsx`: `jobs` is `PanelContext::Agent` with the `GitBranch` icon and no locations (the host lists `Workspace`); `direct-route` is `PanelContext::Workspace`, locations `Workspace` and `Explorer`, `Route` icon; the Command Center item is `Workspace` context with keywords `model`, `effort`, `agent`.
- RPC calls go only through `PluginContext::rpc_as`: `jobs.list`, `jobs.cancel`, `direct.profiles`, `direct.run`, `direct.status`. All input keys are camelCase; `shareWithJev` is `true`; `requestId` is a v4 UUID.
- `jobs.list` decodes each job alone; a job that does not fit `JobView` is skipped and `history` is ignored.
- Planner call: `/delegations` needs an agent and a workspace (from `SlashInput`, else `active_agent_id` and `active_workspace_id`); with either missing it does nothing. `/route` and the Command Center item open `direct-route` with `agent_id: None`. `/route` with arguments calls `show_message("Use /route without arguments.", Error)`.
- Direct route state is per panel view. A host change deactivates the plugin and a workspace change gives a new `PanelInput`, so the state starts fresh without a key of its own.
- Jobs panel polling: the view owns the timer task, the request task, and the cancel task; dropping the view cancels all three.
- Load after `Connected`: both panels read the client state at open. Opened while disconnected, each shows its error text and loads on the next `Connected`.
- Decisions on points the spec left open:
  - Refresh, and the refetch after a successful cancel, skip the 5,000 ms interval but never the in-flight guard. If a request is in flight, one refetch is queued for when it returns. The periodic poll keeps the interval. The Anti-goal counts the periodic poll.
  - GPUI divs have no disabled state in the accessibility tree. A disabled button, profile row, or chip loses its click handler and gets `aria_description("Disabled")`. Checked state uses `aria_toggled`.
  - gpui-kit has no max length on `TextareaState`. The prompt is cut to 12,000 chars in the change handler (`clamp_prompt`). The input is read-only through `Textarea::readonly(locked)`.
  - Texts in the Delegated work panel are not selectable; GPUI plain text has no selection.
  - `/route` arguments that are only whitespace count as no arguments (`slash_route_allowed` trims).
  - The prompt minimum counts chars of the trimmed text, not UTF-16 units.
  - A `job` item that does not fit the schema shows "Plugin timeline item unavailable.", as the edit-diffs row does.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-jev-orchestrator` -> 41 passed, 0 failed. Covered: every Cases line (effort offer and order, `is_luna_model`, one group with a union and `profile_count` 2, last-profile deselect, `toggled_efforts`, `can_route`, `derive_outcome`, ignored records, `route_label`, `keyed_checks`, a malformed job skipped, `cancellable`, `slash_route_allowed`, the camelCase run input with `shareWithJev` and a v4 id), `plugin().id() == "jev-orchestrator"`, `poll_due` (not due at 4,999 ms, due at 5,000 ms, never in flight), the poller and its queued user request, and decode tests of real `jobs.list`, `direct.profiles`, `direct.run`, and `direct.status` payloads.
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)" packages/desktop-gpui/plugins/jev-orchestrator/Cargo.toml` -> no match (exit 1). The same pattern on the `[dependencies]` section alone -> no match.
- `rg -n "gpui-kit" packages/desktop-gpui/plugins/jev-orchestrator/Cargo.toml` -> one line, `15:gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-jev-orchestrator` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0.
- `git diff --stat -- hiep-plugins/plugins/jev-orchestrator` -> empty.
- UI check (Direct route panel and Delegated work panel against Electron 6770) -> DEFERRED (UI checker).
- Code that needs `App` or `Context` (the two views, the contribution closures, the renderer view) was checked by reading and by compiling only. Their look and the live RPC round trip are UNVERIFIED (unattended): code tests only. Live 6768 was down.

## Anti-goal

Tripwire: the jobs panel sends at most one `jobs.list` per 5,000 ms per open panel, never two at once, and none after the panel closes.

- Checkpoint `poll_due_interval`: not due at 4,999 ms, due at 5,000 ms; source test `poll_due_waits_5000_ms_and_never_runs_twice_at_once`; 2026-10-17 01:26.
- Checkpoint `poll_due_in_flight`: `poll_due(60_000, Some(0), true)` and `poll_due(0, None, true)` are false; same test; 2026-10-17 01:26.
- Checkpoint `poller_interval_and_connection`: one send per interval, `wait_ms` is `None` while in flight and while disconnected; source test `the_poller_sends_once_per_interval_and_only_while_connected`; 2026-10-17 01:26.
- Checkpoint `user_request_guard`: Refresh skips the interval but queues behind a request in flight, and a disconnect drops the queued one; source test `a_user_request_skips_the_interval_but_not_the_in_flight_guard`; 2026-10-17 01:26.
- Checkpoint `poll_task_ownership`: `JobsPanel` holds the timer in `_poll`, the request in `_request`, and the cancel in `_cancel` (`jobs_panel.rs` lines 50-52); every send goes through `Poller::begin`, and `send` runs only from `schedule` (after `due`), `received` (a queued request), and `refetch` (after `request_now`). GPUI cancels a dropped `Task`, so dropping the view stops the timer and the request. Source: reading `jobs_panel.rs`; 2026-10-17 01:27.
- Result: no tripwire hit. The user-initiated Refresh, which the spec did not list, can send inside 5,000 ms of the last poll (see Contract).

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground).

- UI check (Direct route panel and Delegated work panel against Electron 6770) -> UNVERIFIED (unattended): needs pointer. No panel can open (AX press on sidebar rows does nothing: `/tmp/wt-ui/gpui-t144.jpg`). The line also assumes jev-orchestrator runs on live, but `plugin ls` on live shows it `disabled` with the source `~/Projects/paseo/hiep-plugins/plugins/jev-orchestrator`, and it shows `failed` on 6770.

## Lessons

- A gpui `div` has no disabled state in the accessibility tree (`aria_disabled` does not exist). Use `aria_description("Disabled")` and drop the click handler.
- gpui-kit `TextareaState` has no max length and `min_h` goes through `Styled` on `Textarea`; a length cap needs `InputEvent::Change` with `set_value`, which emits no event, so it cannot loop.
- A pure `Poller` with a queued user request keeps the in-flight guard testable without a GPUI context.

## UI Checks, round 2

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground).

- UI check (Direct route panel and Delegated work panel against Electron 6770) -> UNVERIFIED (unattended): `jev-orchestrator` is not running on live (round 1: `disabled`), so the Settings plugin screens list `Thinking orb`, `Edit diffs`, `Next prompt actions`, `Prompt translate`, and `Skill pins` only (`/tmp/wt-ui2/plugins.jpg`). The reference lists it as `failed`. Enabling it changes live.
