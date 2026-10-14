# TASK-101 Polish the page toward mock D v2

Group: G (writes client rail, Needs you, and tree view files; shared with TASK-102)
Class: code

## Brief

Goal: Close the visual gaps between the live page and mock D v2.

Change: The rail shows four count boxes, Needs you uses bordered buttons, done rows show the Group, and the tree spine is faint -> the rail shows one segment per TASK, Needs you uses text links, done rows show their commit, and the spine reads clearly.

Design: mock D v2, see [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- Rail progress: keep the percent and "N of M done"; replace the four count boxes with one bar split into one segment per TASK, colored done, running, waiting, or not started. Keep the counts as one short muted line under the bar.
- Needs you: actions become text links in the accent color, one row each ("Answer", "Use default", "View", "Accept", "Reject"). Keep every current action and the confirm step for Reject.
- Done rows: when `commitShas(run.log)` has a SHA for the TASK, the second line reads "committed <sha>". This works once TASK-100 lands; before that it keeps "Group A, code".
- Spine: use the theme border color at full strength and a short tick to each row.
- Use only theme colors and the existing `MONO` constant. No new dependency. Light theme only, per [hiep-plugins/AGENTS.md](hiep-plugins/AGENTS.md).

How:

- Edit the progress card, the Needs you cell, and the tree view.
- Check in live screenshots that nothing overlaps at 1440 px and 500 px.

Files:

- [hiep-plugins/plugins/watchtower-board/client/board-rail.tsx](hiep-plugins/plugins/watchtower-board/client/board-rail.tsx) (progress card)
- [hiep-plugins/plugins/watchtower-board/client/needs-you.tsx](hiep-plugins/plugins/watchtower-board/client/needs-you.tsx) (link actions)
- [hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx](hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx) (commit line, spine)

Expected result:

- The rail shows one segment per TASK and one count line.
- Needs you shows link actions; Reject still asks to confirm.
- A done row with a commit shows "committed <sha>".

Anti-goal: Actions keep working. Tripwire: `tests/actions.test.ts` and `tests/overview.test.ts` keep their pass counts; read before and after.

## Verify

- `cd hiep-plugins/plugins/watchtower-board && npm test` -> every test file passes.
- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck` -> exit 0.
- `cd hiep-plugins/plugins/watchtower-board && rtk proxy npm run lint` -> 0 warnings, 0 errors.
- `rg -n "#[0-9A-Fa-f]{6}" hiep-plugins/plugins/watchtower-board/client/board-rail.tsx hiep-plugins/plugins/watchtower-board/client/needs-you.tsx hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx` -> no new match.
- Needs a human or a browser session: live screenshot at 1440 px and 500 px.
- Anti-goal: the two test files' pass counts before and after are equal or higher.
