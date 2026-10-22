# TASK-295 Outcome

## Status

DONE. Code checks pass. Screen checks are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/plugins/selection-translate/` (new crate): `Cargo.toml`, `src/lib.rs`, `src/request.rs`, `src/settings.rs`, `src/settings_view.rs`.
- `packages/desktop-gpui/crates/app/src/plugins.rs`: `PLUGINS` and `IDS` grow from 15 to 16; the test is renamed `the_registered_ids_are_the_sixteen_in_catalog_order`.
- `packages/desktop-gpui/crates/app/Cargo.toml`: depends on the new crate.
- `packages/desktop-gpui/Cargo.lock`: the new crate entry and one line in `bachuc-app` (+13 lines).

## Contract

- `bachuc_plugin_selection_translate::plugin() -> Box<dyn NativePlugin>`, id `selection-translate`.
- `contribute` adds `SelectionResultAction { id: "translate", title: "Dịch", loading: "Đang dịch..." }` and `SettingsScreen { id: "translate", title: "Selection translate", icon: IconName::Languages }`.
- `request::request_input(&str) -> anyhow::Result<Value>` (trim, empty and UTF-16 over 4000 error, else `{"text"}`); `request::parse_translation(&Value) -> anyhow::Result<String>`.
- `settings::Settings { provider, model }` with `decode` and `to_value`; defaults `openrouter` and `google/gemini-2.5-flash-lite`.

## Verified

- Red: the three red tests were written first in `request.rs` with no implementation. `cargo test -p bachuc-plugin-selection-translate` failed with E0432 (`request_input` and `parse_translation` missing). Then implemented; all pass. `./cargo.sh fmt` re-wrapped the test lines; assertions are unchanged.
- `./cargo.sh test -p bachuc-plugin-selection-translate` -> 7 passed (3 red tests, 2 settings, 2 settings view).
- `./cargo.sh test -p bachuc-app` -> 95 passed, including `the_registered_ids_are_the_sixteen_in_catalog_order` and `every_plugin_id_is_its_crate_id_and_has_a_server_folder`. This is a group-level run of the whole `bachuc-app` binary, not only one filter.
- `./cargo.sh clippy -p bachuc-plugin-selection-translate --no-deps -- -D warnings` -> clean. `--no-deps` per AUTORUN Repo Rules (`bachuc-protocol` and `bachuc-theme` fail outside this run).
- `./cargo.sh fmt --check -p bachuc-plugin-selection-translate` and `-p bachuc-app` -> exit 0.
- No `gitnexus` impact run: the change adds a new crate and one registry entry, with no existing symbol edited.
- Screen checks (toolbar "Cite" then "Dịch", card text, Settings screen, plugin disable): DEFERRED (UI checker).

## Anti-goal

- Tripwire: one new crate in `Cargo.lock`, no second gpui-kit build.
- Before (metadata `grep -o '"name":"gpui-kit"' | wc -l`): 32; `git diff --stat Cargo.lock`: empty.
- After: 33; `git diff --stat Cargo.lock`: 13 insertions, 0 deletions. The diff holds only the new `[[package]]` block and one line in `bachuc-app`.
- The +1 in the metadata count is the dependency edge of the new crate to `gpui-kit`, which the metadata lists once per dependent. `grep -c '^name = "gpui-kit"' Cargo.lock` is 1, so there is one gpui-kit package and no second build. The Verify line "same count" cannot hold as written; the lock entry count is the right check.
- Source: the commands above, 2026-10-22. PASS.

## Lessons

- `./cargo.sh metadata` repeats `"name":"gpui-kit"` once per dependent crate, so a new plugin crate raises that count by 1. Check `grep -c '^name = "gpui-kit"' Cargo.lock` for the package count.
- `InputState::value()` returns `SharedString`; pass `&value` to a `&str` helper.
