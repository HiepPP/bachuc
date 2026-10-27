# ADR-0074 Menu bar daemon host over Electron windows

Status: accepted
Date: 2026-10-27
Supersedes: -
Scope: `packages/desktop/src/**`, `scripts/paseo-release.sh`

## Context

- The GPUI client `/Applications/Bachuc GPUI.app` is the daily app ([ADR-0034](watchtower/decisions/ADR-0034-rust-gpui-client-over-swift-and-electron.md), [docs/gpui.md](docs/gpui.md)).
- The release daemon runs on the runtime of `/Applications/Bachuc.app` (Electron). `scripts/paseo-release.sh` opens that app to start the daemon, so a second Bachuc icon stays in the Dock next to GPUI (owner screenshot, 2026-10-27).
- Today the Electron renderer starts the daemon (`packages/app/src/runtime/daemon-start-service.ts`), not the main process.

## Decision

- `Bachuc.app` runs as a menu bar app with no Dock icon and no window. Its menu shows the daemon status and offers Open Bachuc GPUI, Restart daemon, Open logs, and Quit, which keeps the daemon running.
- The main process starts the daemon itself.
- Hiep chose to drop the Electron windows for good, on 2026-10-27.

## Consequences

- Features that only the Electron window has stop being reachable on release, such as its browser pane and settings screens.
- A deep link, `open-url`, or second-instance launch opens Bachuc GPUI instead of an Electron window.
- Live (`npm run dev:desktop`) is not in scope; the menu bar mode is for the installed release app.

## Options Considered

- Keep the Electron window and open it from the menu: rejected. Hiep does not use it, and it brings the Dock icon back while it is open.
- Make GPUI start or host the daemon: rejected for now. The daemon needs the Node runtime that ships in `Bachuc.app`; moving it into the GPUI bundle changes the whole build.

## Revisit If

- Hiep needs an Electron-only feature on release that GPUI does not have yet.
- The GPUI bundle ships its own Node runtime and server, so `Bachuc.app` is no longer needed.
