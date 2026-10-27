# TASK-313 outcome

Status: DONE (Screen checks DEFERRED (UI checker))

## Changed

- `packages/desktop-gpui/crates/app/src/after_send.rs` (new): `AfterSendStep`, `decide_after_send`, the 4 spec tests.
- `packages/desktop-gpui/crates/app/src/main.rs`: `mod after_send`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `ComposerEvent::Delivered` arm in the composer subscription of `open_agent_pane`; `Shell::after_send`.

## Contract

- `decide_after_send(mode: AfterSend, still_viewed: bool, has_next: bool, board_active: bool) -> AfterSendStep`, `AfterSendStep { Stay, OpenNext, OpenBoard, BoardMissing }`.
- `Shell::after_send` reads `bachuc_settings::after_send::load()` at each `Delivered`; `still_viewed` is `viewed_agent == Some(sent agent)`; `board_active` is `PluginHost::is_active("board")`; the Board opens through `open_surface` as the `board`/`board` Page, empty params; `BoardMissing` shows the error toast "Board is not installed".

## Verified

- Red: before the code, `cargo test -p bachuc-app after_send` failed to compile (missing `decide_after_send`, `AfterSendStep`).
- `./cargo.sh test -p bachuc-app after_send` -> 4 passed (inside the group run below).
- `./cargo.sh test -p bachuc-app -p bachuc-command` -> all pass (106 app tests). Group-level run shared with TASK-312.
- Clippy `-p bachuc-app --no-deps -D warnings`: only the known `plugins.rs:10` error. Group-level run.
- fmt check exit 0.
- Screen checks: DEFERRED (UI checker).

## Anti-goal

0 navigations in Stay mode: PASS by code and test. `stay_never_moves` passes, and the `Delivered` arm calls `decide_after_send`, whose `Stay` branch does nothing. The screen read (one send with no `after-send.json`) is DEFERRED (UI checker).

## Lessons

- `Delivered` fires from the composer task, so `viewed_agent` is current at that time; a page surface sets it to `None`, which counts as "left the thread".

## UI Checks

Run 2026-10-27 by the UI checker (check bundle on live 6768, `cua_repl`, AX actions and background key events only). The mode was set by writing `after-send.json` (the settings pane write is covered by TASK-311); the file is read at each `Delivered`.

- PASS - no `after-send.json`, send a prompt in `uichk-U7`: the thread stayed open (tab `uichk-U7`, reply streaming). This also reads the Stay anti-goal: 0 navigations.
- PASS - mode Board, send: the Board page opened (headings Board, Running, Just finished).
- PASS - mode Next unread with other unread `uichk-U6`, send in `uichk-U5`: `uichk-U6` opened.
- PASS - mode Next unread with no other unread agent, send: the Board page opened. The first Return did not send while the agent was still running; the second Return sent and the Board opened.
- UNVERIFIED - failing send: archiving an agent from the CLI closes its tab, so no composer is left to send from; no other failing send can be made on live without stopping a provider or the daemon.
- UNVERIFIED - Board plugin disabled on live: needs a plugin disable on the live daemon, which counts as a reload; the run rules forbid it.
