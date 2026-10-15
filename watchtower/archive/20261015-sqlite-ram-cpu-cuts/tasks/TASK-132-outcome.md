# TASK-132 Outcome

## Status

DONE

## Changed

- `packages/server/src/server/message-receipts/index.ts`: add `sweep()`, `startSweeping()`, `stopSweeping()`, and `RECEIPT_MAX_AGE_MS`.
- `packages/server/src/server/websocket-server.ts`: start the sweep in `beginAcceptingConnections()`; stop it in `close()`.
- `packages/server/src/server/message-receipts/index.test.ts`: 4 new cases.

## Contract

- `new MessageReceipts(dir, { now?, maxAgeMs?, onError? })`. Defaults: `Date.now`, 7 days, no error sink.
- `sweep()` lists `*.json` files in the folder and deletes each file whose `mtime` is older than `now() - maxAgeMs`. It never reads a file body. A delete or stat error goes to `onError` and the sweep goes on. It returns the delete count.
- `startSweeping()` runs `sweep()` once through `setImmediate`, then every 24 hours with an unref timer. `stopSweeping()` clears it.
- The daemon logs a sweep error as a warning in module `message-receipts`.
- `CreationService` reads old creation receipts (`readLegacyAgent`, `creation/index.ts` line 217) from the same folder. That path is a tagged COMPAT shim "once old creation receipts can expire", so the sweep may delete those files after 7 days.

## Verified

- `npx vitest run packages/server/src/server/message-receipts/index.test.ts --bail=1` -> PASS (7), FAIL (0).
- `npm run typecheck` in `packages/server` -> exit 0 (server and server tests). Root `npm run typecheck` -> exit 2, only in `packages/app/src/runtime/**` (another group's `ReplicaRowStore` work in progress), none in this TASK's files.
- `rtk proxy npm run lint -- <3 files>` -> exit 0, 0 warnings, 0 errors.
- `npm run format:files -- <3 files>` -> done.

## Anti-goal

- Tripwire: the sweep adds 0 ms to daemon startup before listen. PASS.
- Checkpoint `sweep-start-point`: `beginAcceptingConnections()` in `websocket-server.ts`, called at `bootstrap.ts` line 1734 inside the `listening` handler, so the HTTP server already listens. Read 2026-10-15 from source.
- Checkpoint `first-sweep-deferred`: test "startSweeping returns before it deletes anything, then sweeps in the background" -> the file still exists right after `startSweeping()`, then is deleted. Run 2026-10-15, PASS.
- The constructor does no file work.

## Lessons

- The WebSocket server is built inside the HTTP `listening` handler (`bootstrap.ts` line 1662), so any timer started in its constructor already runs after listen. `beginAcceptingConnections()` is the explicit "ready" hook.
- Root `npm run typecheck` runs the app typecheck last; an app failure from another group hides behind a green server result. Run it in `packages/server` to separate them.
