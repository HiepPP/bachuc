# ADR-0058 GPUI pending turn mark over waiting for the Board poll

Status: accepted
Date: 2026-10-21
Supersedes: -
Scope: `packages/desktop-gpui/crates/client/src/pending.rs`, `packages/desktop-gpui/crates/client/src/lib.rs`, `packages/desktop-gpui/plugins/board/src/**`, `packages/desktop-gpui/plugins/next-prompt-actions/src/**`

## Context

- The Board moves a card only when a `board.snapshot` poll answers. The store polls every 2,000 ms while a run is running and every 15,000 ms when none runs ([run.rs](packages/desktop-gpui/plugins/board/src/model/run.rs) `ACTIVE_BOARD_POLL_MS`).
- A What next Send opens the Board before the prompt RPC starts ([rules.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/rules.rs) `send_plan`). The prompt then goes through the next-prompt-actions server, the daemon, and the board server `agent.turn_started` handler. So Hiep sees the card in Just finished for about 1 s, then it jumps to Running.
- GPUI plugin crates never depend on each other ([ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md)). Both plugins already depend on `bachuc-client`.
- Surface views are cached by their params ([plugin_views.rs](packages/desktop-gpui/crates/app/src/plugin_views.rs) `surface`), so a params hint builds a new Board page per agent.

## Decision

- The GPUI `Client` keeps a client-only pending turn mark per agent, with a 15,000 ms limit. It emits `PendingTurnsChanged` when a mark changes.
- next-prompt-actions marks the agent before it opens the Board, and clears the mark when the send fails or answers `sent: false`.
- The Board store shows a marked agent's finished run as Running at once. It drops the mark when a snapshot shows the run running, when the run ended after the mark, or when the limit passes.
- Only the manual What next Send marks a turn. Composer sends and Start in new thread do not.
- Hiep chose this on 2026-10-21, in the `/watchtower new` interview.

## Consequences

- The card is in Running on the first Board frame after Send, with no wait for a poll.
- A failed send puts the card back in Just finished when the RPC answers.
- If the client and the daemon clocks differ by more than a turn length, a fast turn can stay shown as Running until the next running snapshot or the 15,000 ms limit.
- The mark lives only in memory. A reconnect or an app restart loses it, and the Board waits for the poll again.

## Options Considered

- Refresh the Board on `ClientEvent::AgentsChanged`: rejected. The card still jumps 100-300 ms after the Board shows, because the prompt is not sent yet when the Board opens.
- Pass the agent in the Board surface params: rejected. The view cache builds one Board page per agent, which loses scroll and the selected view.
- Mark every composer send too: rejected for now. Hiep asked only for the What next flow.
- Show a placeholder card for Start in new thread: rejected. The new agent has no id yet, and the real card would show twice when the snapshot lands.

## Revisit If

- Hiep wants composer sends or new threads to move at once on the Board.
- The board server pushes snapshots instead of the client polling them.
