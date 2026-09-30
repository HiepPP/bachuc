# TASK-002 Outcome

## Outcome

Status: DONE

Changed:
- [plugins/thread-context-attach/server/qmd.ts](plugins/thread-context-attach/server/qmd.ts): new `findQmd`, `createRunner`, and `createIndexer`.
- [plugins/thread-context-attach/index.server.ts](plugins/thread-context-attach/index.server.ts): starts the indexer at load, runs `update` after exports, and stops it on cleanup.
- [plugins/thread-context-attach/tests/qmd.test.ts](plugins/thread-context-attach/tests/qmd.test.ts): 4 new tests with a fake runner.

Contract:
- The named qmd index `paseo-threads` holds one collection, `paseo-threads`, over the export folder.
- `qmd --index paseo-threads update` runs at most once per 30 seconds, never overlaps, and runs once more after a busy export.
- qmd runs without a shell, with a 120-second timeout. Only exit codes and durations are logged.
- If qmd is missing, the plugin logs one line and export keeps working.

Deviation:
- The runner keeps the user's `PATH` order and adds qmd's folder at the end. The first build put qmd's folder first. That picked Homebrew `node` v25, and qmd's `better-sqlite3` module failed with an ABI mismatch (exit 1). qmd works with the user's first `node`, `~/.local/bin/node` v24.18.0.

Verified:
- `npm test` -> 16/16 pass, 3 runs in a row.
- `npm run typecheck` -> exit 0. `S=lint; npm run $S` -> 0 warnings, 0 errors. `oxfmt --check` -> exit 0.
- Plugin log after reload: `qmd collection add exit 0`, then `qmd update exit 0 in 176 ms`.
- `qmd --index paseo-threads collection list` -> lists `paseo-threads (qmd://paseo-threads/)`. Status shows 47 files.
- `qmd --index paseo-threads search "watchtower sidebar" -n 3 --files` -> first hit `241e4fc4-e2c5-4c6b-9ca6-6fe83b34b984.md`, score 0.80.
- `qmd status` -> the default index still shows 7,665 files. `~/.config/qmd/index.yml` is byte-identical to the copy taken before the run.
