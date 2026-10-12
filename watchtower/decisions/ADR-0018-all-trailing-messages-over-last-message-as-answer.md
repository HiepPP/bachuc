# ADR-0018 All trailing messages over the last message as the turn answer

Status: accepted
Date: 2026-10-13
Supersedes: -
Scope: `packages/app/src/tool-calls/detail-level/**`

## Context

- The turn step list puts the answer of a finished turn below the list. The first rule took only the last assistant message group as the answer, in `findAnswerStart` of [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts).
- A live check on 2026-10-12 showed Claude sending one final answer as two text messages. The first part became the last step note, and only the rest sat below the list (finding L1 of PR #60).

## Decision

- The answer is every assistant message after the last tool call of the turn, with the thoughts between them.
- The owner chose it on 2026-10-12, as the L1 fix of PR #60, and confirmed it at archive on 2026-10-13.

## Consequences

- A note that the agent writes after its last tool call, before the real answer, also moves below the list.
- A step note always has tool work after it, so a test fixture needs a tool call between two notes to keep the first one inside the list.

## Options Considered

- Only the last message group as the answer: rejected. Providers split one answer into several messages, so part of the answer showed as a step note.

## Revisit If

- A provider marks its final answer explicitly, for example with a turn-end flag on the message, so the host no longer has to guess from position.
