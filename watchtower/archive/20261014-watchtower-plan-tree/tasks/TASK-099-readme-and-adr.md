# TASK-099 README and ADR

Group: E (writes docs only)
Class: docs

## Brief

Goal: Make the plugin README describe the tree, and record why the tree replaced the timeline.

Change: The README describes the branch timeline, Running now, and Up next -> it describes the pace line and the plan tree, and an ADR records the choice.

Boundaries:

- Integrate, do not append: rewrite the README paragraphs that describe the right side. Do not restate code.
- The ADR title is "Plan tree over branch timeline". Context: the owner compared mockups A to D v2 on 2026-10-14 in Claude Design project `9aea602d`, and chose the tree. Options considered: keep the timeline; tree as a second tab; tree beside the timeline. Write a `## Revisit If` with a checkable condition. Use the next free ADR ID.
- Write the ADR as `accepted`, because the owner chose it in the session that wrote this plan, and add its index row and one `## Avoid` item in [watchtower/MEMORY.md](watchtower/MEMORY.md).

How:

- Edit the README section of the right side.
- Write the ADR file, its [watchtower/DECISIONS.md](watchtower/DECISIONS.md) row, and the Avoid item.
- Run the plugin formatter and `npm run format:files` on the watchtower files.

Files:

- [hiep-plugins/plugins/watchtower-board/README.md](hiep-plugins/plugins/watchtower-board/README.md) (right side section)
- [watchtower/DECISIONS.md](watchtower/DECISIONS.md) (one row)
- [watchtower/MEMORY.md](watchtower/MEMORY.md) (one Avoid item)
- `watchtower/decisions/ADR-NNNN-plan-tree-over-branch-timeline.md` (new)

Expected result:

- The README names the pace line and the plan tree, and no longer names the branch timeline, Running now, or Up next.
- The ADR, its index row, and its Avoid item exist.

Anti-goal: The README does not grow. Tripwire: README line count after the TASK is not more than 10 lines over the count before; read before and after with `wc -l`.

## Verify

- `rg -n "Branch timeline|branch timeline|Running now|Up next" hiep-plugins/plugins/watchtower-board/README.md` -> no match.
- `rg -n "Plan tree over branch timeline" watchtower/DECISIONS.md` -> one row.
- `git diff --check` -> no output.
- Anti-goal: `wc -l hiep-plugins/plugins/watchtower-board/README.md` before and after; the difference is 10 or fewer.
