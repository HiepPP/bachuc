# TASK-326 GPUI Ponytail pill

Group: B (writes `packages/desktop-gpui/plugins/ponytail/**` and the app registry; start order after TASK-325, no shared file)
Class: code

## Brief

Goal: The GPUI composer shows a "Ponytail: <mode>" pill in the toolbar of every agent composer and New workspace draft. Picking a mode saves it for that agent or draft.

Change: no Ponytail pill -> a toolbar Select pill with Off, Lite, Full, Ultra, next to the Caveman pill.

Boundaries:

- Follows [ADR-0079](watchtower/decisions/ADR-0079-ponytail-plugin-over-extending-prompt-translate.md): a new crate `bachuc-plugin-ponytail`; it never depends on `bachuc-plugin-prompt-translate`.
- Follows [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md): the crate reaches the server only through plugin RPCs.
- Planner call: the label is `Ponytail: <Label>`, because the host shows only the label and the Caveman pill label is a bare mode name. The title, shown as the tooltip, is `Ponytail mode`.
- Planner call: while an agent's mode is unknown, the label is `Ponytail: Full`, the server default. No settings screen.
- Copy the read-once rules of `Modes` in [packages/desktop-gpui/plugins/prompt-translate/src/prefs.rs](packages/desktop-gpui/plugins/prompt-translate/src/prefs.rs): one read per agent, no read loop after a failure, clear on reconnect.
- Trap: `gpui-kit` in the new `Cargo.toml` must be `gpui-kit.workspace = true` with no `features` key. A features key makes a second GPUI build.
- Trap: `PLUGINS` in [packages/desktop-gpui/crates/app/src/plugins.rs](packages/desktop-gpui/crates/app/src/plugins.rs) is a fixed-size array in plugin id order. Raise the length to 17 and put `ponytail` between `plugins-nav` and `prompt-translate`.
- Trap: the tests in the same file hold their own `IDS: [&str; 16]` list and a test named `the_registered_ids_are_the_sixteen_in_catalog_order`. Add `ponytail` to `IDS`, raise it to 17, and rename the test to say seventeen. Another test there needs the folder `hiep-plugins/plugins/ponytail/` from TASK-325.
- Run cargo only through `./cargo.sh` in [packages/desktop-gpui](packages/desktop-gpui). When done, quit any GPUI app you launched and run `./cargo.sh clean --profile dev`, unless another `cargo` or `rustc` process runs.

How:

- `lib.rs`: `plugin()` and `NativePlugin::contribute`, which adds one `ComposerPill` with `PillPlacement::Toolbar` and `ShowOnDraft::Yes`.
- `mode.rs`: the `Mode` enum with `parse`, `value`, and `label`, the draft value rules, the read-once store, and the two RPC names with input builders and the answer parser. Shapes come from prompt-translate `prefs.rs`, `settings.rs`, and `rpc.rs`.
- Register the crate in `crates/app/Cargo.toml` and `PLUGINS` in `crates/app/src/plugins.rs`; `Cargo.lock` updates by the build.

Files:

- [packages/desktop-gpui/plugins/ponytail/Cargo.toml](packages/desktop-gpui/plugins/ponytail/Cargo.toml) (new crate `bachuc-plugin-ponytail`; deps `anyhow`, `bachuc-plugin-host`, `gpui-kit`, `log`, `serde_json`)
- [packages/desktop-gpui/plugins/ponytail/src/lib.rs](packages/desktop-gpui/plugins/ponytail/src/lib.rs) (new; plugin and pill contribution)
- [packages/desktop-gpui/plugins/ponytail/src/mode.rs](packages/desktop-gpui/plugins/ponytail/src/mode.rs) (new; modes, labels, draft value, read-once store, RPC names and parsing, unit tests)
- [packages/desktop-gpui/plugins/ponytail/tests/plugin.rs](packages/desktop-gpui/plugins/ponytail/tests/plugin.rs) (new; plugin id test)
- [packages/desktop-gpui/crates/app/Cargo.toml](packages/desktop-gpui/crates/app/Cargo.toml) (add the dependency)
- [packages/desktop-gpui/crates/app/src/plugins.rs](packages/desktop-gpui/crates/app/src/plugins.rs) (add `ponytail` to `PLUGINS` and to the test `IDS`)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (updated by cargo)

Interfaces:

