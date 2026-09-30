# TASK-001 Recent replies and status in the snapshot

Group: B (standalone; writes snapshot.ts and its test only)
Class: code

## Brief

Goal: Let the snapshot hold 1 to 3 recent assistant replies and an optional status line. Add pure helpers that build the Jev input and read the Jev answer.

Change: one last reply -> newest 1 to 3 replies, oldest first, plus an optional `Status:` line.

How:

- Add `recentAssistantTexts(entries, max)`. It returns up to `max` newest non-empty assistant texts, oldest first. Keep `lastAssistantText` working for current callers.
- Extend `buildSnapshot` with an optional `{ replies, status }` input. With one reply and no status, output must match the current text exactly.
- With a status, add `Status: <status> (Jev judgment)` after the `Cwd:` line.
- With 2 or 3 replies, use `## Recent replies (oldest first)` and a `### Reply N of M` heading per reply.
- Keep the total reply body within `MAX_REPLY_CHARS`. Keep the newest reply first. Drop older replies that no longer fit. Truncate the newest reply only when it alone is too long, with the current marker.
- Add `buildJevState(thread, replies)`. It returns the title and up to 3 replies, each cut to its last 4,000 characters.
- Add `readJevAnswers(answers)`. It returns `{ status, depth }`. Use a status only when its probability is at least 0.6. Use depth 2 or 3 when P(2) + P(3) is above 0.5, and pick the likelier of the two; else use 1. Invalid input returns `{ status: null, depth: 1 }`.

Files:

- [plugins/thread-context-attach/server/snapshot.ts](plugins/thread-context-attach/server/snapshot.ts) (new helpers and the extended `buildSnapshot`)
- [plugins/thread-context-attach/tests/snapshot.test.ts](plugins/thread-context-attach/tests/snapshot.test.ts) (new tests; keep current tests unchanged)

Expected result:

- Current snapshot tests pass without edits.
- New tests cover depth 1 to 3, the status line, the size budget, the 4,000-character Jev cut, and bad Jev answers.

## Verify

- `cd plugins/thread-context-attach && npm test` -> all tests pass, including the unchanged current tests.
- `cd plugins/thread-context-attach && npm run typecheck` -> exit 0.
- `cd plugins/thread-context-attach && S=lint; npm run $S` -> exit 0.
