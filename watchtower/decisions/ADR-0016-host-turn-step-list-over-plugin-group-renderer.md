# ADR-0016 Host turn step list over a plugin group renderer

Status: accepted
Date: 2026-10-12
Scope: `packages/app/src/tool-calls/**`, `packages/app/src/agent-stream/**`, `packages/app/src/plugins/timeline/**`, `hiep-plugins/plugins/edit-diffs/**`

## Context

- The owner chose a new agent response UI: one always-open step list per turn, with a duration strip, numbered steps, and the answer below. The design is the "Agent Response Final" file in Claude Design project `413513e4-9476-4675-abeb-af840c1285b2`.
- The fork rule in [CLAUDE.md](CLAUDE.md) prefers features in plugins and thin host changes.
- A plugin group renderer needs a new, stable plugin API. Plugin components cannot use the stream model, timestamps, virtualization, or app primitives directly.
- The host already owns tool call grouping in [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts) and has tests for it.

## Decision

- The host groups a whole turn and draws the step list in `packages/app/src/tool-calls/`.
- An Edit step embeds the existing `edit-diff` plugin items through the current timeline renderer. The edit-diffs plugin keeps drawing the diff.
- No new plugin API is added for group rendering.

## Consequences

- Fewer new contracts and fewer places for bugs. The view reuses host tests and stream data.
- Upstream merges can conflict in `packages/app/src/tool-calls/` and `packages/app/src/agent-stream/view.tsx`.
- The host needs one generic marker on plugin items that replaced a tool call, so the group can hold them without a plugin ID check.

## Options Considered

- Thin host plus a group renderer contribution, drawn by edit-diffs: rejected. It needs a new plugin API, and the plugin cannot reach app primitives, so bugs are more likely.
- A new `turn-timeline` plugin: rejected. It would copy or split the diff code that edit-diffs already owns.

## Revisit If

- Upstream merges conflict in `packages/app/src/tool-calls/` or `packages/app/src/agent-stream/view.tsx` in two releases in a row.
- The plugin API gains a group or turn renderer contribution upstream.
