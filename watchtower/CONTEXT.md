# Plan Context

## Shared Context

- A composer pill is an `AgentControlTrigger` plus a `Combobox` that opens from it. The trigger lives in [packages/app/src/composer/agent-controls/control.tsx](packages/app/src/composer/agent-controls/control.tsx). The pills are provider, model, mode, thinking, and one per agent feature.
- The model pill is the exception. It renders `CombinedModelSelector` from [packages/app/src/components/combined-model-selector.tsx](packages/app/src/components/combined-model-selector.tsx) with its own `Pressable` trigger, and it selects a provider and a model together.
- Every pill already has a flat option list and a select handler: mode uses `modeOptions` and `onSelectMode`, thinking uses `comboboxThinkingOptions` and `handleThinkingSelect`, a select feature uses `feature.options` and `onSetFeature`, and the model pill uses `providers[].modelSelection.rows` and `onSelect(provider, modelId)`.
- The wheel exists only on web, so the hook ships as `use-wheel-select.web.ts` with a no-op base file, per the Metro platform extension rule in [CLAUDE.md](CLAUDE.md). Do not add a runtime `isWeb` branch for it.
- `packages/app/vitest.config.ts` resolves `.web.ts` before `.ts` in the unit project, so a jsdom test loads the web hook.
- App view tests run in jsdom with the real react-native-web. `testID` becomes `data-testid`, and `fireEvent.wheel(node, { deltaY })` reaches a DOM listener. See [packages/app/src/composer/agent-controls/control.test.tsx](packages/app/src/composer/agent-controls/control.test.tsx).
- A wheel listener must be non-passive to call `preventDefault()`. The repo already attaches wheel listeners this way in [packages/app/src/components/message.tsx](packages/app/src/components/message.tsx).
- Mode already has a cycle action: `resolveNextAgentModeId` in [packages/app/src/composer/agent-controls/mode.ts](packages/app/src/composer/agent-controls/mode.ts) wraps around for the `message-input.mode-cycle` shortcut. The wheel clamps instead of wrapping, so it needs its own index helper.
- Test source changes on live. Do not build a release for this plan.

## Decisions

- The owner answered six questions on 2026-10-13. The wheel works on every pill that selects from a list, including the model pill. It acts on hover while the list is closed. Each tick applies at once. The step clamps at the first and last option, and the only feedback is the pill label. On the model pill the wheel stays inside the current provider. One step needs 48 accumulated pixels, for every pill. Reason: [ADR-0024](watchtower/decisions/ADR-0024-current-provider-wheel-order-over-every-provider.md), which supersedes [ADR-0023](watchtower/decisions/ADR-0023-immediate-wheel-step-on-the-closed-pill-over-opening-its-list.md).
- A toggle feature pill is not a list, so it ignores the wheel.
- The feature adds no new UI text, so no locale file changes.
- The wheel applies only to the `toolbar` surface. The `sheet` surface rows sit inside a scrollable sheet, where `preventDefault()` would block the sheet scroll.

## Open Decisions

- None. Q-028 and Q-029 in [watchtower/QUESTIONS.md](watchtower/QUESTIONS.md) are answered.

## References

- [docs/menus.md](docs/menus.md): the menu engine, popover versus sheet.
- [docs/hover.md](docs/hover.md): the hover pattern and its three failure modes.
- [docs/testing.md](docs/testing.md): the test bar for this repo.
