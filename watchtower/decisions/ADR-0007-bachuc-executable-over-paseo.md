# ADR-0007 Bachuc executable over Paseo

Status: accepted
Date: 2026-10-08
Supersedes: -
Scope: `packages/desktop/electron-builder.yml`, `packages/desktop/src/main.ts`, `packages/desktop/scripts/after-pack.js`, `packages/desktop/scripts/after-sign.js`, `packages/desktop/e2e/packaged-app-smoke.js`, `nix/desktop-package.nix`, `scripts/paseo-release.sh`

## Context

- electron-builder names the macOS bundle after `executableName`. Helper apps take their name from `productName`.
- TASK-003 kept the executable `Paseo`, because five packaging files expected `Paseo.app`. Activity Monitor then showed `Paseo`.
- With the executable `Bachuc`, the build output is `packages/desktop/release/mac-arm64/Bachuc.app`, the same name as the release install.
- Linux and Windows launchers expect the executable `Paseo` ([packages/desktop/bin/paseo.cmd](packages/desktop/bin/paseo.cmd), [packages/desktop/scripts/linux-sandbox/index.js](packages/desktop/scripts/linux-sandbox/index.js)).

## Decision

- Set `executableName: Bachuc` under `mac:` only. Linux and Windows keep `Paseo`.
- Release detection checks the install path prefix `/Applications/Bachuc.app/`, not the bundle name. A build opened in place never takes `~/.paseo` and port 6767.
- The owner chose this on 2026-10-08 (Q-005). TASK-009 shipped it.

## Consequences

- Activity Monitor and the bundle show `Bachuc` on macOS.
- Any new macOS packaging path must use `Bachuc.app` and `MacOS/Bachuc`.

## Options Considered

- Keep the executable `Paseo`: rejected. The owner wanted the process name to match the brand.
- A top-level `executableName: Bachuc`: rejected. It also renames the Linux binary and the Windows `.exe`, which breaks their launchers.

## Revisit If

- The owner builds Bachuc for Linux or Windows and wants the brand there too.
