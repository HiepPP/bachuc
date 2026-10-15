# ADR-0038 Native data kinds and WebView panes over all-WebView or full rewrite

Status: proposed
Date: 2026-10-16
Supersedes: -
Scope: `packages/desktop-gpui/**`, `hiep-plugins/plugins/*/index.client.tsx`, `packages/plugin/src/client/**`

## Context

- The 16 client plugins make 14 kinds of contribution; 10 kinds are data items with no free-form UI. Source: [RESEARCH-001](../research/RESEARCH-001-plugin-ui-host.md).
- Free-form UI is needed by 4 pages and 2 workspace panels, plus some of the 6 timeline renderers.
- One hidden WebView adds about 55 to 83 MB and about 0% CPU idle (TASK-137, TASK-141 outcomes).
- The shell reads 9.7% CPU on a young stream and 13.4% on a 15-minute turn (Q-065); the budget has no margin.
- Live WebViews do not clip in GPUI scroll lists ([ADR-0037](ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md)).
- [ADR-0017](ADR-0017-native-host-features-over-thin-host-changes.md): put a feature where it runs natively with the fewest bugs.

## Decision

- Render the data kinds (command center, sidebar, slash command, settings, composer pill, attachment source, theme, transformers) as native GPUI views fed by a data contract.
- Host a free-form page or panel as a plugin pane: a new `TabContent` variant that owns one live WebView in the tab area, outside any scroll list. Decide per plugin later whether to move it to a declarative contract.
- Render timeline renderers as PNG snapshots through the ADR-0037 path.
- Proposed by the implement run; the owner accepts or rejects.

## Consequences

- The plugin client contract gains a data-only form for each data kind; the React form stays for the WebView pane.
- Only open panes cost memory (55 to 83 MB each). Limit open plugin panes.
- Two paths to keep in step until the free-form plugins move.

## Options Considered

- React in a WebView for every surface: rejected. 9 data kinds would need a bridge each, and a pill or command item would cost 55 to 83 MB. Revisit If: a bridge-free host-side React renderer exists, or footprint per WebView falls under 10 MB.
- Declarative UI that GPUI renders for everything: rejected for now. Free-form pages (board, watchtower-board) need many widget types, and the contract is large before the first port. Revisit If: a first ported page fits the contract with fewer than 10 widget types.
- Rewrite or move every plugin into Rust: rejected. 16 plugins, no hot reload, and ADR-0017 reserves host work for host-bound features. Revisit If: plugin pages drop below 2, or plugin hot reload stops being needed.

## Revisit If

- A visible WebView pane raises shell CPU over 2 points while it redraws.
- A plugin pane needs to sit inside a scroll list.
- Q-065 raises the CPU budget or the shell cost falls under 8% on a 15-minute turn.
