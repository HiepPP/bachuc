# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                                                                                                     | Default                                                         | Status    | Answer |
| ----- | -------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------- | --------- | ------ |
| Q-192 | TASK-307 | -      | The live probe of TASK-305 shows 19 `hook` rows for one short turn, because the owner has many `SessionStart`, `UserPromptSubmit`, and `Stop` hooks. Should GPUI fold neighbouring `hook` rows into one row? | No fold; each non-tool hook keeps its own row, as ADR-0068 says | DEFAULTED | -      |
