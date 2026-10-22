# TASK-295 selection-translate GPUI client

Group: G (writes the new GPUI crate, the app plugin registry, the app Cargo.toml, and Cargo.lock)
Class: code

## Brief

Goal: Add the GPUI client crate of `selection-translate`: a "Dịch" result action on selected assistant text, and a settings screen for provider and model.

Change: no client -> crate `bachuc-plugin-selection-translate` that registers the "Dịch" result action and the settings screen.

Boundaries:

- Follows [ADR-0062](watchtower/decisions/ADR-0062-selection-translate-plugin-copies-llm-logic-over-sharing-prompt-translate.md) and [ADR-0063](watchtower/decisions/ADR-0063-host-drawn-selection-result-card-over-plugin-views.md).
- The crate depends on `bachuc-plugin-host`, `bachuc-icons`, `gpui-kit` (workspace, no `features` key), `serde_json`, `anyhow`, and `log`. It never depends on a view crate ([docs/plugins.md](docs/plugins.md#gpui-native-clients)).
- Action: id `translate`, title `Dịch`, loading text `Đang dịch...`. It calls `ctx.rpc("selection.translate", {"text": ...})` and returns the `translation` string.
- Check the length of the trimmed text before the call: count UTF-16 units with `text.encode_utf16().count()`. Copy this; `utf16_len` in the Cite crate is `pub(crate)` and a plugin crate must not depend on another plugin crate; over 4000 returns the error `Đoạn chọn dài hơn 4000 ký tự` without a request. Empty text after trim returns the error `Không có chữ để dịch` without a request.
- Settings screen title `Selection translate`, icon `IconName::Languages`, two fields: a provider choice (`openrouter`, `vercel`) and a model text field. Follow the field widgets of [packages/desktop-gpui/plugins/prompt-translate/src/settings_view.rs](packages/desktop-gpui/plugins/prompt-translate/src/settings_view.rs); keep only these two fields.
- Register the crate in [packages/desktop-gpui/crates/app/src/plugins.rs](packages/desktop-gpui/crates/app/src/plugins.rs) in plugin id order (after `prompt-translate`): grow the `PLUGINS` array and the `IDS` test array from 15 to 16, and rename the test that names "fifteen".
- Trap: the test `every_plugin_id_is_its_crate_id_and_has_a_server_folder` in that file needs [hiep-plugins/plugins/selection-translate](hiep-plugins/plugins/selection-translate) to exist, so TASK-294 must be closed first.
- Planner call: decode and encode the settings record (`provider`, `model`) in `src/settings.rs`, as `crate::settings` does in prompt-translate.
- Trap: the workspace member glob `plugins/*` in [packages/desktop-gpui/Cargo.toml](packages/desktop-gpui/Cargo.toml) picks the crate up; do not add it there. Run `./cargo.sh metadata` after the build to catch a duplicate `Cargo.lock` entry.

How:

- Add `lib.rs` with `plugin()` and `NativePlugin::contribute` that calls `add_selection_result_action` and `add_settings_screen`.
- Put the length check and the answer decoder in a pure module with unit tests.
- Add `settings_view.rs` for the two fields.

Files:

- [packages/desktop-gpui/plugins/selection-translate](packages/desktop-gpui/plugins/selection-translate) (new crate: `Cargo.toml`, `src/lib.rs`, `src/request.rs`, `src/settings.rs`, `src/settings_view.rs`)
- [packages/desktop-gpui/crates/app/src/plugins.rs](packages/desktop-gpui/crates/app/src/plugins.rs) (register the crate; add to `IDS`)
- [packages/desktop-gpui/crates/app/Cargo.toml](packages/desktop-gpui/crates/app/Cargo.toml) (depend on the crate)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (new crate entry)

Interfaces:

- Consumes: `pub type SelectionResultHandler = Rc<dyn Fn(AssistantSelection, &mut App) -> Task<anyhow::Result<String>>>` in `bachuc_plugin_host`, from TASK-293
- Consumes: `pub struct SelectionResultAction { pub id: &'static str, pub title: SharedString, pub loading: SharedString, pub run: SelectionResultHandler }` in `bachuc_plugin_host`, from TASK-293
- Consumes: `Contributions::add_selection_result_action(&mut self, action: SelectionResultAction)`, from TASK-293
- Consumes: RPC `selection.translate` with input `{ "text": string }` and answer `{ "translation": string }` on plugin `selection-translate`, from TASK-294
- Consumes: settings id `translate` with fields `provider: "vercel" | "openrouter"` and `model: string`, read and written through `settings.translate.read`, `settings.translate.write`, and `settings.translate.reset`, from TASK-294

Expected result:

- WHILE the `selection-translate` plugin runs on the active host, WHEN the owner selects assistant text THE toolbar SHALL show "Dịch" after "Cite".
- WHEN the owner presses "Dịch" THE card SHALL show "Đang dịch...", then the Vietnamese text.
- IF the selection is over 4000 UTF-16 units THEN THE action SHALL return the length error without an RPC.
- IF the RPC fails THEN THE card SHALL show its error message.
- WHEN the owner changes the model in the settings screen THE next translation SHALL use that model.
- WHILE the plugin is not running on the active host, THE toolbar SHALL not show "Dịch".

Red tests:

File: [packages/desktop-gpui/plugins/selection-translate/src/request.rs](packages/desktop-gpui/plugins/selection-translate/src/request.rs) (test module; the builder writes `request_input` and `parse_translation` to pass them)

```rust
#[cfg(test)]
mod tests {
    use super::{parse_translation, request_input};
    use serde_json::json;

    #[test]
    fn the_input_carries_the_text() {
        assert_eq!(request_input("Hello").unwrap(), json!({ "text": "Hello" }));
    }

    #[test]
    fn empty_and_over_long_text_make_no_request() {
        assert_eq!(request_input("  ").unwrap_err().to_string(), "Không có chữ để dịch");
        let long = "a".repeat(4001);
        assert_eq!(request_input(&long).unwrap_err().to_string(), "Đoạn chọn dài hơn 4000 ký tự");
        assert!(request_input(&"a".repeat(4000)).is_ok());
        // An emoji is 2 UTF-16 units, so 2000 of them fill the limit and one more passes it.
        assert!(request_input(&"😀".repeat(2000)).is_ok());
        assert!(request_input(&"😀".repeat(2001)).is_err());
        assert_eq!(request_input("  Hello \n").unwrap(), json!({ "text": "Hello" }));
    }

    #[test]
    fn the_answer_is_the_translation_text() {
        assert_eq!(parse_translation(&json!({ "translation": "Xin chào" })).unwrap(), "Xin chào");
        assert!(parse_translation(&json!({})).is_err());
    }
}
```

Anti-goal: the GPUI app builds with exactly one new crate in `Cargo.lock` and no second gpui-kit build; tripwire; read from `./cargo.sh metadata --format-version 1 | grep -c '"name":"gpui-kit"'` (expect 1) and `git diff --stat packages/desktop-gpui/Cargo.lock` before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-selection-translate -p bachuc-app` -> all tests pass, including the 3 red tests and the `IDS` list test.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-plugin-selection-translate -- -D warnings` -> no warnings.
- Screen check (UI checker, `cua_repl`, live GPUI build, plugin installed on live): select an English sentence in an assistant message -> the toolbar shows "Cite" then "Dịch"; press "Dịch" -> "Đang dịch..." then Vietnamese text.
- Screen check: open Settings, then Selection translate -> a provider choice and a model field show with `openrouter` and `google/gemini-2.5-flash-lite`.
- Screen check: disable the plugin on live (`npm run cli -- plugin disable selection-translate`) -> "Dịch" is gone from the toolbar; enable it again after.
- Anti-goal: `./cargo.sh metadata --format-version 1 | grep -o '"name":"gpui-kit"' | wc -l` -> same count as before the change; `git diff --stat packages/desktop-gpui/Cargo.lock` -> only the new crate entry and its lines in `bachuc-app`.
