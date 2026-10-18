# TASK-251 Outcome

## Status

DONE (code). UI lines are DEFERRED (UI checker).

## Changed

- 280 `px(X)` sites became `zoomed(X, cx)` in `thread-branch` (`card.rs`, `panel.rs`, `pills.rs`), `workspace-spaces` (`page.rs`, `space_menu.rs`, `strip.rs`), `next-prompt-actions` (`panel.rs`), `jev-orchestrator` (`direct_panel.rs`, `jobs_panel.rs`, `job_item.rs`), `skill-pins` (`menu.rs`), `edit-diffs` (`row.rs`), `prompt-translate` (`note_view.rs`). `stop-orb` has no change.
- Every plugin crate already depended on `bachuc-theme`; no `Cargo.toml` changed.
- Both known fixed text sizes are converted: `space_menu.rs` `action_button` and `card.rs` `BADGE_TEXT`.

## Contract

- No public symbol changed. Private helpers took a `cx: &App` argument: `thread-branch` `diff_row`, `workspace-spaces` `actions` and `action_button`, `jev-orchestrator` `v_stack`, `outcome_card` and `attempt_card`.
- `workspace-spaces` `space_menu.rs`: the menu position stays in layout pixels. `menu_left` and `menu_bottom` are 100% rules, so the caller divides the layout values by the zoom and multiplies the result back.
- `next-prompt-actions` `panel.rs`: the measured panel width is divided by the zoom before the 600 compact check, because the threshold is in 100% units.
- `thread-branch` `panel.rs`: the diff gutter width stays computed from the base code size; `diff_row` zooms it.

## Verified

- Group-level run, shared by the whole group: `./cargo.sh test` on the 8 plugin packages in one call -> exit 0, all suites ok.
- Group-level run: `./cargo.sh build -p bachuc-app` -> exit 0; one warning (`next_theme` unused in `crates/app/src/state.rs`), not in a changed file.
- `./cargo.sh fmt` then `fmt --check` on the 8 plugin packages -> exit 0.
- Remaining `px(` in the 8 `src` folders: 12 sites.
  - 4 in `stop-orb/src/face.rs`: dot offsets and radii derived from `props.size` and `props.icon_size`, which the host passes already zoomed.
  - 4 `ListState::new` overdraw distances (`thread-branch` `panel.rs`, `jev-orchestrator` `direct_panel.rs` and `jobs_panel.rs`, `workspace-spaces` `page.rs`).
  - 3 hairlines `px(1.)` (`skill-pins` `menu.rs`, `thread-branch` `pills.rs`, `workspace-spaces` `space_menu.rs`).
  - 1 position read from layout: `space_menu.rs` `point(px(left), px(bottom))`.

## Anti-goal

DEFERRED (UI checker): zoom 100% screenshots before and after. `zoomed(X, cx) == px(X)` at 100%, so no change is expected.

DEFERRED (UI checker): zoom 150% for the turn diff card, the Space menu, next-prompt action buttons, and skill pins.

## Lessons

- A `move` closure that captures `cx: &mut Context<Self>` blocks later uses of `cx`. Compute the zoomed `Pixels` before the closure.
- A Verify grep for `px(` must skip `.px(` method calls; a plain `\bpx\(` matches them.

## UI Checks

- UI checker 1200x800 zoom 150%: turn diff card, Space menu, next-prompt buttons, skill pins -> UNVERIFIED (unattended): background frame
- Extra (b), Q-158: at zoom 150% the stop-orb grows with the composer -> UNVERIFIED (unattended): background frame
- Plan Verify: 1200x800 and 1600x900, zoom 100% and 150%; thread, composer, sidebar, explorer, settings, Board, Watchtower board -> UNVERIFIED (unattended): background frame
- Anti-goal: zoom 100%, 0 changed pixel rows before vs after -> UNVERIFIED (unattended): background frame

Evidence: /tmp/wt-ui/zoom/before-initial-1200.png and before-initial-1200b.png (8 s apart), before-key-try.png, before-key-try2.png: the 1200x800 window still shows "Connecting / live" while the app holds an ESTABLISHED socket to 127.0.0.1:6768. GPUI does not redraw a background window (Q-069), so no frame shows thread, composer, or zoom. A Cmd+= and a Cmd+- key reached the app (appearance.json changed 1.25 -> 1.5 -> 1.25) but the window did not redraw. The check bundle was built (exit 0); a baseline copy is at `Bachuc GPUI Zoom Before.app`. The checker did not use the front, the pointer, or AX clicks.

By reading only (not a check result): the composer passes `size: zoomed(CONTROL, cx)` and `icon_size: zoomed(TOOLBAR_GLYPH, cx)` to `StopButtonProps` for a plugin stop face (`crates/composer/src/composer.rs:2835`). Both grow with zoom, so a plugin orb should scale. The default red face uses `size_full()` in its slot. Q-158 stays open until a 150% screenshot.

Blocked: BUILT, UNVERIFIED (Q-161)
