# TASK-190 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/plugins/watchtower-board/Cargo.toml` (new): crate `bachuc-plugin-watchtower-board`; depends on `bachuc-plugin-host`, `bachuc-client`, `bachuc-theme`, `bachuc-icons`, `gpui-kit.workspace = true` (no `features` key), `serde`, `serde_json`, `anyhow`, `log`, and `regex`. No view crate and no other plugin crate.
- `src/lib.rs` (new): `pub fn plugin()` (id `watchtower-board`) and the six contributions: surface `watchtower`, shortcut `toggle-board` `Mod+G` with `surface: Some("watchtower")`, New workspace panel `board`, Command Center item `open-board` (context `Workspace`), attachment source `tasks`, and composer pill `watchtower` (`Track`, `ShowOnDraft::Yes`, `PillBehavior::Action` with `new_workspace_panel: Some("board")`). 2 tests.
- `src/model/` (new): `dto.rs` (the RPC DTOs, inputs, `attachment_key`, `is_decision_id`), `dashboard.rs` (`task_group`, `with_live_status`, `dashboard_summary`, `group_tasks`, `questions_for`, `plural`, `task_title`, `pill_label`), `markdown.rs` (`parse_markdown`, `Block`), `actions.rs` (the five prompt builders, `AgentCandidate`, `AgentTarget`, `choose_agent`), `builders.rs` (`match_builders`, `live_task_ids`, `BuilderMatch`), `lifecycle.rs` (`lifecycle_phases`, `needs_you`), `timeline.rs` (`build_timeline`, `started_at`, `clock_at`), `plan_tree.rs` (`build_plan_tree`, `commit_shas`, `ago`), `time.rs` (`round_half_up`, regex `parse_iso_ms`), `cache.rs` (`cache_fresh`, `Cache`, `Want`), `page.rs` (`choose_workspace`, `project_name`, `initial_task`, `initial_selection`, `folder_name`, `changed_range`), `mod.rs` (`Loaded<T>`), and `fixtures/*.json` (real server payloads). 104 tests.
- `src/store.rs` (new): `WatchtowerStore`.
- `src/page.rs`, `src/panel.rs`, `src/rail.rs`, `src/task_list.rs`, `src/cell.rs` (new): the overlay page, the New workspace panel, the rail cards, the task list, and the cell parts. 11 tests.
- `tests/plugin.rs` (new): `plugin().id() == "watchtower-board"`.
- `packages/desktop-gpui/Cargo.lock` (generated): the new crate entry.

## Contract

