# ADR-0025 Wheel opens the list and steps over a closed-pill step

Status: accepted
Date: 2026-10-13
Supersedes: ADR-0024
Scope: `packages/app/src/composer/agent-controls/**`, `packages/app/src/components/combined-model-selector.tsx`, `packages/app/src/components/ui/combobox.tsx`, `packages/app/src/components/ui/menu/**`, `packages/app/src/hooks/use-wheel-listener*`, `packages/app/src/hooks/use-pointer-leave-close*`, `packages/app/src/plugins/buttons/**`, `packages/plugin/src/client/buttons.ts`, `hiep-plugins/plugins/prompt-translate/client/**`, `hiep-plugins/plugins/skill-pins/client/menu-row.tsx`

## Context

- [ADR-0024](watchtower/decisions/ADR-0024-current-provider-wheel-order-over-every-provider.md) shipped a wheel that stepped a closed pill and kept its list closed.
- The owner tried it on 2026-10-13. Mode and effort stepped too fast to control, and the Caveman and Skills plugin pills did not react to the wheel.
- The owner asked for the list to open when the wheel turns, so they can see where the value is while it moves.
- An open popover puts a full-window backdrop over the trigger. Wheel events after the first tick land on that backdrop, not on the pill.
- Plugin composer pills render plugin-owned popover content. The host has no option list for them.
- On a second try the same day, the owner found mode and effort good but asked for them a little faster. The Caveman pill still did not step, and the Skills pill picks several skills. The owner wanted a list the wheel opened to close when the pointer leaves it.
- On a third try, the owner still found switching slow. They asked for a hover to open the list and for leaving the pill to close it.

## Decision

- On web, a mouse resting 150 ms on any composer pill, the model pill included, opens its list.
- A wheel step over a host pill opens its list and applies the next option at once. Later steps keep applying while the list is open.
- The pill and its open list's backdrop share one step handler, through the `onBackdropWheel` prop of `Combobox` and `MenuSurface`.
- The first event of a wheel gesture steps once at any size, so one notch is one step on any mouse. Inside the gesture, every 40 px is one more step, and one event can give several. A 150 ms pause ends the gesture. There is no cooldown.
- Until the value prop catches up after a select, the next step starts from the option the wheel picked.
- The step clamps at the first and last option. On the model pill, it stays inside the current provider, and it ignores a model that is not in the list.
- When a hover or the wheel opened the list, a pointer move outside the pill, the list, and the gap in the pill's own column closes it. A neighbor pill is outside, so its hover takes over. A click on the pill keeps the list open. A list opened from the keyboard ignores the pointer.
- The plugin API gets a `select` button behavior for a composer pill's own single choice. The host draws it with `Combobox`, so it steps like the effort pill. The Caveman pill uses it.
- On a plugin menu or popover pill, the wheel opens the surface, then moves focus one row per step. A click or Enter picks the row. The Skills rows carry the host's menu row marker so they join this walk.
- The owner chose these on 2026-10-13.

## Consequences

- The wheel now opens a popover over the composer, so the stream behind it does not scroll while the pointer is on a pill.
- A wheel over the open list itself scrolls the list. A wheel over the backdrop steps the value or the focus.
- An agent gets one control update per step. A fast spin can send several updates in one second.
- An open list takes keyboard focus and gives it back on close. A hover that opens the model list moves focus to its search box until the pointer leaves.
- Trackpad momentum can carry the value several steps after the fingers lift.
- The Caveman list looks like the host agent control lists. Its "Caveman" header and its inline load error are gone.
- The Caveman plugin needs a host with the `select` behavior. Release rejects the pill until it runs a build with this change.
- The `sheet` surface stays excluded.

## Options Considered

- Step the closed pill and keep the list closed (ADR-0024): rejected. The owner found it too fast and gave no overview.
- Open the list and move only the highlight on a single-choice pill; a click or Enter applies: rejected. The owner wants the value to change while the wheel turns.
- Open the list only, then scroll and click: rejected. It needs an extra click for every change.
- Keep the Caveman popover and add a wheel hook to the popover behavior: rejected. It is a second wheel path that does not share the effort pill's list.
- Let a wheel step on the Skills pill pin exactly one skill: rejected. It drops the other pins.
- Close only a wheel-opened list on pointer leave: replaced by hover-open on the third try, where every pointer-opened list closes on leave.
- Keep one step per event with a 60 ms cooldown: rejected. A fast spin still drops notches.
- Leave the model pill out of hover-open: rejected by the owner, who accepts the search box focus.
- Let a click on a hover-opened pill close it: rejected. A habit click after the hover would close the list at once.
- Wrap around at the ends: rejected in ADR-0023. Revisit If: does not hold.

## Revisit If

- One notch gives two steps, or trackpad momentum overshoots, on the owner's devices.
- Hover-open steals focus while the owner types.
- Another plugin needs a multiple-choice list that steps a value.
- A running agent drops or reorders the per-step control updates.
