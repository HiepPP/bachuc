# TASK-071 Outcome

## Outcome

Status: DONE

Changed:

- `client/actions.ts`: `AgentCandidate` includes `provider`; `chooseAgent` skips agents whose provider starts with `mock`.
- `client/needs-you.tsx`: pressing a row's text toggles its full text and, for a question, its default; one row opens at a time; "Actions go to X" is a link that opens thread X.
- `client/overview.tsx`: passes `navigation.openAgent` to `NeedsYouCell`.
- `tests/actions.test.ts`: a newer idle mock and an older idle real agent -> the real agent; only a mock -> none.
- `README.md`: target rule, link, and expand behavior, linking ADR-0013.
- `client/agent-target.ts` needed no change: the agent list already carries `provider`.

Contract:

- A prompt still never goes to a working agent.

Verified:

- `npm run typecheck` -> exit 0; `npm run-script lint` -> 0 warnings, 0 errors; `npm test` -> 75 pass, 1 fail (the pre-existing proposed-ADR case in `tests/overview.test.ts`).
- Live check, 2026-10-12, Expo web on daemon 6768, light theme, workspace `wks_a70c2b9cc272dae9` with a real Claude Haiku agent ("Watchtower target check", idle) and a newer mock agent ("Mock newer"):
  - Q-024 expanded to its full sentence and "Default: No. Keep 50; Timeline says "Older threads are not kept" when the window lost some." Screenshot `/tmp/nyu-wt.png`.
  - "Actions go to Watchtower target check" was an underlined link; pressing it opened that thread.
  - After "Mock newer" went idle and was the newest agent, the target stayed "Watchtower target check".
- Both test agents were archived after the check.

Anti-goal:

- Before: 74 pass, 1 fail (pre-existing); 2026-10-12.
- Final: 75 pass, 1 fail, the same case; 2026-10-12.
- Result: PASS, 0 new failures.

Lessons:

- A Claude test agent asked to run `git branch --show-current` because of the global recap rule; deny it to end the turn quickly.
