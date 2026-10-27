# NEXT

## Current Active Plan

- Title: Show the skills and hooks a Claude agent used in its session
- Slug: 20261027-skill-and-hook-usage-view
- Status: ARCHIVED
- Updated: 2026-10-27

## Tracker

| Order | TASK                                                           | Group | Status | Spec                                                                                                                         | Deps     | Context                                        | Notes                                             |
| ----- | -------------------------------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------------------- | ------------------------------------------------- |
| 1     | TASK-305 Capture Claude hooks and skill use in the provider    | A     | DONE   | [watchtower/tasks/TASK-305-claude-hook-and-skill-capture.md](watchtower/tasks/TASK-305-claude-hook-and-skill-capture.md)     | -        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares protocol and SDK type files with TASK-306. |
| 2     | TASK-306 Count, store, and send the skill and hook summary     | A     | DONE   | [watchtower/tasks/TASK-306-daemon-skill-hook-usage-summary.md](watchtower/tasks/TASK-306-daemon-skill-hook-usage-summary.md) | TASK-305 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Same builder as TASK-305, after it.               |
| 3     | TASK-307 Show hooks in GPUI tool rows and hook rows            | B     | DONE   | [watchtower/tasks/TASK-307-gpui-timeline-hook-rows.md](watchtower/tasks/TASK-307-gpui-timeline-hook-rows.md)                 | TASK-305 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Start order only; no shared file.                 |
| 4     | TASK-308 Show the skill and hook summary as a GPUI header chip | C     | DONE   | [watchtower/tasks/TASK-308-gpui-header-usage-chip.md](watchtower/tasks/TASK-308-gpui-header-usage-chip.md)                   | TASK-306 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Start order only; no shared file.                 |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- Live end to end, UI checker: start live, run one Claude agent turn that calls a skill with `/<skill>` and uses Bash with the GitNexus hook. The Bash row shows the hook badge, and the header chip counts the skill and the hook.
- Restart the live daemon with `npm run dev`, then reopen the agent. The header chip shows the same counts.
- `rg -n "COMPAT\(" packages/protocol/src/messages.ts` -> no new untagged shim; the new fields are plain optional fields.

## Review Focus

- A hook that blocks a tool with exit code 2 -> the tool row shows the hook as error with its exit code; owner TASK-305 and TASK-307.
- Two Bash calls in one assistant message -> each hook lands on its own call, or the outcome states the limit; owner TASK-305.
- Daemon restart in the middle of a session -> the chip keeps its counts; owner TASK-306.
- Provider history replay or resume -> no count is added twice; owner TASK-306.
- A tab that is not an agent, or an agent with no use -> no chip; owner TASK-308.

## Handoff

- Run result (2026-10-27): all 4 TASKs DONE on `feat/skill-and-hook-usage-view`. Code commits: `002d06e34` (group A), `64509dded` (group B), `190c21300` (group C). Nothing pushed.
- Plan Verify: the live end-to-end check passed through the UI checker (a `/feynman` skill and the Bash hooks counted; the chip read "1 skill · 5 hooks"). The daemon restart check stayed UNVERIFIED (unattended), because the run config forbids a live restart; the agent-manager reload test covers it. No new `COMPAT(` tag was needed.
- Next action: the owner tests on live, then reviews and merges the branch to `main`.

## Archive

- See [watchtower/archive/](watchtower/archive/).
- Archived: 2026-10-27 -> watchtower/archive/20261027-skill-and-hook-usage-view/
