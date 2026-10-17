# TASK-163 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/composer/src/features.rs` (new): pure feature and diff-row logic with 11 tests.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: `Watched` gains `features` and `diff_stat`; feature controls after the mode pill; the diff-count row above the input; `bachuc_icons::provider_logo` with `provider_fallback_icon`; `choice_pill` takes an `ElementId`, an owned kind, and an optional tooltip; `Composer::send_feature`.
- `packages/desktop-gpui/crates/composer/src/lib.rs`: `mod features`.
- `packages/desktop-gpui/crates/composer/assets/claude.svg`: deleted, with its `include_str!` loader, `filled_svg`, and the `logo` field.
- `Cargo.toml` and `examples/composer.rs` are unchanged.

## Contract

- `Composer::new`, `agent_id`, `focus`, `attachment_count`, and `ComposerEvent` are unchanged.
- Features draw in the daemon order after the mode pill. A toggle is a 28 px icon button with a tooltip (`tooltip`, else `label`); a select is a pill with a menu of its options.
- Icons: `list-todo`, `shield-check`, `zap`; any other name or none gives `Settings2`.
- A click on a toggle sends one `set_agent_feature` with the opposite of the daemon value. A select pick sends the option id. `FeatureRequests` drops a click while a request for the same feature is in flight.
- The view never holds an optimistic value. After each answer (success or failure) it reads the features again, so a failed request shows the daemon value.
- A select with a null value, or an id its options lack, shows the feature label.
- The diff row reads `workspace_diff_stat` of the agent's `workspace_id`. It hides for `None` and for a stat of 0 and 0 (Electron `hasDiffStatChanges`). Counts use `formatDiffCount` notation (`1.2k`).
- `Watched` is compared on every `AgentsChanged` and `Connected`; the composer redraws only when it differs.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> 37 passed, 0 failed.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0.
- Drawing the lightning button, the select menu, the diff row, and the logos in a window, and a real click on live: UNVERIFIED (unattended): code tests only.

## Anti-goal

- Checkpoint: `features::tests::one_click_sends_one_request` -> pass, `cargo test -p bachuc-composer`, 2026-10-16.
- Checkpoint: `composer::tests::an_unchanged_snapshot_causes_no_redraw` -> pass, same command, 2026-10-16.
- Limit: both tests cover the pure guard and the `Watched` comparison. Nothing drives `Composer` in a GPUI test context, so the wiring of `send_feature` and `cx.notify` is proven by reading the code only.

## Lessons

- `Composer` has no GPUI test context here (the `test-support` feature breaks workspace resolution), so the tests target plain functions: `FeatureRequests`, `toggle_request`, `select_request`, `diff_row`, `provider_glyph`, and `Watched` equality.
- `bachuc_icons::provider_logo` caches per provider and color, so the composer no longer keeps its own logo.

## Questions

- Safe default: the Electron highlight colors `palette.yellow/green/blue[400]` have no theme token, and `crates/theme` is another group's. Fast mode draws `status_warning`, auto accept `status_success`, plan mode `accent`. Replace them when the theme crate exposes the palette.
- Safe default: the diff row is not clickable, because `ComposerEvent` must stay unchanged and cannot ask the owner to open the Changes tab.
- Safe default: a select option with an unknown daemon value shows the feature label, as Electron does.
