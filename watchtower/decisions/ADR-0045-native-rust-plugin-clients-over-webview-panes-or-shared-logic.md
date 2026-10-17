# ADR-0045 Native Rust plugin clients over WebView panes or shared server logic

Status: accepted
Date: 2026-10-16
Supersedes: ADR-0038
Scope: `packages/desktop-gpui/**`, `hiep-plugins/plugins/**`, `packages/plugin/src/client/**`

## Context

- The GPUI client has no plugin host: no crate in `packages/desktop-gpui/crates` holds plugin code, so the composer pills of prompt-translate ("Ultra"), skill-pins ("Skills"), and watchtower-board ("Watchtower 11/11") do not show (owner screenshot, 2026-10-16).
- 20 plugins live in `hiep-plugins/plugins`. 14 have a server part that runs in the daemon; their React client parts call it through `rpc(...)` ([docs/plugins.md](../../docs/plugins.md) line 539).
- The owner wants plugins native in GPUI, with no emulation, the lightest runtime, and smooth behavior, and does not weigh effort (planning interview, 2026-10-16).
- The owner plans GPUI as the daily driver once it is stable. Electron stays installed but gets no more updates, so plugin logic does not need one shared copy for both clients (owner, 2026-10-16).
- [ADR-0038](ADR-0038-native-data-kinds-and-webview-panes-over-all-webview-or-rewrite.md) rejected a Rust rewrite because of hot reload. Its Revisit If "plugin hot reload stops being needed" now holds for the client side: the owner accepts an app rebuild for a plugin UI change.

## Decision

- Port the client part of each plugin, its logic and its UI, to native Rust GPUI code. Each plugin gets its own Rust crate, and the app shows it only when the active host reports the plugin as installed and enabled.
- Keep each plugin's server part (`index.server.ts`) in the daemon as it is. The Rust client calls it through the existing plugin RPCs.
- Freeze the React client parts. They keep working on Electron, with no new features and no sync work.

## Consequences

- No JS runtime and no WebView in the GPUI client for plugins, so plugins add no idle memory beyond their native views.
- A plugin UI change needs a GPUI rebuild; server changes still reload with `plugin reload`.
- Plugin behavior on Electron and GPUI can drift after the freeze; GPUI is the reference.
- The Rust client needs a typed call path for plugin RPCs and plugin install state from the daemon.

## Options Considered

- Hybrid: move client logic into each plugin's server part, draw data kinds natively from pushed data, and write pages as Rust crates: rejected. Its gain is one shared logic copy for two clients, and the owner freezes Electron.
- Declarative widgets from the daemon: rejected. It adds an interpretation layer and a large widget contract, and the owner wants the lightest native path.
- A hidden WebView or an embedded JS engine that runs the React clients: rejected. The owner wants no emulation; a WebView costs 55 to 83 MB (ADR-0038 Context).
- WebView panes for free-form pages (ADR-0038): rejected for the same reason.

## Revisit If

- The owner returns Electron to active use and needs new plugin features on both clients.
- A plugin needs UI changes more often than the owner accepts an app rebuild.
