# Plan Context

## Shared Context

- Goal: make the Bachuc GPUI client in [packages/desktop-gpui](packages/desktop-gpui) look and work like the Electron app for daily use. Exact look, per [ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md).
- Branch: `feat/bachuc-gpui`. Builders never commit, push, or switch branches.
- The previous plan built the client: [watchtower/archive/20261015-gpui-desktop-client](watchtower/archive/20261015-gpui-desktop-client). Read its [LEARN.md](watchtower/archive/20261015-gpui-desktop-client/LEARN.md) and the outcome Lessons of the crate you change.
- Cargo: run only `packages/desktop-gpui/cargo.sh`. The workspace has `members = ["crates/*"]`, so a new crate needs no workspace edit. Pin `gpui-kit = "=0.7.3"` in every crate that uses it. `Cargo.lock` is generated and does not tie groups together; the lead commits it with the last group of a wave.
- gpui-kit components live in `gpui-component` 0.7.3 under `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-component-0.7.3/src`, reached as `gpui_kit::component::*`. Useful ones: `tab`, `resizable`, `dock`, `tree`, `list`, `table`, `command`, `popover`, `menu`, `dialog`, `select`, `switch`, `tooltip`, `kbd`, `diff`, `highlighter`, `title_bar`.
- Do not enable the gpui-kit `test-support` feature or the tree-sitter `css`, `html`, `toml`, `yaml` features: they break `cocoa` resolution. Test views through plain functions.
- Hold a strong `Entity<Client>` in each view. A `cx.subscribe` holds its emitter weakly.
- Every new view uses `bachuc_theme` tokens and metrics and `bachuc_icons` icons, never literal colors, sizes, or glyphs.
- Every button, row, tab, input, and menu item gets a role and a label (gpui-pre `role` and `aria_label`). Keep labeled chrome views uncached: a cached view drops out of the AX tree.
- Performance limit: the app stays at or below 10% CPU of one core and 250 MB phys_footprint, WebKit helpers included, with one mock stream tab and one terminal tab in front for 60 seconds, at a turn age of about 2 and about 15 minutes ([ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md)). Keep per-frame work in proportion to the change.
- Functional checks use the live daemon `ws://127.0.0.1:6768/ws`. Never send prompts to, create on, or archive on release (6767). Never restart a daemon.
- Mock agent: `npm run cli -- run --provider mock --model thirty-minute-stream --cwd /tmp/gpui-bench -d -q "start bench"`. Stop and archive it after the check, and kill the terminals the check made. For a CPU read at a turn age of about 15 minutes, use the run-start `gpui-age` agent from `## Environment` in [watchtower/AUTORUN.md](watchtower/AUTORUN.md) when its age fits; never stop it. `create_terminal` needs a daemon workspace folder: `/tmp/gpui-bench` works, `/tmp` does not.
- Parity reference (planner call): open the in-place Electron build `packages/desktop/release/mac-arm64/Bachuc.app` (built 2026-10-15 14:19, after the last app source change at 11:34). It uses home `~/.paseo-dev` and listen `127.0.0.1:6770` (`packages/desktop/src/main.ts` lines 116-117 and 136; `~/.paseo-dev/config.json` holds `127.0.0.1:6770`) only when the launching shell has no `PASEO_HOME` or `PASEO_ELECTRON_USER_DATA_DIR`. An agent shell inherits `PASEO_HOME=~/.bachuc`, so launch it with `env -u PASEO_HOME -u PASEO_ELECTRON_USER_DATA_DIR open packages/desktop/release/mac-arm64/Bachuc.app`, then confirm with `lsof -iTCP:6770 -sTCP:LISTEN` that its daemon listens. If no daemon listens on 6770, add live `ws://127.0.0.1:6768/ws` as a host in the reference app and compare on live. Never let the reference app connect to 6767. For a side-by-side check, point the GPUI bundle at `ws://127.0.0.1:6770/ws` through a temporary `hosts.json` entry, open the same agent or workspace in both apps at the same window size, and take `cua_repl` screenshots of both. Use only mock agents on 6770. Restore `hosts.json` and quit the reference app after the check.
- Each UI TASK writes a parity list in its outcome: one row per element with layout, spacing, color, icon, and state, and match or mismatch.
- Screen checks: builders run none; they return each app screenshot, `cua_repl` step, AX tree read, and `measure.sh` read as `DEFERRED (UI checker)`. One UI checker agent runs them, one at a time, per the watchtower skill.
- UI automation: only the `cua_repl` MCP `js` tool. Pass `timeout_ms` to every call and wrap each step in `Promise.race` with a 60-second limit. A tested app never comes to the front (owner answer, 2026-10-16): launch it with `open -g`, and never activate or raise it. GPUI draws only while its window is in front (Q-069), so a screenshot or CPU read that needs a drawn window is `UNVERIFIED (unattended): needs foreground`. Read `getAXState` twice, and check a screenshot for a cut window before each CPU read.
- Screen lock: take `until mkdir /tmp/gpui-ui-lock 2>/dev/null; do find /tmp/gpui-ui-lock -maxdepth 0 -mmin +15 -exec rmdir {} \; ; sleep 20; done` before any `cua_repl` session or `measure.sh` read. A lock older than 15 minutes is stale and breaks. Release with `rmdir /tmp/gpui-ui-lock`, hold it at most 15 minutes, and quit the bundle first.
- Bundle a crate example with `packages/desktop-gpui/bundle.sh example <example> <AppName> <bundle id>`; measure with `packages/desktop-gpui/measure.sh 60 app="pids:<pid>" wk="pids:<WebKit pids>"`.
- Format markdown and JS with `npm run format:files -- <file> <file>`, one argument per file. Treat "Expected at least one target file" as a failed check.

