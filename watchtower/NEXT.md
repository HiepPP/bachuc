# NEXT

## Current Active Plan

- Title: Native GLM provider on the Claude client
- Slug: 20261014-native-glm-provider
- Status: ACTIVE
- Updated: 2026-10-14

## Tracker

One row per TASK. Group ties together items that ship as one transaction.

| Order | TASK                                   | Group | Status | Spec                                                                                                     | Deps               | Context                                        | Notes                                                    |
| ----- | -------------------------------------- | ----- | ------ | -------------------------------------------------------------------------------------------------------- | ------------------ | ---------------------------------------------- | -------------------------------------------------------- |
| 1     | TASK-104 Built-in glm provider         | A     | TODO   | [watchtower/tasks/TASK-104-built-in-glm-provider.md](watchtower/tasks/TASK-104-built-in-glm-provider.md) | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Server and protocol. Owns the key and config dir helper. |
| 2     | TASK-105 GLM quota on the glm provider | B     | TODO   | [watchtower/tasks/TASK-105-glm-quota.md](watchtower/tasks/TASK-105-glm-quota.md)                         | TASK-104           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Imports the key helper from TASK-104.                    |
| 3     | TASK-106 GLM in the app                | C     | TODO   | [watchtower/tasks/TASK-106-glm-in-the-app.md](watchtower/tasks/TASK-106-glm-in-the-app.md)               | TASK-104           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Icon and Claude-only stream gates.                       |
| 4     | TASK-107 GLM docs                      | D     | TODO   | [watchtower/tasks/TASK-107-glm-docs.md](watchtower/tasks/TASK-107-glm-docs.md)                           | TASK-104, TASK-105 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Docs follow shipped behavior.                            |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0.
- `npm run lint` -> exit 0.
- On live, `npm run cli -- provider ls` -> a `glm` row with label `GLM` and status `available`.
- On live, run one glm agent with an image, `--thinking high`, and one subagent, as in the probe in [watchtower/CONTEXT.md](watchtower/CONTEXT.md) -> the image text is read, the Claude transcript under the glm config dir has `thinking` blocks, and every assistant `message.model` is a GLM id.
- In the same probe, ask the agent to list its tools -> `WebSearch` is not in the list.
- After the probe, `ls ~/.claude/projects | rg -c glm-native-probe` and `jq -r '.projects | keys[]' ~/.claude.json | rg -c glm-native-probe` -> no match for both. Other Claude sessions change `~/.claude.json`, so do not compare its hash.
- In the live app, open the provider usage list -> one GLM card with the GLM icon. Needs a human or a browser check.

## Handoff

- Next action: run `/watchtower implement` to build TASK-104 first.

## Archive

- None.
