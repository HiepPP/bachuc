# TASK-001 Outcome

## Outcome

Status: DONE

## Changed

- plugins/thread-context-attach/server/snapshot.ts
- plugins/thread-context-attach/tests/snapshot.test.ts (new tests appended and imports extended; existing tests unchanged)

## Contract (for TASK-003)

Exported from `server/snapshot.ts`:

- `recentAssistantTexts(entries, max): string[]`: up to `max` newest non-empty assistant texts, oldest first. `lastAssistantText` still works and now delegates to it.
- `buildSnapshot(thread, input: string | null | SnapshotInput, emptyText?)`: a string or null behaves exactly as before. `SnapshotInput = { replies: readonly string[]; status?: JevStatus | null }`. Only the newest 3 replies are used. `Status: <status> (Jev judgment)` follows `Cwd:`. One reply renders `## Last reply`; 2 or 3 render `## Recent replies (oldest first)` with a `### Reply N of M` heading per reply. Budget: reply text (headings excluded) stays within `MAX_REPLY_CHARS`. The newest reply is always kept, and older replies are dropped from the first one that does not fit. The newest reply is cut with the current marker only when it alone is too long, and then it is shown alone. The return shape is unchanged (matches `threadSnapshotSchema`).
- `buildJevState(thread: { title }, replies): JevState = { title, replies }`: up to 3 newest replies, oldest first, each cut to its last `JEV_REPLY_CHARS` (4,000) characters.
- `readJevAnswers(answers: unknown): JevAnswers = { status: JevStatus | null; depth: ReplyDepth }`: pass the Jev `result.answers` object with the choice questions `status` and `depth` (keys `"1"`, `"2"`, `"3"`). A choice answer is `{ type: "choice", choice, probabilities }`. The status is used only when `probabilities[choice] >= 0.6` and it is a known choice. Depth is 2 or 3 only when `probabilities[choice] > 0.5`, else 1. Invalid input gives `{ status: null, depth: 1 }`, and each field falls back on its own.
- Constants and types: `JEV_STATUSES`, `JevStatus`, `ReplyDepth`, `MAX_REPLY_DEPTH` (3), `JEV_REPLY_CHARS` (4000), `SnapshotInput`, `JevState`, `JevAnswers`.

## Verified

These checks were run once at the group level (group B has only this task), from `plugins/thread-context-attach`:

- `npm test`: exit 0. 10 of 10 pass, including the 4 unchanged current tests. The new tests cover depth 1 to 3, the status line, the size budget, the 4,000-character Jev cut, and bad Jev answers.
- `npm run typecheck`: exit 0.
- `S=lint; npm run $S`: exit 0, with 0 warnings and 0 errors.
- `npx oxfmt --check server/snapshot.ts tests/snapshot.test.ts`: clean.
- Impact: GitNexus has no index for this repo. `rg` shows `buildSnapshot` and `lastAssistantText` are used only in `server/threads.ts` (3 calls, all with the unchanged string/null signature) and in tests. Risk is LOW.
- Note: `index.client.tsx`, `package.json`, `client/`, and `shared/settings.ts` changed in the same checkout because of other groups, not this task.

Follow-up (2026-09-23):
- The live test showed Jev splits depth across 2 and 3, for example 0.23 / 0.48 / 0.29, so the old rule always gave depth 1.
- `readJevAnswers` now walks back when P(2) + P(3) > 0.5 and picks the likelier of 2 or 3. Ties pick 2.
- `npm test` -> 16/16 pass, with new depth cases in the Jev answers test.
