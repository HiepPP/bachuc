# TASK-293 Outcome

## Status

DONE. Owner check Q-187 PASS (Hiep, 2026-10-22).

## Changed

- `packages/desktop-gpui/crates/plugin-host/src/contributions.rs`: `SelectionResultHandler`, `SelectionResultAction`, field and `add_selection_result_action`.
- `packages/desktop-gpui/crates/plugin-host/src/index.rs`: `selection_result_actions` stored in `build` and listed.
- `packages/desktop-gpui/crates/plugin-host/src/lib.rs`: exports.
- `packages/desktop-gpui/crates/timeline/src/plugin_view.rs`: result buttons after plain ones, card state, card render, close rules.
- `packages/desktop-gpui/crates/timeline/src/plugins.rs`: `card_size`, plus `card_origin` and the `CARD_MAX_*` constants.
- `packages/desktop-gpui/crates/timeline/src/lib.rs` (not in Files:, required): one field `result_card: ResultCards`, its initializer, and the `use` line. The private `TimelineView` struct lives there.
- `packages/desktop-gpui/crates/timeline/tests/plugins.rs`: tests of `card_size`, `card_origin`, and the index.
- `docs/plugins.md`: one GPUI result action line, and the selection action text now names GPUI support.

## Contract

- `pub type SelectionResultHandler = Rc<dyn Fn(AssistantSelection, &mut App) -> Task<anyhow::Result<String>>>`
- `pub struct SelectionResultAction { pub id: &'static str, pub title: SharedString, pub loading: SharedString, pub run: SelectionResultHandler }`
- `Contributions::add_selection_result_action(&mut self, action: SelectionResultAction)`
- `ContributionIndex::selection_result_actions() -> &[Contributed<SelectionResultAction>]`
- `bachuc_timeline::plugins::card_size(content, zoom)`, `card_origin(anchor, card, area)`.

## Verified

- Red: the tests were added before any implementation edit. The first compile raced my implementation edits, so a separate red result was not captured.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-plugin-host -p bachuc-timeline` -> exit 0, including `the_result_card_fits_its_content_up_to_420_by_320_times_the_zoom` (200x100 stays; 900x900 -> 420x320 at 1, 525x400 at 1.25), `the_result_card_stays_inside_the_timeline`, `the_index_lists_result_actions_after_registration_order`. Group-level run, one TASK.
- `./cargo.sh check -p bachuc-plugin-assistant-cite` -> exit 0, Cite crate unchanged.
- `./cargo.sh fmt --check -p bachuc-plugin-host -p bachuc-timeline` -> exit 0.
- Clippy `-p bachuc-plugin-host -p bachuc-timeline --no-deps -- -D warnings`: UNSATISFIABLE on this toolchain (rust 1.97). It reports 13 `type_complexity` errors in plugin-host (`composer.rs`, `context.rs`, `contributions.rs` lines 57-230) and 3 in timeline (`plugin_view.rs:185` unit let in `reveal_now`, `lib.rs:376`, `lib.rs:3345`). All are in code this TASK did not touch. No new lint in the added lines. Replace with: clippy on the diff lines, or fix the old lints in their own TASK.
- DEFERRED (UI checker): card shows "Đang dịch...", then Vietnamese text and Copy; press twice; Esc and new selection close; plugin stopped shows error and no Copy.
- UNVERIFIED (owner): click outside closes; Copy writes plain text.

## Anti-goal

Cite keeps working the same way. Code evidence: the plain action path is the same `run_selection_action` handler, and the Cite crate compiles unchanged. Screen read of Cite on live before and at completion: DEFERRED (UI checker), not measured here.

## Lessons

- `Selection` cleanup fires `text_state_changed` with no payload; only a new `Some` payload closes the card.
- `TimelineView` fields are private to the crate root, so a card state field needs a small `lib.rs` edit.
- Clippy on this toolchain fails on old code in plugin-host and timeline; judge new code by line numbers.
- The card body uses `bachuc_plugin_host::ui::markdown` (same style as the plugin cards). The body scroll layout (`min_h_0`, `flex_shrink(1.)`, `overflow_y_scroll` under a `max_h` card) is read-checked only; confirm on screen.

## UI Checks

UI checker, 2026-10-22, check bundle at `74d73ff80` on live (6768), zoom 1.25, plugin `selection-translate` running. Screenshots in `/tmp/wt-ui/` (`293-*.png`). Selection was made with the AX press on the assistant message plus `cmd+a` (no pointer).

- Screen check, press "Dịch": PASS. The card shows header `Dịch` and body `Đang dịch...` (`293-loading.png`), then `Xem README.md:60 và CONTRIBUTING.md:20 để biết chi tiết.` with a `Copy` button (`293-result.png`). AX: `container Dịch` with `button Copy`.
- Screen check, press "Dịch" twice fast: PASS with a changed method. The toolbar closes after the first press, so a literal double press is impossible. Press, select other text (card closes, `after other selection card? false`), select the assistant text, press again: one card with `Copy` at t+1.2, 2.2, 3.7, and 6.7 s (`293-second-answer.png`). The answer arrives in under 1.2 s, so a dropped late first answer was not separately observable.
- Screen check, Esc closes the card: PASS (AX after Esc has no `container Dịch`). Select other text closes the card: PASS (the user message selection closed it; `293-newselection.png`).
- Screen check, plugin stopped: the button hides. `npm run cli -- plugin disable selection-translate` -> the toolbar has only `Cite` (`295-disabled.png`). The error card path and its missing Copy button: UNVERIFIED (no way to reach it with the button hidden). The plugin was enabled again and `Dịch` returned.
- Owner check, click outside closes and Copy writes plain text: UNVERIFIED (owner).
- Anti-goal, Cite: PASS. Select, press `Cite` -> one `Quote` attachment in the composer and the toolbar closed (`293-cite.png`). The chip was removed after. Before: code read, the plain action path (`run_selection_action`) is unchanged.

Resolved: owner check Q-187 PASS (Hiep, 2026-10-22).
