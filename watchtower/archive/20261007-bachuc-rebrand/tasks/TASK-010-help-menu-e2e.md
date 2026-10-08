# TASK-010 Update the help menu e2e spec

Group: I (standalone; writes [packages/app/e2e/browser/sidebar-help.spec.ts](packages/app/e2e/browser/sidebar-help.spec.ts) only)
Class: mechanical

## Brief

Goal: The help menu e2e spec checks the menu as TASK-002 and TASK-007 left it.

Change: the spec checks "What's new", "Report an issue", the in-app changelog, and the name `Paseo` -> the spec checks the current items and the name `Bachuc`.

Boundaries:

- TASK-007 removed "What's new" and the "Report an issue" section (Discord and the upstream GitHub issue link) from [packages/app/src/components/sidebar/sidebar-help-menu.tsx](packages/app/src/components/sidebar/sidebar-help-menu.tsx). TASK-002 shows the brand `Bachuc`.
- The stale lines are 13 and 46 to 104 of the spec (TASK-007 outcome).
- Delete the checks for removed items. Keep one check per item that still shows. Do not weaken a check for an item that still shows.
- Read the menu source first, and assert what it renders.

How:

- Read the help menu source and the spec.
- Change the name check to `Bachuc`. Delete the tests for removed items. Keep the rest.
- Run the spec once, as in Verify.

Files:

- [packages/app/e2e/browser/sidebar-help.spec.ts](packages/app/e2e/browser/sidebar-help.spec.ts) (match the current menu)

Expected result:

- The spec passes against the current help menu.
- No test checks a removed item or the name `Paseo` as the app name.

Anti-goal: checks for items that still show do not drop; drift gauge; count them before and after the edit. The count drops only by the checks for removed items.

## Verify

- `cd packages/app && npx playwright test --project=browser e2e/browser/sidebar-help.spec.ts` -> pass. The global setup starts its own test daemon, not live. If it cannot start, record the error as `UNVERIFIED (autonomous run)`.
- `npm run lint -- packages/app/e2e/browser/sidebar-help.spec.ts` -> exit 0.
- `npm run typecheck` -> exit 0.
- Anti-goal: the outcome lists the checks before and after. Each removed check names a removed item.
