# TASK-241 Turn diff recap shows on each response

Group: D ([packages/desktop-gpui/plugins/thread-branch](packages/desktop-gpui/plugins/thread-branch), [hiep-plugins/plugins/thread-branch](hiep-plugins/plugins/thread-branch))
Class: code

## Brief

Goal: After each agent response that changed files, GPUI shows the turn diff recap card, like Electron.

Change: no card in GPUI -> card on each response with changes.

Boundaries:

- Owner reports the card does not show at all in GPUI (2026-10-18).
- The server part appends a plugin timeline item (`TURN_DIFF_KIND`, [hiep-plugins/plugins/thread-branch/index.server.ts](hiep-plugins/plugins/thread-branch/index.server.ts) about line 72). The GPUI crate registers a renderer with `add_timeline_renderer` in [packages/desktop-gpui/plugins/thread-branch/src/lib.rs](packages/desktop-gpui/plugins/thread-branch/src/lib.rs) about line 90. Host side: [packages/desktop-gpui/crates/plugin-host/src/contributions.rs](packages/desktop-gpui/crates/plugin-host/src/contributions.rs) and [packages/desktop-gpui/crates/timeline/src/plugins.rs](packages/desktop-gpui/crates/timeline/src/plugins.rs).
- The kind string already matches: GPUI `ITEM_KIND` (lib.rs line 31) equals the server `TURN_DIFF_KIND` (`shared/turn-diff.ts` line 4). Find where the chain breaks: plugin not enabled on the host, item not sent or not delivered to GPUI, transformer or renderer not registered in the app, or the row not drawn. Classify the failure before fixing (CLAUDE.md global rules).
- Fix in the layer that breaks. If it is the server part, edit [hiep-plugins/plugins/thread-branch](hiep-plugins/plugins/thread-branch) and reload it on live with `npm run cli -- plugin reload thread-branch`. Do not reload release.

How:

- Trace one turn with a file change on live: daemon log, the timeline item, and the GPUI renderer match.
- Fix the broken link and add a test at that layer.

Files:

- [packages/desktop-gpui/plugins/thread-branch/src](packages/desktop-gpui/plugins/thread-branch/src) (renderer match or card)
- [hiep-plugins/plugins/thread-branch](hiep-plugins/plugins/thread-branch) (only if the server part is the cause)

Expected result:

- A response that edits a file shows the turn diff card under it, with the same files and counts as Electron.
- A response with no file change shows no card.

Anti-goal: zero extra cards; a card on a turn with no change is the tripwire; read in the UI checker run.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-thread-branch` -> all pass, including a test for the broken link.
- If the server part changed: run its tests in [hiep-plugins/plugins/thread-branch/tests](hiep-plugins/plugins/thread-branch/tests) -> pass.
- UI checker on live: send a prompt that edits one file -> card shows with that file; send a prompt with no edit -> no card.
