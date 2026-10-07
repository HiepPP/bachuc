# TASK-007 Overview screen

Group: F (standalone: `client/overview.tsx`)
Class: code

## Brief

Goal: Compose the full overview screen: the header, the lifecycle stepper, and a bento grid with the timeline, Needs you, the run log, manual checks, and decisions and history.

Change: no screen -> a `WatchtowerOverview` component that reads `watchtower.overview.read` once and renders every part.

Design: option 3f in the mock named in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- One `useQuery` for the overview, keyed by host and workspace. No `refetchInterval`. The Refresh button calls `refetch`.
- Run `lifecyclePhases`, `needsYou`, and `buildTimeline` in `useMemo` on the query data. Read the data for real in the memo body; see the React Compiler note in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).
- Keep the screen inside a vertical `ScrollView`, so a short window can still reach every cell.

How:

- Props: `PluginSurfaceProps` plus `workspaceId` and `projectName`.
- Header: plan title, then `"<slug>, updated <date>"`, then a Refresh icon button on the right.
- Grid: row one is the timeline cell (two columns wide) and Needs you. Row two is the run log, manual checks, and decisions and history. Below 1000 px of width, use two columns. The timeline cell stays full width.
- Empty state, when the overview has a `message` and no tasks: show "No plan yet", the message, Needs you (ADRs can still exist), and decisions and history.
- Loading state: the header and cell outlines with muted placeholder bars. Error state: an inline alert with the error text and a Retry button.

Files:

- [hiep-plugins/plugins/watchtower-board/client/overview.tsx](hiep-plugins/plugins/watchtower-board/client/overview.tsx) (new)

Expected result:

- With a plan, the screen shows all six parts with data from one RPC call.
- With no `NEXT.md`, the screen shows the empty state, not an error.
- Refresh refetches the overview once.
- The visible result is checked on live in Plan Verify, after TASK-008.

Anti-goal: one data query per open: `useQuery` calls in `client/overview.tsx` stay at 1 and `refetchInterval` stays absent; tripwire; read with `rg -c "useQuery\(" client/overview.tsx` and `rg -n "refetchInterval" client/overview.tsx` after the first render code and at completion.

## Verify

- In the plugin folder: `npm run typecheck` and `npm run lint` -> exit 0.
- `rg -c "useQuery\(" hiep-plugins/plugins/watchtower-board/client/overview.tsx` -> 1.
- `rg -n "refetchInterval" hiep-plugins/plugins/watchtower-board/client/overview.tsx` -> no match.
- `rg -n "readOverviewRpc" hiep-plugins/plugins/watchtower-board/client/overview.tsx` -> at least one match.
