# TASK-001 Outcome

## Outcome

Status: DONE

Changed:
- [plugins/thread-context-attach/server/export.ts](plugins/thread-context-attach/server/export.ts): new `renderThreadMarkdown`, `readTimeline`, `writeIfChanged`, `exportThread`, `exportDir`, and `createExporter`.
- [plugins/thread-context-attach/index.server.ts](plugins/thread-context-attach/index.server.ts): exports on `agent.turn_ended`, starts one backfill on the first hook or RPC, and stops the exporter on cleanup.
- [plugins/thread-context-attach/tests/export.test.ts](plugins/thread-context-attach/tests/export.test.ts): 8 new tests.

Contract:
- One `<agentId>.md` per thread in `$PASEO_HOME/plugin-data/thread-context-attach/threads/`. The folder is mode 700 and files are mode 600.
- Only user messages and assistant replies are written, one `## Turn N` per turn. Replies before the first user message form Turn 0.
- `createExporter().onExported(listener)` fires after a changed export and once after the backfill.
- The composer picker code is unchanged.

Deviation:
- The backfill starts on the first hook or RPC, not at plugin load. A plugin gets a Paseo API only inside hooks and RPCs. The spec was updated.
- The cut line says turn numbers start at the oldest kept message. The real first turn number is unknown after a cut. The spec was updated.

Verified:
- `npm test` -> 12/12 pass (8 new).
- `npm run typecheck` -> exit 0.
- `S=lint; npm run $S` -> 0 warnings, 0 errors.
- After `paseo plugin reload thread-context-attach` and one `thread-context.list-threads` RPC: the log shows `backfill exported 47 of 48 threads` within 1 second. The folder holds 47 files. `stat` shows 700 for the folder and 600 for a file.
