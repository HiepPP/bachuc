# ADR-0040 One host with plain ws:// URLs over merged hosts and built-in transports

Status: accepted
Date: 2026-10-16
Supersedes: -
Scope: `packages/desktop-gpui/crates/client/**`, `packages/desktop-gpui/crates/app/**`

## Context

- The Electron app has an "All machines" mode with a null `activeServerId`, which needs extra cases in every view (root `CLAUDE.md`, Debugging).
- The owner reaches remote hosts over LAN or Tailscale, and upstream hosted services are off ([ADR-0005](ADR-0005-upstream-services-off-over-self-hosting.md)).
- The GPUI plan chose this scope in its planning interview on 2026-10-15 ([archived CONTEXT.md](watchtower/archive/20261015-gpui-desktop-client/CONTEXT.md)).

## Decision

- The GPUI client connects to one host at a time and switches hosts with a host switcher. A host switch closes the old host's tabs.
- A host is a plain `ws://` URL in `hosts.json`. A remote host is reached through a tunnel the owner opens, for example `ssh -L 6769:127.0.0.1:6767 macmini`.

## Consequences

- Every view reads one client and one directory; no view handles a merged host list.
- Remote use needs a tunnel per host, opened outside the app.

## Options Considered

- A merged "All machines" view: rejected. It needs null-host cases in every view.
- `wss`, the relay, or built-in SSH: rejected. They add transport code that a tunnel already covers.

## Revisit If

- The owner needs agents of two hosts on screen at the same time.
- A host can be reached only through the relay.
