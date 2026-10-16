# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                    | Default                                                                          | Status    | Answer |
| ----- | -------- | ------ | --------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | --------- | ------ |
| Q-089 | TASK-166 | -      | ADR-0044 and the spec Change line say "role and label" for `input-state`; the Boundaries planner call says no label. Which? | `Role::Group` with no label, so the outer `TextInput` frame keeps the only label | DEFAULTED | -      |
| Q-090 | TASK-157 | -      | What does `checkout_diff` return for a folder that is not a git repo?                                                       | An empty list, as `checkout_status` does; other checkout errors fail             | DEFAULTED | -      |
| Q-091 | TASK-157 | -      | How does the client map a diff file status that a newer daemon adds?                                                        | `FileDiffStatus::Binary`, so the file shows a placeholder with no diff           | DEFAULTED | -      |
| Q-092 | TASK-163 | -      | The Electron feature colors (palette 400) have no theme token. Which colors?                                                | Fast mode `status_warning`, auto accept `status_success`, plan mode `accent`     | DEFAULTED | -      |
| Q-093 | TASK-163 | -      | Does a click on the composer diff row open the Changes tab?                                                                 | No: `ComposerEvent` stays unchanged, so the row is not clickable                 | DEFAULTED | -      |
| Q-094 | TASK-163 | -      | What does a select show when the daemon value is not one of its options?                                                    | The feature label, as Electron does                                              | DEFAULTED | -      |
