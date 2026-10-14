# Learn 20261014-native-glm-provider

## Summary

Discrepancy: 4 found. All four TASKs shipped as planned in behavior, but two specs missed a file, one spec had a wrong expected result, and the live checks moved out of the run.

## Per TASK

- TASK-104: plan listed 5 files -> shipped those plus [packages/protocol/src/provider-config.ts](packages/protocol/src/provider-config.ts). Mistake: the plan knew only two built-in id lists, but the protocol config schema has a third. Without it, a user `agents.providers.glm` entry fails validation. Fix: list all built-in id lists when a plan adds a provider; see the MEMORY.md learning of 2026-10-14.
- TASK-104: plan said the model alias env vars follow the chosen model -> shipped them fixed to `glm-5.3-flash`. Mistake: the spec gave no source for the session model at factory time. Fix: accept it, or add a later TASK if subagents must follow the main GLM model.
- TASK-105: match. Q-042 took its default: the usage fetch reads only the daemon env key.
- TASK-106: plan listed 3 files -> shipped those plus [packages/app/src/components/provider-icon-name.ts](packages/app/src/components/provider-icon-name.ts). Mistake: the plan did not trace how the icon id resolves. Fix: the lead allowed it in the builder prompt; trace the full icon path at plan time.
- TASK-106: Expected result 3 said a codex `TodoWrite` builds no todo list -> the generic parser builds one for every provider. Mistake: the plan read only the `stream.ts` branch, not the parser. Fix: the lead rewrote the line at implement time.
- TASK-107: match.

## Plan-Level

- The verify lines of TASK-104 and Plan Verify asked for a live daemon restart, but the carried-over repo rule forbids it during implement. All live checks stayed UNVERIFIED until the owner asked for them after the run. They then passed on 2026-10-14, except the icon on screen.

## Lessons

- Before a plan writes a live check, read the `## Repo Rules` that implement will carry over. Put a live check that needs a restart into a separate owner step.
- Trace a provider id through every list that names built-ins: protocol manifest, protocol config, persisted config, and the app icon resolver.
