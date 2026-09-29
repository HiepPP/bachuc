# TASK-003 Composer skills pill

Group: C (writes client/ and index.client.tsx; ordered after TASK-001 by dependency only)
Class: risky

## Brief

Goal: Add a multi-select `Skills` pill to the desktop composer that saves the selection, and snapshot the selection for each queued prompt.

Change: no composer control -> `Skills` or `Skills · N` pill with a checkbox menu.

Design: [plugins/skill-pins/README.md](plugins/skill-pins/README.md) (sections Composer control, Pending snapshots, Coexistence with prompt-translate)

Boundaries:

- Copy `ROOT`, `composerModeKey`, the `owns` host check, and menu styles from [plugins/prompt-translate/client/mode-menu.ts](plugins/prompt-translate/client/mode-menu.ts) and [plugins/prompt-translate/client/agent-mode.ts](plugins/prompt-translate/client/agent-mode.ts).
- The plugin never calls `preventDefault` on host events and never changes the draft. A failed RPC must not block anything.
- A click toggles one skill and keeps the menu open. Esc or an outside click closes it.
- The pill is disabled when the composer has no agent ID.
- The pill must not break the prompt-translate Caveman menu or its Cmd/Ctrl+Enter enhance.

How:

- Write `client/skills-menu.ts` for the pill and menu.
- Write `client/queue.ts` to snapshot each new queue row and cancel it on Edit. Revised during implement: a direct send needs no snapshot, so no Enter or send-button listener is installed.
- Wire both in `index.client.tsx`, with cleanup returned.
- Add `tests/skills-menu.test.ts` and `tests/queue.test.ts` with the same DOM fakes that prompt-translate tests use.

Files:

- [plugins/skill-pins/index.client.tsx](plugins/skill-pins/index.client.tsx) (new)
- [plugins/skill-pins/client/skills-menu.ts](plugins/skill-pins/client/skills-menu.ts) (new)
- [plugins/skill-pins/client/queue.ts](plugins/skill-pins/client/queue.ts) (new)
- [plugins/skill-pins/client/dom.ts](plugins/skill-pins/client/dom.ts) (new; copied DOM types and fiber helpers)
- [plugins/skill-pins/client/state.ts](plugins/skill-pins/client/state.ts) (new; per-agent selection cache)
- [plugins/skill-pins/package.json](plugins/skill-pins/package.json) (add client paths to lint and format; add `linkedom`)
- [plugins/skill-pins/tests/skills-menu.test.ts](plugins/skill-pins/tests/skills-menu.test.ts) (new)
- [plugins/skill-pins/tests/queue.test.ts](plugins/skill-pins/tests/queue.test.ts) (new)

Expected result:

- Label shows `Skills` with none pinned and `Skills · 2` with two pinned.
- Toggling calls `skillsWriteRpc` with the full set, and the menu stays open.
- A new queue row calls `prepareSkillsRpc` with the row text and current set, then `bindQueueSkillsRpc` with the row ID.
- Edit on a queued row calls `cancelQueueSkillsRpc` and does not block the click.
- No agent ID -> the pill is disabled and no snapshot is made.
- Composer owned by another host -> no pill and no RPC call.

Anti-goal: Added delay between Enter and the host send stays at 0 ms, because the plugin installs no keydown listener; tripwire; read from `tests/queue.test.ts` at completion.

## Verify

- `cd plugins/skill-pins && npm run typecheck && S=lint; npm run $S` -> exit 0.
- `cd plugins/skill-pins && npm test` -> menu and queue tests pass, covering every case in Expected result.
- Test in `tests/queue.test.ts` -> the only document listener installed is `click`, so no keydown handler can delay Enter.
- `cd plugins/prompt-translate && npm test` -> still passes, which proves no shared file changed.
