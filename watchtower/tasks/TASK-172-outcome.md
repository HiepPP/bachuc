# TASK-172 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/composer/Cargo.toml`: adds `bachuc-plugin-host`, and `futures` as a dev dependency (the interceptor chain test runs a future with `futures::executor::block_on`).
- `packages/desktop-gpui/Cargo.lock` (generated).
- `packages/desktop-gpui/crates/composer/src/draft.rs` (new): `SubmitIcon`, `DraftOptions`, `DraftSubmission` (with `build`), `min_input_height`, `draft_config`, `default_provider`, `model_groups`, `model_key`, `Generation`, and the local draft feature rules.
- `packages/desktop-gpui/crates/composer/src/plugin_slots.rs` (new): `pill_slots`, `press_effect`, `option_checked`, `command_visible`, `visible_commands`, `stop_button`, `picker_sources`, `toggle_chip`, `upsert_chip`, `host_action`, `needs_intercept`, `run_chain`, `plugin_pill`, `draft_pill_row`, and the `CornerPills` view.
- `packages/desktop-gpui/crates/composer/src/slash.rs`: `SlashItem::plugin`, `merge_items`, `resolve_plugin_command`; `menu_items` is now test-only.
- `packages/desktop-gpui/crates/composer/src/agent_controls/mod.rs`: the grouped draft model pill (`ControlsView::model_groups`).
- `packages/desktop-gpui/crates/composer/src/attachment_tray.rs`: `plugin_chip`.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: draft mode, the plugin slots, interceptors, the stop face, the attach menu and picker, chips, plugin slash commands, and the host event handling.
- `packages/desktop-gpui/crates/composer/src/lib.rs`: exports `DraftOptions`, `DraftSubmission`, `SubmitIcon`, `CornerPills`.

## Contract

- `Composer::new_draft(client, key, options, window, cx) -> Composer`; `agent_id()` returns `key`. `draft_submission(&self, cx: &App) -> Option<DraftSubmission>` (`None` for an agent composer), `set_draft_cwd`, `set_submitting`, `clear`.
- `ComposerEvent::DraftSubmit` and `ComposerEvent::ShowNewWorkspacePanel { plugin_id: &'static str, panel_id: &'static str }`.
- `CornerPills::new(client, agent_id, cx)`: absolute top 12, right 12, gap 8, right aligned.
- Slots read `bachuc_plugin_host::host(cx)` and its `ContributionIndex`, which holds only active plugins. No host or an empty index draws exactly the TASK-171 composer: the attach button picks images at once, no interceptor task starts, and the stop face is the red one.
- Pill `state` closures run in render. The composer redraws on `ContributionsChanged` and `PluginStateChanged`, and handles `SetComposerText` and `AddComposerAttachment` for its own agent only.
- Draft: model pill grouped by available provider (a pick sets provider and model); default provider is the first available one, default model `default_model`. Modes and features load for provider, model, or `cwd` changes through a `Generation`, so an older answer is dropped. Without `cwd` there is no `DraftConfig`, and only the provider and model pill shows. Submit shows `CornerDownLeft`, works with empty text, emits `DraftSubmit`; while submitting the area is at opacity 0.6 and the primary button spins.
- Interceptors run in registration order on send and queue; `Cancel` keeps the input. A draft runs them too and writes a replaced text into the input before `DraftSubmit`.
- Picker search calls `Client::invoke_plugin_rpc` with `search_method` and `{query}` and decodes with `decode_attachment_search` (planner call; `search_attachments` needs a `PluginContext`).

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> exit 0; 91 passed, 0 failed, no warning; every TASK-171 test included (`/tmp/wt-c-final-test.txt`). Tests cover: the 5-line minimum (105 px) with the Return icon and empty submit; the slot split by `placement` and `pill_applies` on draft and agent targets (`Only` on a draft, hidden on an agent; a pill scoped to another agent hidden); an active fake plugin with track and toolbar pills, an interceptor, two stop buttons (the first draws), and a picker source, and the same plugin inactive drawing none; the draft panel action giving `ShowPanel` without `on_press`; the `Select` value check; the interceptor chain order (`Replace("b")` then `Keep` sends "b", seen texts "a" then "b") and `Cancel`; the chip toggle (same item twice leaves no chip); the slash merge (`/clear` behind the built-in, a daemon command a plugin claims hidden) and `/route fast  ` giving `args: "fast"`, and with an attachment no match; `SetComposerText` and `AddComposerAttachment` for another agent ignored; `DraftSubmission::environment` from the draft; no `cwd` giving `config: None`; a features answer after a newer provider pick dropped; and the two anti-goal tests.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (`/tmp/wt-c-final-app.txt`).
- `rg -n "bachuc-plugin-host" packages/desktop-gpui/crates/composer/Cargo.toml` -> one line, `12:bachuc-plugin-host = { path = "../plugin-host" }`.
- UI check (New workspace draft composer against Electron 6770) -> DEFERRED (UI checker); it needs TASK-173 to place the draft.
- A `state` that returns `None` hiding the pill and the strip: checked by reading only (`pill_states` needs `App`); the strip rule itself is tested through `has_agent_tracks`.

## Anti-goal

- Checkpoint 1, before the plugin slots: 71 passed with no plugin code in the crate; source `/tmp/wt-c-171-test1.txt`; 2026-10-16 22:52.
- Checkpoint 2, after the slots landed: 90 passed, including `with_no_active_plugin_the_slots_are_empty` and `a_send_with_no_interceptor_starts_no_task`; source `/tmp/wt-c-172-test1.txt`; 2026-10-16 23:00.
- Checkpoint 3, at completion: 91 passed with both tripwire tests; source `/tmp/wt-c-final-test.txt`; 2026-10-16 23:03.

## Lessons

- `Contributions::default()` with `add_*` and `ContributionIndex::build` make a fake plugin for pure tests; `StopButton` and interceptor closures can be `unreachable!()` when the test never calls them.
- Defaults taken (see questions): a popover pill's view is cached per pill until its popover closes, so a redraw keeps its state; the picker is an overlay above the card, not an anchored combobox; plugin chips are not commentable when added from the picker, as Electron's toggle adds them.
