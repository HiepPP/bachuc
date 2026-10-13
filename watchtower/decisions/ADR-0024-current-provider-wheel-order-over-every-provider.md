# ADR-0024 Current provider wheel order over every provider

Status: accepted
Date: 2026-10-13
Supersedes: ADR-0023
Scope: `packages/app/src/composer/agent-controls/**`, `packages/app/src/components/combined-model-selector.tsx`

## Context

- Every composer pill pairs an `AgentControlTrigger` with a `Combobox` that opens on a click. The owner wanted to pick a value with the wheel while hovering the pill.
- The pills are provider, model, mode, thinking, and one per agent feature. Each one already holds a flat option list and a select handler.
- [ADR-0023](watchtower/decisions/ADR-0023-immediate-wheel-step-on-the-closed-pill-over-opening-its-list.md) chose the same wheel behavior, but let the model pill step across every provider in provider order. The owner reopened that one point on 2026-10-13, before any code shipped.
- The model pill selects a provider and a model together, so a cross-provider step changes two values from one wheel tick. The provider pill already exists next to it.
- The wheel exists only on web. Mobile and tablet keep the sheet, which scrolls.

## Decision

- The wheel acts on a pill whose value comes from a list, including the model pill. A toggle feature pill ignores it.
- The wheel acts while the pointer hovers the pill and its list is closed. It does not open the list.
- Each wheel step applies at once through the pill's existing select handler.
- The step clamps at the first and the last option. The pill label is the only feedback.
- On the model pill, the wheel steps only the models of the current provider, in their row order, and clamps at that provider's first and last model. The wheel never changes the provider.
- One step needs 48 accumulated wheel pixels, through one shared constant for every pill.
- The owner chose the wheel behavior on 2026-10-13, then chose the current-provider order and the 48 pixel step later the same day.

## Consequences

- The pill needs a non-passive `wheel` listener so it can call `preventDefault()` and keep the stream behind the composer still.
- The wheel needs no hover state, because a wheel event fires only over the node. The pill stays clear of the failure modes in [docs/hover.md](docs/hover.md).
- The `sheet` surface is excluded, because `preventDefault()` there would block the sheet scroll.
- An immediate apply sends one agent control update per step, so the wheel delta needs the 48 pixel threshold.
- One wheel tick changes one value. The provider stays an explicit choice on the provider pill or in the popover.
- A provider with one model gives the model pill nothing to step, so the wheel does nothing there.
- Models of another provider stay reachable only through the provider pill or the popover.

## Options Considered

- Step across every provider in provider order: rejected in ADR-0023's own terms, now reversed. One wheel tick would change both the provider and the model, and the flat order differs from the popover order, so the owner could land on a model from a provider they did not choose. Revisit If: holds.
- Cross into the next provider only at the provider edge: rejected. It still changes the provider from a wheel tick, with no signal before it happens.
- Wheel moves the highlight inside the open list, with a click or Enter to confirm: rejected in ADR-0023: the owner wanted a hover shortcut, and this option needs the list open first. Revisit If: does not hold.
- Preview the value and commit on pointer leave: rejected in ADR-0023: it adds a pending value and gives no clear way to cancel. Revisit If: does not hold.
- Wrap around at the ends: rejected in ADR-0023: the owner can cycle past the wanted value without noticing. Revisit If: does not hold.
- Open the list on the first tick: rejected in ADR-0023: the popover covers the composer and fights hover-to-scroll. Revisit If: does not hold.
- Simple pills only, with the model pill left out: rejected in ADR-0023: the owner wants the model pill included. Revisit If: does not hold.
- A 24 pixel step, or a per-pill step size: rejected. 24 pixels fires on a light trackpad brush, and a per-pill override adds a second value to tune and test.

## Revisit If

- The owner asks to reach another provider's models without leaving the pill.
- The 48 pixel step feels heavy on a trackpad or light on a mouse wheel, measured by the owner needing more than two notches per step.
- One wheel tick sends more than one agent control update, or the agent reports a dropped update.
- The sheet surface gains a mouse wheel path, for example a desktop window narrow enough to show sheets.
- The owner asks for a visible option list while the wheel moves.
