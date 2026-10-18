# TASK-250 Zoom board and watchtower-board

Group: B (plugins `board` and `watchtower-board`)
Class: code

## Brief

Goal: at zoom 150%, the paddings, gaps, icons, and fixed text sizes of the Board and Watchtower board plugin views grow with the text, as Q-151 answered.

Change: fixed `px(X)` sizes stay the same at every zoom -> they use `bachuc_theme::zoomed(X, cx)`, so they scale with Cmd+= and Cmd+-.

Boundaries:

- Q-151 (owner, 2026-10-18): convert with `zoomed`, not a gpui-pre scale patch.
- Follow the Zoom Conversion Rule in [watchtower/CONTEXT.md](watchtower/CONTEXT.md): what to convert, what keeps `px`, and no new constants.
- Count at plan time: about 321 `px(` sites in these crates, including tests and hairlines.
- `board/src/header_chip.rs` line 99 is a known fixed `text_size`.
- Non-goal: no layout redesign; only the size source changes.

Files:

- [packages/desktop-gpui/plugins/board/src](packages/desktop-gpui/plugins/board/src) (not `pills.rs` state logic; only sizes)
- [packages/desktop-gpui/plugins/watchtower-board/src](packages/desktop-gpui/plugins/watchtower-board/src)

Interfaces:

- None. No Deps; shares no file with group T.

Expected result:

- At zoom 150%, every converted padding, icon, and control height is 1.5 times its 100% size, and no label clips.
- At zoom 100%, the screens look as before.

Anti-goal: at zoom 100%, a screenshot of each screen in Verify differs from the same shot taken before the change by 0 changed pixel rows outside of text cursors and timestamps; read by the UI checker before and after.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-board -p bachuc-plugin-watchtower-board` -> pass, in one cargo call.
- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds; no new warnings in changed files.
- `rg -n "\bpx\(" packages/desktop-gpui/plugins/board/src packages/desktop-gpui/plugins/watchtower-board/src` -> every remaining site is one of the kept cases of the Zoom Conversion Rule; the outcome lists the count per case.
- UI checker at `BACHUC_WINDOW=1200x800`, zoom 150%: the Board with runs and the Watchtower board with a TASK dialog open -> paddings and icons are larger in proportion, no label clips, nothing overlaps.
- Anti-goal: the same screens at zoom 100%, before and after -> 0 changed pixel rows outside text cursors and timestamps.
