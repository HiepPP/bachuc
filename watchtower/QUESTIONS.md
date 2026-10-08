# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK               | Blocks   | Question                                                                     | Default                                                                                     | Status    | Answer                                                                       |
| ----- | ------------------ | -------- | ---------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------- | --------- | ---------------------------------------------------------------------------- |
| Q-001 | TASK-002, TASK-003 | -        | Is the display name `Bachuc` or `Ba Chúc`?                                   | `Bachuc` everywhere. Plain ASCII keeps paths, bundle names, and scripts simple.             | ANSWERED  | `Bachuc` everywhere (2026-10-08).                                            |
| Q-002 | TASK-001           | TASK-004 | Which logo concept does the owner pick?                                      | none                                                                                        | ANSWERED  | Concept 1, the sugar palm, in `/tmp/bachuc-logo/concept-1.png` (2026-10-08). |
| Q-003 | TASK-003           | -        | What is the desktop bundle ID?                                               | `io.github.hieppp.bachuc`, based on the owner's GitHub account `HiepPP`.                    | ANSWERED  | `io.github.hieppp.bachuc` (2026-10-08).                                      |
| Q-004 | TASK-007           | -        | What happens to the Discord, Sponsors, upstream GitHub, and changelog links? | Remove them from the UI. Keep the docs links to `paseo.sh/docs`.                            | ANSWERED  | Remove them from the UI. Keep the docs links (2026-10-08).                   |
| Q-005 | TASK-003           | -        | Is the macOS executable renamed to `Bachuc` too, as the spec asked?          | Keep `Paseo`. The bundle file name follows it, and five packaging files expect `Paseo.app`. | DEFAULTED | -                                                                            |

Status: OPEN, DEFAULTED (a run took the default; the owner may still answer), ANSWERED, DROPPED.
Blocks: the TASKs that cannot start until the question is answered, or `-` when the spec gives a safe default.
