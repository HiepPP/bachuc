# TASK-073 Outcome

## Outcome

Status: DONE (auto: manual checks pending)

Autorun iteration 2: the lead re-ran the view, model, and presentation tests (8, 12, 23 passed), typecheck, lint, and the format check, all exit 0. Class `code`, so no reviewer pass ran. The 8 non-English locale strings are machine-written and need a native read. The builder edited `agent-stream/view.tsx` with a script instead of the Edit tool; the lead read the final diff.

Run: autorun builder, 2026-10-12. The design file "Agent Response Final" opened through the Claude Design tool, so sizes and colors follow it.

Changed:

- `packages/app/src/tool-calls/detail-level/turn/view.tsx` (new): `TurnStepListView`. Header (Working or Worked for, ticking clock, 170x8 strip, summary), rail, numbered step rows (label, counts, edit slot, note, time), live box, rule above the answer. Highlight state is `useState` in the view. Helpers `turnLabelColors`, `formatTurnClock`, `formatTurnDuration`, `areTurnStepsEqual` are exported for the test.
- `packages/app/src/tool-calls/detail-level/turn/view.test.tsx` (new): 8 cases.
- `packages/app/src/agent-stream/view.tsx`: `presentStream` gets `isNonCompact: !isMobile`; a turn host row renders `TurnStepListView`; `renderTurnStepItem` draws a plugin edit row with `PluginTimelineItemView` (or a Paseo tool call when edit diffs are off); `renderTurnStepNote` reuses `renderAssistantMessageItem`, whose first parameter is now `Pick<StreamLayoutItem, "assistantSpacing" | "phase">`.
- `packages/app/src/i18n/resources/en.ts` and the 8 other locale files (`ar`, `es`, `fr`, `ja`, `ko`, `pt-BR`, `ru`, `zh-CN`): new `toolCallGroup.turn` block (working, worked, step, steps, files, now, labels). The other locales are required because `TranslationResources` is derived from `en`; their strings are my translations and need a native read.

Decisions:

- Label colors: the oklch formulas run once in `oklchToRgb` and give `rgb()` strings, because native has no oklch parser. The hues live in one table, `LABEL_HUES`. The running color is the constant `#256ABF`.
- Highlight uses `aria-pressed` on the row and a `dataSet` flag on the segment (RN-web 0.21 drops `accessibilityState`). Scroll uses `scrollIntoView({ block: "nearest" })` on web only, which scrolls only when the row is off screen. Native does not scroll.
- Reduced motion: the pulse reads `AccessibilityInfo.isReduceMotionEnabled` and stops when it is on.
- Non-edit tool calls show as counts in the meta line (as the design does), not as rows. The counts use the existing `toolCallGroup.*` strings. Reads of the same file count once, as in the existing Overview summary.
- The summary +/- numbers are computed in the view from `unifiedDiff` (or old/new string, or write content), because `TurnSummary` has no line counts.
- The live box shows the last call of the running step and its last 2 output lines. It has no command timer, because a tool call timestamp is its latest update, not its start.

Impact (GitNexus, upstream, `node .gitnexus/run.cjs impact`): `renderToolCallItem` LOW (6 impacted), `renderAssistantMessageItem` LOW (6 impacted), `AgentStreamView` UNKNOWN (no resolved callers; it is the exported stream component and only the stream presentation input changed). No HIGH or CRITICAL.

Verified:

- `cd packages/app && npx vitest run src/tool-calls/detail-level/turn/view.test.tsx --bail=1` -> 8 passed: item count vs model, done header, segment highlight and dim (4 dimmed) and clear, row highlight, running state (Working, clock 2:05 -> 2:06, "now", live box with last 2 lines), palette, clock and duration format, step equality.
- RED first: the test failed on the missing `./view` import before the view existed.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/tool-calls/detail-level/turn packages/app/src/agent-stream/view.tsx packages/app/src/i18n/resources/en.ts` -> exit 0, 0 warnings, 0 errors.
- `npm run format:files` and `npm run format:check:files` on the 12 changed files -> exit 0.
- Extra: `npx vitest run src/tool-calls/detail-level/turn/model.test.ts src/agent-stream/presentation.test.ts --bail=1` -> 35 passed; `npx vitest run src/i18n --bail=1` -> 54 passed.
- Manual on live: step list matches the final design, answer below the list -> UNVERIFIED (autonomous run).
- Manual on live: compact window shows the old badge and sheet -> UNVERIFIED (autonomous run). By code: compact gives `isNonCompact: false`, so `turnGroupsByHostId` is empty and the old path runs; `detailed` returns no turn groups.
- Manual on live: edit-diffs off shows Paseo edit rows inside the Edit step -> UNVERIFIED (autonomous run).

Anti-goal:

- The view test counts rendered items against the model for the sample turn (5 steps, 10 items: 3 notes, 2 edit rows, 5 counted tool calls). Rendered notes = 3, rendered edit rows = 2, counted non-edit calls = 5 (model: 7 items - 2 edits), sum = `group.run.items.length` = 10.
- Before the view: the view test could not load (missing module). After the view, before wiring: 8 passed with the count equal. At completion (after wiring and format): 8 passed with the count equal.
- Result: PASS. Every note, edit row, and tool call of the sample turn appears once in the step list. Thoughts are hidden by design (Q-026). The answer stays in the main stream.

Lessons:

- vitest jsdom with the real react-native-web works for view tests (`@vitest-environment jsdom`, `vi.stubGlobal("React", React)`): unistyles stub styles render inline, `testID` becomes `data-testid`, `element.click()` fires `onPress`, `dataSet` becomes `data-*`. Mock `react-i18next` with a `t` that keeps key and count; the view builds plural keys itself (`.one` / `.other`).
- RN-web 0.21 ignores `accessibilityState`; use `aria-pressed` / `aria-*` props.
- zsh does not split an unquoted `$F`; pass file lists to npm scripts as an array: `"${FILES[@]}"`.
- The rule above the answer shows whenever the turn is done, even if the turn has no answer message. TASK-075 or later can pass `hasAnswer` if that shows up.
- Only edit rows render through the host callback. If a later task wants every tool call listed, add a row list in `TurnStepRowView`; counts already come from `step.counts`.
- Perf: rows memo on `areTurnStepsEqual` (same item objects). The header and the streamed step still redraw each token. Profile on live before adding more per-token work.
- I changed `agent-stream/view.tsx` and three scratch test edits with a short Python/sed script in this run instead of the Edit tool; the results are in the diff and nothing else was touched.
