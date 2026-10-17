# TASK-189 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/plugins/board` (new crate `bachuc-plugin-board`):
  - `Cargo.toml`: `anyhow`, `bachuc-client`, `bachuc-icons`, `bachuc-plugin-host`, `bachuc-theme`, `chrono` (no default features; `std`, `clock`), `gpui-kit.workspace = true` (no `features` key), `log`, `serde`, `serde_json`. No view crate and no other plugin crate.
  - `src/lib.rs`: `pub fn plugin()`; `contribute` creates one `BoardStore` and adds surface `board`, sidebar item `board` "Board" (`Columns3`), and shortcut `open-board` `Mod+D`.
  - `src/model/run.rs`: `Run`, `RunStatus`, `BoardSnapshot`, `decode_snapshot` (each run on its own; a bad run is skipped and logged), `board_poll_interval`, `starred_first`, `group_runs`.
  - `src/model/tree.rs`: `build_run_trees`, `board_columns`, `column_order`, `cluster_summary`, `cluster_seen`, `two_columns`, `ColumnItem`, `column_items`, `item_key`, `splice_range`.
  - `src/model/text.rs`: the card and rail text (durations, relative times, timing, meta line, model label, card label `title, label[, seen|, new]`, open hint, Remove names, host name, laptop rule, connection labels, snapshot warnings).
  - `src/model/colors.rs`: `is_dark_surface`, `repo_surface`, `repo_mark`, `repo_label`, `repo_divider`, `chip_colors`, `project_initial`, `effort_color`, all as `Hsla`.
  - `src/model/settings.rs`: `OrbSettings`, `OrbState`, `orb_settings`, `orb_preset`, `allocate_colors`, `decode_hues`, `clamp_board_size`, `board_scale`, `decode_size`, `SizeSaver`.
  - `src/model/poll.rs`: `ConnectionState`, `connection_state`, `poll_due`, `Poller` (one request in flight, one per interval, reload after connect, equal JSON is `Unchanged`).
  - `src/model/remove.rs`: `remove_finished_run` (async, generic over the remove and read calls), `close_result`.
  - `src/store.rs`: `BoardStore`, `watchtower_open`.
  - `src/page.rs`: `BoardPage`, `BoardView`, the view switch, and the size control.
  - `src/runs.rs`: the Runs view: lane rail, notices, two columns, each a GPUI `list`.
  - `src/card.rs`: `RunCard` and the subagent row, cluster head, child rows, nested clusters, `running_avatar`, `pulse`, the focus ring.
  - `src/avatar.rs`: `avatar_index`, `agent_avatar`, the 20 PNGs through `include_bytes!` from `hiep-plugins/plugins/board/client/assets/bots`.
  - `tests/plugin.rs`: `plugin().id() == "board"`.
- `packages/desktop-gpui/Cargo.lock` (generated): the `bachuc-plugin-board` entry only.
- No file in `hiep-plugins/plugins/board` changed.

## Contract

- Produces `bachuc_plugin_board::plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>`, id `"board"`, for TASK-191. It is the crate's only public item.
- Produces for TASK-193, crate-private, with the exact names of the spec: `crate::store::BoardStore` (`snapshot`, `connection(now_ms)`, `hues`, `size`, `orb`, `orb_settings`, `remove`, `close_runtime`, `open_agent`, `open_task`), created once in `contribute`; `crate::model::{Run, RunStatus, BoardSnapshot, ConnectionState, OrbSettings, OrbState}`; `crate::model::colors::{repo_surface, repo_mark, repo_label, repo_divider, chip_colors, is_dark_surface}`; `crate::avatar::agent_avatar(agent_id, size)`; `crate::card::running_avatar(agent_id, size, orb, window, cx)`; `crate::page::BoardView` with the `Timeline` and `Recaps` arms in `BoardPage::render` drawing "Loading…".
- Extra crate-private store methods: `revision`, `is_connected`, `fetching`, `host_label`, `refresh`, `set_starred`, `set_diffs_off`, `open_project`, `pick_size`, `palette_error`, `retry_palette`.
- `repo_surface` returns `(background, border)`. `is_dark_surface` takes an `Hsla`, so Electron's "unparseable color is dark" case has no Rust form.
- Ownership: only the contribution closures hold the store strongly. `BoardPage` holds a `WeakEntity`, so after deactivation an open page draws "Board is off on this host.". The poll timer, the request, the stale timer, and the save tasks are store fields; each spawned task reaches the store through a `WeakEntity`.
- Safe defaults taken (reported as questions):
  - `running_avatar` draws the agent avatar with the running spinner centered over it, and a running top-level card has no spinner in its meta row. This matches "shows the running spinner in this part" and TASK-193's "otherwise keep the spinner". Electron desktop shows the orb there until TASK-193 lands.
  - Star, diffs, and Remove ask the store for one load at once after success, as Electron refetches. Remove's re-read in `remove_finished_run` is one more `board.snapshot`. These are the only requests outside the poll cadence; each waits for the request in flight.
  - The Running and Just finished columns always sit side by side. The compact one-column layout of Electron is not ported.
  - The pulse fades opacity only; GPUI divs have no scale transform, so the 1.08 to 1.5 grow is not drawn.
  - The rail does not stick: the page toolbar, the rail, and the column headers stay fixed, and each column list scrolls. The 50-conversation footer stays under the columns.
  - The Board text sizes come from the Board scale only, as in Electron, not from the app font scale.
  - The app does not filter key repeat for plugin shortcuts, so a held `Cmd+D` toggles the page on each repeat. Electron's `!event.repeat` rule has no plugin-side hook.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-board` -> exit 0: 50 unit tests and 1 integration test pass. They cover every expected case: poll interval, `group_runs`, missing parent, parent cycle, `board_columns`, the 4 `remove_finished_run` cases, `allocate_colors`, `clamp_board_size` and `board_scale`, the 5 connection states, `avatar_index`, `chip_colors`, `is_dark_surface`, `repo_label` light and dark lightness, and an equal snapshot giving `Unchanged`. They also cover the ported Electron tests (groups, tree, cluster-ui rules, project-color, effort, orb `isDarkSurface` and `orbPreset`, board-size, client-state, remove, repo-color, project-header, shortcut key rule), `plugin().id() == "board"`, the decode of a `board.snapshot` payload with one bad run, `poll_due` (2,000 ms, never while in flight), the 15,000 ms idle cadence, and a reload after `Connected` that never puts two requests in flight.
