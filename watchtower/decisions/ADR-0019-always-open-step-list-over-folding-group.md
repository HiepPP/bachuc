# ADR-0019 Always-open step list over a folding group

Status: accepted
Date: 2026-10-13
Supersedes: -
Scope: `packages/app/src/tool-calls/detail-level/turn/**`, `packages/app/src/agent-stream/view.tsx`

## Context

- Before this plan, the Overview level showed each tool group as a folded badge that opens a sheet. A turn with notes and edits showed many small blocks.
- The owner compared layouts in the Claude Design project "Agent Response Layout" and chose the "Agent Response Final" file: one step list per turn that stays open, with a duration strip, numbered steps, and the answer below.

## Decision

- On a non-compact screen at the Overview level, each turn draws as one always-open step list. It has no fold, chevron, or Collapse button.
- Edit diffs inside a step are open by default. A click on the file row hides or shows one diff.
- The owner chose it on 2026-10-12 and confirmed it at archive on 2026-10-13.

## Consequences

- A long turn takes more vertical space than a folded badge. The strip and its highlight are the way to jump to a step.
- Compact screens and the `detailed` level keep their own views, so three layouts exist.

## Options Considered

- A folding group with a chevron, closed by default: rejected. The owner wanted to read a turn's steps without a click per turn.
- Today's Overview badge and sheet on desktop: rejected. One turn split into many small blocks.

## Revisit If

- The owner reports that long turns push the answer too far down, for example a turn over 20 steps.
- Compact screens adopt the step list, because a folded list may then fit a phone better.
