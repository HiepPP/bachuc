# TASK-006 Outcome

## Outcome

Status: DONE

Changed:
- Created the Board task and generated a UI concept with the built-in imagegen tool.
- Implemented and installed the separate Board plugin with a native sidebar item and two-column page.
- Added lifecycle tracking, active-run reconciliation, bounded history, and explicit missing-outcome states.
- Added offline/stale feedback after independent review reproduced paused queries falsely showing Live.

Contract:
- Each card now represents one Paseo conversation. Idle status never proves completion.
- The latest 50 observed finished conversations stay in memory until reload or disable. Earlier finished history is unavailable.
- The UI polls every two seconds. It uses host theme colors and standard plugin APIs, without private sidebar changes.
- Paseo source, existing plugins, agents, and Space membership are preserved.

Verified:
- The concept places Board immediately below Schedules and shows Running and Just finished columns.
- The image uses synthetic example runs. It does not prove runtime integration.
- The user approved the UI and confirmed that each task represents a Paseo agent run on 2026-09-20.
- Base revision: `cbb70fa`, branch `main`, with the local uncommitted Board implementation.
- From the plugin directory: `npm run format`, `npm run typecheck`, `npm run lint`, and `npm test` pass.
- Twelve tests cover repeated/reused IDs, null IDs, snapshot races, missing/late outcomes, retention, pagination, and offline/stale state.
- `git diff --check` passes. `paseo plugin ls board --json` reports running without a load error.
- Desktop Paseo 0.8.0: Board appears immediately below Schedules and shows the existing active run.
- Smoke agent `0b817fe2-10ae-40fe-a3b0-4151d7c835e3` appeared in Running, then Completed with a 24-second duration.
- `paseo logs 0b817fe2-10ae-40fe-a3b0-4151d7c835e3 --tail 5 --json` confirmed the test reply and read-only wait.
- Desktop light/dark layouts, empty finished history, and existing Space switching were checked through the native app.
- Disabling Board removed its sidebar item and showed the unavailable surface. Logs confirmed plugin stop and subsequent ready state.
- Final enable loaded the current source. The active run returned, history reset, and the final full-height layout rendered correctly.

Limitations:
- Native mobile and compact layout were not exercised on a device. Desktop checks used the installed macOS app.
- Loading/error/offline classification has automated coverage; no real host outage was induced.
- Failure and cancellation transitions have automated coverage; the live smoke covered successful completion.

Cleanup:
- Archived the smoke agent after completion. Final enable cleared its temporary in-memory card.
- Restored System theme and Space 1. No temporary server, credential, or worktree was created.
- Board remains enabled for use. No commit or push was made.

Handoff: Desktop Board is ready. TASK-004 physical swipe acceptance remains separate.

## Cmd+D shortcut

- Added macOS desktop Cmd+D to open Board through the supported surface navigation method.
- The keyboard listener ignores repeats, composition, and additional modifiers; cleanup unregisters it.
- Cleared the conflicting Start/stop dictation shortcut through the current client's native Settings → Shortcuts.
- Live Cmd+D opened Board from the focused chat composer and the settings screen.
- Format, typecheck, lint, and 14 tests pass. The reloaded Board plugin is running.
- No commit or push was made.

## Open conversation from card

- Card content now opens its agent conversation using the host navigation API, in both columns.
- Remove remains a separate sibling button, so removing a card does not trigger navigation.
- Format, typecheck, lint, and 13 tests pass; the reloaded plugin is running.
- Live card click changed the route from the Board surface to workspace `wks_a73a6882142dce65`.
- The matching conversation and message composer were visible and focused. No commit or push was made.

## One item per conversation

- The latest user correction replaces per-turn cards with one item per Paseo agent conversation.
- New turns reuse the conversation ID and move its item from Just finished to Running, then back after completion.
- Status and duration describe the latest turn. Removed items reappear when the conversation starts a new turn.
- Remove validates the observed completion timestamp, preventing an old request from hiding a newer completed turn.
- Format, typecheck, lint, and 13 tests pass. The reloaded plugin reports running.
- Live smoke conversation `8736e3e1-b2d3-47f9-ae4f-a0152f2c1806` completed two turns with exactly one card.
- Verified Finished → Running → Finished without duplication; removed the test card and archived the test agent.

## Finished card removal

- Added Remove to finished cards. It hides the card for the selected host without changing agents or chats.
- Running cards cannot be removed. Repeated events do not restore a removed card within retained history.
- Requests include the plugin observation scope to reject stale requests after reload.
- Format, typecheck, lint, and all 13 tests pass. Reload reports running without a load error.
- Live check: removed Board remove smoke from Just finished; the empty state appeared immediately.
- Smoke agent `4c6acad3-e99e-4473-9d21-d288416133a2` remained available after removal, then was archived for cleanup.
- No commit or push was made.
