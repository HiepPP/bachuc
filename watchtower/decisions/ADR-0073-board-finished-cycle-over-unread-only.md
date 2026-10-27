# ADR-0073 Mod+J cycles all Board Just finished cards, unread first, over unread cards only

Status: accepted
Date: 2026-10-27
Supersedes: ADR-0072
Scope: `packages/desktop-gpui/crates/app/**`, `packages/desktop-gpui/crates/command/src/shortcuts.rs`, `packages/desktop-gpui/crates/client/**`

## Context

- ADR-0072 limited the Mod+J queue to unread Board Just finished cards. A thread left the queue once it was opened, so Mod+J could not reach a read card.
- Hiep asked on 2026-10-27 to also switch to read finished threads, with Mod+J cycling through them.
- The Board plugin answers `board.snapshot` with its visible runs: `agentId`, `status`, and `endedAt`. The host calls it through `Client::invoke_plugin_rpc("board", "board.snapshot", {})`.
- The rest of ADR-0072 holds: the host app, the popover, one snapshot call per key press, no cache, no permission or subagent rows, and one host at a time ([ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md)).

## Decision

- The Mod+J cycle holds every non-running Board run that is not a subagent and has no permission request.
- Unread agents (client attention reason finished or error) come first, oldest `attentionTimestamp` first. Read agents follow, latest `endedAt` first, as in the Board Just finished column.
- The cycle starts after the viewed thread and wraps. The viewed thread is not a row. So a Mod+J tap on a read card opens the next card, and repeated taps cycle through all cards.
- The switcher marks read rows as read.
- After send Next unread stays unread-only, as its name says ([ADR-0071](watchtower/decisions/ADR-0071-after-send-setting-over-fixed-jump.md)).
- With no rows, Mod+J shows "No finished threads".
- Hiep chose the read cards and the cycle on 2026-10-27. The agent chose the order and the After send rule, and both are open to change.

## Consequences

- Mod+J no longer runs dry after the unread threads are read. It cycles until the Board has no Just finished card.
- The order of read cards follows the Board, so a card that finishes again moves to the unread group.

## Options Considered

- Unread cards only (ADR-0072): rejected. Hiep needs to reach read cards again for more prompts.
- Read cards first, or one list in Board order: rejected. Unread cards are the work that waits, so they come first.
- Include read cards in After send Next unread: rejected. The setting name promises unread, and a send to a read card loop would never end.

## Revisit If

- Hiep wants read cards in Next unread, or a different order of read cards.
