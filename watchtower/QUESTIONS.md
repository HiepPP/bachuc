# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                                                                                                           | Default                                                                          | Status    | Answer |
| ----- | -------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | -------------------------------------------------------------------------------- | --------- | ------ |
| Q-055 | TASK-139 | -      | Should the GPUI mode and model pills get the wheel behavior of [ADR-0025](watchtower/decisions/ADR-0025-wheel-opens-the-list-and-steps-over-closed-pill-step.md) (hover opens the list, one wheel notch per step)? | Click opens a menu; the wheel behavior comes in a later plan.                    | OPEN      | -      |
| Q-056 | TASK-134 | -      | Should a later TASK port the Bachuc palette colors into `bachuc-theme`, beyond the turn label colors?                                                                                                              | Use the gpui-kit default light and dark themes, following the system appearance. | DEFAULTED | -      |
| Q-057 | TASK-135 | -      | Should a later TASK export `ServerInfoStatusPayloadSchema` as a third schema root, so `server_info` decodes to a typed struct?                                                                                     | TASK-136 reads `server_info` fields from the untyped `StatusPayload.extra` map.  | OPEN      | -      |

Status: OPEN, DEFAULTED (a run took the default; the owner may still answer), ANSWERED, DROPPED.
Blocks: the TASKs that cannot start until the question is answered, or `-` when the spec gives a safe default.
