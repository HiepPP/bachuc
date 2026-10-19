# ADR-0049 GPUI keeps sidebar choices in its own sidebar.json over shared stores

Status: accepted
Date: 2026-10-19
Supersedes: -
Scope: `packages/desktop-gpui/crates/sidebar/src/prefs.rs`, `packages/desktop-gpui/crates/sidebar/src/sidebar.rs`

## Context

- GPUI forgot collapsed projects on relaunch, because `ViewState` lived only in memory (TASK-258).
- Electron keeps the same choices in client-local stores (`sidebar-collapsed-sections`, `sidebar-view`, app settings), keyed by its own project view keys.
- TASK-261 added Electron's Display preferences, which need the same kind of storage.

## Decision

- GPUI saves collapse state and every display choice in `sidebar.json` in its data folder (`bachuc_command::data_dir`), with the `LayoutPrefs` pattern: field-by-field defaults on parse, temp file plus rename on save.

## Consequences

- Running GPUI and Electron side by side never makes them overwrite each other.
- The two apps do not share these choices; a change in one app does not show in the other.
- A check bundle uses the same folder as the daily GPUI app, so screen checks must restore `sidebar.json`.

## Options Considered

- Read Electron's stores: rejected. They are client-local and keyed differently.
- Store the choices on the daemon: rejected. It needs a new protocol field for a per-client view choice.

## Revisit If

- The owner wants the sidebar choices to follow him between GPUI and Electron, or between machines.
