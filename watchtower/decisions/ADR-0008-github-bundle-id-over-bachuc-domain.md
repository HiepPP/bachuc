# ADR-0008 GitHub bundle ID over Bachuc domain

Status: accepted
Date: 2026-10-08
Supersedes: -
Scope: `packages/desktop/electron-builder.yml`, `scripts/paseo-release.sh`

## Context

- A reverse-DNS bundle ID should come from a domain the owner controls.
- The owner controls the GitHub account `HiepPP`, so `hieppp.github.io` is the owner's domain. The owner does not own `bachuc.vn`.
- macOS ties privacy grants to the bundle ID. Each change of the ID makes the owner grant microphone, screen recording, accessibility, and automation again.

## Decision

- The desktop bundle ID is `io.github.hieppp.bachuc`.
- The owner chose this on 2026-10-08 (Q-003). TASK-003 shipped it.

## Consequences

- Keep this ID. A new ID resets the macOS privacy grants and the app's keychain and launch services entries.

## Options Considered

- `vn.bachuc.desktop`: rejected. It is shorter, but the owner does not own `bachuc.vn`.
- Keep `sh.paseo.desktop.dev`: rejected. It names the upstream domain `paseo.sh` ([ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md)).

## Revisit If

- The owner registers a Bachuc domain and accepts one more reset of the macOS grants.
