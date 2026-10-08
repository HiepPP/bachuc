# TASK-010 Outcome

## Outcome

Status: DONE

Changed:

- [packages/app/e2e/browser/sidebar-help.spec.ts](packages/app/e2e/browser/sidebar-help.spec.ts): the version row check expects `Bachuc` and accepts the fork version form `hiep-X.Y.Z` next to `vX.Y.Z`. The checks for "Report an issue", "What's new", the Discord and upstream GitHub issue pages, and the in-app changelog test are deleted. The unused constants, the `expectExternalPage` helper, and the changelog helper import went with them.
- The three tests that open the sidebar help menu are `test.fixme` (Q-008, DEFAULTED). The fork hides the Help button since commit `5e8568f6d` (2026-09-30, on `main`) through `SHOW_FOOTER_EXTRAS = false` in [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx). No test can open the menu, before or after this TASK.

Contract:

- The spec asserts only items that [packages/app/src/components/sidebar/sidebar-help-menu.tsx](packages/app/src/components/sidebar/sidebar-help-menu.tsx) renders: the Help label, keyboard shortcuts, diagnostics, and the version row `Bachuc hiep-1.0.0`.
- The diagnostics test from Settings still runs.

Verified:

- First run, before `test.fixme`: 3 failed, 1 passed. Each failure waits for `getByTestId('sidebar-help')`; the page snapshot shows the footer with only Add project and Settings. Classified as pre-existing: the button is hidden in source on `main`.
- `cd packages/app && npx playwright test --project=browser e2e/browser/sidebar-help.spec.ts` -> exit 0, 1 passed, 3 skipped (fixme). The global setup started its own test daemon and Metro.
- `npm run lint -- packages/app/e2e/browser/sidebar-help.spec.ts` -> exit 0, 0 warnings.
- `npm run typecheck` -> exit 0.
- `npm run format:files -- packages/app/e2e/browser/sidebar-help.spec.ts` -> formatted.
- GitNexus `impact expectExternalPage` -> 0 callers. `detect-changes --scope all` -> 1 file, risk low.
- The fixme help menu checks: UNVERIFIED (autonomous run). They run again when the Help button shows.

Anti-goal:

- Before: 25 `expect(` calls (`rg -c "expect\(" packages/app/e2e/browser/sidebar-help.spec.ts`, 12:05).
- Removed, 10: "Report an issue" (1), "What's new" (1), the popup URL check inside `expectExternalPage` that served Discord, the GitHub issue page, and the changelog website (1), and the in-app changelog test (7). Each names a removed item.
- Final: 15 `expect(` calls (12:15). The checks for the Help label, diagnostics, shortcuts, shortcut search, the compact menu, and Settings diagnostics stay.
- Result: PASS. The count drops only by the checks for removed items.

Lessons:

- The fork hides the sidebar footer Hosts, Import session, and Help buttons with `SHOW_FOOTER_EXTRAS = false` in `packages/app/src/components/left-sidebar.tsx`. An e2e spec that clicks `sidebar-help`, `sidebar-import-session`, or the host picker fails before its own checks. Source: commit `5e8568f6d`; the first spec run of this TASK.
- [packages/app/e2e/support/helpers/changelog.ts](packages/app/e2e/support/helpers/changelog.ts) has no user now. knip lists `e2e/**` files as entries, so it reports nothing. The in-app changelog opens only from the update callout, and updates are off (TASK-006).
- macOS has no `timeout` command. Use the Bash tool timeout instead.