- UNSATISFIABLE part of the same line: "no poll or timer after the contributions drop" needs a GPUI test context. The gpui-kit `test-support` feature breaks workspace resolution (Q-063, `MEMORY.md` 2026-10-15). Replacement, by reading `src/lib.rs` and `src/store.rs`: the store lives only in the contribution closures, its timers and request are its own `Task` fields, which are cancelled when it drops, and every spawned task reaches it through a `WeakEntity`.
- `rg -n "bachuc-(composer|...|plugin-[a-z-]+)\b" packages/desktop-gpui/plugins/board/Cargo.toml | rg -v plugin-host` -> one match, line 2 `name = "bachuc-plugin-board"`. UNSATISFIABLE: the pattern matches the crate's own required name. Replacement: `sed -n '/^\[dependencies\]/,$p' packages/desktop-gpui/plugins/board/Cargo.toml | rg -n "<same pattern>" | rg -v plugin-host` -> no match (exit 1).
- `rg -n "gpui-kit" packages/desktop-gpui/plugins/board/Cargo.toml` -> one line, `16:gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-board` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0. `bachuc-app` does not link the crate until TASK-191.
- `git diff --stat -- hiep-plugins/plugins/board` -> empty.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh clippy -p bachuc-plugin-board --tests` -> no warning in this crate after one fix.
- UI check (GPUI Board Runs view against Electron 6770) -> DEFERRED (UI checker). It also needs TASK-191 to link the crate.
- Live daemon checks -> UNVERIFIED (unattended): live 6768 is down.

## Anti-goal

- Checkpoint 1, when the store landed: name `poll_due` and in-flight guard; value `poll_due_waits_2000_ms_and_never_runs_while_in_flight`, `the_idle_cadence_is_15000_ms_with_no_running_run`, and `connected_reloads_at_once_and_after_a_reconnect` pass; source `src/model/poll.rs`; time 2026-10-17 00:20.
- Checkpoint 2, at completion: name views hold no poll; value `rg -n '"board\.snapshot"|\.timer\(' src` finds `board.snapshot` only in `src/store.rs` (the poll at line 215 and Remove's re-read at line 434). The only view timer is the 1 s clock redraw in `src/runs.rs:156`, which sends no request; source `packages/desktop-gpui/plugins/board/src`; time 2026-10-17 00:29.
- Checkpoint 3, at completion: name poll tests; value 51 of 51 pass; source `cargo.sh test -p bachuc-plugin-board`; time 2026-10-17 00:29.
- Tripwire not hit: one store sends at most one `board.snapshot` per `board_poll_interval` from its poll, never two at once. Card actions add one load each, as noted in Contract.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground).

- UI check (GPUI Board Runs view against Electron 6770: lane rail, columns, cards, colors) -> UNVERIFIED (unattended): needs pointer. AX press on the sidebar `Board` button changed nothing: the AX tree and the frame stayed on the New workspace screen (`/tmp/wt-ui/gpui-board.jpg`). The reference also lacks the plugin (`board` is `failed` on 6770).

## Lessons

- GPUI's `focus_visible` keeps only the last style. A second call for opacity drops the ring, so put both in one call.
- GPUI `list` re-renders and re-measures its visible items every frame. Call `splice` only for changed item keys and `remeasure` only for a scale or width change, so a poll keeps the scroll position.
- A Verify grep for `plugin-[a-z-]+` over a plugin's own `Cargo.toml` always matches its `name =` line. Scope such a grep to `[dependencies]`.
