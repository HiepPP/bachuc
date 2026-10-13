# TASK-080 Outcome

## Outcome

Status: DONE

Changed:

- `packages/app/src/composer/agent-controls/control.tsx` (new optional `wheelSelect` prop and exported `AgentControlWheelSelect` type; a local `useTriggerWheelRef` hook calls `useWheelSelect` and memoizes `composeViewRefs(ref, wheelRef)`, which goes to `ComboboxTrigger`)
- `packages/app/src/composer/agent-controls/control.test.tsx` (wheel cases: one step, clamp at the last option, clamp at the first option on a wheel up, open list, sheet surface, disabled trigger, and a rerender with a new value that steps from the new value; `afterEach(cleanup)` inside the new block)
- `packages/app/src/composer/agent-controls/mode-control.tsx` (mode pill passes a memoized `wheelSelect` from the unfiltered `modeOptions`, `selectedMode.id`, `onSelectMode`)
- `packages/app/src/composer/agent-controls/index.tsx` (thinking pill and select feature pill pass a memoized `wheelSelect`; the thinking `valueId` falls back to the first option id like its label (review fix); provider pill calls the hook directly, see Contract)

Contract:

- `wheelSelect?: { optionIds: readonly string[]; valueId: string | null; onSelect: (id: string) => void; enabled?: boolean }` on `AgentControlTrigger`.
- The wheel is off when `wheelSelect` is absent, when `wheelSelect.enabled` is false, when the trigger is disabled, when `open` is true, or when `surface` is `sheet`. The hook also stays off for fewer than 2 options.
- The forwarded ref (popover anchor or `TooltipTrigger` callback) and the wheel ref are composed, not replaced.
- Mode, thinking, and select feature pills use the `wheelSelect` prop. The toggle feature pill gets no `wheelSelect`, so it ignores the wheel.
- Deviation from How: the desktop provider pill renders a plain `ComboboxTrigger`, not an `AgentControlTrigger`, so it cannot take the `wheelSelect` prop. `DesktopAgentControlsContent` calls `useWheelSelect` itself with the provider option ids, `selectedProviderId`, and `handleProviderSelect` (a thin wrapper of `onSelectProvider`). It composes the result with `providerAnchorRef`. It is enabled only when not disabled, `canSelectProvider`, and the provider list is closed. `DesktopAgentControlsContent` renders only on the toolbar (non-compact) path.
- Test correction: the spec How suggests `deltaY: 30`, but Q-029 set 48 px per step, so 30 px gives no step. The tests use `deltaY: 60` (one step).
- No hover handlers, no Combobox change, no locale change. The mode cycle shortcut code is untouched.

Verified:

- `cd packages/app && npx vitest run src/composer/agent-controls/control.test.tsx --bail=1` -> PASS (8) FAIL (0); includes the step, clamp, open-list, and sheet cases.
- `cd packages/app && npx vitest run src/composer/agent-controls/mode.test.ts src/composer/agent-controls/utils.test.ts --bail=1` -> PASS (19) FAIL (0)
- After the review fixes: `cd packages/app && npx vitest run src/composer/agent-controls/control.test.tsx src/composer/agent-controls/mode.test.ts src/composer/agent-controls/utils.test.ts --bail=1` -> 3 files passed, 30 tests passed, 0 failed (`control.test.tsx` now has 11 cases).
- `npm run typecheck` -> exit 0 (also after the review fixes)
- `npm run lint -- packages/app/src/composer/agent-controls` -> 0 warnings, 0 errors (also after the review fixes)
- `rg -n "onPointerEnter|onPointerLeave|onHoverIn" packages/app/src/composer/agent-controls/control.tsx` -> no match (exit 1)
- Live host check (hover the mode pill, one wheel tick steps the label, list stays closed, clamp at the last option, stream does not scroll): UNVERIFIED (autonomous run)
- GitNexus impact before edits: `AgentControlTrigger` MEDIUM (9 impacted, 6 direct), `AgentModeControl` LOW, `DesktopFeatureItem` LOW, `DesktopAgentControlsContent` LOW. No HIGH or CRITICAL.

Anti-goal:

Command: `cd packages/app && npx vitest run src/composer/agent-controls/control.test.tsx src/composer/agent-controls/mode.test.ts src/composer/agent-controls/utils.test.ts --bail=1`

- Baseline, before the first edit: PASS (23) FAIL (0)
- After `control.tsx`, its tests, and the mode pill: PASS (27) FAIL (0)
- After the thinking pill: PASS (27) FAIL (0)
- After the provider pill: PASS (27) FAIL (0)
- After the select feature pill and format: PASS (27) FAIL (0)
- After the lint and type fixes: PASS (27) FAIL (0)
- Final, after the review fixes: 30 passed, 0 failed
- Result: PASS

Lessons:

- Check the real component of each pill before you plan to wire it through a shared prop. The desktop provider pill is a raw `ComboboxTrigger`, so a `wheelSelect` prop on `AgentControlTrigger` does not reach it.
- `control.test.tsx` has no automatic cleanup. A second render in the same file finds duplicate `data-testid` nodes unless the block calls `afterEach(cleanup)` or each test unmounts.
- `AgentControlTrigger` sits near the lint complexity limit of 20. Put new prop logic in a small hook outside the component.
- The `react-perf/jsx-no-new-object-as-prop` lint rule also fires in tests. Build the prop object through a function call or `useMemo`.
- `ComboboxTrigger` types `disabled` as `boolean | null`, so a helper that takes it must accept `null`.
- TooltipTrigger passes a new callback ref each render, so the composed trigger ref recomputes each render; React 19 resolves it to one extra no-op render, no listener churn (review, 2026-10-13).
- Feed the wheel the same resolved value the label shows. A raw `undefined` value with a label that falls back to option 0 lets a wheel up turn "unset" into an explicit value (review, 2026-10-13).
