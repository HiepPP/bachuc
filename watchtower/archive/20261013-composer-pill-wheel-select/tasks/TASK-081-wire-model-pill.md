# TASK-081 Wire the model pill

Group: C (shared file: `combined-model-selector.tsx`)
Class: risky

## Brief

Goal: let the wheel step the model pill through the models of the current provider while the pointer hovers the pill and the popover is closed.

Change: the model pill changes only through its popover -> a wheel tick over the closed pill picks the next model of the same provider at once.

Boundaries:

- Follows [ADR-0024](watchtower/decisions/ADR-0024-current-provider-wheel-order-over-every-provider.md): hover with the list closed, one step per tick applied at once, clamped at both ends, and the wheel never changes the provider.
- Q-028 sets the wheel list: the rows of the provider whose id equals `selectedProvider`, in their given row order. Ignore every other provider. When that provider's `modelSelection.kind` is not `models`, the list is empty and the wheel does nothing.
- The step clamps at that provider's first and last model. It does not cross into another provider, and it does not wrap.
- The wheel list is the current provider's row list, not the popover view. Do not change `ModelBrowser`, its search, its provider sub-views, or its profile rows.
- A row carries a stable `favoriteKey` of `provider:modelId`. Use it as the option id, and split it back into the provider and the model id for `onSelect`.
- The model pill is shared. It also renders in the Command Center and in forms, so keep the new behavior inside the `renderTrigger` path that the composer uses, and leave the other trigger paths unchanged.
- This TASK writes one file only, so it stays out of the TASK-080 group and can run beside it.

How:

- Build the current provider's row list with a small pure helper that takes `providers` and `selectedProvider` and returns the `favoriteKey` list, and cover it in the matching test file.
- Call `useWheelSelect` from TASK-079 with `optionIds` from that list, `valueId` from the selected provider and model, and an `onSelect` that splits the key and calls the existing `onSelect(provider, modelId)`.
- Set `enabled` false while the popover is open, while `disabled` is true, while `isLoading` is true, and when the list holds fewer than two rows.
- Compose the hook ref callback with the existing `anchorRef`, so the popover anchor keeps working.

Files:

- [packages/app/src/components/combined-model-selector.tsx](packages/app/src/components/combined-model-selector.tsx) (flat row helper, hook call, composed ref)

Expected result:

- One wheel tick over the closed model pill selects the next model of the current provider, and the pill label updates.
- A wheel tick at that provider's last model keeps that model. A tick up at its first model keeps the first.
- The wheel never changes the provider, so the provider pill label stays the same through any number of ticks.
- A provider with one model, a loading provider, and an error provider all ignore the wheel.
- While the popover is open, or while the selector is loading or disabled, the wheel changes nothing.
- The popover still opens on click, with its search, provider views, and profile rows unchanged.

Anti-goal: the wheel never changes the provider, and the model browser view model does not change; measured by the new helper test asserting every returned key starts with the selected provider id, by `npx vitest run packages/app/src/components/model-browser-view.test.ts --bail=1` reporting 0 failures, and by `git diff --name-only -- packages/app/src/components` listing only `combined-model-selector.tsx` and its test file; tripwire; read before the first edit as a baseline, after the helper lands, and at completion.

## Verify

- `npx vitest run packages/app/src/components/model-browser-view.test.ts --bail=1` -> 0 failures.
- `npx vitest run <the test file that covers the row helper> --bail=1` -> the row order, the empty list for a loading provider, the clamp at both ends, and the case where every returned key starts with the selected provider id all pass.
- `npm run typecheck` -> passes.
- `npm run lint -- packages/app/src/components/combined-model-selector.tsx` -> passes.
- `git diff --name-only -- packages/app/src/components` -> only the model selector file and its test file.
- Live host check, needs a human and a mouse: hover the model pill and turn the wheel -> the model label steps one model per tick, stops at the provider's last model, the provider pill label does not change, and the popover stays closed.
