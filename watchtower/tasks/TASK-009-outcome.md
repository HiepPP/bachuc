# TASK-009 Outcome

## Outcome

Status: DONE

Changed:

- Added [hiep-plugins/plugins/watchtower-board/client/pill-label.ts](hiep-plugins/plugins/watchtower-board/client/pill-label.ts) with the pure `pillLabel(board)`.
- Added [hiep-plugins/plugins/watchtower-board/client/pill.tsx](hiep-plugins/plugins/watchtower-board/client/pill.tsx) with `WatchtowerPillLabel`. It reads `watchtower.read` for the composer's workspace with `staleTime: 30_000` and `retry: false`. For a `draft` context, it runs no query and returns `"Watchtower"`.
- [hiep-plugins/plugins/watchtower-board/index.client.tsx](hiep-plugins/plugins/watchtower-board/index.client.tsx) registers one unscoped composer pill with `id: "watchtower"`, `showOnDraft: true`, the `ListTodo` icon, and the default `track` placement. A press calls `preselectWorkspace` for an agent context, then `client.openSurface("watchtower")`. The cleanup removes the pill.
- Added [hiep-plugins/plugins/watchtower-board/tests/pill.test.ts](hiep-plugins/plugins/watchtower-board/tests/pill.test.ts) with 3 tests.

Contract:

- Labels: `"Watchtower"` with no plan or on New workspace, `"Watchtower <done>/<total>"`, and `", <n> question"` or `", <n> questions"` for blocking open questions (Q-002 answered: show on New workspace too).
- The pill uses the existing `watchtower.read` RPC. Its query key matches the Explorer panel's key, so the panel and the pill share one cache entry.

Verified:

- Provider check: the host renders a custom label inside `ButtonEnvironment`, which wraps the trigger in `PluginRuntimeBoundary`. That boundary provides `QueryClientProvider`, `PaseoApiProvider`, and `PluginRpcProvider`. Source: [packages/app/src/plugins/buttons/view.tsx](packages/app/src/plugins/buttons/view.tsx) and [packages/app/src/plugins/runtime-boundary.tsx](packages/app/src/plugins/runtime-boundary.tsx).
- `npx tsx --test tests/pill.test.ts` -> 3 pass, 0 fail.
- `npm test` in the plugin folder -> 49 pass, 0 fail.
- `npm run typecheck` -> exit 0. `npm run lint` -> exit 0, 0 warnings.
- `rg -n "refetchInterval" client/pill.tsx` -> no match.
- Live check: UNVERIFIED (autonomous run). The live daemon was stopped at 13:07, and the run must not start it.

Anti-goal:

- After the first label code: no `refetchInterval`, `staleTime: 30_000`; `rg -n "refetchInterval|staleTime" client/pill.tsx`, 2026-10-07 13:10.
- Final: no `refetchInterval`, `staleTime: 30_000`; same command after formatting, 2026-10-07 13:11.
- Result: PASS against no polling and a stale time of 30000 ms or more.

Lessons:

- A pill label component can use `useRpc` and `useQuery`, because the host wraps the trigger in the plugin's runtime providers. Source: `ButtonEnvironment` in [packages/app/src/plugins/buttons/view.tsx](packages/app/src/plugins/buttons/view.tsx).
