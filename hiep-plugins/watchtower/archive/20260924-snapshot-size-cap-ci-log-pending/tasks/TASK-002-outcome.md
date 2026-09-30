# TASK-002 Outcome

## Outcome

Status: DONE

Changed:
- [plugins/thread-branch/server/pr-search.ts](plugins/thread-branch/server/pr-search.ts): `fetchLog` treats `gh run view` as not ready when it exits 0 with under 1,000 characters of stdout and stdout or stderr matches `/still in progress/i`. That state is cached for `NOT_READY_TTL` (60 seconds) with no log line. Real logs and real failures keep the 5-minute cache and the one log line.
- [plugins/thread-branch/tests/pr-search.test.ts](plugins/thread-branch/tests/pr-search.test.ts): the fake runner can return a full result for `gh run view`, plus 3 new tests.
- [plugins/thread-branch/README.md](plugins/thread-branch/README.md): one bullet.

Contract:
- A running workflow never shows its notice as log text, and it is never logged as a failure.
- Once the log exists, a search after 60 seconds fetches it, and the next search offers the CI failure item.

Verified:
- `npm test` -> 57/57 pass. The two `treats a running workflow as not ready` tests (notice on stdout, notice on stderr) fail on the old code, when stashed. They cover no fetch at 59 seconds, a fetch at 61 seconds, the item after the log exists, and no log lines. `still logs a real CI log failure once and keeps it for 5 minutes` guards the old behavior.
- `npm run typecheck` -> exit 0. `S=lint; npm run $S` -> 0 warnings, 0 errors. oxfmt -> clean.
- Manual check with a running workflow: not done. No run was in progress on 2026-09-24 to test against.