- Consumes: RPC `ponytail.mode.read` with input `{ agentId: string (uuid) }` and output `{ mode: "off" | "lite" | "full" | "ultra" }` from TASK-325
- Consumes: RPC `ponytail.mode.write` with input `{ agentId: string (uuid), mode: "off" | "lite" | "full" | "ultra" }` and output `{ mode: "off" | "lite" | "full" | "ultra" }` from TASK-325
- Consumes: draft composer value under plugin id `ponytail`: `{ mode: "off" | "lite" | "full" | "ultra" }`, read from env `PASEO_PLUGIN_COMPOSER_ponytail` at `agent.session_open` from TASK-325

Expected result:

- WHEN an agent composer draws THE pill SHALL show `Ponytail: <Label>` of the agent's saved mode, after one `ponytail.mode.read`.
- WHEN the user picks a mode on an agent composer THE pill SHALL show it at once and send `ponytail.mode.write`.
- WHEN the user picks a mode on a New workspace draft THE pill SHALL store `{ mode }` in the draft value under `ponytail` and send no RPC.
- WHILE a draft has no stored value, THE pill SHALL show `Ponytail: Full`.
- IF a mode read fails THEN THE pill SHALL show `Ponytail: Full` and start no new read until a reconnect.

Red tests:

File: [packages/desktop-gpui/plugins/ponytail/src/mode.rs](packages/desktop-gpui/plugins/ponytail/src/mode.rs)

```rust
#[cfg(test)]
mod tests {
    use super::{Mode, Modes, draft_mode, draft_payload, mode_options, pill_label};
    use serde_json::json;

    #[test]
    fn labels_and_options() {
        assert_eq!(pill_label(Some(Mode::Ultra)), "Ponytail: Ultra");
        assert_eq!(pill_label(Mode::parse("off")), "Ponytail: Off");
        assert_eq!(pill_label(Mode::parse("nope")), "Ponytail: Full");
        assert_eq!(pill_label(None), "Ponytail: Full");
        let options = mode_options();
        assert_eq!(
            options.iter().map(|o| o.id.as_ref()).collect::<Vec<_>>(),
            ["off", "lite", "full", "ultra"]
        );
    }

    #[test]
    fn a_draft_reads_its_saved_mode_then_full() {
        assert_eq!(draft_mode(Some(&draft_payload(Mode::Off))), Mode::Off);
        assert_eq!(draft_mode(None), Mode::Full);
        assert_eq!(draft_mode(Some(&json!({ "mode": "nope" }))), Mode::Full);
    }

    #[test]
    fn read_once_per_agent_and_no_loop_after_failure() {
        let mut modes = Modes::default();
        assert_eq!((0..100).filter(|_| modes.ask("a1")).count(), 1);
        assert!(modes.publish("a1", Mode::Lite));
        assert!(!modes.publish("a1", Mode::Lite));
        assert!(modes.ask("a2"));
        modes.read_failed("a2");
        assert_eq!((0..100).filter(|_| modes.ask("a2")).count(), 0);
        modes.clear();
        assert!(modes.ask("a2"));
    }
}
```

Anti-goal: the pill starts at most one `ponytail.mode.read` per agent between reconnects; tripwire; read from the `read_once_per_agent_and_no_loop_after_failure` test before changes, after the store is written, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-ponytail` -> all tests pass, including the three red tests.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app plugins` -> the plugin registry tests pass with 17 entries, after TASK-325 created `hiep-plugins/plugins/ponytail/`.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-plugin-ponytail -- -D warnings` -> exit 0.
- `rg -n 'features' packages/desktop-gpui/plugins/ponytail/Cargo.toml` -> no match on the `gpui-kit` line.
- Screen check on a GPUI build against live, done by the UI checker with AX actions only: an agent composer shows `Ponytail: Full` next to the Caveman pill; picking Ultra changes the label to `Ponytail: Ultra` -> screenshot of both states.
- Screen check: a New workspace draft picks Off, then creates the thread -> the new thread's pill shows `Ponytail: Off`. The daemon side of this case is checked in TASK-327.
- Anti-goal: the `read_once_per_agent_and_no_loop_after_failure` test passes -> at most one read per agent.
- After the checks: quit the GPUI app and run `./cargo.sh clean --profile dev` -> `target/debug` is gone; `target/release` stays.
