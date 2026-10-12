# ADR-0021 Note opens its step over tools before the note

Status: accepted
Date: 2026-10-13
Supersedes: -
Scope: `packages/app/src/tool-calls/detail-level/turn/**`, `packages/app/src/tool-calls/detail-level/grouping.ts`

## Context

- The first step model made a step from one assistant note plus the tool calls before it, in `splitSteps` of [packages/app/src/tool-calls/detail-level/turn/model.ts](packages/app/src/tool-calls/detail-level/turn/model.ts).
- A live check on 2026-10-12 showed Claude writing each note before it acts, for example "Step 6: running calc.py again" before the shell call. So every note sat under the work of the step before it.

## Decision

- A note opens its step: a step is one note plus the tool calls after it, up to the next note. Tool calls before the first note form a first step with no note.
- The answer rule of [ADR-0018](watchtower/decisions/ADR-0018-all-trailing-messages-over-last-message-as-answer.md) stays: every assistant message after the last tool call is the answer.
- The owner chose it on 2026-10-13.

## Consequences

- A step's start time is its note time, and its end time is its last tool call.
- An edit after a note joins that note's step, so the Edit label lands on the note that announced the edit.

## Options Considered

- Tool calls before the note, the first model: rejected. Agents narrate before they act, so each note described the next step.
- Choose the rule per provider: rejected. No provider in use narrates after acting, so a mapping table adds code with no gain.

## Revisit If

- A provider in use writes its notes after its tool calls, for example a summary after each batch of work.
