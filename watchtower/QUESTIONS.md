# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                                   | Default                                                                                   | Status   | Answer                                                                          |
| ----- | -------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------- | -------- | ------------------------------------------------------------------------------- |
| Q-022 | TASK-061 | -      | Should the migration also renumber the 10 old plugin plans in `hiep-plugins/watchtower/archive/`? The skill never reads that folder today. | No. Migrate only `watchtower/` at the git root.                                           | ANSWERED | 2026-10-12: owner took the default. Migrate only `watchtower/` at the git root. |
| Q-023 | TASK-062 | -      | Add an optional `params` record to `openSurface` and pass it to the surface as a prop, so Board can open Watchtower on a named workspace?  | Yes. `params?: Record<string, string>` on the client plugin API only; no protocol change. | ANSWERED | 2026-10-12: owner took the default. Add `params?: Record<string, string>`.      |
| Q-024 | TASK-069 | -      | Board keeps 50 finished runs. Should it also keep every run of the last 24 hours, so Timeline never loses threads on a busy day?           | No. Keep 50; Timeline says "Older threads are not kept" when the window lost some.        | OPEN     | -                                                                               |

Status: OPEN, DEFAULTED (a run took the default; the owner may still answer), ANSWERED, DROPPED.
Blocks: the TASKs that cannot start until the question is answered, or `-` when the spec gives a safe default.
