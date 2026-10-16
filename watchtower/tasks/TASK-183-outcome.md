# TASK-183 outcome

## Status

DONE. The UI check lines are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/plugins/prompt-translate/Cargo.toml` (new crate `bachuc-plugin-prompt-translate`)
- `packages/desktop-gpui/plugins/prompt-translate/src/lib.rs`: `plugin()`, the contributions, the send interceptor wiring
- `packages/desktop-gpui/plugins/prompt-translate/src/vietnamese.rs`: `has_vietnamese`
- `packages/desktop-gpui/plugins/prompt-translate/src/settings.rs`: `Settings`, `Mode`, `RewriteMode`, `rewrites_on_send_key`
- `packages/desktop-gpui/plugins/prompt-translate/src/state.rs`: `Current`, the `active_since` rule
- `packages/desktop-gpui/plugins/prompt-translate/src/notes.rs`: `Cache`, note label rule, cache-only rule
- `packages/desktop-gpui/plugins/prompt-translate/src/note_view.rs`: the `prompt-note` renderer view
- `packages/desktop-gpui/plugins/prompt-translate/src/intercept.rs`: interceptor plan and outcome, transformer
- `packages/desktop-gpui/plugins/prompt-translate/src/prefs.rs`: mode store, pill state, draft mode
- `packages/desktop-gpui/plugins/prompt-translate/src/rpc.rs`: RPC names, inputs, answer decoders
- `packages/desktop-gpui/plugins/prompt-translate/src/shared.rs`: shared state, settings watcher, lookup start
- `packages/desktop-gpui/plugins/prompt-translate/src/settings_view.rs`: the "Prompt translate" screen
- `packages/desktop-gpui/plugins/prompt-translate/tests/plugin.rs`
- `packages/desktop-gpui/Cargo.lock` (generated; also holds the entry of the TASK-188 crate)

## Contract

- Produces `pub fn plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>`, whose `id()` is `"prompt-translate"`, for TASK-191.
- Contributions: transformer `prompt-note` (`user_message`), renderer `prompt-note` v1, interceptor `rewrite-english`, pill `prompt-prefs` (`Toolbar`, `ShowOnDraft::Yes`, `Select` with seven modes), settings screen `translate` (title `Prompt translate`, icon `IconName::Languages`).
- RPCs called: `translate.translate`, `translate.enhance`, `translate.original`, `translate.mode.read`, `translate.mode.write`. Settings id `translate`. The server part is unchanged.
- Depends on no view crate. `gpui-kit.workspace = true` has no `features` key.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-prompt-translate` -> 21 unit tests and 1 integration test pass, no warning from this crate.
  - Ported client cases: `each_distinct_text_is_looked_up_once_across_renders`, `a_failed_lookup_is_retried_on_the_next_render` (`notes.test.ts`), `switching_translation_back_on_moves_active_since_forward_and_notifies` (`state.test.ts`), `has_vietnamese_detects_diacritics_including_words_shared_with_other_languages`, `automatic_rewrites_on_enter_and_the_button_cmd_enter_mode_rewrites_only_on_cmd_enter` (`shared.test.ts`).
  - Named in Verify: `transformer_cases`, `interceptor_keeps_commands`, `interceptor_cancels_on_error`, `note_label_rule`, `note_cache_one_lookup`, `failed_lookup_retries`, `missing_timestamp_is_cache_only`, `settings_defaults_fill_missing_fields`, `pill_label_rule`, `failed_mode_read_no_loop`.
- `[dependencies]` section of `Cargo.toml` against `bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)` -> no match (rg exit 1).
- `rg -n "gpui-kit" packages/desktop-gpui/plugins/prompt-translate/Cargo.toml` -> one line, `gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-prompt-translate` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0. The app does not register this crate before TASK-191, so this proves only that the workspace builds. The one warning is in `bachuc-app`, not in this crate.
- UI check (after TASK-191): the `EN` note screenshot -> DEFERRED (UI checker).
- UI check (after TASK-191): the Caveman mode pill screenshot -> DEFERRED (UI checker).
- Code that needs `App`, `Window`, or `Context` (note view, pill closures, settings view, watcher, interceptor task) is checked by compile and reading only; no headless GPUI test context exists.

## Anti-goal

One lookup per text and cache mode, however many rows render it.

| Checkpoint                                   | Value            | Source                                                      | Time       |
| -------------------------------------------- | ---------------- | ----------------------------------------------------------- | ---------- |
| After the cache landed: 100 asks of one note | 1 lookup started | `note_cache_one_lookup` (`src/notes.rs`)                    | 2026-10-17 |
| At completion: 100 asks of one note          | 1 lookup started | `note_cache_one_lookup`, rerun in the final `cargo.sh test` | 2026-10-17 |

The same text cache-only and live gives 2 lookups; the cache key holds the mode.

## Lessons

- Safe defaults taken:
  - A stored field the schema would reject takes its default (`Settings::decode`), not an error.
  - `InterceptOutcome` has no error variant, so an enhance error shows `show_message(Error)` and returns `Cancel`.
  - A failed note lookup leaves the cache and does not redraw the view. The view tries again on a later render, but not within 2 seconds of the failure (`RETRY_AFTER`), so a window that redraws each frame sends no request per frame.
  - The view recomputes `cache_only` on each render, as Electron does. A flip (switching translation back on) asks again, and an answer for the old mode is dropped.
  - A pick on a draft composer always saves into the draft value. Electron saves only when the draft mode is known (`prefs.ts:56`); the difference shows only before the settings load.
  - A failed `translate.mode.write` keeps the optimistic mode and logs a warning, as Electron does.
- A failed mode read retries after the next reconnect or after the plugin reactivates (a new store), not on a timer. This matches the skill-pins crate.
- The model inputs are seeded once, when the settings first load. A change of the stored models from another client does not reach the inputs, as `initialValue` in Electron.
- In a file with `use gpui_kit::*`, the test module imports by name, not `use super::*`.
