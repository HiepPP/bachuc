# TASK-081 Outcome

## Outcome

Status: DONE

Default taken: Q-033 (DEFAULTED). The spec named the `renderTrigger` path, but the composer takes the `ComboboxTrigger` branch with `toolbar` set. The wheel sits on that branch and is on only when `toolbar` is set.

Changed:

- `packages/app/src/components/combined-model-selector.tsx`: exported pure helpers `resolveCurrentProviderModelKeys`, `resolveCurrentProviderModelKey`, `isModelWheelEnabled`, and `splitModelRowKey`, plus a local hook `useModelWheelAnchorRef` that calls `useWheelSelect` and returns the `ComboboxTrigger` ref.
- `packages/app/src/components/combined-model-selector.test.ts` (new; 13 cases).

Contract:

- Option ids are the `favoriteKey` values of the rows of the provider whose id equals the trimmed `selectedProvider`, in row order (Q-028). A loading, error, or unknown provider gives `[]`.
- `valueId` uses the same fallback as the label (`resolveSelectedModelLabel`): the matching row, else the default row, else the first row. If the model id is not in the list, `valueId` is null, because the label shows the raw id.
- An off-list model ignores the wheel (review fix): `isModelWheelEnabled` returns false when `valueId` is null. Without this guard, one tick down would replace the off-list model with row 1. The popover is the only way to leave an off-list model.
- `onSelect` splits the key at the first colon and calls the caller's `onSelect(provider, modelId)` directly. It does not call `handleSelect`, so it does not run `onClose` or a popover reset.
- `isModelWheelEnabled` turns the wheel on only when `toolbar` is set, the popover is closed, `disabled` and `isLoading` are false, the list has at least 2 rows, and `valueId` is not null. Clamping comes from `resolveWheelOptionIndex`, with no wrap.
- When `toolbar` is set, the `ComboboxTrigger` gets a memoized `composeViewRefs(anchorRef, wheelRef)`, and `Combobox` still gets `anchorRef`. Without `toolbar`, the trigger gets the plain `anchorRef` as before. The `renderTrigger` branch (schedule form) and the no-`toolbar` branch (settings metadata page) do not change.
- `ModelBrowser` and `model-browser-view` do not change.
- Test note: the `.tsx` cannot load in the node test environment as is (`SyntaxError: Unexpected token 'typeof'` from native UI imports). The test stubs `model-browser`, `ui/combobox`, `ui/combobox-trigger`, and `ui/loading-spinner` with `vi.mock`, so the helpers stay in the one spec file.

Verified:

- GitNexus impact, `node .gitnexus/run.cjs impact "CombinedModelSelector" --direction upstream --repo .` -> HIGH (10 impacted, 3 direct: `ScheduleTargetFields`, `DesktopAgentControlsContent`, `MetadataGenerationPage`). The change is additive and gated on `toolbar`, which only `DesktopAgentControlsContent` passes.
- `cd packages/app && npx vitest run src/components/model-browser-view.test.ts --bail=1` -> PASS (18) FAIL (0)
- `cd packages/app && npx vitest run src/components/combined-model-selector.test.ts --bail=1` -> PASS (8) FAIL (0). The cases cover row order, every key starting with the selected provider id, an empty list for loading, error, or unknown providers, the clamp at both ends, the label fallback, and the key split.
- `npm run typecheck` -> exit 0
- `npm run lint -- packages/app/src/components/combined-model-selector.tsx packages/app/src/components/combined-model-selector.test.ts` -> 0 warnings, 0 errors. The first run hit `complexity` 23 > 20 on `CombinedModelSelector`, and moving the gate into the hook fixed it.
- `git diff --name-only -- packages/app/src/components` -> `combined-model-selector.tsx` only. `git status --short` also lists the untracked `combined-model-selector.test.ts`.
- After the review fix (off-list guard and `isModelWheelEnabled` cases): `cd packages/app && npx vitest run src/components/model-browser-view.test.ts src/components/combined-model-selector.test.ts --bail=1` -> 2 files passed, 31 tests passed, 0 failed (18 + 13). The new cases cover toolbar off, open, disabled, loading, count 1 and 0, valueId null, and all conditions met.
- After the review fix: `npm run typecheck` -> exit 0; `npm run lint -- packages/app/src/components/combined-model-selector.tsx packages/app/src/components/combined-model-selector.test.ts` -> 0 warnings, 0 errors.
- Live host check (hover the composer model pill, turn the wheel, label steps one model per tick, stops at the last model, provider pill unchanged, popover stays closed): UNVERIFIED (autonomous run)

Anti-goal:

Command: `cd packages/app && npx vitest run src/components/model-browser-view.test.ts --bail=1`, plus `git diff --name-only -- packages/app/src/components`, plus the helper test case that checks every key starts with the selected provider id.

- Baseline, before the first edit: PASS (18) FAIL (0); diff empty.
- After the helper: PASS (18) FAIL (0); diff `combined-model-selector.tsx`; new test file untracked; helper test PASS (8).
- Before the review fix: PASS (18) FAIL (0); diff `combined-model-selector.tsx`; new test file untracked; helper test PASS (8) FAIL (0).
- Final, after the review fix: model-browser-view 18 passed and helper test 13 passed (31 total, 0 failed); diff `combined-model-selector.tsx`; new test file untracked.
- Result: PASS

Lessons:

- Before a plan names a trigger path in a shared component, check which branch each caller takes. The spec named `renderTrigger`, but the composer passes `toolbar` and takes the `ComboboxTrigger` branch.
- A component `.tsx` with native UI imports does not load in the node test environment. Stub its UI imports with `vi.mock` to test its pure exports in place.
- `CombinedModelSelector` sits near the complexity limit of 20. Pass the raw flags into a hook and combine them there.
- `resolveWheelOptionIndex` treats an unknown current index as 0. If the current value is not in the option list, turn the wheel off, so one stray tick cannot replace that value (review, 2026-10-13).
- Put the wheel `enabled` rule in a pure exported function, so a unit test covers each condition.
