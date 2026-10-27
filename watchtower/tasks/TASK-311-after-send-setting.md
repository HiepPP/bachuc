# TASK-311 After send setting

Group: C (packages/desktop-gpui/crates/settings/src/after_send.rs, packages/desktop-gpui/crates/settings/src/lib.rs, packages/desktop-gpui/crates/settings/src/pane_agent_defaults.rs)
Class: code

## Brief

Goal: Hiep picks what happens after a prompt is sent: Stay, Back to Board, or Next unread. Stay is the default.

Change: no setting -> `after-send.json` in the app data folder, edited in the New agent defaults settings pane.

Boundaries:

- Follows [ADR-0071](watchtower/decisions/ADR-0071-after-send-setting-over-fixed-jump.md): three modes, default Stay.
- Copy the file pattern of `agent_defaults.rs`: `load_from`, `save_to` through a temp file and a rename, and `load` and `save` on `bachuc_command::data_dir()`.
- Stored JSON: `{"mode": "stay" | "board" | "next-unread"}`. A missing, corrupt, or unknown value gives `Stay`.
- Planner call: the row sits at the end of `AgentDefaultsPane`, labeled "After send", because that pane holds the other composer defaults. `choice_dropdown` in that pane adds a forced "Default" row, so add a small three-option select with no Default row, in the same style.
- `AfterSend` derives `Clone`, `Copy`, `Debug`, `Default`, `PartialEq`, and `Eq`; `save_to(dir: &Path, mode: AfterSend)` takes it by value.

How:

- Add `after_send.rs` with `AfterSend`, `load_from`, `save_to`, `load`, and `save`; declare `pub mod after_send` in `lib.rs`.
- Add the "After send" row to `AgentDefaultsPane` in `pane_agent_defaults.rs`; a change saves at once.

Files:

- [packages/desktop-gpui/crates/settings/src/after_send.rs](packages/desktop-gpui/crates/settings/src/after_send.rs) (new module with tests)
- [packages/desktop-gpui/crates/settings/src/lib.rs](packages/desktop-gpui/crates/settings/src/lib.rs) (module declaration)
- [packages/desktop-gpui/crates/settings/src/pane_agent_defaults.rs](packages/desktop-gpui/crates/settings/src/pane_agent_defaults.rs) (settings row)

Interfaces:

- Produces: `bachuc_settings::after_send::AfterSend` enum `{ Stay, Board, NextUnread }` with `Default` = `Stay`, for TASK-313
- Produces: `bachuc_settings::after_send::load() -> AfterSend` for TASK-313

Expected result:

- WHEN no file exists THE setting SHALL read `Stay`.
- WHEN Hiep saves a mode THE setting SHALL read the same mode back.
- IF the file is corrupt or holds an unknown mode THEN THE setting SHALL read `Stay`.
- WHEN Hiep picks a mode in the settings pane THE pane SHALL save it at once.

Red tests:

File: [packages/desktop-gpui/crates/settings/src/after_send.rs](packages/desktop-gpui/crates/settings/src/after_send.rs)

```rust
#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir() -> std::path::PathBuf {
        let dir = std::env::temp_dir().join(format!("after-send-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn a_missing_file_reads_stay() {
        assert_eq!(load_from(&temp_dir()), AfterSend::Stay);
    }

    #[test]
    fn every_mode_round_trips() {
        let dir = temp_dir();
        for mode in [AfterSend::Stay, AfterSend::Board, AfterSend::NextUnread] {
            save_to(&dir, mode).unwrap();
            assert_eq!(load_from(&dir), mode);
        }
        assert!(!dir.join("after-send.json.tmp").exists());
    }

    #[test]
    fn a_corrupt_file_or_unknown_mode_reads_stay() {
        let dir = temp_dir();
        std::fs::write(dir.join("after-send.json"), "[1]").unwrap();
        assert_eq!(load_from(&dir), AfterSend::Stay);
        std::fs::write(dir.join("after-send.json"), r#"{"mode":"jump"}"#).unwrap();
        assert_eq!(load_from(&dir), AfterSend::Stay);
        std::fs::write(dir.join("after-send.json"), r#"{"mode":"next-unread"}"#).unwrap();
        assert_eq!(load_from(&dir), AfterSend::NextUnread);
    }
}
```

Anti-goal: `agent-defaults.json` keeps its exact keys and values after a save of the new setting, 0 changed bytes; tripwire; read from `defaults_round_trip_and_unset_fields_stay_out` in `agent_defaults.rs` and a manual file diff at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-settings after_send` -> 3 tests pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-settings` -> all tests pass, including `defaults_round_trip_and_unset_fields_stay_out`.
- Screen check (UI checker, live GPUI app): Settings, New agent defaults pane -> an "After send" row shows Stay, Back to Board, Next unread; picking Next unread writes `{"mode":"next-unread"}` to `after-send.json`.
