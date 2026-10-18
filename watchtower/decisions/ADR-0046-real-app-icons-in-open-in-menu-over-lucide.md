# ADR-0046 Real app icons in the Open-in menu over lucide icons

Status: accepted
Date: 2026-10-18
Supersedes: -
Scope: `packages/desktop-gpui/crates/app/src/chrome.rs`, `packages/desktop-gpui/crates/app/src/state.rs`

## Context

- The GPUI Open-in menu draws generic lucide icons: `Code` for VS Code, Cursor, and Zed, `FolderOpen` for Finder, and `SquareTerminal` for Terminal ([packages/desktop-gpui/crates/app/src/state.rs](packages/desktop-gpui/crates/app/src/state.rs) about line 355).
- [ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md) chose lucide icons for the GPUI client to match Electron.
- The owner wants each app in this menu to show its own icon (planning interview, 2026-10-18).

## Decision

- In the Open-in menu and its split button, draw the real icon of each app from its macOS bundle, and the GitHub mark for "Open on GitHub".
- Load each icon once per launch and cache it. Fall back to the lucide icon when the app is not installed.
- Every other GPUI icon stays lucide per ADR-0043.

## Consequences

- The menu no longer matches Electron exactly; it looks better than Electron here by owner choice.
- Icon loading needs a macOS API call, done once and off the render path.

## Options Considered

- Keep lucide icons and fix only their display: rejected. The owner wants recognizable app icons.

## Revisit If

- The GPUI client must run on a platform without app bundle icons.
