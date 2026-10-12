# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                        | Default                                                                                            | Status   | Answer                                                                           |
| ----- | -------- | ------ | ------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------- | -------- | -------------------------------------------------------------------------------- |
| Q-025 | TASK-073 | -      | The design merges two edits of one file into one row and one diff. Should the Edit step merge them, or show each edit-diff row? | Show each edit-diff row in order inside the Edit step. Merging per file is a later TASK.           | ANSWERED | Show each edit-diff row in order. Merging per file is a later TASK. (2026-10-12) |
| Q-026 | TASK-072 | -      | Thought (reasoning) items inside a turn: show them in the step list, or hide them?                                              | Hide them in the step list. The `detailed` level still shows them.                                 | ANSWERED | Hide them in the step list. (2026-10-12)                                         |
| Q-027 | TASK-075 | -      | Should the default tool call detail level change from `detailed` to `overview`? Phones then get the current Overview badges.    | Yes. It changes new installs only; release and live keep their stored setting until you switch it. | ANSWERED | Yes, change the default to `overview`. (2026-10-12)                              |

Status: OPEN, DEFAULTED (a run took the default; the owner may still answer), ANSWERED, DROPPED.
Blocks: the TASKs that cannot start until the question is answered, or `-` when the spec gives a safe default.
