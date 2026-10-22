# ADR-0063 Host-drawn selection result card over plugin-drawn views

Status: accepted
Date: 2026-10-22
Supersedes: -
Scope: `packages/desktop-gpui/crates/plugin-host/src/contributions.rs`, `packages/desktop-gpui/crates/timeline/src/plugin_view.rs`, `packages/desktop-gpui/crates/timeline/src/plugins.rs`

## Context

- A GPUI selection action only runs a handler: `run_selection_action` in [packages/desktop-gpui/crates/timeline/src/plugin_view.rs](packages/desktop-gpui/crates/timeline/src/plugin_view.rs) clears the selection and calls it. No plugin API shows content at the selection.
- Selection translate needs to show text next to the selected passage.

## Decision

- The host draws a result card at the selection. A plugin adds a `SelectionResultAction` whose handler returns text; the host shows loading, the text as markdown with a Copy button, or the error.
- Hiep chose this on 2026-10-22 in the watchtower interview for TASK-293.

## Consequences

- Every result card has the same look, and a plugin writes no UI for it.
- A plugin cannot add its own buttons or layout to the card.

## Options Considered

- The host anchors a plugin-made `AnyView` at the selection: rejected. Each plugin would have to draw loading, errors, and style itself.
- A note under the message, through a timeline renderer: rejected. It is far from the reading point and makes the timeline longer.
- A toast: rejected. It closes on its own and cuts long text.

## Revisit If

- A second result action needs controls other than Copy.
