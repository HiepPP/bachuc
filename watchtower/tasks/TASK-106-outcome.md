# TASK-106 Outcome

Lead re-check, wave 2: the stream, provider-icons, and provider-icon-name tests pass (80), with 0 failures. `provider-icon-name.ts` is outside `Files:`; the run notes allowed it. The lead fixed Expected result 3 in the spec. Commit `c2c99f274`.

## Status

DONE. The live picker check is UNVERIFIED (unattended), because AUTORUN forbids a live restart. Unit tests prove the icon mapping and the stream behavior.

## Changed

- `packages/app/src/types/stream.ts`: new `isClaudeCodeProvider(provider)` helper (true for `claude` and `glm`), used at both `reduceTimelineToolCall` branches (`TodoWrite` and `TaskCreate`/`TaskUpdate`/`TaskList`).
- `packages/app/src/types/stream.test.ts`: glm `TodoWrite` builds a todo list; glm and claude `TaskCreate` are suppressed; an unparseable `TodoWrite` is suppressed for claude and glm and stays a tool call for codex.
- `packages/app/src/components/provider-icons.ts`: `glm` in `BUILTIN_PROVIDER_ICONS`, built with `createSvgIcon` from the existing `glm-acp-agent` svg in `ACP_PROVIDER_ICON_SVGS`. No new asset.
- `packages/app/src/components/provider-icon-name.ts` (outside the spec `Files:`, allowed by the run notes from the TASK-104 lesson): `BUILTIN_PROVIDER_IDS` now also holds `glm`. Without it `resolveProviderIconName("glm")` returns `bot` and the new icon is never reached. The protocol icon name list is not touched.

## Contract

- `resolveProviderIconName("glm")` returns `{ kind: "builtin", id: "glm" }`, and `getProviderIcon("glm")` returns the GLM svg icon. A snapshot `iconSvg` for `glm` is ignored, because built-in ids win.
- The `glm-acp-agent` provider keeps its own catalog icon and is unchanged.

## Verified

Measured 2026-10-14. Typecheck ran once as a group-level run for group C. Group B edits `packages/server/src/services/quota-fetcher` at the same time, and the repo-wide typecheck still passed.

- `npx vitest run src/types/stream.test.ts --bail=1` (from `packages/app`) -> PASS (71) FAIL (0); 6 new cases.
- `npx vitest run src/types/stream.test.ts src/components/provider-icons.test.ts src/components/provider-icon-name.test.ts --bail=1` -> PASS (80) FAIL (0).
- `npm run typecheck` -> exit 0 (group-level).
- `rtk proxy npm run lint -- <4 changed files>` -> 0 warnings, 0 errors. Plain `npm run lint` printed "ESLint output (JSON parse failed)" through the rtk wrapper.
- `npm run format:check:files -- <4 changed files>` -> exit 0.
- GitNexus `detect_changes --scope all` -> risk low, 0 affected processes (it also lists the group B server files).
- Pre-edit impact: `reduceTimelineToolCall` LOW (6 impacted). `resolveProviderIconName` CRITICAL by fan-out (33 impacted, 16 UI callers); I did not edit the function, only the id set it reads, and the change adds one id.
- Live picker shows the GLM icon -> UNVERIFIED (unattended).

## Anti-goal

Tripwire: failing tests in `packages/app/src/types/stream.test.ts` stay at 0.

| Checkpoint              | Value                                               | Source                                             | Time (UTC)           |
| ----------------------- | --------------------------------------------------- | -------------------------------------------------- | -------------------- |
| Before changes          | PASS (65) FAIL (0)                                  | `npx vitest run src/types/stream.test.ts --bail=1` | 2026-10-14T07:42:19Z |
| After the helper change | PASS (65) FAIL (0), then 1 failing new test I wrote | same command                                       | 2026-10-14T07:43Z    |
| At completion           | PASS (71) FAIL (0)                                  | same command                                       | 2026-10-14T07:44:42Z |

Result: PASS. The one failure after the helper change was my own new codex test with a wrong expectation (see Lessons). I fixed the test; no existing test failed.

## Spec deviations

- Expected result 3 ("A codex `TodoWrite`-named call still does not build a todo list") is wrong against the code. The generic `extractTaskEntriesFromToolCall` path builds a todo list from `TodoWrite` for any provider, and it did before this TASK. I did not change that (the spec says no change for other providers). The test instead pins the real glm/codex difference: an unparseable `TodoWrite` is suppressed for claude and glm and stays visible for codex.
- The glm `TodoWrite` test also passes without the helper, because the generic path builds the list. The `TaskCreate` and unparseable-`TodoWrite` cases are the ones that need the helper.

## Other `"claude"` checks left alone

None. `rg` over `packages/app/src` (non-test) found no other `=== "claude"` provider check. The other `"claude"` matches are a doc comment in `context-window-meter.tsx` and a theme name in `styles/theme.ts`.

## Lessons

- The generic tool parser (`packages/app/src/utils/tool-call-parsers.ts`) builds todo lists from `TodoWrite` for every provider. The Claude branch in `stream.ts` only changes what happens when parsing fails (suppress instead of show) and hides `TaskCreate`/`TaskUpdate`/`TaskList`.
- Run app vitest files from `packages/app`, not the repo root (`__DEV__` error).
- `ProviderIconName` resolution reads `BUILTIN_PROVIDER_ICON_NAMES` from `@getpaseo/protocol/provider-icon-names`. A later TASK that wants `glm` there must edit the protocol file and drop the local addition.
