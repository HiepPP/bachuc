# ADR-0011 Repo-wide IDs over per-plan IDs

Status: accepted
Date: 2026-10-12
Supersedes: -
Scope: `watchtower/**`, `hiep-plugins/plugins/board/**`, `hiep-plugins/plugins/watchtower-board/**`

## Context

- Every plan restarted its TASK and Q numbers at 001, so the archive held 5 plans with overlapping IDs (56 TASKs, 21 Q-IDs).
- The label `watchtower.task=TASK-NNN` and commit messages name a TASK without its plan, so an agent cannot trace an old ID.
- ADR-IDs were already repo-wide.

## Decision

- TASK and Q-IDs increase across the repo and never restart. The next ID is one more than the highest in `watchtower/` and `watchtower/archive/`.
- Each repo migrates its archive the first time a new plan is created there: archive date, then slug, then old ID.
- The migration rewrites TASK and Q-ID references in ADR bodies and `watchtower/MEMORY.md`. This is the only exception to R3.
- No ID map file is kept.
- The owner chose this on 2026-10-12, for all repos that use the Watchtower skill.

## Consequences

- The skill gains a migration script and an Author Plan step that runs it.
- 42 commit messages in this repo keep IDs that no longer match their plan files.
- Board can use a TASK-ID as a unique key inside one repo.

## Options Considered

- Per-plan prefixes such as `TASK-<slug>-001`: rejected. IDs get long and hard to type in prompts.
- Prefixes by kind such as FEAT-16 and BUG-96: rejected. TASKs have no kind field, and every parser would change.
- New plans only, with the archive left as is: rejected. The owner wants one sequence across old and new plans.
- An ID map file next to the archive: rejected by the owner.

## Revisit If

- An agent fails to trace an ID from a commit message because no map exists.
- Two repos share one `watchtower/` folder, so repo-wide numbering collides.
