# ADR-0002 Networked HTML replies over offline-only frames

Status: accepted
Date: 2026-10-06
Supersedes: -
Scope: `hiep-plugins/plugins/html-reply/**`, `packages/app/src/plugins/react-native/html-frame*`, `packages/plugin/src/client/ui.ts`

## Context

- Plan `20261006-t3code-orchestration-port` adds HTML replies: an agent publishes a self-contained HTML page that renders in its thread (TASK-008, TASK-009).
- The file preview frame in [packages/app/src/file-pane/html-preview.web.tsx](packages/app/src/file-pane/html-preview.web.tsx) uses `sandbox="allow-scripts"` and a CSP that blocks the network.
- An offline frame cannot load chart libraries from a CDN or call an API, which limits what an agent page can show.
- The frame sandbox has no `allow-same-origin`, so a page cannot read app cookies, storage, or the daemon session.

## Decision

- HTML reply frames may use the network: scripts, styles, images, and fetch over `https:`.
- The sandbox stays `allow-scripts` only. No `allow-same-origin`, no top navigation, no popups.
- `HtmlFrame` takes a network option. The file preview keeps its offline CSP.
- The owner chose this on 2026-10-06 during the autorun Start questions (Q-008).

## Consequences

- An agent page can send data it holds to a remote server. Agents already have network access, so this adds no new data path for the agent itself.
- A page loaded from an untrusted prompt can make network calls from the app. The sandbox still blocks access to app state.
- Tests must assert the sandbox tokens, because they are now the main guard.

## Options Considered

- Offline-only frame with the preview CSP: rejected. The owner wants agent pages that load CDN libraries and call APIs.

## Revisit If

- An HTML reply page leaks app or daemon data, or a sandbox escape is found in the frame.
- The owner asks to show pages from untrusted sources, such as third-party agents or shared threads.
