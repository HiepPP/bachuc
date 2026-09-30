# TASK-001 Outcome

## Outcome

Status: DONE (code and offline checks). In-app checks: PENDING-USER.

## Changed

- plugins/board/server/recaps.ts (new): `parseRecap`, `recapEntry`, `localDay`, `groupRecaps`, `createRecapStore` (JSONL, mode 0600, dedupe on agentId+turnId, 90 days / 2,000 entries, serialized writes, torn-line repair).
- plugins/board/shared/recaps.ts (new): `recapEntrySchema`, `recapsRpc` (`board.recaps`, input `{ days }` 1-30 default 7), `recapDayMarkdown`.
- plugins/board/index.server.ts: on completed `agent.turn_ended`, parse the last `assistant_message`; detached placement lookup + append. Serves `board.recaps` with `projectId` mapped like `board.snapshot`.
- plugins/board/client/recaps.tsx (new): `BoardViewSwitch` (Runs / Recaps) and `RecapsView` (day > project > rows, row opens thread, per-day Copy via `copyText`).
- plugins/board/client/page.tsx: switch beside `BoardSizeControl`; Recaps view replaces the run columns when selected.
- plugins/board/tests/recaps.test.ts (new): 14 tests.
- plugins/board/README.md: new Recaps section; storage line updated.

## Contract

- Entry: agentId, turnId, title, cwd, workspaceId, project, projectKey, endedAt, day (host local YYYY-MM-DD), branch, did, commitPush, raw (<= 2,000 chars). Missing fields are null.
- Project naming matches server/store.ts: host placement (`project:<key>`), else cwd basename (`cwd:<cwd>`).
- Store path: `$PASEO_HOME/plugin-data/board/recaps.jsonl` (default `~/.paseo`). No backfill.
- A null turnId is never treated as a duplicate.
- Project color uses the saved Board palette; a project not yet colored by the Runs view shows a neutral mark.

## Verified

- Group-level run (only TASK in group A), plugins/board: `npm run typecheck` exit 0; `S=lint; npm run $S` exit 0 (0 warnings, 0 errors); `npm test` exit 0 (73 pass, 0 fail, including 14 in tests/recaps.test.ts covering bullets, no bullets, Vietnamese, missing field, no recap, recap not last section, duplicate turnId, 90-day and 2,000-entry limits, torn line, day/project grouping, markdown copy); `npx oxfmt --check` on all 7 changed files: clean.
- GitNexus impact: repo not indexed; used rg instead. `BoardPage` is used only by index.client.tsx; risk LOW.
- PENDING-USER: `paseo plugin reload board`, one completed turn ending with `## Recap`, then `stat -f %Lp ~/.paseo/plugin-data/board/recaps.jsonl` -> 600 and one new line; Recaps view shows it under today and its project; row click opens the thread; Copy pastes the day's markdown; Runs view unchanged.

In-app evidence 2026-09-24:
- `recaps.jsonl` is mode 600 with 4 lines. Turns 12-14 of agent `9ad0b344` (10:35-10:37 local) each added one entry with the right day, project `.claude`, branch, did, and commit/push.
- Recaps view, row click, and Copy: not seen by the main session. They need a user check.
- PR #3 review fix 2026-09-24: recaps were dropped when a session reload restarted turn ids. It was reproduced live: turn 2 had id `foreground-turn-1` and its recap was lost. Dedupe now skips only a repeat of the latest entry for the agent with the same `turnId` and `raw`. Row keys use `endedAt`. Tests 74/74; the new test fails on the old code. Live recheck: both `turn one` and `turn two` recaps were saved.
