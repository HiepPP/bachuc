# NEXT

## Current Active Plan

- Title: App-wide active machine switch
- Slug: 20260930-active-machine-switch
- Status: ARCHIVED
- Updated: 2026-09-30

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                                  | Group | Status | Spec                                                                                                       | Deps                                                       | Context                                        | Notes                                       |
| ----- | ----------------------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------- | ---------------------------------------------- | ------------------------------------------- |
| 1     | TASK-001 Active host store and visible hosts hook     | A     | DONE   | [watchtower/tasks/TASK-001-active-host-store.md](watchtower/tasks/TASK-001-active-host-store.md)           | -                                                          | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Base for every other TASK.                  |
| 2     | TASK-002 Machine switch in the sidebar                | B     | DONE   | [watchtower/tasks/TASK-002-machine-switch-ui.md](watchtower/tasks/TASK-002-machine-switch-ui.md)           | TASK-001                                                   | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares left-sidebar.tsx with TASK-003.      |
| 3     | TASK-003 Sidebar follows the active machine           | B     | DONE   | [watchtower/tasks/TASK-003-sidebar-follows-active.md](watchtower/tasks/TASK-003-sidebar-follows-active.md) | TASK-001                                                   | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares left-sidebar.tsx with TASK-002.      |
| 4     | TASK-004 Aggregated screens follow the active machine | C     | DONE   | [watchtower/tasks/TASK-004-aggregated-screens.md](watchtower/tasks/TASK-004-aggregated-screens.md)         | TASK-001                                                   | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Command center, schedules, history, labels. |
| 5     | TASK-005 Creation defaults use the active machine     | D     | DONE   | [watchtower/tasks/TASK-005-creation-defaults.md](watchtower/tasks/TASK-005-creation-defaults.md)           | TASK-001                                                   | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Landing, new workspace, host chooser.       |
| 6     | TASK-006 Plugin pages follow the active machine       | E     | DONE   | [watchtower/tasks/TASK-006-plugin-pages.md](watchtower/tasks/TASK-006-plugin-pages.md)                     | TASK-001                                                   | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Board uses the route host.                  |
| 7     | TASK-007 Build the Paseo Dev app                      | F     | DONE   | [watchtower/tasks/TASK-007-build-paseo-dev.md](watchtower/tasks/TASK-007-build-paseo-dev.md)               | TASK-001, TASK-002, TASK-003, TASK-004, TASK-005, TASK-006 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Never touch the 6767 daemon.                |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed files>` -> exit 0.
- `npx vitest run packages/app/src/stores/active-host-store.test.ts packages/app/src/stores/sidebar-view-store.test.ts packages/app/src/screens/new-workspace-initial-context.test.ts packages/app/src/plugins/host-navigation.test.ts packages/app/src/hooks/use-projects.test.ts packages/app/src/hooks/use-schedules.test.ts --bail=1` -> all pass.
- `rg -c "useHosts\(" packages/app/src/screens/settings-screen.tsx packages/app/src/screens/settings/host-page.tsx "packages/app/src/app/settings/[section].tsx" packages/app/src/components/add-host-modal.tsx packages/app/src/components/pair-link-modal.tsx packages/app/src/components/add-remote-ssh-host-modal.tsx` -> total 8, so Settings and pairing still see every host.
- Manual check in the Paseo Dev app (human): pick one machine, then sidebar, Search, Schedules, History, New workspace, and Board show only that machine.

## Handoff

- All 7 TASKs are DONE. Verify ran on 2026-09-30T17:40: typecheck exit 0, lint 0 errors, 8 test files with 81 passing tests, and anti-goal reads within their limits.
- The Paseo Dev app is at `packages/desktop/release/mac-arm64/Paseo.app`. The macmini host was added to it over SSH (`ssh://macmini`).
- The manual check covered the switch, sidebar, Search, History, New workspace, Board, the attention badge, and persistence after restart. All matched the plan.
- Not verified by UI: Schedules, because neither host has an active schedule. Q4 (a deep link to another host switches the active host) was not tested manually.
- The workspace-spaces plugin (external) filters sidebar projects by Space. An empty sidebar can come from the Space, not the host switch.
- Follow-up on 2026-09-30: `Cmd+`` (mac only, action `host.cycle`) switches to the next host, local host first. `Ctrl+`` stays on the Explorer sidebar. macOS reserves `Cmd+`` for "Move focus to next window" (symbolic hotkey 27). The user had it turned off in System Settings, and the app must restart after that change. Checked in Paseo Dev: MacBook, then macmini, then MacBook.
- Follow-up on 2026-09-30: switching the host keeps the page. See `resolveHostSwitchRoute` in [packages/app/src/hosts/host-switch-route.ts](packages/app/src/hosts/host-switch-route.ts). A plugin page opens the same contribution on the new host, and History, Schedules, New, and Settings stay. Checked in Paseo Dev: Board went from `srv_qcIO` to `srv_A1fy` and back, and History stayed on `/sessions`.
- Follow-up on 2026-09-30: each host remembers its last screen in `lastRouteByServerId`. Switching goes to the target's last screen, then to the same plugin page, then to `/h/<id>/open-project`. It never goes to the bare `/h/<id>` from another host's page, because that returned to the old host and made the switch look like it did nothing. Checked in Paseo Dev: MacBook thread `wks_7898…`, then macmini thread `wks_42d0…`, then back, each time landing on the same thread.
- Follow-up on 2026-09-30: switching hosts used to push a new root `h/[serverId]` route every time (`router.navigate`), which cold-mounted the UI. The fix reuses the root host route. Workspace targets go through `navigateToWorkspace` and `POP_TO` (`dispatchHostWorkspacePopTo` now handles a focused route on a different host); other targets go through `router.dismissTo`. The existing `WorkspaceDeck` keeps recent workspaces mounted.
- Measured in the renderer over CDP (`/tmp/cdp-switch-bench.mjs`): cold 577 ms, warm median 119-121 ms. Renderer RSS peak during rapid switching was 384-502 MB (before: 1392 MB). Idle RSS was 163-289 MB (before: 536-643 MB).
- Follow-up on 2026-09-30: on native, the sidebar also keeps directory demand for the previous host (`previousServerId`, session-only; `resolveSidebarDemandServerIds`). Demand is now acquired by diff (`diffDemandServerIds`): new hosts first, released hosts after. A host that stays in the set is never released, because `setDemand` drops subscriptions as soon as the count reaches 0. Covered by unit tests only; not yet run on a device.
- Next action: commit and push the app changes when the user asks. Keep `watchtower/` out of the commit unless requested.

## Archive

- Archived: 2026-09-30 -> watchtower/archive/20260930-active-machine-switch/
