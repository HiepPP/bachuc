# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                                  | Default                                                                                | Status   | Answer                                                       |
| ----- | -------- | ------ | ----------------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------- | -------- | ------------------------------------------------------------ |
| Q-001 | TASK-006 | -      | Should the overview write to plan files (answer a question, take a default, accept an ADR, mark a check), or stay read-only in this plan? | Read-only. Each Needs you row has a Copy action that copies its text for the composer. | ANSWERED | Take the default: read-only with Copy actions (2026-10-07).  |
| Q-002 | TASK-009 | -      | Should the Watchtower pill also show on the New workspace composer?                                                                       | No. The pill shows only on agent composers, which have a workspace.                    | ANSWERED | Yes, show it on the New workspace composer too (2026-10-07). |

Status: OPEN, DEFAULTED (a run took the default; the owner may still answer), ANSWERED, DROPPED.
Blocks: the TASKs that cannot start until the question is answered, or `-` when the spec gives a safe default.
