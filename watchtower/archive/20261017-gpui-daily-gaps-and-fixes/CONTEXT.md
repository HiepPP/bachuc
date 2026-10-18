# Plan Context

## Shared Context

- The GPUI client is the Cargo workspace [packages/desktop-gpui](packages/desktop-gpui). Run cargo only through `./cargo.sh` in that folder.
- Electron is the reference look and behavior. It is frozen: read [packages/app/src](packages/app/src) and [hiep-plugins/plugins](hiep-plugins/plugins), never edit them ([ADR-0034](watchtower/decisions/ADR-0034-rust-gpui-client-over-swift-and-electron.md), [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md)).
- This machine runs at most 2 builders, each with `CARGO_BUILD_JOBS=4`. More builders filled 24 GB of RAM and rebooted the Mac on 2026-10-16.
- GPUI view crates have no headless test context. Put each rule (policy, parse, filter, clamp, path rule) in a pure function with a unit test. Check view code by reading it, then by a UI check.
- Test one crate with `./cargo.sh test -p <crate>`. Never run the full JS test suite.
- Set an explicit line height on every text you add. The GPUI default is 1.618, and Electron uses about 1.2 to 1.4.
- Give every pressable element `Role::Button`, an aria label, and an explicit `AccessibleAction::Click` handler. Without the handler, an AX press does nothing.
- Never toggle `display` from hover or `group_hover`; it panics with "must call prepaint before paint". Swap hover content by opacity or visibility.
- Never read a `ListState` or query the system (for example `all_font_names`) inside a row or render path.
- Gate a new daemon capability once on `server_info.features.*` through `Client::server_info`. If the host lacks it, hide the control or show "Update the host to use this." No fallback path.
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) is a single-writer file. Only group S TASKs edit it, one at a time.
- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs), [lib.rs](packages/desktop-gpui/crates/client/src/lib.rs), and [model.rs](packages/desktop-gpui/crates/client/src/model.rs) are single-writer files. Only group C TASKs edit them.
- The app matches `SidebarEvent`, `CommandEvent`, `ExplorerEvent`, `TimelineEvent`, and `DirectoryEvent` exhaustively or with an irrefutable `let`. A new variant there breaks the build of a crate another group owns. Add a new event type with its own `EventEmitter` impl instead, as `SidebarPluginEvent` does.
- Client calls follow the existing pattern: a `core.rs` request with a `Pending::*` reply, and a `lib.rs` wrapper `fn name(&mut self, ..., cx: &mut Context<Client>) -> Task<anyhow::Result<T>>`. Add a test per call in [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs).
- UI checks run on live: daemon `127.0.0.1:6768`, `PASEO_HOME=.dev/paseo-home`. Start live detached with `nohup` and confirm the port listens before a check. Point the GPUI app at 6768, not release 6767.
- Before each commit, check `Cargo.lock` with `./cargo.sh metadata --format-version 1 > /dev/null`. Two builders once left a duplicate entry that broke every cargo command.
- When builders are done, quit any GPUI app they launched and run `./cargo.sh clean --profile dev`, only when no other cargo or rustc runs.

## Decisions

- Owner scope (2026-10-17): fix the 3 reported bugs and the step list look, then port 4 daily gap groups: git actions and PR, History with unarchive and fork, file mentions with drop and the file menu, terminal find and links with shortcuts.
- Fork is a full port: fork to a new tab and to a new workspace, as Electron does.
- A dropped file that is not an image becomes a quoted path mention in the composer.
- Extras in scope: Revert in the file menu, the Scripts header button, and the Open in editor header button. Download is out.
- The Pull request tab opens once by itself when a PR is found, as Electron does.
- Acceptance: unit tests, then a background UI checker on live, then the owner's hand test.

## Open Decisions

- See [watchtower/QUESTIONS.md](watchtower/QUESTIONS.md).

## References

- Last plans: [watchtower/archive/20261016-gpui-electron-parity/NEXT.md](watchtower/archive/20261016-gpui-electron-parity/NEXT.md), [watchtower/archive/20261016-gpui-parity-follow-ups/LEARN.md](watchtower/archive/20261016-gpui-parity-follow-ups/LEARN.md).
- Protocol messages: [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts). Generated Rust schema: [packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json](packages/desktop-gpui/crates/protocol/codegen/protocol.schema.json).
