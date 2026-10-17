# TASK-185 outcome

## Status

DONE. Code tests only; the UI check is DEFERRED (UI checker), after TASK-191 registers the crate.

## Changed

- `packages/desktop-gpui/plugins/next-prompt-actions` (new crate `bachuc-plugin-next-prompt-actions`):
  - `Cargo.toml`: the spec's dependencies; `fancy-regex = "=0.19.2"` and `unicode-normalization = "=0.1.25"` pinned.
  - `src/lib.rs`: `plugin()`, the `next-prompt-panel` transformer and renderer, the `send` settings screen, and the `toggle-auto-run` command.
  - `src/section.rs`, `src/prompts.rs`, `src/next_prompts.rs`, `src/selection.rs`, `src/polling.rs`, `src/contracts.rs`: ports of the `shared/*.ts` rules and RPC shapes.
  - `src/rules.rs`: pure panel rules: card faces, candidate match, RFC 3339 time parse, busy rule, selection texts, Send order, command visibility.
  - `src/model.rs`: `Reads` (pure read schedule) and the per-agent `PanelModel` entity with the weak `Models` map.
  - `src/panel.rs`: the panel view. `src/settings_view.rs`: the settings screen.
  - `tests/plugin.rs`: the public plugin id.
- `packages/desktop-gpui/Cargo.lock` (generated): adds this crate, `fancy-regex`, `unicode-normalization`, `bit-set 0.8.0`, and `bit-vec 0.8.0`. The lock also holds the other builder's `bachuc-plugin-thread-branch` entry, because cargo resolves the whole workspace.

## Contract

- Produces: `pub fn plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>` in `bachuc-plugin-next-prompt-actions`, with `id()` `"next-prompt-actions"`, for TASK-191.
- Consumes, as named: `NativePlugin`, `Contributions` (`add_timeline_transformer`, `add_timeline_renderer`, `add_settings_screen`, `add_command_item`), `PluginContext` (`client`, `rpc_as`, `settings`, `set_composer_text`, `open_surface`, `show_message`, `notify`), `OpenSurfaceOptions`, `MessageLevel`, `PluginSettings`, `SettingsState`, `CommandCenterItem`, `CommandContext`, `CommandInput`, `SettingsScreen`, the timeline kinds, `ui::{note, section, card, switch_row, action_row, markdown, RowText}`, `Client::agent_attention`, `ClientEvent`, and `IconName::{Send, CornerDownRight}`.
- The server part in `hiep-plugins/plugins/next-prompt-actions` is unchanged.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-next-prompt-actions` -> pass: 57 unit tests and 1 integration test. It has 15 tests for `tests/prompts.test.ts`, 15 for `tests/section.test.ts`, 3 for `tests/polling.test.ts`, and 5 for the `parsePrompts`-only cases of `tests/next-prompts.test.ts`. Each named test passes: `transformer_cases`, `card_face_rule`, `candidate_match_rule`, `missing_timestamp_no_candidate`, `busy_rule`, `selection_footer_texts`, `back_to_board_opens_before_send`, `command_hidden_without_evaluator`, `idle_agent_no_timed_inspect`, `jev_review_polls_every_2500_ms`, and `scope_once_per_workspace`.
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)" packages/desktop-gpui/plugins/next-prompt-actions/Cargo.toml` -> no match (exit 1).
- `rg -n "gpui-kit" packages/desktop-gpui/plugins/next-prompt-actions/Cargo.toml` -> one line, `17:gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-next-prompt-actions` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0. `bachuc-app` does not depend on this crate until TASK-191.
- Extra: `cargo.sh clippy -p bachuc-plugin-next-prompt-actions --tests` -> no warning in this crate.
- UI check (panel with Recap chips and cards, against Electron 6770) -> DEFERRED (UI checker).

## Anti-goal

- Tripwire: an idle agent sends no timed `prompts.inspect`, and `prompts.scope` goes out once per agent and workspace.
- Checkpoint 1, model landed: `idle_agent_no_timed_inspect` -> ok, `scope_once_per_workspace` -> ok. Source: the first full crate test run on 2026-10-17, before 01:10 +07; the exact time was not captured.
- Checkpoint 2, completion: `idle_agent_no_timed_inspect` -> ok, `scope_once_per_workspace` -> ok, `jev_review_polls_every_2500_ms` -> ok. Source: `cargo.sh test -p bachuc-plugin-next-prompt-actions --lib model::tests`, 2026-10-17 01:10:19 +0700.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground).

- UI check (panel with Recap chips and one card per prompt, Edit and Send, against Electron 6770) -> UNVERIFIED (unattended): needs pointer. No agent thread can open (AX press on sidebar rows does nothing: `/tmp/wt-ui/gpui-t144.jpg`). The reference also lacks the plugin (`failed` on 6770).

## Lessons

- The GPUI client has no `lastActivityAt`, so `Reads` keys an inspect read on the agent status alone, as the spec states. `inspection_activity` is ported and tested, and the model calls it with no activity time.
- GPUI sends `ModifiersChangedEvent` only along the focused dispatch path, and the composer holds focus. The Send button reads the platform modifier on hover and on each mouse move over it. A modifier pressed with a still pointer shows the swapped face only after the next move.
- The models map holds weak entities: a model lives while one of its agent's panels holds it, so an agent that has no mounted panel gets no reads.
- The `prompts.host` read for the command waits for `ClientEvent::Connected` when the client is not yet connected, because an earlier request fails.
- A JS replace string is literal, but `regex` and `fancy-regex` expand `$` in replacements. The port replaces the first match by its byte range.
