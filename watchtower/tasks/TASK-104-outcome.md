# TASK-104 Outcome

## Status

DONE. The live verify lines are UNVERIFIED (unattended), because AUTORUN forbids a live daemon restart. Unit tests prove the behavior.

## Changed

- `packages/protocol/src/provider-manifest.ts`: `glm` definition after `claude`, label `GLM`, Claude modes, default mode `auto`, no voice.
- `packages/protocol/src/provider-config.ts`: `glm` added to the built-in id list. Required dependency fix: without it, `ProviderOverridesSchema` rejects a user `agents.providers.glm` entry that has no `extends` and `label`.
- `packages/server/src/server/persisted-config.ts`: `glm` added to the built-in id list.
- `packages/server/src/server/agent/providers/glm/runtime.ts` (new): `resolveGlmApiKey`, `resolveGlmConfigDir`, `resolveGlmRuntime`, `GLM_MODELS`, `GLM_BASE_URL`, `GLM_MISSING_KEY_REASON`, `GLM_DISALLOWED_TOOLS`.
- `packages/server/src/server/agent/provider-registry.ts`: `GlmAgentClient` (a `ClaudeAgentClient` subclass that gates availability on the key), the `glm` factory, `glm` in `PROVIDER_CONTRACTS` with the Claude contract, and `BUILTIN_PROFILE_MODELS` for the static model list.
- `packages/server/src/server/agent/provider-registry.test.ts`: 8 new glm cases. One existing test now filters out the glm Claude client when it counts Claude client constructions.

## Contract

- `resolveGlmApiKey({ providerEnv?, env? }): string | null` in `packages/server/src/server/agent/providers/glm/runtime.ts`. Order: `providerEnv.ANTHROPIC_AUTH_TOKEN`, then `env.ZAI_API_KEY`, then `env.GLM_API_KEY`. Blank values count as missing. `env` defaults to `process.env` and accepts any `Record<string, string | undefined>`.
- The glm factory merges the glm runtime settings under the user settings. User `env` wins, and `disallowedTools` is `["WebSearch", ...user]`.
- The glm client keeps provider id `claude` internally. `createResolvedProviderClient` sees a different id and wraps it with `wrapClientProvider`, which maps ids to `glm`. Session config sent to the Claude client still says `claude`, so lines 1716 and 989 in the Claude agent need no change.
- Session env: `CLAUDE_CONFIG_DIR=$PASEO_HOME/providers/glm/claude-config`, `ANTHROPIC_BASE_URL=https://api.z.ai/api/anthropic`, `API_TIMEOUT_MS=3000000`, `ANTHROPIC_AUTH_TOKEN=<key>` when a key resolves, and the 4 model alias vars set to `glm-5.3-flash`.
- No key: `isAvailable()` returns false, so the snapshot shows `unavailable`. `getDiagnostic()` returns an Auth row that names `ZAI_API_KEY`.

## Verified

Measured 2026-10-14. Builds, typecheck, lint, and format ran once as a group-level run for group A, which holds only this TASK.

- `npx vitest run packages/server/src/server/agent/provider-registry.test.ts --bail=1` -> PASS (57) FAIL (0); `-t "built-in glm provider"` -> PASS (8). The cases cover key order, missing key, config dir, `WebSearch`, user override, and an unchanged `claude` provider.
- `npm run build:server` -> exit 0 (group-level).
- `npm run typecheck` -> exit 0 (group-level).
- `npm run lint -- <6 changed files>` -> exit 0, 0 warnings, 0 errors.
- `npm run format:check:files -- <6 changed files>` -> exit 0.
- GitNexus `detect-changes --scope all` -> risk low, 0 affected processes.
- Pre-edit impact: `buildProviderRegistry` LOW (6 impacted). `PROVIDER_CLIENT_FACTORIES`, `AGENT_PROVIDER_DEFINITIONS`, and `PROVIDER_CONTRACTS` returned UNKNOWN (module consts). A text search confirmed the readers: the registry, the snapshot manager, the plugin index, and the app provider icon list.
- Live daemon restart, `npm run cli -- provider ls`, `npm run cli -- provider models glm`, and the live probe -> UNVERIFIED (unattended). AUTORUN forbids a live restart.

## Anti-goal

Tripwire: `ls ~/.claude/projects | rg -c glm-native-probe` -> no match expected.

| Checkpoint             | Value                  | Source        | Time (UTC)           |
| ---------------------- | ---------------------- | ------------- | -------------------- |
| Before changes         | no match (rg exit 1)   | command above | 2026-10-14T07:31:23Z |
| After first live probe | not measured, no probe | probe not run | -                    |
| At completion          | no match (rg exit 1)   | command above | 2026-10-14T07:37:46Z |
| Lead re-check          | no match (rg exit 1)   | command above | 2026-10-14, wave 1   |

Result: PASS against a limit of 0 probe folders. The tripwire was not crossed. Lead re-check also ran the registry test file: PASS (57) FAIL (0). Commit `fe8160553`.

## Lessons

- TASK-105: import `resolveGlmApiKey` from `packages/server/src/server/agent/providers/glm/runtime.ts`. Pass `providerEnv` from the glm provider config env, so the config key wins.
- The provider snapshot has no reason field for `unavailable`. The missing-key reason appears only in the provider diagnostic.
- The model alias env vars use the default model, not the session model. Subagents stay on GLM, but they run on `glm-5.3-flash` even when the main model is another GLM model.
- `provider-registry.test.ts` mocks `ClaudeAgentClient`, so glm adds one entry to `mockState.constructorArgs.claude`. Filter on `ANTHROPIC_BASE_URL` to tell them apart.
- In this repo, `NodeJS.ProcessEnv` requires `NODE_ENV`. Type env inputs as `Record<string, string | undefined>` when tests pass plain objects.
- The app provider icon list (`packages/app/src/components/provider-icon-name.ts`) does not know `glm` yet. ADR-0030 scopes `provider-icons.ts` to a later TASK.
