# Plan Context

## Shared Context

- The GPUI client is [packages/desktop-gpui](packages/desktop-gpui). Read [docs/gpui.md](docs/gpui.md) before any change.
- The Electron app in [packages/app](packages/app) is the reference for look and behavior. GPUI must match it.
- Run cargo only through `./cargo.sh` in `packages/desktop-gpui`. Run few crates per test command, because each `-p` builds gpui-kit.
- Small client prefs live under `~/Library/Application Support/BachucGpui/`. Follow the `LayoutPrefs` pattern in [packages/desktop-gpui/crates/app/src/state.rs](packages/desktop-gpui/crates/app/src/state.rs) (lines 219-278): field-by-field defaults on parse, temp file plus rename on save.
- Hover-shown controls hide by opacity, never by `display` ([docs/gpui.md](docs/gpui.md) line 88).
- Never query the system (for example `all_font_names`) or read a `ListState` in a render or row path. See [watchtower/MEMORY.md](watchtower/MEMORY.md).
- Give every pressable element an explicit `AccessibleAction::Click` handler.
- Popup menus with submenus use `gpui_kit::component::menu::PopupMenu`. The sidebar example is `add_plugin_items` in [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (lines 1780-1808). Do not copy the hand-built timeline fork menu.
- New client RPCs follow the fork pattern: `fork_supported` and `fork_context` in [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs) (lines 1308-1336) and [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs) (lines 995-1013), tested like [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs) (lines 2240-2287).
- Screen checks use `cua_repl` AX actions only and never take the cursor or the front ([docs/gpui.md](docs/gpui.md) lines 108-115). Hover, right-click menus, and real menu bar clicks are owner hand checks.
- After building or testing, quit the launched app and run `./cargo.sh clean --profile dev`, unless another cargo process runs.

## Decisions

- GPUI connects to one host ([ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md)), so the Host row item and the Host filter are out of scope.
- Electron tokens and lucide icons are the design source ([ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md)).
- TASK IDs continue at TASK-256. The owner skipped the archive ID migration on 2026-10-19.

## Open Decisions

- None.

## References

- `cd packages/desktop-gpui && ./cargo.sh test -p <crate>`
- [packages/desktop-gpui/docs/parity](packages/desktop-gpui/docs/parity) (Electron vs GPUI reference screenshots)
