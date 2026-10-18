# TASK-251 Zoom the other plugin crates

Group: P (plugins thread-branch, workspace-spaces, next-prompt-actions, jev-orchestrator, skill-pins, edit-diffs, stop-orb, prompt-translate)
Class: code

## Brief

Goal: at zoom 150%, the paddings, gaps, icons, and fixed text sizes of the other GPUI plugin views grow with the text, as Q-151 answered.

Change: fixed `px(X)` sizes stay the same at every zoom -> they use `bachuc_theme::zoomed(X, cx)`, so they scale with Cmd+= and Cmd+-.

Boundaries:

- Q-151 (owner, 2026-10-18): convert with `zoomed`, not a gpui-pre scale patch.
- Follow the Zoom Conversion Rule in [watchtower/CONTEXT.md](watchtower/CONTEXT.md): what to convert, what keeps `px`, and no new constants.
- Count at plan time: about 289 `px(` sites in these crates, including tests and hairlines.
- Known fixed text sizes: `workspace-spaces/src/space_menu.rs` line 572 and `thread-branch/src/card.rs` line 306.
- Non-goal: no layout redesign; only the size source changes.

Files:

- [packages/desktop-gpui/plugins/thread-branch/src](packages/desktop-gpui/plugins/thread-branch/src)
- [packages/desktop-gpui/plugins/workspace-spaces/src](packages/desktop-gpui/plugins/workspace-spaces/src)
- [packages/desktop-gpui/plugins/next-prompt-actions/src](packages/desktop-gpui/plugins/next-prompt-actions/src)
- [packages/desktop-gpui/plugins/jev-orchestrator/src](packages/desktop-gpui/plugins/jev-orchestrator/src)
- [packages/desktop-gpui/plugins/skill-pins/src](packages/desktop-gpui/plugins/skill-pins/src)
- [packages/desktop-gpui/plugins/edit-diffs/src](packages/desktop-gpui/plugins/edit-diffs/src)
- [packages/desktop-gpui/plugins/stop-orb/src](packages/desktop-gpui/plugins/stop-orb/src)
- [packages/desktop-gpui/plugins/prompt-translate/src](packages/desktop-gpui/plugins/prompt-translate/src)

Interfaces:

- None. No Deps; shares no file with group T.

Expected result:

- At zoom 150%, every converted padding, icon, and control height is 1.5 times its 100% size, and no label clips.
- At zoom 100%, the screens look as before.

Anti-goal: at zoom 100%, a screenshot of each screen in Verify differs from the same shot taken before the change by 0 changed pixel rows outside of text cursors and timestamps; read by the UI checker before and after.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-thread-branch -p bachuc-plugin-workspace-spaces -p bachuc-plugin-next-prompt-actions -p bachuc-plugin-jev-orchestrator -p bachuc-plugin-skill-pins -p bachuc-plugin-edit-diffs -p bachuc-plugin-stop-orb -p bachuc-plugin-prompt-translate` -> pass, in one cargo call.
- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds; no new warnings in changed files.
- `rg -n "\bpx\(" packages/desktop-gpui/plugins/{thread-branch,workspace-spaces,next-prompt-actions,jev-orchestrator,skill-pins,edit-diffs,stop-orb,prompt-translate}/src` -> every remaining site is one of the kept cases of the Zoom Conversion Rule; the outcome lists the count per case.
- UI checker at `BACHUC_WINDOW=1200x800`, zoom 150%: the turn diff card, the Space menu, next-prompt action buttons, and skill pins -> paddings and icons are larger in proportion, no label clips, nothing overlaps.
- Anti-goal: the same screens at zoom 100%, before and after -> 0 changed pixel rows outside text cursors and timestamps.
