# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                   | Default                                                                                                                | Status | Answer |
| ----- | -------- | ------ | -------------------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------- | ------ | ------ |
| Q-034 | TASK-089 | -      | If Metro cannot split `import("three")` into a lazy web chunk, may `three` (about 150 KB gzip) sit in the main web bundle? | Yes. Electron loads the bundle from disk, so there is no network cost. Record the measured size change in the outcome. | OPEN   | -      |

Status: OPEN, DEFAULTED (a run took the default; the owner may still answer), ANSWERED, DROPPED.
Blocks: the TASKs that cannot start until the question is answered, or `-` when the spec gives a safe default.
