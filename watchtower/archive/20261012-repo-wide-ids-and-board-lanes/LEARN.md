# Learn 20261012-repo-wide-ids-and-board-lanes

## Summary

Discrepancy: 2 found. The plan marked Board work DONE that was never committed, and one Tracker note went stale.

## Per TASK

- TASK-059: match. The Tracker note says "not committed", but the skill change is in `~/.claude` commit `72f8b26`. Fix: update a Tracker note when its fact changes.
- TASK-060: match.
- TASK-061: match.
- TASK-062: match. Shipped in commit `130d154e1`. Release still needs a new build to use it.
- TASK-063 (Watchtower Needs you clarity): match. Shipped in commit `62b3dd440`.
- TASK-064 (Board Timeline tab): match. Live check found one bug, an empty window folded into a 24h break; it was fixed before DONE.

## Plan-Level

- Plan misdescribes reality: 7 Board TASKs (plan reading, task lanes, keys and search, plan line, Board Needs you) were marked DONE with live checks, but their code was never committed. The commit pass kept only the TASK chip and `openWatchtower`. The owner dropped the work, and the TASKs were removed and the rest renumbered.
- ADR-0012 and ADR-0013 still describe a plan line and a Board Needs you that do not exist. Their core choices still hold: Board stays on two status columns with TASK chips, and questions live on the Watchtower page.

## Lessons

- Mark a TASK DONE only after its code is committed, or record the commit SHA in the outcome.
- Before a commit pass that splits or rewrites commits, compare `git status` with the files each outcome lists under `Changed`.
