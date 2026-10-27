# ADR-0072 Unread queue from Board Just finished cards over every unread agent of the host

Status: superseded by ADR-0073
Date: 2026-10-27
Supersedes: ADR-0070
Scope: `packages/desktop-gpui/crates/app/**`, `packages/desktop-gpui/crates/command/src/shortcuts.rs`, `packages/desktop-gpui/crates/client/**`

## Context

- ADR-0070 built the Mod+J queue from every unread agent of the active host.
- On live on 2026-10-27, the switcher listed about 15 old finished threads from Oct 19 and Oct 20. They were not on the Board, because Hiep had removed them or the Board keeps only the latest 50 finished runs.
- Hiep asked on 2026-10-27 to list only threads that are on the Board.
- The Board plugin answers `board.snapshot` with its visible runs: `agentId`, `status`, and `unread`. Removed cards are not in it. The host can call it through `Client::invoke_plugin_rpc("board", "board.snapshot", {})`.
- The rest of ADR-0070 still holds: the host app, the popover, the Mod+J keys, oldest first, and no permission or subagent rows. The GPUI client shows one host at a time ([ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md)).

## Decision

- Build the switcher in the GPUI host app, as in ADR-0070.
- The queue holds agents that are both a Board run with a status other than `running`, and unread in the client attention flag with reason finished or error. It leaves out permission, subagents, and the viewed agent. The oldest `attentionTimestamp` comes first.
- The host reads the Board runs with one `board.snapshot` call at each Mod+J and at each After send Next unread. It caches no membership.
- With the Board plugin inactive, or a failed call, the queue is empty and the shell shows a toast.
- Mod+J opens a popover, Cmd+Tab style; a tap opens the oldest thread. Mod+Shift+J returns to the thread left last.
- Hiep chose this on 2026-10-27.

## Consequences

- Mod+J waits for one plugin call before it opens anything. If Cmd is already up when the answer comes, the shell opens the oldest thread at once.
- The switcher needs the Board plugin; without it, Mod+J shows a toast.

## Options Considered

- Every unread agent of the host (ADR-0070): rejected. It showed old threads that Hiep removed from the Board.
- The Board `unread` field alone: rejected. It refreshes every 2 seconds, while the client attention flag changes at once when a thread is opened.
- A membership cache refreshed in the background: rejected. A stale cache shows removed cards, and one call per key press costs little on the local daemon.
- A Board mini board, a chip strip, a side panel, or a header counter: rejected in ADR-0070, for the reasons given there.

## Revisit If

- The Board snapshot call takes more than 200 ms on the tunnel host, so Mod+J feels slow.
- Hiep wants unread threads that the Board does not show.
