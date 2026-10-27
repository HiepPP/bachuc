# ADR-0070 Host unread switcher on Mod+J over a Board mini board in threads

Status: superseded by ADR-0072
Date: 2026-10-27
Supersedes: -
Scope: `packages/desktop-gpui/crates/app/**`, `packages/desktop-gpui/crates/command/src/shortcuts.rs`, `packages/desktop-gpui/crates/client/**`

## Context

- Hiep wants to move fast between finished threads to send follow-up prompts, without going back to the Board each time.
- The feature needs navigation, a popover, the composer send result, and key release events. These are app internals, so [ADR-0017](watchtower/decisions/ADR-0017-native-host-features-over-thin-host-changes.md) puts it in the host.
- The plugin composer interceptor runs before a send, so a Board plugin could not see a good send without a new plugin API.
- The daemon attention flag (`requiresAttention`, `attentionReason`, `attentionTimestamp`) is the read state the sidebar and the Board already use.
- The GPUI client shows one host at a time ([ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md)).
- Hiep chose in this session: unread only, oldest first, a quick switcher popover, and Mod+J in the Cmd+Tab style with Mod+Shift+J to go back.

## Decision

- Build the switcher in the GPUI host app, from the client attention flag of the active host.
- The queue holds unread agents with reason finished or error. It leaves out permission, subagents, and the viewed agent. The oldest `attentionTimestamp` comes first.
- Mod+J opens a popover, Cmd+Tab style. A tap opens the oldest thread. Mod+Shift+J returns to the thread left last.
- Hiep chose this on 2026-10-27.

## Consequences

- The GPUI client parses `attentionTimestamp`.
- A thread leaves the queue once it is opened, because opening clears its attention flag. Mod+Shift+J is the way back to it.
- The queue does not cover other hosts.

## Options Considered

- A mini board in the Board plugin: rejected. It needs a new plugin API for the send result and the popover, against ADR-0017.
- A chip strip above the composer: rejected. It costs one line of height in every thread and gets crowded with many threads.
- A side panel with compact Board cards: rejected. It takes width from the timeline.
- A header counter with next and previous buttons: rejected. It does not show which threads wait.
- All Just finished cards, or the Board order: rejected. Hiep wants only threads not yet read, oldest first, so none are forgotten.

## Revisit If

- The GPUI client shows more than one host at a time.
- Hiep asks for a preview of the last reply in the switcher (Q-195).
