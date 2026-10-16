# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                    | Default                                                                          | Status    | Answer |
| ----- | -------- | ------ | --------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | --------- | ------ |
| Q-089 | TASK-166 | -      | ADR-0044 and the spec Change line say "role and label" for `input-state`; the Boundaries planner call says no label. Which? | `Role::Group` with no label, so the outer `TextInput` frame keeps the only label | DEFAULTED | -      |
| Q-090 | TASK-157 | -      | What does `checkout_diff` return for a folder that is not a git repo?                                                       | An empty list, as `checkout_status` does; other checkout errors fail             | DEFAULTED | -      |
| Q-091 | TASK-157 | -      | How does the client map a diff file status that a newer daemon adds?                                                        | `FileDiffStatus::Binary`, so the file shows a placeholder with no diff           | DEFAULTED | -      |
