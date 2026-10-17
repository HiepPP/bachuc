# ADR-0041 GPUI client connects only over daemon management in the client

Status: accepted
Date: 2026-10-16
Supersedes: -
Scope: `packages/desktop-gpui/**`

## Context

- Electron and the CLI already start and stop daemons, install updates and the CLI, and handle `paseo://` deep links (`packages/desktop/src/main.ts`).
- The release daemon owns every running agent and must not restart by accident (root `CLAUDE.md`, Critical rules).
- The GPUI plan chose this scope in its planning interview on 2026-10-15 ([archived CONTEXT.md](watchtower/archive/20261015-gpui-desktop-client/CONTEXT.md)).

## Decision

- The GPUI client only connects to a running daemon. It shows notifications and sets the dock badge.
- Electron and the CLI keep daemon start and stop, updates, CLI install, and deep links.

## Consequences

- The GPUI app needs a daemon that something else started.
- The GPUI app cannot restart the release daemon.

## Options Considered

- Port daemon management into the GPUI client now: rejected. It duplicates Electron code and adds a path that can stop every running agent.

## Revisit If

- The owner retires the Electron app, so nothing else starts the daemon.
