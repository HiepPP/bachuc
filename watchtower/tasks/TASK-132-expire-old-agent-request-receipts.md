# TASK-132 Expire old agent request receipts

Group: F (writes `packages/server/src/server/message-receipts/index.ts` and `packages/server/src/server/websocket-server.ts`; no shared file with other groups)
Class: code

## Brief

Goal: Receipt files in `$PASEO_HOME/agent-requests/` older than 7 days are deleted, so the folder stops growing.

Change: receipts are never cleaned -> a sweep deletes receipt files older than 7 days at startup and every 24 hours.

Boundaries:

- Today `MessageReceipts` writes one JSON file per request key and never deletes one ([packages/server/src/server/message-receipts/index.ts](packages/server/src/server/message-receipts/index.ts) line 41). The release home held 4,274 files on 2026-10-15.
- `CreationService` uses the same folder ([packages/server/src/server/websocket-server.ts](packages/server/src/server/websocket-server.ts) line 695). The sweep must not delete a file it still needs. Check its read path at `creation/index.ts` line 368 before you pick the files to sweep.
- A receipt guards a retried request. 7 days is far past any retry window.
- Use the file `mtime` for age. Never read file bodies for the sweep.
- The sweep runs off the startup path and never blocks a request. A delete error is logged and skipped.

How:

- Add a sweep method with an injectable clock and age limit.
- Start it after the server listens, then every 24 hours with an unref timer. Stop the timer on shutdown.
- Add tests: an old file is deleted, a new file stays, and a delete error does not stop the sweep.

Files:

- [packages/server/src/server/message-receipts/index.ts](packages/server/src/server/message-receipts/index.ts) (sweep)
- [packages/server/src/server/websocket-server.ts](packages/server/src/server/websocket-server.ts) (start and stop the sweep)
- [packages/server/src/server/message-receipts/index.test.ts](packages/server/src/server/message-receipts/index.test.ts) (sweep cases)

Expected result:

- Files older than 7 days are gone after the sweep. Newer files stay.
- A retried request within 7 days still finds its receipt.

Anti-goal: The sweep adds 0 ms to daemon startup before listen; tripwire; read from the startup order in `websocket-server.ts` and the sweep test at completion.

## Verify

- `npx vitest run packages/server/src/server/message-receipts/index.test.ts --bail=1` -> pass, including the 3 cases above.
- `npm run typecheck` -> exit 0.
- Anti-goal: the sweep starts only after listen, per the code path and a test that the sweep runs after start -> within limit.
