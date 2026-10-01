# TASK-011 P3 skill-pins uses the prompt hook and a shared pill

Group: E (hiep-plugins/plugins/skill-pins)
Class: risky

## Brief

Goal: Pin skills per conversation with no DOM code and no native provider hook.

Change: DOM menu, queue snapshots, and a native `UserPromptSubmit` hook -> a shared composer pill with a popover and a `before("agent.prompt")` hook that appends the skill context.

Boundaries:

- The pill uses `addComposerPill` without `agentId`. Its popover gets the agent from the button context and toggles skills through the existing RPCs.
- The prompt hook appends the same context text the native hook wrote today, for every provider.
- Store selections under `plugin-data/skill-pins/v2/`, so the old native hook in `~/Projects/hiep-paseo-plugin` finds nothing for Paseo Dev agents.
- New agents start with the settings default. Draft selection and queue snapshots are removed.
- Keep [hiep-plugins/plugins/skill-pins/scripts](hiep-plugins/plugins/skill-pins/scripts) and the native hook file. Mark them in the README as for sessions outside Paseo.
- Raise `requirements.paseo` to `>=0.10.2-beta.900`.

How:

- Remove [hiep-plugins/plugins/skill-pins/client/dom.ts](hiep-plugins/plugins/skill-pins/client/dom.ts), [hiep-plugins/plugins/skill-pins/client/skills-menu.ts](hiep-plugins/plugins/skill-pins/client/skills-menu.ts), and [hiep-plugins/plugins/skill-pins/client/queue.ts](hiep-plugins/plugins/skill-pins/client/queue.ts).
- Add a pill popover component.
- Move the context text builder into a shared server module used by the prompt hook.
- Update tests and the README.

Files:

- [hiep-plugins/plugins/skill-pins/index.client.tsx](hiep-plugins/plugins/skill-pins/index.client.tsx) (pill)
- [hiep-plugins/plugins/skill-pins/index.server.ts](hiep-plugins/plugins/skill-pins/index.server.ts) (prompt hook)
- Files under [hiep-plugins/plugins/skill-pins/client](hiep-plugins/plugins/skill-pins/client), [hiep-plugins/plugins/skill-pins/server](hiep-plugins/plugins/skill-pins/server), [hiep-plugins/plugins/skill-pins/shared](hiep-plugins/plugins/skill-pins/shared), and [hiep-plugins/plugins/skill-pins/tests](hiep-plugins/plugins/skill-pins/tests)
- [hiep-plugins/plugins/skill-pins/paseo-plugin.json](hiep-plugins/plugins/skill-pins/paseo-plugin.json), [hiep-plugins/plugins/skill-pins/package.json](hiep-plugins/plugins/skill-pins/package.json), [hiep-plugins/plugins/skill-pins/README.md](hiep-plugins/plugins/skill-pins/README.md)

Expected result:

- The pill shows on every agent composer on desktop and mobile.
- A turn of an agent with pinned skills carries the skill context. The user bubble shows only the typed text.

Anti-goal: The injected context text stays the same; tripwire; limit the text for a fixed skill set equals the native hook output for the same set; read with a test that compares both at completion.

## Verify

- `rg -n "querySelector|MutationObserver|document\." hiep-plugins/plugins/skill-pins/client hiep-plugins/plugins/skill-pins/index.client.tsx` -> no match.
- `cd hiep-plugins/plugins/skill-pins && npm run typecheck && npm run lint && npm test` -> exit 0.
- Real host check in Paseo Dev (TASK-017): pin a skill, send a prompt, the agent reports the pinned context.
- Anti-goal: the context comparison test passes.
