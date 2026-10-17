# Plan Context

## Shared Context

- Goal of the plan: turn the GPUI prototype into a desktop client that can replace the Electron UI for the daily flows. Plugins are ported in a later plan.
- Branch: `feat/bachuc-gpui`. Never push from a builder.
- Crate home: [packages/desktop-gpui](packages/desktop-gpui), a Cargo workspace with `members = ["crates/*"]`. Each TASK owns its own crate folder under `crates/`. Do not edit the workspace `Cargo.toml` after TASK-134.
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) is generated. Any builder's cargo run may update it, and it does not tie groups together. Builders in one wave share the workspace and one `target` folder; cargo serializes their builds with its own lock. The lead commits the lock file with the last group of a wave.
- Each crate declares its own dependencies. Pin `gpui-kit = "=0.7.3"` and `gpui-wry = "=0.7.3"` in every crate that uses them, because gpui-kit pins the `gpui-pre =0.3.9` snapshot. A crate may add macOS bindings such as `objc2-app-kit` or `objc2-web-kit` when its TASK says so.
- Build with the rustup toolchain, not Homebrew `rustc` 1.79. TASK-134 adds the wrapper `packages/desktop-gpui/cargo.sh <cargo args>`. It sets `PATH="$HOME/.cargo/bin:$PATH"` and `RUSTUP_TOOLCHAIN=1.97.1`.
- TASK-134 adds `packages/desktop-gpui/bundle.sh <bin|example> <target> <app-name> <bundle-id>`. It builds the release binary or example and wraps it in `packages/desktop-gpui/target/bundles/<app-name>.app`. UI checks run from such a bundle, because `cua_repl` cannot attach to a bare binary.
- The machine has no Xcode Metal compiler. gpui-kit enables `runtime_shaders`, so do not add a feature that needs `xcrun metal`.
- Free disk was 22 GB on 2026-10-15. One `target` folder is about 1.6 GB. Do not create extra target folders per crate.
- Test against the live daemon only: `ws://127.0.0.1:6768/ws`. Never send prompts or create agents on release (6767) in a check, and never restart either daemon.
- Make a test agent with `npm run cli -- run --provider mock --model thirty-minute-stream --cwd /tmp/gpui-bench -d -q "start bench"`. The mock emits a chunk every 50 ms; the clients counted about 10 timeline events per second on 2026-10-15. Stop and archive it after the check with `npm run cli -- stop <id>` and `npm run cli -- archive <id>`, and kill the terminals the check created with `npm run cli -- terminal kill <id>`.
- `hello` needs `clientId`, `clientType`, and `protocolVersion`. The daemon closes the socket if a session message arrives before it answers `hello` with `status` `server_info`. Queue outgoing session messages until then.
- Terminal data uses binary frames `[opcode, slot, payload]`: `0x01` output bytes, `0x02` input bytes, `0x03` resize (JSON `{rows, cols, intent}`), `0x04` snapshot (a JSON cell grid, not bytes), `0x05` restore. Source: [packages/protocol/src/binary-frames/terminal.ts](packages/protocol/src/binary-frames/terminal.ts).
- Performance limit for UI TASKs: at most 10% CPU of one core and at most 250 MB phys_footprint, WebKit helpers included, while showing the mock stream in the foreground for 60 seconds. Measure with [packages/desktop-gpui/measure.sh](packages/desktop-gpui/measure.sh); match WebKit helper pids to the app by start time. Baseline on 2026-10-15: 6.5% CPU and about 197 MB.
- Accessibility: every UI TASK gives its buttons, list rows, messages, and inputs a role and a label with the gpui-pre 0.3.9 element API (`role` and `aria_label` on `div`, built on accesskit). Agents then check UI through the accessibility tree with `cua_repl` `getAXState`, not only by screenshots.

## Decisions

- One host at a time, with a host switcher. No merged "All machines" view in this plan.
- Hosts are plain `ws://` URLs. A remote host such as macmini is reached through a tunnel the owner opens, for example `ssh -L 6769:127.0.0.1:6767 macmini`, then added as `ws://127.0.0.1:6769/ws`. No `wss`, relay, or built-in SSH in this plan.
- The GPUI client only connects to a daemon. Electron and the CLI keep starting and stopping daemons, updates, CLI install, and `paseo://` deep links. The GPUI client does show notifications and the dock badge.
- The timeline follows the existing turn design: [ADR-0018](watchtower/decisions/ADR-0018-all-trailing-messages-over-last-message-as-answer.md), [ADR-0019](watchtower/decisions/ADR-0019-always-open-step-list-over-folding-group.md), [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md), [ADR-0021](watchtower/decisions/ADR-0021-note-opens-its-step-over-tools-before-the-note.md), and [ADR-0022](watchtower/decisions/ADR-0022-scheme-label-tones-in-the-theme-over-color-scheme-branches.md).
- Rust and GPUI client: [ADR-0034](watchtower/decisions/ADR-0034-rust-gpui-client-over-swift-and-electron.md). Generated protocol types: [ADR-0035](watchtower/decisions/ADR-0035-generated-protocol-types-over-hand-written-rust-types.md). Diagrams as PNG snapshots of one hidden WebView: [ADR-0037](watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md).
- Plugin UI is out of scope, except the design TASK-143.

## Open Decisions

- None.

## References

- [docs/timeline-sync.md](docs/timeline-sync.md) (timeline cursors, epochs, gaps)
- [docs/protocol-compatibility.md](docs/protocol-compatibility.md) (optional fields, capability gating)
- [docs/rpc-namespacing.md](docs/rpc-namespacing.md) (request and response names)
- [packages/client/src/daemon-client.ts](packages/client/src/daemon-client.ts) (reference client in TypeScript)
- [packages/protocol/src/client-capabilities.ts](packages/protocol/src/client-capabilities.ts) (hello capability names)
