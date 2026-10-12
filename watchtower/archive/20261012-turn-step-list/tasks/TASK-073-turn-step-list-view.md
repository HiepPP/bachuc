# TASK-073 Turn step list view

Group: B (writes the agent stream view, the new turn view, and locale files; depends on TASK-072 by order only)
Class: code

## Brief

Goal: Draw the turn step list from the TASK-072 model, as in the final design, on desktop and web.

Change: the Overview badge and sheet on desktop/web -> an always-open step list with a header strip, numbered steps, embedded edit rows, and the answer below.

Design: [watchtower/CONTEXT.md](watchtower/CONTEXT.md) (design link; its Spec table holds sizes, colors, and rules)

Boundaries:

- Follows [ADR-0016](watchtower/decisions/ADR-0016-host-turn-step-list-over-plugin-group-renderer.md): render `edit-diff` plugin items through the existing plugin timeline renderer inside the Edit step. Do not draw diffs in the host.
- Always open: no fold, no chevron, no Collapse button.
- Edit step shows each edit-diff row in order (Q-025 default).
- Header: running shows "Working" and a ticking clock; done shows "Worked for <duration>". The rest of the header does not move when the turn ends.
- Running step: number circle in #256ABF that pulses, "now" in the time column, and a live box with the current command and its last 2 output lines. Respect reduced motion.
- Highlight: a click on a strip segment or a row highlights the step and dims other segments; a second click clears it; scroll to the row only when it is off screen. Keep highlight state per turn in the view, not in the store.
- Follow [docs/unistyles.md](docs/unistyles.md) (no `useUnistyles()`), [docs/hover.md](docs/hover.md), and [docs/design.md](docs/design.md). Use the theme tokens; put the label hues in one place.
- Compact form factor and the `detailed` level keep their current views.

How:

- Add [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx): header (title, clock, strip, summary), rail, step rows (number, label, counts, note, time), Edit step slot, running box.
- In [packages/app/src/agent-stream/view.tsx](packages/app/src/agent-stream/view.tsx), render turn groups with the new view and pass a callback that renders a plugin stream item with the existing plugin renderer.
- Add the new strings ("Working", "Worked for", step count, Explore, Edit, Verify, "now") to the locale files that the i18n types require, under the existing `toolCallGroup` keys.
- Add a browser or component test next to the view for the highlight toggle and the running state.

Files:

- [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx) (new step list view)
- [packages/app/src/tool-calls/detail-level/turn/view.test.tsx](packages/app/src/tool-calls/detail-level/turn/view.test.tsx) (new test)
- [packages/app/src/agent-stream/view.tsx](packages/app/src/agent-stream/view.tsx) (use the new view for turn groups)
- [packages/app/src/i18n/resources/en.ts](packages/app/src/i18n/resources/en.ts) (new strings; other locale files only if their types require the keys)

Expected result:

- On live, desktop, Overview level: one agent turn shows one step list with a header strip, numbered steps with labels and times, edit rows inside the Edit step, and the answer below a rule.
- While the agent runs, the header clock ticks, the last segment and number pulse, and the live box shows the current command.
- A click on segment 3 highlights row 3 and dims 4 segments; a second click clears it.
- Cases: compact window width -> today's badge and sheet; `detailed` level -> today's rows; edit-diffs off in a thread -> Paseo edit rows appear inside the Edit step.

Anti-goal: no stream item is lost; every tool call, note, and edit item of a turn appears once, either in the step list or in the main stream; tripwire; read from the view test that counts rendered items against the model, before changes, after wiring, and at completion.

## Verify

- `cd packages/app && npx vitest run src/tool-calls/detail-level/turn/view.test.tsx --bail=1` -> highlight toggle, running state, and item count cases pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/tool-calls/detail-level/turn packages/app/src/agent-stream/view.tsx packages/app/src/i18n/resources/en.ts` -> exit 0.
- Manual on live (needs a screen): run an agent turn with edits on desktop at Overview level -> the step list matches the final design, and the answer sits below the list.
- Manual on live: narrow the window to the compact breakpoint -> the old badge and sheet show.
- Anti-goal: the view test item count equals the model item count for the sample turn -> equal.
