# TASK-247 Zoom composer, command, plugin host

Group: C (composer, command, plugin-host crates, except `plugin_slots.rs`)
Class: code

## Brief

Goal: at zoom 150%, the paddings, gaps, icons, and fixed text sizes of the composer, the Command Center, and the plugin host views grow with the text, as Q-151 answered.

Change: fixed `px(X)` sizes stay the same at every zoom -> they use `bachuc_theme::zoomed(X, cx)`, so they scale with Cmd+= and Cmd+-.

Boundaries:

- Q-151 (owner, 2026-10-18): convert with `zoomed`, not a gpui-pre scale patch.
- Follow the Zoom Conversion Rule in [watchtower/CONTEXT.md](watchtower/CONTEXT.md): what to convert, what keeps `px`, and no new constants.
- Count at plan time: about 287 (309 minus the 22 of `plugin_slots.rs`) `px(` sites in these crates, including tests and hairlines.
- Do not edit [plugin_slots.rs](packages/desktop-gpui/crates/composer/src/plugin_slots.rs); TASK-245 owns it.
- `window_rems` in the timeline is private. Planner call: in `plugin-host/src/ui.rs:355`, replace `rems(PARAGRAPH_GAP / FONT_BASE)` with `rems(f32::from(zoomed(PARAGRAPH_GAP, cx)) / f32::from(rem_size(cx)))` inline; do not touch the theme crate.
- Non-goal: no layout redesign; only the size source changes.

Files:

- [packages/desktop-gpui/crates/composer/src](packages/desktop-gpui/crates/composer/src) (all view files except `plugin_slots.rs`)
- [packages/desktop-gpui/crates/command/src](packages/desktop-gpui/crates/command/src)
- [packages/desktop-gpui/crates/plugin-host/src](packages/desktop-gpui/crates/plugin-host/src)

Interfaces:

- None. No Deps; shares no file with group T.

Expected result:

- At zoom 150%, every converted padding, icon, and control height is 1.5 times its 100% size, and no label clips.
- At zoom 100%, the screens look as before.

Anti-goal: at zoom 100%, a screenshot of each screen in Verify differs from the same shot taken before the change by 0 changed pixel rows outside of text cursors and timestamps; read by the UI checker before and after.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer -p bachuc-command -p bachuc-plugin-host` -> pass, in one cargo call.
- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds; no new warnings in changed files.
- `rg -n "\bpx\(" packages/desktop-gpui/crates/composer/src packages/desktop-gpui/crates/command/src packages/desktop-gpui/crates/plugin-host/src` -> every remaining site is one of the kept cases of the Zoom Conversion Rule; the outcome lists the count per case.
- UI checker at `BACHUC_WINDOW=1200x800`, zoom 150%: the composer with a model pill and the slash picker open, and the Command Center -> paddings and icons are larger in proportion, no label clips, nothing overlaps.
- Anti-goal: the same screens at zoom 100%, before and after -> 0 changed pixel rows outside text cursors and timestamps.
