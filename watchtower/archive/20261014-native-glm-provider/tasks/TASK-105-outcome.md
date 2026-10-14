# TASK-105 Outcome

## Status

DONE. The live app check is UNVERIFIED (unattended), because AUTORUN forbids a live daemon restart.

## Changed

- `packages/server/src/services/quota-fetcher/providers/zai.ts`: `providerId` is `glm`, `displayName` is `GLM`, and the key comes from `resolveGlmApiKey()`. The endpoint and response schema are unchanged.
- `packages/server/src/services/quota-fetcher/manifest.ts`: the row id is `glm`.
- `packages/server/src/services/quota-fetcher/service.test.ts`: the Z.ai case now finds `glm` and checks the `GLM` label. One new case checks that `glm` is `unavailable` without a key.

## Contract

- The usage list has a `glm` entry and no `zai` entry. The class name `ZaiQuotaProvider` and the file name `zai.ts` stay, because the spec lists that path.
- The fetcher calls `resolveGlmApiKey()` with no arguments, so it reads `ZAI_API_KEY`, then `GLM_API_KEY`, from the daemon env. The fetcher has no access to provider config, so a config `ANTHROPIC_AUTH_TOKEN` is not used for usage.
- No key: the `glm` entry is `unavailable`.

## Verified

Measured 2026-10-14. Typecheck ran once as the group-level run for group B.

- `npx vitest run src/services/quota-fetcher/service.test.ts --bail=1` (from `packages/server`) -> PASS (72) FAIL (0).
- `rg -n '"zai"' packages/server/src/services/quota-fetcher` -> no match (rg exit 1).
- `npm run typecheck` -> exit 0 (group-level, run with the group C app edits in the tree).
- `rtk proxy npm run lint -- <3 changed files>` -> exit 0, 0 warnings, 0 errors. Plain `npm run lint` exits 2 with the known rtk wrapper JSON error.
- `npm run format:files -- <3 changed files>` -> ran, 3 files.
- GitNexus pre-edit impact: `ZaiQuotaProvider` LOW (2 direct importers: manifest; 6 total), 0 processes.
- Live provider usage settings GLM card -> UNVERIFIED (unattended).

## Anti-goal

Drift gauge: `rg -c "providerId:" packages/server/src/services/quota-fetcher/manifest.ts`.

| Checkpoint     | Value | Source        | Time (UTC)           |
| -------------- | ----- | ------------- | -------------------- |
| Before changes | 8     | command above | 2026-10-14T07:41:50Z |
| At completion  | 8     | command above | 2026-10-14T07:42:15Z |

Result: PASS. The count is the same as the baseline.

Lead re-check, wave 2: the count is 8, `service.test.ts` passes (72), and `rg -n '"zai"'` has no match. Commit `441d73bd2`. The config-key question is Q-042 (DEFAULTED).

## Lessons

- The `rg -n '"zai"'` verify line matches any test that asserts the old id is gone. Keep that assertion out of the test file.
- `ZaiQuotaProvider` reads the key from env only. A config-level key set for the glm agent does not reach the usage fetch.
