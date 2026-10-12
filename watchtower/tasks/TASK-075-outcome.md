# TASK-075 Outcome

Status: DONE (auto: manual checks pending)

Autorun iteration 4: the lead re-ran the storage test (82 passed), typecheck, lint, and the format check on both files in one call, all exit 0. The test diff removes only the empty-default expectation. An unknown stored level still falls back to `detailed` through the schema `.catch`; that path was not in scope.

## Changed files

- packages/app/src/hooks/use-settings/storage.ts: `DEFAULT_CLIENT_SETTINGS.toolCallDetailLevel` `detailed` -> `overview`; migration fallback is now `compactToolCalls === false ? "detailed" : "overview"`.
- packages/app/src/hooks/use-settings/storage.test.ts: empty-default expectation `detailed` -> `overview`; new cases for stored `detailed`, stored `concise`, `compactToolCalls: false`.

No other file hard-codes the default (`rg -n toolCallDetailLevel packages/app/src`: only view.tsx and chat-section.tsx read the value).

## Mapping answer

`compactToolCalls: false` with no level -> `detailed` (an explicit old choice stays). Absent `compactToolCalls` and no level -> `overview` (new default). Stored `compactToolCalls: true` -> `overview` (unchanged). Stored unrecognized level still falls to `detailed` via the schema `.catch` (unchanged, not touched).

## Verify

- `cd packages/app && npx vitest run src/hooks/use-settings/storage.test.ts --bail=1` -> pass, 82 tests (79 before the change).
- `npm run typecheck` -> exit 0.
- `npm run lint -- <both files>` -> the npm output is filtered by the rtk wrapper ("ESLint output (JSON parse failed)"), so no result shown. `npx oxlint <both files>` -> 0 warnings, 0 errors, exit 0.
- `npm run format:files` and `npm run format:check:files` -> each file reports "correct format" when run one file per call (two files in one call printed "Expected at least one target file").
- Anti-goal: `git diff -- packages/app/src/hooks/use-settings/storage.test.ts` -> one changed expectation line (empty-blob default `detailed` -> `overview`), plus 3 added cases.

## Impact

`node .gitnexus/run.cjs impact DEFAULT_CLIENT_SETTINGS --direction upstream --repo .` -> risk UNKNOWN, 0 callers resolved (index stale; not an all-clear). Text search shows readers only through `useSettings`. No hang.

## Anti-goal

Result: pass. Storage test read before (79 pass) and after (82 pass). Only the empty-blob default expectation changed; stored `detailed`, `concise`, `compactToolCalls: true/false`, and unrecognized values map as before.

Lessons: `npm run lint` output is swallowed by the rtk wrapper here; use `npx oxlint` to see the result. Pass one file per `format:files` call.
