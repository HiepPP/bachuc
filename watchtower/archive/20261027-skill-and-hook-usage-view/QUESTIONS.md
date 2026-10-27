# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                                                                                                     | Default                                                                  | Status    | Answer |
| ----- | -------- | ------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ | --------- | ------ |
| Q-192 | TASK-307 | -      | The live probe of TASK-305 shows 19 `hook` rows for one short turn, because the owner has many `SessionStart`, `UserPromptSubmit`, and `Stop` hooks. Should GPUI fold neighbouring `hook` rows into one row? | No fold; each non-tool hook keeps its own row, as ADR-0068 says          | DEFAULTED | -      |
| Q-193 | TASK-307 | -      | Should a `hook` row also show the Zap badge with count 1 and its own Hooks section?                                                                                                                          | No; the row is one hook, and its output already shows as the detail text | DEFAULTED | -      |
| Q-194 | TASK-307 | -      | Step summaries now count `hook` rows as tools, for example "ran 1 command, used 9 other tools". Should the summary leave out `hook` rows or count them as hooks?                                             | Keep the current count; no TASK changed the summary                      | DEFAULTED | -      |
