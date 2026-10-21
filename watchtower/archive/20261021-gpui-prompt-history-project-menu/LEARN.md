# Learn 20261021-gpui-prompt-history-project-menu

## Summary

Discrepancy: 2 small spec gaps in TASK-287. Both TASKs shipped as planned and passed every code and screen check.

## Per TASK

- TASK-286: match. The builder also made Up at the oldest entry keep that entry, and dismissed the slash and mention menus for a recalled `/name` or `@file`. Both fit the CLI behavior the spec asked for.
- TASK-287: plan said the red tests only call `connect` -> shipped tests also call `server.add_agent("a1", "idle")`. Mistake: the spec did not know `Harness::connected` waits for an agent. Fix: the spec now has the line, and [watchtower/MEMORY.md](watchtower/MEMORY.md) records it.
- TASK-287: plan named `crates/app/src/state.rs` as a possible exhaustive `ClientEvent` match -> the real matches were in `crates/timeline/src/lib.rs` and `crates/client/examples/client_probe.rs`. Mistake: the planner guessed from one grep hit, and Builder Check did not check the claim. Fix: name such files only after `./cargo.sh build --workspace --all-targets` or a match-arm search.

## Plan-Level

- Remove project leaves the daemon worktree folder on disk (`.dev/paseo-home/worktrees/01lg0gqg/wt-ui-x` on live). This is daemon behavior, outside this plan.

## Lessons

- For a GPUI client test, check the harness setup that `connect` needs before you write red tests.
- Mock agents from `npm run cli -- run -d --provider mock/ten-second-stream` make cheap live fixtures for composer screen checks.
