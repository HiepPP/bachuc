# Learn 20261015-sqlite-ram-cpu-cuts

## Summary

Discrepancy: 4 found. All 8 TASKs shipped, but two specs missed a cross-cutting effect, and three follow-up fixes were needed after the plan closed.

## Per TASK

- TASK-126: match. One test file outside `Files:` (`host-runtime.test.ts`) changed, because its fake row store implements the changed interface.
- TASK-127: plan "viewed set" -> shipped the visible set. Mistake: the spec did not say which set, and the open-tab set made the 12-thread case impossible. Fix: name the exact set in the spec (Q-044).
- TASK-128: match, with two owner defaults: diacritic folding on the cache path only (Q-047), and word-prefix matches only (Q-048).
- TASK-129: plan "move or list each full-timeline caller" -> shipped a list only. Mistake: callers that read the whole timeline (`get_agent_activity`, `timelineSize`, plugin `agent.turn_ended`) kept reading the memory tail, and a cache failure after a trim left only the tail. Fix: follow-ups `21a5b892a` and `bfd79b58f`. The live check needed an RPC paging script, because 3,701 source rows projected to 88 items, which fit in one app page.
- TASK-130: match.
- TASK-131: plan "awaited write settles on commit" -> shipped as specified (Q-043, option A), but the spec missed `appendTimelineItem`, which awaits the record write. Mistake: every plugin append and worktree setup step became 250 ms slower, and 3 durable timeline tests timed out. The wave check missed it, because those tests ran before group E landed. Fix: follow-up `a58f5632c` and the `writeWindowMs` test option.
- TASK-132: match.
- TASK-133: match. The spec's `format:check:files` line cannot check `docs/`, because the formatter ignores that folder.

## Plan-Level

- Wrong order: group E (TASK-131) landed after group D's tests ran. No later re-run covered D's tests, so the regression reached the plan close. A plan that changes a shared write path must re-run the dependent test files after the last group.
- Scope: another session committed 3 board plugin commits onto the run branch. They shipped in PR #70.
- ADR candidates, not recorded per the archive rule: the 8-thread stream cap chosen over 3 and over a byte budget, and awaited record writes that settle on commit (Q-043).

## Lessons

- When a spec debounces a write, list every caller that awaits it, including timeline appends.
- After the last group of a wave, re-run the test files of every earlier group that touches the same subsystem.
- Plan live paging checks as RPC scripts. Source rows can project to few items.
- Do not put a format check for `docs/` files in a Verify line.