- Produces as specified: `bachuc_plugin_watchtower_board::plugin() -> Box<dyn NativePlugin>`, the only public item.
- Produces `crate::store::WatchtowerStore` with `board`, `project_board`, `overview` (each `-> Loaded<_>`), `refresh`, `decision`, `builders`, `agent_target`, and `send` at the specified signatures. Extra crate-private items: `ensure`, `ensure_project`, `refresh_project`, `is_fetching`, `is_fetching_project`, `revision`, `live`, `preselect`, `preselected`, `set_initial_task`, `initial_task`, and `load_active`.
- Produces `crate::model::Loaded<T>` (`Loading`, `Ready(T)`, `Error(String)`) and every named model type and pure function. `Block` variants: `Heading { level, text }`, `Bullet { depth, text }`, `Paragraph { text }`, `Code { text }`. `BuilderMatch` is `{ by_task: BTreeMap<String, TaskBuilder>, lead: Option<RunAgent> }`.
- Produces the views `crate::cell::{overview_cell, show_more}` (plus `cell_slice`, `cell_title`, `empty`, `limit`), `crate::rail::{ProgressCard, WaitingCard, TasksCard, AutorunLogCard}` (unit structs with render functions), the `Rail` state, the `RailHost` trait, `RailItem`, and `crate::task_list::TaskList` (`new`, `select`, `select_from_outside`, `toggle_group`, `set_notice`, `rows`, `render_group`, `render_task`).
- Consumes the TASK-168, TASK-169, and TASK-170 names as specified: `PluginContext::{client, rpc_as, active_workspace_id, open_surface, notify}`, `OpenSurfaceOptions`, `SurfacePresentation::Overlay`, the contribution structs, `Handler`, `host`, `Client::{agents, workspaces, projects, is_connected, agent_labels, agent_attention, agent_updated_at, send_message}`, `OutgoingMessage`, and `TurnBehavior::Interrupt`.
- Load rules: the active workspace's board and overview load when missing or 30,000 ms old (`Want::Fresh`), from a host observer; the page and the panel load once (`Want::Missing`); Refresh forces (`Want::Force`); `ClientEvent::Connected` reloads every cached key. No request is sent before the client is connected. The pill `state` closure only reads.
- Builder matching reads `Client` state and recomputes on `ClientEvent::AgentsChanged`; the store caches the live TASK set per workspace and redraws the pill only when a set changes. The two Electron 15 s polls are dropped.
- Workspace choice: `params.workspaceId`, else the preselected workspace, else the active one, else the first; a named workspace missing from a connected client shows "Workspace not found.". `params.task` is kept in the store and selects that task once the board loads.
- Lists are lazy: the panel and the page are each one GPUI `list`; every card start, group header, task row, log row, and card end is its own item, and a card draws as joined slices.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-watchtower-board` -> exit 0; 117 lib tests and 1 integration test pass (`/tmp/wt190/test.txt`). They cover every expected case: `pill_label` (no plan, 3 of 5 with 1 live and 1 question, a live DONE task not counted, plus the ported `pill` tests), `parse_markdown` (ported tests plus a `~~~` fence and CRLF), `task_group` per status and `task_title` (strips "TASK-12: ", falls back to the full title), `choose_agent` (none, busy, subagent and `mock` skipped, newest `agent_updated_at`, name falls back to the id, unreadable time last), the prompt builders (exact Electron strings, trailing periods stripped), `lifecycle_phases` and `needs_you` (ported), `match_builders` (ported, with `active_ago` from `agent_updated_at` and `None` when unreadable), `build_plan_tree` and `build_timeline` (ported, including the dependency cycle in the last wave), `clock_at` (23:50 plus 20 -> "00:10", unreadable -> `None`), `commit_shas`, `attachment_key("w1", "TASK-3")`, the workspace choice, decode tests of real `watchtower.read`, `watchtower.overview.read`, and `watchtower.decision.read` payloads, and `cache_fresh` (fresh under 30,000 ms, stale at 30,000 ms, no second request while one is in flight).
- Lead test (`params.task`): `a_surface_opened_with_params_task_selects_that_task_and_an_unknown_id_selects_none` and `the_openers_task_starts_selected_with_its_group_open` -> pass.
- `rg -n "bachuc-(composer|...|plugin-[a-z-]+)\b" packages/desktop-gpui/plugins/watchtower-board/Cargo.toml | rg -v plugin-host` -> UNSATISFIABLE: it matches line 2, the crate's own `name = "bachuc-plugin-watchtower-board"`. Replacement: the same command with `rg -v 'plugin-host|^2:name = "bachuc-plugin-watchtower-board"$'` -> no match (exit 1).
- `rg -n "gpui-kit" packages/desktop-gpui/plugins/watchtower-board/Cargo.toml` -> one line, `15:gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-watchtower-board` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (`/tmp/wt190/build-app.txt`). `bachuc-app` does not link this crate until TASK-191, so this build does not compile it; the crate builds on its own in the test run with no warning.
- `git diff --stat -- hiep-plugins/plugins/watchtower-board` -> empty.
- `npm run format:files` on the 5 fixture JSON files -> formatted.
- UI check (composer pill and `board` panel against Electron 6770) -> DEFERRED (UI checker).
- The view wiring (pill redraw after a load, overlay open and preselection, list slices, Refresh, copy notice) needs a window: UNVERIFIED (unattended): code tests only. Live daemon down: no live check.
- GitNexus impact: no existing symbol was edited; every change is a new file in the new crate.

## Anti-goal

- Checkpoint 1, when the store landed: `one_hundred_redraws_send_at_most_one_request_per_30_000_ms` (100 redraws over 65 s send board and overview requests at 0, 30,550, and 61,100 ms only), `no_second_request_while_one_is_in_flight`, and `cache_fresh_is_fresh_under_30_000_ms_and_stale_at_30_000_ms` passed; source: the first crate test run with `src/store.rs`; 2026-10-17 about 00:05.
- Checkpoint 2, at completion: the same 3 tests passed; source `/tmp/wt190/test.txt`; 2026-10-17 00:10.
- Tripwire: not tripped. The pill `state` closure calls only `store.read` and `pill_label`; loads run from the host observer, the page and panel constructors, Refresh, and `Connected`.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Electron 6770 shows `watchtower-board` as `failed`, so it draws no pill.

- UI check (composer pill and New workspace `board` panel against Electron 6770: pill label, progress strip, waiting card, task list) -> UNVERIFIED (unattended): needs pointer. Seen: the New workspace composer shows a `Watchtower` pill with an icon (AX `Open Watchtower`) next to the branch pill (`/tmp/wt-ui/gpui-sidepanel2.jpg`). AX press on the pill and on `Show side panel` changed nothing (`/tmp/wt-ui/gpui-watchtower-overlay.jpg`, `/tmp/wt-ui/gpui-sidepanel.jpg`), so the progress strip, waiting card, and task list never showed. The project `.claude` may also have no active plan.

## Lessons

- Safe defaults taken (see questions): `params.task` selection and the opener's group follow Electron; `active_workspace_id` counts as a named workspace, so a stale active id shows "Workspace not found." instead of another workspace; a time without a zone parses as UTC; `copy` cannot fail in GPUI, so the Electron failure notice never shows; `active_ago` no longer reads `lastActivityAt`, which the client does not expose.
- A regex compile per task row made `build_plan_tree` take 47 ms on 200 tasks in a debug build; `task_title` now strips the ID by hand.
- `commit_shas` needs JS lookahead, which the `regex` crate lacks: it matches whole hex words of 7 to 40 characters and keeps the first with a digit, which is the same set.
- Real payloads for decode tests come from running the server's `readBoard`, `readOverview`, and `readDecision` with `tsx` on this repo and on a `/tmp` copy of an archived plan that has a RUN.md.

## UI Checks, round 2

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Electron 6770 shows `watchtower-board` as `failed`, so it draws no pill.

- UI check (composer pill and New workspace `board` panel against Electron 6770: pill label, progress strip, waiting card, task list) -> UNVERIFIED (unattended): the reference has no plugin, so nothing is compared. GPUI shows the pill `Watchtower` with an icon in the track row of the New workspace draft of project `bachuc`. AX press on it opens the side panel with the plan title `GPUI parity follow-ups: ...`, `95%`, `39 of 41 done`, a dot strip, `39 done · 2 waiting · 0 not started`, and a `Tasks 41` list with `Active 0`, `Blocked 2` (`TASK-175`, `TASK-179`, with `BLOCKED` badges), `Todo 0`, and `Done 39` (`/tmp/wt-ui2/watchtower-bachuc.jpg`). Round 1 saw nothing on press. No separate waiting card shows in this panel.
