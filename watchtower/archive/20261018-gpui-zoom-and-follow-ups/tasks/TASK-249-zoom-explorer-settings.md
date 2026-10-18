# TASK-249 Zoom explorer, terminal, history, settings

Group: E (explorer, terminal, history, settings crates)
Class: code

## Brief

Goal: at zoom 150%, the paddings, gaps, icons, and fixed text sizes of the file explorer, terminal chrome, history, and settings grow with the text, as Q-151 answered.

Change: fixed `px(X)` sizes stay the same at every zoom -> they use `bachuc_theme::zoomed(X, cx)`, so they scale with Cmd+= and Cmd+-.

Boundaries:

- Q-151 (owner, 2026-10-18): convert with `zoomed`, not a gpui-pre scale patch.
- Follow the Zoom Conversion Rule in [watchtower/CONTEXT.md](watchtower/CONTEXT.md): what to convert, what keeps `px`, and no new constants.
- Count at plan time: about 502 `px(` sites in these crates, including tests and hairlines.
- The terminal grid font is already zoomed: `typeface.size` comes from `code_font_size` (`terminal/src/view.rs:122`), so `text_size(px(self.typeface.size))` at line 1068 and the line height after it keep `px`. Change only paddings and chrome.
- Non-goal: no layout redesign; only the size source changes.

Files:

- [packages/desktop-gpui/crates/explorer/src](packages/desktop-gpui/crates/explorer/src)
- [packages/desktop-gpui/crates/terminal/src](packages/desktop-gpui/crates/terminal/src)
- [packages/desktop-gpui/crates/history/src](packages/desktop-gpui/crates/history/src)
- [packages/desktop-gpui/crates/settings/src](packages/desktop-gpui/crates/settings/src)

Interfaces:

- None. No Deps; shares no file with group T.

Expected result:

- At zoom 150%, every converted padding, icon, and control height is 1.5 times its 100% size, and no label clips.
- At zoom 100%, the screens look as before.

Anti-goal: at zoom 100%, a screenshot of each screen in Verify differs from the same shot taken before the change by 0 changed pixel rows outside of text cursors and timestamps; read by the UI checker before and after.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer -p bachuc-terminal -p bachuc-history -p bachuc-settings` -> pass, in one cargo call.
- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds; no new warnings in changed files.
- `rg -n "\bpx\(" packages/desktop-gpui/crates/explorer/src packages/desktop-gpui/crates/terminal/src packages/desktop-gpui/crates/history/src packages/desktop-gpui/crates/settings/src` -> every remaining site is one of the kept cases of the Zoom Conversion Rule; the outcome lists the count per case.
- UI checker at `BACHUC_WINDOW=1200x800`, zoom 150%: the explorer tree and a diff, a terminal, history, and the Appearance and Projects settings panes -> paddings and icons are larger in proportion, no label clips, nothing overlaps.
- Anti-goal: the same screens at zoom 100%, before and after -> 0 changed pixel rows outside text cursors and timestamps.
