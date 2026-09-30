# TASK-002 Keep a qmd index of exported threads

Group: A (index.server.ts is shared with TASK-001)
Class: code

## Brief

Goal: Keep the qmd index `paseo-threads` current, so `qmd --index paseo-threads search` finds new turns within about 1 minute.

Change: exported files are not searchable -> the plugin adds them to a named qmd index and refreshes it after exports.

How:

- Add `server/qmd.ts`. Find the qmd binary: first `QMD_BIN`, then each folder in `PATH`, then `/opt/homebrew/bin/qmd` and `/usr/local/bin/qmd`. The daemon `PATH` may not include Homebrew.
- On start, run `qmd --index paseo-threads collection list`. If it has no `paseo-threads` collection, run `qmd --index paseo-threads collection add <exportDir> --name paseo-threads --mask "**/*.md"`.
- After `onExported` fires, schedule `qmd --index paseo-threads update`. Run it at most once per 30 seconds. Never run two at once. If an export lands during a run, run once more after it.
- Run the startup update after the backfill from TASK-001 finishes.
- Spawn with an argument list, never a shell string. Use a 120-second timeout. Log the exit status and duration. Do not log qmd output, because it can quote thread text.
- If qmd is not found, log one line and do nothing more. Export must keep working.
- Make the spawn function injectable, so tests use a fake runner.
- Wire it in [plugins/thread-context-attach/index.server.ts](plugins/thread-context-attach/index.server.ts). Stop timers and kill a running qmd process in the cleanup function.

Files:

- [plugins/thread-context-attach/server/qmd.ts](plugins/thread-context-attach/server/qmd.ts) (new binary lookup, collection setup, debounced update)
- [plugins/thread-context-attach/index.server.ts](plugins/thread-context-attach/index.server.ts) (wire the indexer to the exporter)
- [plugins/thread-context-attach/tests/qmd.test.ts](plugins/thread-context-attach/tests/qmd.test.ts) (new tests with a fake runner)

Expected result:

- The first start creates the `paseo-threads` collection and indexes all exported files.
- The default qmd index and its 8 collections are not touched.
- Search finds a new turn within about 1 minute after the turn ends.

## Verify

- `cd plugins/thread-context-attach && npm test` -> all pass. New tests cover: binary lookup order, collection added only when missing, one update per 30 seconds, no overlap, one extra run after a busy export, and qmd missing.
- `cd plugins/thread-context-attach && npm run typecheck` -> exit 0.
- `cd plugins/thread-context-attach && S=lint; npm run $S` -> exit 0.
- After reload and 1 minute: `qmd --index paseo-threads collection list` -> lists `paseo-threads`.
- `qmd --index paseo-threads search "watchtower sidebar" -n 3` -> the results include `241e4fc4-e2c5-4c6b-9ca6-6fe83b34b984.md`.
- `qmd status` -> the default index still shows 7,665 files and the same collection list as before.
