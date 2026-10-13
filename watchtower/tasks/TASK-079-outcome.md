# TASK-079 Outcome

## Outcome

Status: DONE

Changed:

- `packages/app/src/composer/agent-controls/wheel-select.ts` (new; pure step and index arithmetic, plus `INITIAL_WHEEL_STEP_STATE`)
- `packages/app/src/composer/agent-controls/wheel-select.test.ts` (new; 11 cases)
- `packages/app/src/composer/agent-controls/use-wheel-select.ts` (new; native and base no-op `useWheelSelect`, `composeViewRefs`)
- `packages/app/src/composer/agent-controls/use-wheel-select.web.ts` (new; non-passive `wheel` listener hook, `composeViewRefs`)

Contract:

- `WHEEL_STEP_THRESHOLD_PX = 48`, `WHEEL_IDLE_RESET_MS = 300`, type `WheelStepState`, `INITIAL_WHEEL_STEP_STATE` (`lastEventAt: -Infinity`), `resolveWheelSteps({ state, deltaY, now, threshold?, idleMs? })` -> `{ steps, state }`, `resolveWheelOptionIndex({ currentIndex, steps, count })` -> clamped index, `-1` for an empty list, unknown (negative) current index treated as 0. `threshold` and `idleMs` default to the shared constants.
- `useWheelSelect({ enabled, optionIds, valueId, onSelect })` returns a stable `(node: View | null) => void` ref callback. Same export on native (no-op) and web. Web listener is attached per node only while `enabled` and `optionIds.length >= 2`, reads live props from a ref, calls `preventDefault()`, calls `onSelect` only when the id changes, and detaches on unmount, disable, or fewer than 2 options. The node is held in state, so a node that mounts later still gets the listener.
- `composeViewRefs(...refs)` is exported from both hook files (duplicated, because the spec allows no shared file). Pills pass its result as `ref` and keep their popover anchor and tooltip refs.
- Nothing imports the hook yet, so no pill changes.

Verified:

- `cd packages/app && npx vitest run src/composer/agent-controls/wheel-select.test.ts --bail=1` -> PASS (11) FAIL (0)
- `npm run typecheck` -> exit 0
- `npm run lint -- packages/app/src/composer/agent-controls` -> 0 warnings, 0 errors
- `rg -n "document|window|addEventListener|react" .../wheel-select.ts` -> no match (exit 1)
- `rg -n "isWeb" .../use-wheel-select.web.ts .../use-wheel-select.ts` -> no match (exit 1)
- `git diff --name-only` -> empty, because the four files are untracked; `git status --short` lists exactly the four new files.

Anti-goal:

- After the helper was written: `rg -n "document|window|addEventListener|react" packages/app/src/composer/agent-controls/wheel-select.ts` -> no match.
- Final: same command -> no match.
- Result: PASS

Lessons:

- Run app vitest files from `packages/app` with a path relative to it (`src/...`); the root-relative path in the spec's Verify line does not resolve from there.
- `git diff --name-only` does not list new untracked files; use `git status --short` to check the file list.
- `useWheelSelect` hands back one ref callback. A pill must wrap it with `composeViewRefs(existingAnchorRef, wheelRef)` instead of replacing its anchor or tooltip ref.
- `event.timeStamp` feeds `now`; it is monotonic within the page, so the 300 ms idle reset works with it.
- `composeViewRefs` returns a new callback on each call. Wrap it in `useMemo` or `useCallback` in the pill, so React does not detach and reattach the ref on every render (lead review, 2026-10-13).
