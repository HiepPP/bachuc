# TASK-311 outcome

Status: DONE (Screen check DEFERRED (UI checker))

## Changed

- `packages/desktop-gpui/crates/settings/src/after_send.rs` (new): `AfterSend`, `load_from`, `save_to`, `load`, `save`, 3 tests.
- `packages/desktop-gpui/crates/settings/src/lib.rs`: `pub mod after_send`.
- `packages/desktop-gpui/crates/settings/src/pane_agent_defaults.rs`: "After send" row (Stay, Back to Board, Next unread; no Default row), saves at once.

## Contract

- `bachuc_settings::after_send::AfterSend { Stay, Board, NextUnread }`, `Default` = `Stay`.
- `bachuc_settings::after_send::load() -> AfterSend`; `save(mode: AfterSend) -> Result<()>`.
- File `after-send.json`: `{"mode": "stay" | "board" | "next-unread"}`.

## Verified

- Red: before the implementation the 3 tests failed to compile (`AfterSend`, `load_from`, `save_to` missing).
- `./cargo.sh test -p bachuc-settings after_send` -> 3 passed.
- `./cargo.sh test -p bachuc-settings` -> 62 lib + 4 integration passed, incl. `defaults_round_trip_and_unset_fields_stay_out`.
- `./cargo.sh fmt --check -p bachuc-settings` -> exit 0.
- Clippy `--no-deps -D warnings`: one error, `too_many_arguments` at `pane_appearance.rs:423`; the file is untouched by this TASK (not in my diff). Files of this TASK lint clean.
- Screen check (UI checker): DEFERRED (UI checker).

## Anti-goal

`agent-defaults.json` untouched: `after_send` writes only `after-send.json`; `agent_defaults.rs` has 0 diff; `defaults_round_trip_and_unset_fields_stay_out` passes. Manual file diff of a live save: DEFERRED (UI checker).

## Lessons

- TASK-313 reads the mode with `bachuc_settings::after_send::load()` at send time (no watch; the file is read fresh).
- `select_trigger` takes the shown label as `String`.
