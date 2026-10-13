# ADR-0023 Immediate wheel step on the closed pill over opening its list

Status: superseded by ADR-0024
Date: 2026-10-13
Supersedes: -
Scope: `packages/app/src/composer/agent-controls/**`, `packages/app/src/components/combined-model-selector.tsx`

## Context

- Every composer pill already pairs an `AgentControlTrigger` with a `Combobox` that opens on a click. The owner wanted to pick a value with the wheel while hovering the pill.
- The pills are provider, model, mode, thinking, and one per agent feature. Each one already holds a flat option list and a select handler.
- The model pill is different. It renders `CombinedModelSelector`, whose popover has a search field and one sub-view per provider, so the popover order is not a single flat list.
- The wheel exists only on web. Mobile and tablet keep the sheet, which scrolls.
- The owner answered four questions on 2026-10-13 through `AskUserQuestion`.

## Decision

- The wheel acts on a pill whose value comes from a list, including the model pill. A toggle feature pill ignores it.
- The wheel acts while the pointer hovers the pill and its list is closed. It does not open the list.
- Each wheel step applies at once through the pill's existing select handler.
- The step clamps at the first and the last option. The pill label is the only feedback.
- For the model pill, the wheel order is the flat provider and model list in provider order, not the popover view.
- The owner chose all four on 2026-10-13.

## Consequences

- The pill needs a non-passive `wheel` listener so it can call `preventDefault()` and keep the stream behind the composer still.
- The wheel needs no hover state, because a wheel event fires only over the node. The pill stays clear of the failure modes in [docs/hover.md](docs/hover.md).
- The `sheet` surface is excluded, because `preventDefault()` there would block the sheet scroll.
- An immediate apply sends one agent control update per step, so the wheel delta needs an accumulation threshold.
- The model pill gains a second order for its options: the flat wheel order next to the popover view order. A future change to the provider order changes what the wheel picks.

## Options Considered

- Wheel moves the highlight inside the open list, with a click or Enter to confirm: rejected. The owner wanted a hover shortcut, and this option needs the list open first.
- Preview the value and commit on pointer leave: rejected. It adds a pending value and gives no clear way to cancel.
- Wrap around at the ends: rejected. The owner can cycle past the wanted value without noticing.
- Open the list on the first tick: rejected. The popover covers the composer and fights hover-to-scroll.
- Simple pills only, with the model pill left out: rejected. The owner wants the model pill included, even though its popover order differs from the flat wheel order.

## Revisit If

- The wheel picks a model the owner did not expect, because the flat order and the popover order differ.
- One wheel tick sends more than one agent control update, or the agent reports a dropped update.
- The sheet surface gains a mouse wheel path, for example a desktop window narrow enough to show sheets.
- The owner asks for a visible option list while the wheel moves.
