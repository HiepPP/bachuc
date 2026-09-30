# TASK-001 Hide archived Board cards

Group: board
Class: code

## Brief

Goal: Remove archived conversations from Board on the next successful refresh, including restored cards.
Change: Reconcile card membership against the complete non-archived agent directory.
How: Add failing regression tests, update directory reads and reconciliation, run checks, reload Board, verify live RPC.
Files: [Snapshot](../../../../plugins/board/server/snapshot.ts), [store](../../../../plugins/board/server/store.ts), [server](../../../../plugins/board/index.server.ts), and their tests.
Expected result: Archived cards disappear; idle unarchived cards retain their outcomes. Stale or failed reads preserve state.
Anti-goal: Zero agent archive, unarchive, or delete operations. Check changed source and live target archive state before and after validation.

## Verify

- Run Board format, typecheck, lint, and tests.
- Cover archived finished and restored running cards, idle survivors, pagination, stale reads, and repeated terminal events.
- Reload Board and check status and logs.
- Read live board.snapshot. Confirm the reported archived agent is absent and remains archived.
