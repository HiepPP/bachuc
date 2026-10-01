# TASK-014 Outcome

## Outcome

Status: DONE

Changed:

- Deleted `client/web.ts` (586 lines of DOM). Its uncommitted per-host edits were saved first to [watchtower/tasks/TASK-014-web-ts-uncommitted.patch](watchtower/tasks/TASK-014-web-ts-uncommitted.patch); the old test file diff is in [watchtower/tasks/TASK-014-sidebar-test-uncommitted.patch](watchtower/tasks/TASK-014-sidebar-test-uncommitted.patch).
- Added [hiep-plugins/plugins/workspace-spaces/client/spaces-sidebar.ts](hiep-plugins/plugins/workspace-spaces/client/spaces-sidebar.ts) (selected Space per host, filter, Move to items) and [hiep-plugins/plugins/workspace-spaces/client/spaces-strip.tsx](hiep-plugins/plugins/workspace-spaces/client/spaces-strip.tsx) (tabs, create, rename, remove).
- [hiep-plugins/plugins/workspace-spaces/index.client.tsx](hiep-plugins/plugins/workspace-spaces/index.client.tsx) registers the filter, menu items, and section. The active host comes from the section props.
- [hiep-plugins/plugins/workspace-spaces/client/sidebar-state.ts](hiep-plugins/plugins/workspace-spaces/client/sidebar-state.ts) gains `notify`. [hiep-plugins/plugins/workspace-spaces/client/page.tsx](hiep-plugins/plugins/workspace-spaces/client/page.tsx) drops the DOM wheel binding.
- [hiep-plugins/plugins/workspace-spaces/tests/sidebar.test.ts](hiep-plugins/plugins/workspace-spaces/tests/sidebar.test.ts) keeps the 3 DOM-free tests and adds 3 sidebar model tests; 12 DOM tests were removed.
- Requirements, `file:` SDK links, tsconfig type paths, lint flag, README.

Contract:

- Deviation from the spec: `client/gesture.ts` stays, because it has no DOM code and the page uses its touch direction.
- The settings schema in [hiep-plugins/plugins/workspace-spaces/shared/spaces.ts](hiep-plugins/plugins/workspace-spaces/shared/spaces.ts) was not changed by this TASK.

- User feedback, 2026-10-01: the trackpad swipe was missing, and Rename and Remove showed all the time. The host now sends horizontal wheel gestures over the project list to a new optional `onSwipe` on `addSidebarProjectFilter` ([packages/app/src/plugins/sidebar/swipe.web.ts](packages/app/src/plugins/sidebar/swipe.web.ts), gesture logic in [packages/app/src/plugins/sidebar/wheel-gesture.ts](packages/app/src/plugins/sidebar/wheel-gesture.ts)). The plugin moves to the adjacent Space. Rename, Remove, and Done show only after a right-click or long-press on a tab.

Verified:

- After that change, in Paseo Dev over CDP: `mouseWheel` deltaX 15,15,15,10,5 moved Workspace 1 to Workspace 2, and the negative series moved back; a right-click on tab 2 showed the actions and Done hid them. Plugin tests 20 passed; app plugin unit tests pass.

- `npx tsc --noEmit` -> exit 0. `npm run lint` -> 0 warnings, 0 errors. `npm test` -> 19 passed.
- DOM check (including `localStorage`) -> no match.
- Real host check, Paseo Dev 2026-10-01T01:00: the strip created Workspace 2, the project menu showed `Move to Workspace 2` before Remove, and after the React Compiler fix in TASK-007 switching tabs filtered projects live (pd-check only in Workspace 2, .claude only in Workspace 1). Workspace 2 was left in the Paseo Dev Spaces settings.

Anti-goal:

- Before changes: 28 passed; the saved shape is read by `preferencesSchema`; 2026-10-01T00:44.
- Final: `shared/spaces.ts` unchanged by this TASK; the kept revision-conflict and rename tests read saved state through the same schema and pass; 2026-10-01T00:50.
- Result: PASS.
