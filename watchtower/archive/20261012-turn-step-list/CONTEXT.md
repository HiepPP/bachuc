# Plan Context

## Shared Context

- Design: "Agent Response Final" in Claude Design, [open the file](https://claude.ai/design/p/413513e4-9476-4675-abeb-af840c1285b2?file=Agent+Response+Final.dc.html). Its Spec table holds every token, size, and rule. The explorations are in "Agent Response Layout" in the same project.
- Today every assistant sentence ends the current tool group, in [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts). The edit-diffs transformer turns each edit tool call into a `plugin` stream item, which also ends the group. So one turn shows many small blocks.
- The new view is a turn step list. It applies only when the tool call detail level is `overview` and the form factor is not compact. Compact (phone) and the `detailed` level keep their current behavior.
- A step is one assistant note plus the tool calls before it. Edits with no note become one Edit step. The last assistant message of a finished turn is the answer, outside the list. While the turn runs, every message is a step note.
- Step labels come from position, with no model call: before the first edit is Explore, a step with an edit is Edit, after the last edit is Verify. A turn with no edit is all Explore.
- Times come from `timestamp` on stream items in [packages/app/src/types/stream.ts](packages/app/src/types/stream.ts). Items also carry an optional `turnId`.
- Label hues: oklch hue 155 Explore, 55 Edit, 300 Verify. Strip segments use oklch(0.74 0.09 h); text and number color use oklch(0.52 0.11 h); number borders use oklch(0.80 0.06 h). Running color is #256ABF.
- Run app tests from `packages/app`, one file at a time: `npx vitest run <file> --bail=1`.
- Test UI changes on live (`npm run dev`, then `npm run dev:desktop`). Never restart the release daemon on port 6767.

## Decisions

- The host draws the step list; edit-diffs keeps drawing the diff. See [ADR-0016](watchtower/decisions/ADR-0016-host-turn-step-list-over-plugin-group-renderer.md).
- The list is always open. There is no fold, chevron, or Collapse button.
- Edit diffs are open by default. A click on the file row hides or shows the diff.
- A click on a strip segment or a row highlights that step and dims the other segments. A second click clears it. If the row is off screen, the list scrolls to it.
- The Edit step shows each edit-diff row in order; merging per file is a later TASK (Q-025).
- Thought items are hidden in the step list (Q-026).
- The default detail level becomes `overview` (Q-027).

## Open Decisions

- None.

## References

- [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts)
- [packages/app/src/tool-calls/detail-level/projection.ts](packages/app/src/tool-calls/detail-level/projection.ts)
- [packages/app/src/agent-stream/view.tsx](packages/app/src/agent-stream/view.tsx)
- [packages/app/src/plugins/timeline/projection.ts](packages/app/src/plugins/timeline/projection.ts)
- [hiep-plugins/plugins/edit-diffs/README.md](hiep-plugins/plugins/edit-diffs/README.md)
- [docs/design.md](docs/design.md), [docs/hover.md](docs/hover.md), [docs/unistyles.md](docs/unistyles.md)
