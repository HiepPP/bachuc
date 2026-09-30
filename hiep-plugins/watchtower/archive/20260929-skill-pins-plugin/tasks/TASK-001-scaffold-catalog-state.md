# TASK-001 Scaffold, catalog, and server state

Group: A (writes the plugin scaffold, shared/, server/state.ts, index.server.ts)
Class: code

## Brief

Goal: Create the `skill-pins` plugin with its fixed catalog, RPC contracts, and per-agent state on the daemon.

Change: no plugin -> installable plugin that stores pinned skills and pending send snapshots per agent.

Design: [plugins/skill-pins/README.md](plugins/skill-pins/README.md) (sections Catalog, State, Pending snapshots, RPC contracts)

Boundaries:

- Start only after Q0 in [watchtower/CONTEXT.md](watchtower/CONTEXT.md) is resolved.
- Keep the existing [plugins/skill-pins/README.md](plugins/skill-pins/README.md). Do not let `paseo plugin init` overwrite it.
- Copy the atomic write, UUID check, token format, and queue binding from [plugins/prompt-translate/server/modes.ts](plugins/prompt-translate/server/modes.ts).
- Snapshots store `{ hash, skills, createdAt }` only, never prompt text.
- Unknown skill IDs are rejected on write.

How:

- Scaffold with `paseo plugin init` into a temp folder, then copy the files that are missing. Keep the README.
- Add `shared/catalog.ts` with the three entries: `id`, `label`, and Claude skill name.
- Add `shared/contracts.ts` with the seven RPCs from the README table, using the listed wire names.
- Add `server/state.ts` with `get`, `set`, `prepare`, `cancel`, `bindQueue`, and `cancelQueue`.
- Wire the handlers in `index.server.ts`, with cleanup returned.
- Set `requirements.paseo` to `^0.8.0 || >=0.9.0-beta.2`, matching prompt-translate.
- Add `tests/state.test.ts` and `tests/catalog.test.ts`.

Files:

- [plugins/skill-pins/paseo-plugin.json](plugins/skill-pins/paseo-plugin.json) (new manifest)
- [plugins/skill-pins/package.json](plugins/skill-pins/package.json) (new; scripts match prompt-translate)
- [plugins/skill-pins/package-lock.json](plugins/skill-pins/package-lock.json) (new)
- [plugins/skill-pins/tsconfig.json](plugins/skill-pins/tsconfig.json) (new)
- [plugins/skill-pins/index.server.ts](plugins/skill-pins/index.server.ts) (new; RPC handlers)
- [plugins/skill-pins/shared/catalog.ts](plugins/skill-pins/shared/catalog.ts) (new)
- [plugins/skill-pins/shared/contracts.ts](plugins/skill-pins/shared/contracts.ts) (new)
- [plugins/skill-pins/server/state.ts](plugins/skill-pins/server/state.ts) (new)
- [plugins/skill-pins/tests/state.test.ts](plugins/skill-pins/tests/state.test.ts) (new)
- [plugins/skill-pins/tests/catalog.test.ts](plugins/skill-pins/tests/catalog.test.ts) (new)

Expected result:

- `get` on an agent with no file returns `{ skills: [] }`.
- `set` with an unknown skill ID throws. `set` with a bad agent ID throws.
- `prepare` writes `pending/<token>.json` with the SHA-256 of the CRLF-normalized text and no text field.
- `bindQueue` then `cancelQueue` removes both the `.json` and `.queue` files.
- `bindQueue` after the snapshot is already consumed removes the new `.queue` file.

Anti-goal: Stored bytes per snapshot stay at or under 512 bytes, with no prompt text; tripwire; read from the `state.test.ts` snapshot-size test at completion.

## Verify

- `cd plugins/skill-pins && npm run typecheck` -> exit 0.
- `cd plugins/skill-pins && S=lint; npm run $S` -> exit 0.
- `cd plugins/skill-pins && npm test` -> state and catalog tests pass.
- `git diff --stat -- plugins/skill-pins/README.md` -> no change from the scaffold step.
- Snapshot-size test in `tests/state.test.ts` -> file size is at most 512 bytes and the file does not contain the prompt text.
