# TASK-079 Wheel step helper and hook

Group: A (new files only: `wheel-select.ts`, `wheel-select.test.ts`, `use-wheel-select.ts`, `use-wheel-select.web.ts`)
Class: code

## Brief

Goal: add the pure wheel-step logic and the web-only hook that a composer pill can attach to its trigger. No pill changes in this TASK.

Change: no wheel handling anywhere -> one tested helper plus one hook, ready to wire.

Boundaries:

- Follows [ADR-0024](watchtower/decisions/ADR-0024-current-provider-wheel-order-over-every-provider.md): the wheel steps the value while the list is closed, each tick applies at once, and the step clamps at both ends.
- The step clamps. Do not reuse `resolveNextAgentModeId` from [packages/app/src/composer/agent-controls/mode.ts](packages/app/src/composer/agent-controls/mode.ts), because it wraps around, and do not change that function.
- Keep `wheel-select.ts` free of DOM and React. It holds arithmetic only, so a node test can cover it.
- The hook is web-only through the file extension, not through a runtime `isWeb` check.
- Q-029 sets the sensitivity: 48 accumulated pixels per step, and the accumulator resets after 300 ms of no wheel event. Export both as named constants. The 48 pixels apply to every pill, through this one constant. Do not add a per-pill override.

How:

- Write `wheel-select.ts` with `WHEEL_STEP_THRESHOLD_PX = 48`, `WHEEL_IDLE_RESET_MS = 300`, a `WheelStepState` type of `{ accumulated: number; lastEventAt: number }`, and two pure functions.
- `resolveWheelSteps({ state, deltaY, now, threshold, idleMs })` returns `{ steps, state }`. It resets the accumulator when `now - state.lastEventAt` is at least `idleMs`, adds `deltaY`, and converts whole thresholds into signed steps, keeping the remainder in the returned state. A wheel down (positive `deltaY`) gives positive steps.
- `resolveWheelOptionIndex({ currentIndex, steps, count })` returns the clamped next index. It returns `-1` when `count` is `0`, and treats a `currentIndex` below `0` as `0`.
- Write `use-wheel-select.ts` as the native and base file. It returns a stable no-op ref callback and ignores its input.
- Write `use-wheel-select.web.ts`. `useWheelSelect({ enabled, optionIds, valueId, onSelect })` returns a ref callback. On a node, and only while `enabled` is true and `optionIds.length` is at least 2, it adds a `wheel` listener with `{ passive: false }`. The listener calls `preventDefault()`, feeds `deltaY` through `resolveWheelSteps`, and on a non-zero step resolves the next index from the current `valueId` and calls `onSelect` with that option id. It calls `onSelect` only when the id changes.
- Read the live `optionIds`, `valueId`, and `onSelect` from a ref inside the listener, so the listener is added once per node and does not churn on each render.
- Remove the listener when the node unmounts, when `enabled` turns false, or when the option count drops below 2.
- Add `composeViewRefs(...refs)` to `use-wheel-select.ts` and `use-wheel-select.web.ts`, or to a small shared file both import. It assigns one node to several `Ref<View>` values, because each pill already uses its ref for the popover anchor and the tooltip.
- Cover the helper in `wheel-select.test.ts`: a 40 pixel delta gives no step, two 40 pixel deltas inside the idle window give one step, a 100 pixel delta gives two steps, an idle gap resets the accumulator, a negative delta steps up, the index clamps at the first and last option, and an empty list returns `-1`.

Files:

- [packages/app/src/composer/agent-controls/wheel-select.ts](packages/app/src/composer/agent-controls/wheel-select.ts) (new; pure step and index arithmetic)
- [packages/app/src/composer/agent-controls/wheel-select.test.ts](packages/app/src/composer/agent-controls/wheel-select.test.ts) (new; unit tests for the helper)
- [packages/app/src/composer/agent-controls/use-wheel-select.ts](packages/app/src/composer/agent-controls/use-wheel-select.ts) (new; native and base no-op, plus the ref helper)
- [packages/app/src/composer/agent-controls/use-wheel-select.web.ts](packages/app/src/composer/agent-controls/use-wheel-select.web.ts) (new; the web wheel listener)

Expected result:

- `resolveWheelSteps` turns accumulated wheel delta into signed option steps and keeps the remainder. One step needs 48 accumulated pixels.
- `resolveWheelOptionIndex` clamps: at the last option a further down step returns the last index, and at the first option a further up step returns the first index.
- The web hook attaches one non-passive `wheel` listener per node and detaches it on unmount.
- The hook does nothing when the list has fewer than 2 options, when `enabled` is false, or on native.
- No pill behaves differently yet, because nothing imports the hook.

Anti-goal: `wheel-select.ts` stays platform-free and dependency-free; measured by `rg -n "document|window|addEventListener|react" packages/app/src/composer/agent-controls/wheel-select.ts` returning no match; tripwire; read after the helper is written and at completion.

## Verify

- `npx vitest run packages/app/src/composer/agent-controls/wheel-select.test.ts --bail=1` -> all cases pass.
- `npm run typecheck` -> passes.
- `npm run lint -- packages/app/src/composer/agent-controls` -> passes.
- `rg -n "document|window|addEventListener|react" packages/app/src/composer/agent-controls/wheel-select.ts` -> no match, so the helper stays pure.
- `rg -n "isWeb" packages/app/src/composer/agent-controls/use-wheel-select.web.ts packages/app/src/composer/agent-controls/use-wheel-select.ts` -> no match, because the platform split is the file extension.
- `git diff --name-only` -> lists only the four new files.
