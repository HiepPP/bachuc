# TASK-014 Outcome

## Outcome

Status: BLOCKED

Changed:

- Added project containers and unstarred counts inside each status column.
- Starred cards appear above all project containers. Empty sections disappear.
- Host project keys separate projects with equal names and join workspaces from one project.
- Hook-only finished cards resolve project placement through the existing SDK.
- Known project metadata survives new turns. Unavailable placement falls back to the full working directory.
- Fixed the blank line that separated TASK-014 from the Tracker table.

Contract:

- Each conversation appears once. Existing card order, actions, themes, and scaling remain.
- Stars and history remain in memory. Reload still resets both.
- No collapse controls, new settings, dependencies, or Paseo application changes.

Verified:

- 2026-09-22: format, typecheck, lint, and all 27 tests passed.
- Commands: `npm --prefix plugins/board run format`, `npm --prefix plugins/board run typecheck`.
- Commands: `npm --prefix plugins/board run lint`, `npm --prefix plugins/board test`, `git diff --check`.
- Tests cover group order, same-name projects, multiple workspaces, empty/all-starred columns, and star lifecycle transitions.
- `paseo plugin reload board` succeeded on the local 0.8.0 daemon.
- `paseo plugin ls board --json` reports running at the current plugin directory.
- `paseo plugin logs board --json` records Plugin ready at 2026-09-22T03:05:21.533Z without a new load error.
- Native macOS UI showed two running cards in separate project groups, with correct totals.
- Starring the task conversation removed its empty project group and placed the card above all groups.
- Unstarring restored its project group. The total stayed two, without duplicate cards.
- UI size changed from 100% to 110%, then reset to 100%.
- Light and dark screenshots showed readable group headers, counts, cards, and status labels.
- Clicking a grouped conversation opened the matching workspace.

Remaining acceptance:

- No second connected client was exercised, so cross-client star updates remain unverified.
- Compact/mobile remains unverified. The native window resize attempt did not change the viewport.
- The observed finished column was empty after reload. Live finished-card Remove and transitions need a completed conversation.
- No live Needs input card was available. Its existing state logic passed unit tests.
- These are missing runtime checks, not observed product failures. Keep the task open until they pass.

Cleanup:

- Restored the System theme, 100% scale, original unstarred state, and the workspace open before Board testing.
- No test conversations, servers, or credentials were created.
- No commit or push. Shared planning memory remains unchanged.
