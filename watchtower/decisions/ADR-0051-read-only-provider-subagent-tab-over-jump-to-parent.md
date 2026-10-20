# ADR-0051 Read-only provider subagent tab over a jump to the parent's tool call

Status: accepted
Date: 2026-10-19
Supersedes: -
Scope: `packages/desktop-gpui/crates/client/src/**`, `packages/desktop-gpui/crates/timeline/src/lib.rs`, `packages/desktop-gpui/crates/composer/src/subagents_track.rs`, `packages/desktop-gpui/crates/app/src/**`

## Context

- In GPUI, a click on a provider subagent row (a Claude or Codex child) does nothing (`packages/desktop-gpui/crates/composer/src/subagents_track.rs:236`).
- Electron opens a read-only tab with the child's live timeline (`packages/app/src/panels/provider-subagent-panel.tsx`), through `agent.provider_subagents.timeline.get` and the `timeline` kind of `agent.provider_subagents.update`.
- The GPUI client already lists provider subagents (`packages/desktop-gpui/crates/client/src/core.rs:876`) but does not fetch their timelines.

## Decision

- Port the Electron tab: the GPUI client loads the child timeline, the timeline view gets a read-only mode, and a row click opens the child in a tab.
- The owner chose this in the planning interview on 2026-10-19.

## Consequences

- The client keeps child timelines by a key that joins the parent and child IDs.
- The timeline view must hide every action that needs a real agent ID, such as fork and rewind.

## Options Considered

- Open the parent agent and scroll to the Agent tool call that started the child: rejected. It shows only the call, not the child's own steps, which is what the owner wants to read.

## Revisit If

- The daemon drops `agent.provider_subagents.timeline.get` or the `projectedSubagentTimeline` feature.
- The owner wants nested child tracks inside the subagent tab, which this port leaves out.
