# TASK-080 Wire the toolbar pills

Group: B (shared files: `control.tsx`, `control.test.tsx`, `index.tsx`, `mode-control.tsx`)
Class: risky

## Brief

Goal: let the wheel step the value of the mode, thinking, provider, and select feature pills while the pointer hovers the pill and the list is closed.

Change: a pill changes only by a click that opens its list -> a wheel tick over the closed pill steps its value at once.

Boundaries:

- Follows [ADR-0024](watchtower/decisions/ADR-0024-current-provider-wheel-order-over-every-provider.md): hover with the list closed, one step per tick applied at once, clamped at both ends, and the pill label is the only feedback.
- Wire only the `toolbar` surface. A `sheet` row sits inside a scrollable sheet, where `preventDefault()` would block the sheet scroll.
- A toggle feature pill is not a list. Leave it unchanged.
- Do not add hover state to the pill. The wheel event already fires only while the pointer is over the node, so this TASK adds no `onPointerEnter`, no `onPointerLeave`, and no `onHoverIn`. That also keeps the pill clear of the failure modes in [docs/hover.md](docs/hover.md).
- Do not change the open list. Keyboard and click behavior inside the `Combobox` stays as it is, and the `message-input.mode-cycle` shortcut keeps wrapping.
- The pill ref is already taken twice, by the popover anchor and by `TooltipTrigger`. Use `composeViewRefs` from TASK-079 instead of replacing either one.
- Add no new UI text, so no locale file changes.

How:

- Add an optional `wheelSelect` prop to `AgentControlTrigger`: `{ optionIds: readonly string[]; valueId: string | null; onSelect: (id: string) => void; enabled?: boolean }`.
- Inside `AgentControlTrigger`, call `useWheelSelect` with `enabled` false when the trigger is disabled, when the list is open, or when `surface` is `sheet`. Compose the returned ref callback with the forwarded ref and pass the result to `ComboboxTrigger`.
- Mode pill in `mode-control.tsx`: pass `optionIds` from `modeOptions` ids, `valueId` from `selectedMode.id`, and `onSelect` from `onSelectMode`. Use the unfiltered `modeOptions`, not the search-filtered `options`.
- Thinking pill in `index.tsx`: pass the ids of `comboboxThinkingOptions`, `selectedThinkingOptionId`, and the existing thinking select handler.
- Provider pill in `index.tsx`: pass the provider option ids, `selectedProviderId`, and `onSelectProvider`.
- Select feature pill in `DesktopFeatureItem` in `index.tsx`: pass the ids of `feature.options`, `String(feature.value)`, and `handleSelectOption`.
- Extend `control.test.tsx`: render a trigger with `wheelSelect`, fire `fireEvent.wheel(node, { deltaY: 30 })` on the `data-testid` node, and assert `onSelect` received the next option id. Add a case for the clamp at the last option, a case where `open` is true and the wheel changes nothing, and a case where `surface` is `sheet` and the wheel changes nothing.

Files:

- [packages/app/src/composer/agent-controls/control.tsx](packages/app/src/composer/agent-controls/control.tsx) (new `wheelSelect` prop, hook call, composed ref)
- [packages/app/src/composer/agent-controls/control.test.tsx](packages/app/src/composer/agent-controls/control.test.tsx) (wheel cases for step, clamp, open list, sheet surface)
- [packages/app/src/composer/agent-controls/mode-control.tsx](packages/app/src/composer/agent-controls/mode-control.tsx) (pass `wheelSelect` for the mode pill)
- [packages/app/src/composer/agent-controls/index.tsx](packages/app/src/composer/agent-controls/index.tsx) (pass `wheelSelect` for the thinking, provider, and select feature pills)

Expected result:

- One wheel tick over a closed mode, thinking, provider, or select feature pill moves the value by one option, and the pill label updates.
- A wheel tick down at the last option keeps the last option. A wheel tick up at the first option keeps the first option.
- While the list is open, the wheel does not change the value.
- A pill with fewer than two options, a disabled pill, and a toggle feature pill ignore the wheel.
- The `sheet` surface ignores the wheel, so a sheet still scrolls.
- A click still opens the list, and the mode cycle shortcut still wraps.
- The agent stream behind the composer does not scroll while the pointer is over a pill.

Anti-goal: the existing pill tests keep passing; measured by `npx vitest run packages/app/src/composer/agent-controls/control.test.tsx packages/app/src/composer/agent-controls/mode.test.ts packages/app/src/composer/agent-controls/utils.test.ts --bail=1` reporting 0 failures; tripwire; read before the first edit as a baseline, after each pill is wired, and at completion.

## Verify

- `npx vitest run packages/app/src/composer/agent-controls/control.test.tsx --bail=1` -> the wheel step, clamp, open-list, and sheet cases pass.
- `npx vitest run packages/app/src/composer/agent-controls/mode.test.ts packages/app/src/composer/agent-controls/utils.test.ts --bail=1` -> 0 failures, so the old behavior holds.
- `npm run typecheck` -> passes.
- `npm run lint -- packages/app/src/composer/agent-controls` -> passes.
- `rg -n "onPointerEnter|onPointerLeave|onHoverIn" packages/app/src/composer/agent-controls/control.tsx` -> no match, because the wheel needs no hover state.
- Live host check, needs a human and a mouse: hover the mode pill and turn the wheel one tick -> the mode label steps by one option and the list stays closed. Turn the wheel at the last option -> the label does not change and the stream does not scroll.

Prompt:

```text
Read watchtower/tasks/TASK-079-wheel-step-helper-and-hook.md first, because this TASK uses the hook it adds.
Wire the wheel only on the toolbar surface. Do not add hover handlers, and do not change the open Combobox.
```
