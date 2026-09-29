# TASK-003 Outcome

## Outcome

Status: DONE

Changed:
- Added [plugins/skill-pins/index.client.tsx](plugins/skill-pins/index.client.tsx), [plugins/skill-pins/client/skills-menu.ts](plugins/skill-pins/client/skills-menu.ts), [plugins/skill-pins/client/queue.ts](plugins/skill-pins/client/queue.ts), [plugins/skill-pins/client/state.ts](plugins/skill-pins/client/state.ts), and [plugins/skill-pins/client/dom.ts](plugins/skill-pins/client/dom.ts).
- Added [plugins/skill-pins/tests/skills-menu.test.ts](plugins/skill-pins/tests/skills-menu.test.ts) and [plugins/skill-pins/tests/queue.test.ts](plugins/skill-pins/tests/queue.test.ts).
- `SkillPins.bindQueue` in [plugins/skill-pins/server/state.ts](plugins/skill-pins/server/state.ts) now drops a second snapshot for a queue row that is already bound. A test was added to [plugins/skill-pins/tests/state.test.ts](plugins/skill-pins/tests/state.test.ts).
- [plugins/skill-pins/package.json](plugins/skill-pins/package.json): client paths added to `lint` and `format`, and `linkedom` added for DOM tests.
- The design changed from the spec. The plan captured a snapshot on Enter. `prompt-translate` stops the original Enter and replays its own, so the two capture listeners would race or record twice. The client now snapshots each new queue row instead. A direct send uses `skills.json`, which each toggle saves first. The spec, [plugins/skill-pins/README.md](plugins/skill-pins/README.md), and this TASK's Verify lines were updated to match.

Contract:
- The pill sits before the `prompt-translate` Caveman pill, or before the model selector. It never moves the Caveman pill.
- The plugin installs one document `click` listener and no keydown listener. It never calls `preventDefault` on host events.
- Rows that exist before the plugin loads get no snapshot.

Verified:
- `npx tsc --noEmit` -> exit 0.
- `S=lint; npm run $S` -> 0 warnings, 0 errors on 17 files.
- `npm test` in [plugins/skill-pins/](plugins/skill-pins/) -> 37 pass, 0 fail. The tests cover label, toggle and save, menu staying open, Escape, disabled pill without an agent ID, other-host composer, pill order, queue snapshot and bind, pre-existing rows, and Edit cancel.
- `npm test` in [plugins/prompt-translate/](plugins/prompt-translate/) -> 77 pass, 0 fail.
- Not verified here: behavior in the real Paseo desktop. TASK-004 covers it.

Anti-goal:
- Final: 0 ms added Enter delay. The only document listener is `click`; `tests/queue.test.ts`, 2026-09-29 11:54.
- Result: PASS against the 0 ms limit.
