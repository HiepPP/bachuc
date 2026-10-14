# TASK-099 Outcome

## Status

DONE

## Changed

- `hiep-plugins/plugins/watchtower-board/README.md`: the right-side list now describes the pace line and the plan tree. It no longer names the branch timeline, Running now, or Up next. "timeline-first" became "plan-tree".
- `watchtower/decisions/ADR-0029-plan-tree-over-branch-timeline.md` (new, accepted).
- `watchtower/DECISIONS.md`: one ADR-0029 row.
- `watchtower/MEMORY.md`: one `## Avoid` item that links ADR-0029.

## Contract

- README facts come from `client/overview.tsx` (pace labels, 900 px and 520 px layout, run state fallback) and the TASK-095 to TASK-098 outcomes (waves, collapsed done wave, amber row with Open thread, row press selects the task in the rail).
- ADR-0029 is accepted: the owner chose the tree in the session that wrote the plan.

## Verified

- `rg -n "Branch timeline|branch timeline|Running now|Up next" hiep-plugins/plugins/watchtower-board/README.md` -> no match.
- `rg -c "Plan tree over branch timeline" watchtower/DECISIONS.md` -> 1.
- `git diff --check` -> no output.
- `npm run format` (plugin) -> exit 0. `npm run format:files` on the three watchtower files -> ran, 3 files.

## Anti-goal

README `wc -l`: 143 at 12:14:20, 145 at 12:14:53 (+2, limit +10). Not crossed. Final PASS.

## Lessons

- The old right-side bullets named the timeline in two places (the intro sentence and the list); check both when you rewrite.
