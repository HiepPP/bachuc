# TASK-009 Composer pill entry

Group: H (standalone: `index.client.tsx` and `client/pill.tsx`)
Class: code

## Brief

Goal: Add a Watchtower pill to the composer track. It shows the plan state of the composer's workspace, and a press opens the full-screen Watchtower surface on that workspace.

Change: no pill -> a track pill labeled, for example, `"Watchtower 5/10"` or `"Watchtower 5/10, 1 question"`, that opens the surface.

Design: option 3a, "Entry pill", in the mock named in [watchtower/CONTEXT.md](watchtower/CONTEXT.md).

Boundaries:

- Use `client.addComposerPill` with no `agentId` and no `workspaceId`, so one pill serves every agent composer on the host. Keep the default `placement: "track"`.
- Q-002 in [watchtower/QUESTIONS.md](watchtower/QUESTIONS.md) is answered: show the pill on the New workspace composer too. Set `showOnDraft: true`. The draft context is `{ context: "draft", workspaceId: "", draft }` and has no project, so the draft label is plain `"Watchtower"` and a draft press opens the surface without `preselectWorkspace`. Do not add a new RPC for the draft label.
- The label component reads the existing `watchtower.read` RPC. Do not add a new RPC for the pill.
- The label component renders inside the host `Text` of the pill. Before you rely on `useRpc` and `useQuery` there, confirm that the plugin's rpc and query providers wrap the pill trigger. Read `renderCustomLabel` in [packages/app/src/plugins/buttons/view.tsx](packages/app/src/plugins/buttons/view.tsx). If they do not, set this TASK `BLOCKED` and write the finding as a new question.

How:

- Create `client/pill.tsx` with `WatchtowerPillLabel` and a pure `pillLabel(board)` helper.
- `pillLabel`: no tasks gives `"Watchtower"`. Tasks give `"Watchtower <done>/<total>"`. Blocking open questions add `", <n> question"` or `", <n> questions"`.
- The label query uses `staleTime: 30_000` and no `refetchInterval`.
- The action reads the press context. For an `agent` context, it calls `preselectWorkspace(workspaceId)`. Then it calls `client.openSurface("watchtower")`.
- The label component returns `"Watchtower"` for a `draft` context and runs no query there.
- Register the pill in `index.client.tsx` and remove it in the cleanup function.

Files:

- [hiep-plugins/plugins/watchtower-board/client/pill.tsx](hiep-plugins/plugins/watchtower-board/client/pill.tsx) (new label component and `pillLabel`)
- [hiep-plugins/plugins/watchtower-board/index.client.tsx](hiep-plugins/plugins/watchtower-board/index.client.tsx) (register and remove the pill)
- [hiep-plugins/plugins/watchtower-board/tests/pill.test.ts](hiep-plugins/plugins/watchtower-board/tests/pill.test.ts) (new tests for `pillLabel`, if `pillLabel` lives in a `.ts` file; otherwise move it to `client/pill-label.ts`)

Expected result:

- `pillLabel` gives `"Watchtower"`, `"Watchtower 5/10"`, and `"Watchtower 5/10, 1 question"` for the three cases.
- The pill shows on agent composers and on the New workspace composer.
- On an agent composer, a press opens the full-screen surface on the composer's workspace.
- On the New workspace composer, the label is `"Watchtower"`, and a press opens the surface on its default workspace.

Anti-goal: the pill adds no polling: `refetchInterval` in `client/pill.tsx` stays absent and `staleTime` stays at 30000 ms or more; tripwire; read with `rg -n "refetchInterval|staleTime" client/pill.tsx` after the first label code and at completion.

## Verify

- In the plugin folder: `npx tsx --test tests/pill.test.ts` -> the three label cases pass.
- In the plugin folder: `npm run typecheck` and `npm run lint` -> exit 0.
- `rg -n "refetchInterval" hiep-plugins/plugins/watchtower-board/client/pill.tsx` -> no match.
- Live check, when the live daemon is up: `npm run cli -- plugin reload watchtower-board`, then look at an agent composer -> the Watchtower pill shows beside Tasks and Subagents, and a press opens the surface. Record it as `UNVERIFIED (autonomous run)` when live is down.
