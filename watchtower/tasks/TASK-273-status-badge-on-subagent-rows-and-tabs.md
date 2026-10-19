# TASK-273 Status badge on subagent rows and tabs

Group: C (the `bachuc-sidebar`, `bachuc-composer`, and `bachuc-workspace` crates and the app shell; shares files with TASK-268 and TASK-272)
Class: code

## Brief

Goal: A running subagent row and a running agent tab show the turning ring on their provider icon, and other states show their dot or alert, as Electron's `WorkspaceTabIcon` does.

Change: a bare provider icon on subagent rows and tabs -> the icon with a status badge.

Design: `WorkspaceTabIcon` in [packages/app/src/screens/workspace/workspace-tab-presentation.tsx](packages/app/src/screens/workspace/workspace-tab-presentation.tsx) (lines 136-187, styles at 268-290), used by subagent rows (`packages/app/src/subagents/track.tsx:209`) and desktop tabs (`packages/app/src/screens/workspace/workspace-desktop-tabs-row.tsx:680`).

Boundaries:

- Badge by bucket, from `WorkspaceTabIcon`: `running` draws `StatusRing` at the bottom right; `failed` and `attention` draw a dot with a 1 px border; `needs_input` draws a `CircleAlert`; `done` draws nothing. Copy the offsets and sizes from the Electron constants (`DEFAULT_STATUS_DOT_OFFSET`, `STATUS_INDICATOR_DOT_SIZE`, `getStatusRingOffset`, `STATUS_ALERT_OFFSET`, `STATUS_INDICATOR_ALERT_SIZE`).
- The ring is the sidebar `RunningRing` ([packages/desktop-gpui/crates/sidebar/src/ring.rs](packages/desktop-gpui/crates/sidebar/src/ring.rs)), which TASK-268 exports. Electron passes a `backdrop` (`surface1` or `surface2`) for the knockout. `Knockout` has only `None` and `Sidebar` and derives `Eq` (`ring.rs:38`). Add token variants such as `Surface1` and `Surface2`, read at paint as `Sidebar` reads `t.surface_sidebar` (`ring.rs:104`). A stored `Hsla` is not `Eq` and goes stale on a theme change.
- Electron constants: `DEFAULT_STATUS_DOT_OFFSET = -2` and `STATUS_ALERT_OFFSET = -3` (`workspace-tab-presentation.tsx:34-35`), dot size 8, alert size 12, and the ring offset from `getStatusRingOffset` (`packages/app/src/components/status-ring/geometry.ts:47`).
- Put the badge in one shared view in `bachuc-sidebar`, next to `RunningRing`, so rows and tabs draw the same thing. `bachuc-composer` and `bachuc-app` already depend on `bachuc-sidebar`; `bachuc-workspace` does not.
- Tabs: GPUI tabs already get the provider logo through `set_item_logo` (`packages/desktop-gpui/crates/app/src/shell.rs:1115`; `TabExtras` at `packages/desktop-gpui/crates/workspace/src/tab_menu.rs:221`). Add a badge slot to `TabExtras` the same way, as an `AnyView` the shell sets per tab key, so `bachuc-workspace` needs no new dependency. `render_tab` draws the glyph at `workspace.rs:1020`. The drag chip uses `tab_glyph` too (`workspace.rs:105`); draw the badge there as Electron's drag overlay does.
- The shell has no per-tab bucket today. Compute it with `bachuc_sidebar::model::bucket_for_agent` (`sidebar/src/model.rs:240`) from the agent's status, permissions, and attention (`Client` attention at `core.rs:606`), and update the badge when the bucket changes.
- Rows: subagent rows draw `provider_icon` (`packages/desktop-gpui/crates/composer/src/subagents_track.rs:163-174`). Add the badge from `row_bucket(row)` (`subagents_track.rs:43`).
- Keep one badge entity per row key and per tab key, cached as `Sidebar::ring` does, so a ring does not restart its clock on every render.
- Follows [ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md): use the theme tokens and lucide icons.

How:

- Add the fill knockout and the shared badge view to `bachuc-sidebar`.
- Draw the badge on subagent rows in the composer.
- Add the workspace badge slot and draw it on the tab icon.
- Set and update each agent tab's badge from the shell.

Files:

- [packages/desktop-gpui/crates/sidebar/src/ring.rs](packages/desktop-gpui/crates/sidebar/src/ring.rs) (fill knockout)
- [packages/desktop-gpui/crates/sidebar/src/lib.rs](packages/desktop-gpui/crates/sidebar/src/lib.rs) (badge view export)
- [packages/desktop-gpui/crates/sidebar/src/badge.rs](packages/desktop-gpui/crates/sidebar/src/badge.rs) (new: the shared status badge view)
- [packages/desktop-gpui/crates/composer/src/subagents_track.rs](packages/desktop-gpui/crates/composer/src/subagents_track.rs) (badge on rows)
- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (badge cache by row key)
- [packages/desktop-gpui/crates/workspace/src/tab_menu.rs](packages/desktop-gpui/crates/workspace/src/tab_menu.rs) (badge slot by key)
- [packages/desktop-gpui/crates/workspace/src/workspace.rs](packages/desktop-gpui/crates/workspace/src/workspace.rs) (`render_tab` draws the badge)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (set and update tab badges)

Expected result:

- A running Claude subagent row shows its logo with a turning ring at the bottom right.
- An agent tab shows the same ring while its agent runs, and the ring goes when the agent stops.
- A failed agent tab shows the red dot; an agent that waits for input shows the alert.
- Cases:
  - A row or tab in the `done` bucket -> logo only.
  - A theme change -> the badge colors follow the theme.

Anti-goal: GPUI app CPU with two running agents in open tabs and one running subagent row shown stays at or below the baseline plus 2 points over 60 s; drift gauge; read with `packages/desktop-gpui/measure.sh 60 gpui=<check bundle binary>` before changes and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar -p bachuc-composer -p bachuc-workspace -p bachuc-app` -> every test passes.
- Pure test in `badge.rs`: bucket to badge -> `running` gives the ring, `failed` and `attention` give a dot, `needs_input` gives the alert, `done` gives none.
- Test in `crates/workspace/tests/tab_menu.rs`: set, replace, and clear a badge by key, as `set_item_logo_sets_replaces_clears_and_forgets` (`tests/tab_menu.rs:334`) does.
- `cd packages/desktop-gpui && ./cargo.sh build` -> the workspace builds.
- UI checker on live (screen check): start a live agent with a running Agent tool subagent. Raise the GPUI window and take two screenshots about 200 ms apart. Expect a turning ring on the subagent row icon and on the agent's tab, at a different angle in each shot.
- UI checker on live (screen check): after the agent stops -> the tab shows the logo only.
- Anti-goal: `measure.sh 60` before and after, with the same agents running -> after is at most baseline + 2 points; record both values.
