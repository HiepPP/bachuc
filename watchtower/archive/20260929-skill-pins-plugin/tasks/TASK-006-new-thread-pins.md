# TASK-006 Pins in new-thread composers

Group: E (shares [plugins/skill-pins/index.server.ts](plugins/skill-pins/index.server.ts) and [plugins/skill-pins/server/skill-pins-hook.cjs](plugins/skill-pins/server/skill-pins-hook.cjs) with TASK-005)
Class: risky

## Brief

Goal: Let the user pin skills in a new-thread composer. The first turn of the new agent loads them.

Change: disabled pill in new-thread composers -> working pill whose pins carry into the created agent.

Design: [plugins/skill-pins/README.md](plugins/skill-pins/README.md) (sections Composer control, Compatibility and limits, Open questions Q1)

Boundaries:

- Do not prepend text to the prompt. The bubble stays unchanged.
- The new-thread pill starts from the default set of TASK-005.
- Each toggle saves a host-level draft through a new RPC, for example `skill-pins.draft`. Store it as `draft.json` with `createdAt`. Use mode `0600` and atomic writes.
- `before("agent.create")` consumes a draft younger than 10 minutes and puts it in `SKILL_PINS_INITIAL`. Otherwise it uses the default set. A consumed draft is deleted.
- Known limit: two new threads created within the draft window share one draft. Document it; do not add per-composer keys in this TASK.
- Keep the pill scoped to the composer's own host, as today.

How:

- Remove the new-thread disable path in [plugins/skill-pins/client/skills-menu.ts](plugins/skill-pins/client/skills-menu.ts). Keep a client draft selection when the composer has no agent ID.
- Add the draft RPC contract, server state, and the consume step in `before("agent.create")`.
- Add tests for draft expiry and consume-once. Update the README.

Files:

- [plugins/skill-pins/client/skills-menu.ts](plugins/skill-pins/client/skills-menu.ts) (enable the pill without an agent ID)
- [plugins/skill-pins/client/state.ts](plugins/skill-pins/client/state.ts) (draft selection)
- [plugins/skill-pins/shared/contracts.ts](plugins/skill-pins/shared/contracts.ts) (draft RPC)
- [plugins/skill-pins/server/state.ts](plugins/skill-pins/server/state.ts) (draft file)
- [plugins/skill-pins/index.server.ts](plugins/skill-pins/index.server.ts) (draft handler, consume on create)
- plugins/skill-pins/tests/ (new cases)
- [plugins/skill-pins/README.md](plugins/skill-pins/README.md) (remove the new-thread limit, close Q1, document the shared draft limit)

Expected result:

- In a new-thread composer, the pill opens and toggles skills.
- After send, the new agent's first turn calls the pinned skills, and its pill shows them.
- The user bubble shows only the typed prompt.

Anti-goal: `draft.json` never outlives one create or 10 minutes; tripwire; read by listing `$PASEO_HOME/plugin-data/skill-pins/` after the live check and after an expiry test.

## Verify

- `cd plugins/skill-pins && npm run typecheck && S=lint; npm run $S && npm test` -> exit 0.
- Test: a draft older than 10 minutes -> create uses the default set.
- Test: two creates after one draft -> only the first gets the draft.
- `paseo plugin reload skill-pins && paseo plugin ls skill-pins --json` -> `running`, no load error.
- Manual in Paseo desktop (needs a human): pin Sequential thinking in a new-thread composer and send -> the timeline shows `[Skill] sequential-thinking` on the first turn. The bubble has no added text.
- `ls "$PASEO_HOME/plugin-data/skill-pins/"` after the live check -> no `draft.json`.
