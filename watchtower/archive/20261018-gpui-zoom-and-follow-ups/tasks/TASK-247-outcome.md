# TASK-247 Outcome

## Status

DONE (code checks pass; UI checker lines DEFERRED).

## Changed

- `packages/desktop-gpui/crates/composer/src`: `agent_controls/control.rs`, `agent_controls/mod.rs`, `attachment_tray.rs`, `composer.rs`, `context_window_meter.rs`, `diff_stat_pill.rs`, `dock.rs`, `input.rs`, `pill_styles.rs`, `queue.rs`, `subagents_track.rs`, `task_list.rs`, `tracks.rs`. `plugin_slots.rs` untouched.
- `packages/desktop-gpui/crates/command/src`: `center.rs`, `shortcuts_dialog.rs`.
- `packages/desktop-gpui/crates/plugin-host/src`: `ui.rs`.

## Contract

- Fixed `px(X)` sizes became `bachuc_theme::zoomed(X, cx)`. At zoom 100% the value is unchanged.
- Added a `cx: &App` last parameter to crate-internal helpers: `round_button`, `attachment_tray`, `glyph_element`, `choice_pill`, `feature_toggle`, `features_menu`, `grouped_model_pill`, `popup_anchor`, `provider_icon` (private in `subagents_track.rs`). All callers are in the composer crate (GitNexus impact on `round_button`: MEDIUM, 9 callers, all in the crate; confirmed with `rg`).
- Public signatures `composer_dock`, `track_pill`, `task_pill`, `subagents_pill` are unchanged. `composer_dock` and `track_pill` now return a private `RenderOnce` wrapper that reads the zoom at render time (the app crate calls `composer_dock`; `task_list.rs` tests call `task_pill` without an `App`).
- `command/src/center.rs`: `input_height` and `header_height` take `zoom: f32` (font size is already zoomed; the SPACE paddings scale by `zoom`). `v_section` takes `cx`. Section `height` and `ROW_HEIGHT` from `search.rs` are zoomed at render. The two icons that used the default `icon()` size now call `.with_size(zoomed(ICON_MD, cx))` (`input.rs` round button, command item icon).
- `plugin-host/src/ui.rs`: paragraph gap is `rems(f32::from(zoomed(PARAGRAPH_GAP, cx)) / f32::from(rem_size(cx)))` as planned. `heading_style` takes `zoom: f32`.
- `context_window_meter.rs`: the canvas paint closure has no `App`, so it captures `zoom(cx)` and scales inline.

## Verified

Group-level runs (one call each, shared by TASK-247 only in this group):

- `./cargo.sh test -p bachuc-composer -p bachuc-command -p bachuc-plugin-host` -> exit 0; all tests pass (composer 124, command 43+5+10, plugin-host 12+13). The known timing test `search_500_items_stays_under_16_ms_per_keystroke` passed.
- `./cargo.sh build -p bachuc-app` -> exit 0. Two earlier tries failed only in files of groups A and E (`sidebar.rs`, `pane_appearance.rs`); a retry after 2 minutes passed. One warning, `next_theme` unused in `app/src/state.rs`, is not in a changed file.
- `./cargo.sh fmt --check -p bachuc-composer -p bachuc-command -p bachuc-plugin-host` -> exit 0 (no diff, including `plugin_slots.rs`). I formatted only my own files with `rustfmt --edition 2024`.
- `rg "(?<![\w.])px\(" ... --glob '!plugin_slots.rs'` -> 27 sites remain, all kept cases:
  - 17 already zoomed or derived from a zoomed value: `ui.rs` 4 (heading `top * zoom`), `context_window_meter.rs` 8 (paint closure `* zoom`, plus the arc points derived from the zoomed radius), `composer.rs` 2 (`input_line`, `min_h(px(height))`), center 3 (`header_height`, `input_height`, `input_line_height` of the zoomed font).
  - 2 hairlines (`px(1.)` divider in `center.rs`, `tracks.rs`).
  - 4 `INPUT_INSET` (`composer.rs`): offsets that cancel the fixed 4 px side padding of the gpui-kit `xsmall` text input, which does not scale.
  - 1 `px(OVERDRAW)` (`center.rs`): list pre-draw distance, not a visible size.
  - 3 test-code sites (`composer.rs` tests).
  - Count per case: already zoomed 17, hairline 2, INPUT_INSET 4, OVERDRAW 1, test 3 (= 27).
