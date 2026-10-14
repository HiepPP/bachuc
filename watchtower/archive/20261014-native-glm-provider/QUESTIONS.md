# Questions

Owner questions of the active plan. One row per question. Specs and outcomes cite the ID instead of repeating the text.

| ID    | TASK     | Blocks | Question                                                                                                                                                              | Default                                                                        | Status    | Answer |
| ----- | -------- | ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------ | --------- | ------ |
| Q-042 | TASK-105 | -      | Should the GLM usage fetch also read a key set in the glm provider config env (`ANTHROPIC_AUTH_TOKEN`), not only `ZAI_API_KEY` and `GLM_API_KEY` from the daemon env? | The fetcher has no access to provider config, so it reads the daemon env only. | DEFAULTED | -      |

Status: OPEN, DEFAULTED (a run took the default; the owner may still answer), ANSWERED, DROPPED.
Blocks: the TASKs that cannot start until the question is answered, or `-` when the spec gives a safe default.
