# TASK-182 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/plugins/skill-pins` (new crate `bachuc-plugin-skill-pins`):
  - `Cargo.toml`: `anyhow`, `bachuc-client`, `bachuc-icons`, `bachuc-plugin-host`, `bachuc-theme`, `gpui-kit.workspace = true`, `log`, `serde`, `serde_json`. No view crate.
  - `src/lib.rs`: `plugin()`, id `skill-pins`; contributes the settings screen and the `skills` pill.
  - `src/catalog.rs`: catalog, `pill_label`, `normalize`, `parse_answer` (strict), `saved_skills` (drops unknown), `draft_value`, `draft_skills`, `toggle`, `set_default`, wire inputs, `skill_hint`. All pure.
  - `src/pins.rs`: pure `State` (known, in-flight, failed, errors) and the `Pins` store (`current`, `save`, reads and writes through `PluginContext::rpc`); `Watcher` entity holds the reconnect and settings subscriptions.
  - `src/menu.rs`: `SkillsMenu` popover view and its rows.
  - `src/settings_view.rs`: pure `screen` and `SkillPinsSettingsView`, built only from `bachuc_plugin_host::ui`.
  - `tests/plugin.rs`: the plugin id.
- `packages/desktop-gpui/Cargo.lock` (generated): adds `bachuc-plugin-skill-pins`.

## Contract

- Produces `pub fn plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>` with `id()` `"skill-pins"`. The app registers it in TASK-191.
- Pill: id `skills`, `ShowOnDraft::Yes`, `PillPlacement::Toolbar`, title `Pinned skills`, label `pill_label(skills)`, `PillBehavior::Popover { flush: true, .. }`. It hides when the target has neither an agent nor a draft.
- Agent pins: `skill-pins.read` on the first `state` call per agent. A save publishes at once, sends `skill-pins.write`, and publishes the answer. A failed read or write keeps the shown list and sets the error line.
- Draft pins: `ComposerDraft::get("skill-pins")`, else the settings `defaults`, else unknown. A save calls `ComposerDraft::set("skill-pins", {"skills": next})`.
- Reconnect: `ClientEvent::Connected` clears the store and notifies. The `Watcher` entity is captured by the pill `state` closure, so deactivation drops it.
- Settings screen: id `skill-pins`, title `Skill pins`, icon `Pin`, texts as `settings.tsx`.
- Decisions on points the spec left open:
  - A new save clears the agent's error line (Electron keeps it until remount). A successful answer clears it too.
  - The failed-read mark lives in a `failed` set. It is cleared on reconnect, and a plugin state change builds a new store.
  - The menu width is 240 px, not 200, so a description fits two lines. It is not measured against Electron.
  - Rows draw the `surface2` fill on hover only. They have no `track_focus`, so there is no focus fill.
  - Rows carry `Role::MenuItem` and a label; toggle rows set `aria_selected`.
  - The `claude` field of `catalog.ts` is not in the Rust catalog; the spec lists only labels, short names, descriptions, and warning.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-skill-pins` -> exit 0; 20 unit tests and 1 integration test passed, with no warning from this crate. Required names present: `pill_label_none_and_two`, `read_answer_sorted_and_deduped`, `unknown_default_dropped`, `unknown_rpc_id_is_error`, `draft_value_then_defaults_then_unknown`, `toggle_keeps_catalog_order`, `failed_write_keeps_list_and_sets_error`, `equal_list_does_not_notify`, `read_once_per_agent`, `reconnect_clears_store`, `failed_read_no_loop`. The five cases of `catalog.test.ts` map to `catalog_holds_the_three_pinned_skills_in_menu_order`, `read_answer_sorted_and_deduped`, `pill_label_none_and_two`, `unknown_rpc_id_is_error`, `rpc_wire_names_are_lowercase_with_dots_and_hyphens`.
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)" packages/desktop-gpui/plugins/skill-pins/Cargo.toml` -> no match (exit 1).
- `rg -n "gpui-kit" packages/desktop-gpui/plugins/skill-pins/Cargo.toml` -> one line, `15:gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-skill-pins` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (the crate is not in the app until TASK-191, so this checks the workspace and lock).
- UI check (after TASK-191): the Skills pill in the toolbar -> DEFERRED (UI checker).
- UI check (after TASK-191): the Skill pins settings page -> DEFERRED (UI checker).
- The menu look, the 240 px width, and the pill redraw after a draft or settings change are UNVERIFIED (unattended): code tests only. The case "100 `state` calls for one unknown agent -> one read" is covered by the pure `read_once_per_agent` on `State::ask`, not by a fake-client run.

## Anti-goal

- Tripwire: at most one `skill-pins.read` per agent between reconnects.
- Checkpoint 1, after the store landed: `read_once_per_agent` passed (100 asks for `a1` -> 1 start; `a2` -> 1 start); source `/tmp/wt-l-test2.txt`; 2026-10-17.
- Checkpoint 2, at completion: the same test passed in the final run, together with `failed_read_no_loop` (100 asks after a failure -> 0 starts) and `reconnect_clears_store`; source the final `cargo.sh test -p bachuc-plugin-skill-pins`; 2026-10-17.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Electron 6770 shows every plugin as `failed`, so it draws no plugin UI.

- UI check (Skills pill in the toolbar of an agent composer, with pinned short names, placed and sized as in Electron 6770) -> UNVERIFIED (unattended): needs pointer. No agent composer can open (AX press on sidebar rows does nothing: `/tmp/wt-ui/gpui-t144.jpg`). Seen instead: the New workspace draft composer toolbar shows a `Skills` pill (AX `Pinned skills`) after `Default`, with no pinned names (`/tmp/wt-ui/gpui-sidepanel2.jpg`); the reference has no pill to compare with.
- UI check (Settings page `Skill pins` with three switch rows) -> UNVERIFIED (unattended): needs pointer. AX press on the sidebar `Settings` button changed nothing (`/tmp/wt-ui/gpui-settings.jpg`).

## Lessons

- `.when` on a `Stateful` element needs `use gpui_kit::prelude::FluentBuilder as _;`; `use gpui_kit::*` does not bring it in.
- gpui `ElementId` has no `From<(&str, &str)>`; use `(&str, usize)` with the row index.
- `cargo fmt` rewrites the files at once; a read after it shows "changed on disk" notices that come from the formatter.

## UI Checks, round 2

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Electron 6770 shows every plugin as `failed`, so it draws no plugin UI.

- UI check (Skills pill in the toolbar of an agent composer, with pinned short names, placed and sized as in Electron 6770) -> UNVERIFIED (unattended): the pill shows in the agent composer toolbar after `Default` and before the model pill, and an AX press opens its menu (`Watchtower`, `Chase goal`, `Sequential thinking`, clear-all; `/tmp/wt-ui2/skills-popover.jpg`, `/tmp/wt-ui2/after-press-ws31.jpg`). No skill is pinned on live, so no short names show, and the reference has no pill to compare. Pinning would change live settings.
- UI check (Settings page `Skill pins` with three switch rows) -> PASS. The `Plugins` row and `Settings` open the Settings page, and the `Skill pins` screen shows the heading `skill-pins · Skill pins`, the section `Default pins for new agents`, and three switch rows (`Watchtower`, `Chase goal`, `Sequential thinking`, all off) (`/tmp/wt-ui2/settings-skill-pins.jpg`). The texts match `hiep-plugins/plugins/skill-pins/shared/catalog.ts` and `client/settings.tsx:41`. The reference has no such page, so the layout is not compared.
