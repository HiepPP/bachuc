# Learn 20260923-thread-export-qmd-search

## Summary

Discrepancy: 3 found. All three TASKs shipped. Two specs were updated during implement, and one manual check is still open.

## Per TASK

- TASK-001: plan said the backfill starts at plugin load -> shipped a backfill that starts on the first hook or RPC. Mistake: the spec assumed a Paseo API exists at load. Fix: the spec was updated, and the rule is now in [watchtower/MEMORY.md](../../MEMORY.md).
- TASK-002: plan said to run qmd from its own folder -> shipped a runner that keeps the user's `PATH` order. Mistake: putting `/opt/homebrew/bin` first picked Node v25, and qmd's `better-sqlite3` failed. Fix: keep `PATH` order. The lesson is in [watchtower/MEMORY.md](../../MEMORY.md).
- TASK-003: match, except the manual in-app check is still PENDING-USER.

## Plan-Level

- none. Order and dependencies held. The shared file [plugins/thread-context-attach/index.server.ts](../../../plugins/thread-context-attach/index.server.ts) was correctly kept in one group.

## Lessons

- Check when a plugin can first call Paseo before a spec plans startup work.
- When a plugin spawns a CLI tool, test it with the daemon's real `PATH`.
- Close manual in-app checks before archive, or carry them into the next plan's Handoff.