## Decisions

- Scope: daily-use parity (owner, 2026-10-16). Out of scope: plugin surfaces ([ADR-0038](watchtower/decisions/ADR-0038-native-data-kinds-and-webview-panes-over-all-webview-or-rewrite.md) is a later plan), the Electron browser pane, daemon management ([ADR-0041](watchtower/decisions/ADR-0041-gpui-client-connects-only-over-daemon-management.md)), schedules, session import, dictation, provider usage, the PR panel, rewind and fork, the status-grouped sidebar list, sidebar display options, drag-to-split, the new workspace screen, the open project flow, the skills picker, and the send-behavior setting.
- The sidebar follows Electron exactly: project > workspace rows, no agent rows; agents open as tabs of their workspace (planner call from the Builder Check, 2026-10-16).
- Themes: `light` and `claude` dark, with a system switch ([ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md)).
- Splits: split right and split down from a menu and shortcuts, with resizable dividers. No drag-to-split drop zones.
- Settings: hosts, appearance, keyboard shortcuts, providers, and project settings.
- Proof: side-by-side screenshots with a parity list per surface.
- New views go into new crates so builders run in parallel. Only TASK-155 edits `crates/app`.

## Open Decisions

- None.

## References

- Electron surfaces: [packages/app/src/components/left-sidebar.tsx](packages/app/src/components/left-sidebar.tsx), [packages/app/src/screens/workspace/workspace-screen.tsx](packages/app/src/screens/workspace/workspace-screen.tsx), [packages/app/src/panels/agent-panel.tsx](packages/app/src/panels/agent-panel.tsx), [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx), [packages/app/src/command-center/command-center.tsx](packages/app/src/command-center/command-center.tsx)
- Design: [docs/design.md](docs/design.md), [packages/app/src/styles/theme.ts](packages/app/src/styles/theme.ts), [packages/app/src/constants/layout.ts](packages/app/src/constants/layout.ts), [packages/app/src/components/ui/control-geometry.ts](packages/app/src/components/ui/control-geometry.ts)
- Protocol: [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts), [docs/protocol-compatibility.md](docs/protocol-compatibility.md), [docs/rpc-namespacing.md](docs/rpc-namespacing.md)
