# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks   | Question                                                                                                                                                            | Default                                                                                             | Status    | Answer |
| ----- | -------- | -------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------- | --------- | ------ |
| Q-195 | TASK-312 | -        | Should a switcher row show a preview of the last reply? The agent snapshot has no reply text, so a preview needs one timeline fetch per row.                        | No preview. A row shows the title, the workspace name, the reason (finished or error), and the age. | DEFAULTED | -      |
| Q-196 | TASK-312 | TASK-312 | Hand check on live: hold Cmd, press J twice, release Cmd. Does the second row open, with no popover flash on a quick tap? AX automation cannot hold or release Cmd. | none                                                                                                | OPEN      | -      |

Status: OPEN, DEFAULTED (a run took the default; the owner may still answer), ANSWERED, DROPPED.
Blocks: the TASKs that cannot start until the question is answered, or `-` when the spec gives a safe default.
