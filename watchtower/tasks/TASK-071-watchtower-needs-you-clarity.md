# TASK-071 Watchtower Needs you: full text, target link, no mock targets

Group: W2 (`hiep-plugins/plugins/watchtower-board/client/needs-you.tsx`, `client/actions.ts`, `client/agent-target.ts`, `client/overview.tsx`, its tests and README)
Class: code

## Brief

Goal: On the Watchtower page, a question shows its whole text when pressed, the answer target is a link to its thread, and answers never go to a mock agent.

Change: 2-line title and plain "Actions go to X" with any newest idle agent -> expandable title, linked target, and real agents only.

Boundaries:

- Follows [ADR-0013](watchtower/decisions/ADR-0013-thread-input-only-in-board-needs-you.md).
- Pressing a question or check row toggles its full text; the expanded row also shows the default answer when there is one. One row is expanded at a time.
- "Actions go to X" opens thread X through `props.navigation.openAgent`; without navigation it stays plain text.
- `chooseAgent` skips agents whose provider starts with `mock`. When no agent fits, the cell reads "No agent in this workspace" as today, and the actions stay disabled.
- Keep the rule that a prompt never goes to a working agent.

How:

- Add `provider` to `AgentCandidate`, and filter mock providers in `chooseAgent`.
- Add the expand state and the default line in `client/needs-you.tsx`; pass `navigation` from `client/overview.tsx` to `NeedsYouCell`.
- Make the agent note a link when the target is ready.
- Add cases to `tests/actions.test.ts` and `tests/lifecycle.test.ts` where they fit; update the README Needs you text.

Files:

- [hiep-plugins/plugins/watchtower-board/client/actions.ts](hiep-plugins/plugins/watchtower-board/client/actions.ts) (skip mock agents)
- [hiep-plugins/plugins/watchtower-board/client/agent-target.ts](hiep-plugins/plugins/watchtower-board/client/agent-target.ts) (request the provider field if the lookup trims fields)
- [hiep-plugins/plugins/watchtower-board/client/needs-you.tsx](hiep-plugins/plugins/watchtower-board/client/needs-you.tsx) (expand, default, link)
- [hiep-plugins/plugins/watchtower-board/client/overview.tsx](hiep-plugins/plugins/watchtower-board/client/overview.tsx) (pass navigation)
- [hiep-plugins/plugins/watchtower-board/tests/actions.test.ts](hiep-plugins/plugins/watchtower-board/tests/actions.test.ts)
- [hiep-plugins/plugins/watchtower-board/README.md](hiep-plugins/plugins/watchtower-board/README.md)

Expected result:

- Q-024 shows its full sentence and "Default: No. Keep 50; ..." after a press.
- "Actions go to <thread>" opens that thread.
- With only a mock agent idle in the workspace, the target skips it; the newest real idle agent is chosen, or the cell says none.

Anti-goal: watchtower-board tests keep their count of failures at the 1 pre-existing proposed-ADR failure; tripwire; read from `npm test` before and at completion.

## Verify

- `cd hiep-plugins/plugins/watchtower-board && npm run typecheck && npm run-script lint` -> pass.
- `cd hiep-plugins/plugins/watchtower-board && npm test` -> only the pre-existing `tests/overview.test.ts` proposed-ADR case fails.
- A `tests/actions.test.ts` case: a newer idle mock agent and an older idle real agent -> the real agent is chosen.
- Human check on live (Expo web on daemon 6768): Q-024 expands to its full text with its default; the target link opens its thread.