- Every other `px(` site (about 260) now uses `zoomed`.

## Anti-goal

DEFERRED (UI checker): the 100% before/after screenshots of the composer with a model pill and slash picker, and of the Command Center. By construction `zoomed(X, cx) == px(X)` at zoom 100%; `input_height` and `header_height` equal the old values at `zoom = 1` (the existing 25 and 50 tests pass with `1.`).

UI checker line at 150% also DEFERRED (UI checker).

## Lessons

- `bachuc_icons::icon(name)` has a fixed default size `px(ICON_MD)`. An icon with no `.with_size(zoomed(..))` does not scale; add `Sizable as _` to use `with_size`.
- `INPUT_INSET` (composer) assumes the gpui-kit xsmall input padding is a fixed 4 px at every zoom. The UI checker should look at the input text alignment at 150%; if the text edge drifts, scale the inset.
- `resolve_density` and the width constants in `agent_controls/layout.rs` are used only by tests, not by the composer; the toolbar density is not driven by width, so zoom does not affect it.
- Wrapping a builder (`Popover`, dock layout) in a private `RenderOnce` struct is the way to read zoom without changing a public signature.
- `sed -i` on macOS needs `-i ''`.

## UI Checks

- UI checker 1200x800 zoom 150%: composer with model pill, slash picker, and Command Center -> UNVERIFIED (unattended): background frame
- Extra (a): at zoom 150% the composer input text edge lines up with the pill row (INPUT_INSET) -> UNVERIFIED (unattended): background frame
- Anti-goal: zoom 100%, 0 changed pixel rows before vs after -> UNVERIFIED (unattended): background frame

Evidence: /tmp/wt-ui/zoom/before-initial-1200.png and before-initial-1200b.png (8 s apart), before-key-try.png, before-key-try2.png: the 1200x800 window still shows "Connecting / live" while the app holds an ESTABLISHED socket to 127.0.0.1:6768. GPUI does not redraw a background window (Q-069), so no frame shows thread, composer, or zoom. A Cmd+= and a Cmd+- key reached the app (appearance.json changed 1.25 -> 1.5 -> 1.25) but the window did not redraw. The check bundle was built (exit 0); a baseline copy is at `Bachuc GPUI Zoom Before.app`. The checker did not use the front, the pointer, or AX clicks.

By reading only (not a check result): `INPUT_INSET` is a fixed `4.` used as `px(INPUT_INSET)` (`crates/composer/src/composer.rs:94`, `2654`, `2677`), so it assumes the gpui-kit input padding stays 4 px at every zoom. A screenshot is still needed at 150%.

## UI Checks

Run 2026-10-19 on live with the check bundle; screenshots in `/tmp/wt-ui/zoom/`.

- Zoom 150%: the toolbar density used 100% floors on zoomed widths and clipped the mode pill. Fixed in `132164b` (`after-150-1200-t244-fix.png`).
- Slash picker and Command Center at 150% -> PASS (`after-150-1200-slash-fix.png`, `after-150-1200-command.png`).
- INPUT_INSET at 150%: input text edge lines up with the pill row -> PASS.
- Q-158: the stop orb grows with the composer at 150% -> PASS (`after-150-1200-longrun.png`).
- Pre-existing, also at 100%: with the send button the toolbar can overflow up to 12 px before it compacts (Q-162); the "⌘L to focus" hint overlaps the placeholder in a narrow composer (Q-163).
