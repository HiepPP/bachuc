# TASK-220 Client epoch, discard gate, suffix match, check and script fields

Group: C (shared files [core.rs](packages/desktop-gpui/crates/client/src/core.rs), [lib.rs](packages/desktop-gpui/crates/client/src/lib.rs), [model.rs](packages/desktop-gpui/crates/client/src/model.rs))
Class: code

## Brief

Goal: The client gives later TASKs the data they need for fork cursors, the Discard gate, suffix file links, PR check details, and script URLs.

Change: these values are private or not parsed -> public accessors and fields, each with a test.

Boundaries:

- Timeline epoch: `TimelineSync.epoch` is private ([timeline.rs](packages/desktop-gpui/crates/client/src/timeline.rs) line 113). Add `Client::timeline_epoch`, which returns the epoch of the agent's current timeline window. Electron builds the cursor from the item's `timelineCursor` ([packages/app/src/agent-stream/turn-boundary.ts](packages/app/src/agent-stream/turn-boundary.ts) lines 7-23).
- Discard gate: `features.checkoutDiscardChanges` ([packages/protocol/src/messages.ts](packages/protocol/src/messages.ts) lines 3835-3836), gated as `fork_cursor_supported` is (`core.rs` lines 1273-1275, wrapper at `lib.rs` line 965). Q-123.
- Suffix match: the daemon accepts `matchMode: "fuzzy" | "suffix"` on `directory_suggestions_request` (`messages.ts` lines 2579-2584). Add a new method; keep `directory_suggestions` unchanged, because the composer, the terminal, and the app call it. The test at `tests/rpcs.rs` line 2251 that asserts no `matchMode` stays true for the old method. Q-134.
- PR checks: parse `duration: Option<String>` and `traits: Vec<String>` (`messages.ts` lines 4083-4095) into `PrCheck`. Q-124.
- Scripts: parse `proxyUrl`, `localProxyUrl`, and `publicProxyUrl` (`messages.ts` lines 4036-4048) into `WorkspaceScript`. Q-130.
- Adding fields to `PrCheck` and `WorkspaceScript` also breaks the literals in `tests/rpcs.rs` (lines 1573, 1579, 1864), which this TASK fixes, and two test literals outside this crate: `crates/explorer/src/pull_request_view.rs` line 1578 and `crates/app/src/state.rs` line 808. Do not edit them; TASK-222 fixes the explorer literal and TASK-231 fixes the app literal, because their groups own those files. Until then `cargo test -p bachuc-explorer` and `-p bachuc-app` do not compile; `cargo build -p bachuc-app` (no tests) must still pass. Planner call.

How:

- Add the accessors, the enum, the method, and the fields. Follow the existing `core.rs` request plus `lib.rs` wrapper pattern.
- Add one test per item in `tests/rpcs.rs`: epoch after a page and after a reset page, `None` for an agent with no timeline, discard gate on and off, `matchMode: "suffix"` sent by the new method, check duration and traits parsed, script URLs parsed.

Files:

- [packages/desktop-gpui/crates/client/src/core.rs](packages/desktop-gpui/crates/client/src/core.rs)
- [packages/desktop-gpui/crates/client/src/lib.rs](packages/desktop-gpui/crates/client/src/lib.rs)
- [packages/desktop-gpui/crates/client/src/model.rs](packages/desktop-gpui/crates/client/src/model.rs)
- [packages/desktop-gpui/crates/client/src/timeline.rs](packages/desktop-gpui/crates/client/src/timeline.rs) (epoch accessor)
- [packages/desktop-gpui/crates/client/tests/rpcs.rs](packages/desktop-gpui/crates/client/tests/rpcs.rs)

Interfaces:

- Produces: `Client::timeline_epoch(&self, agent_id: &str) -> Option<&str>` for TASK-221.
- Produces: `Client::discard_changes_supported(&self) -> bool` for TASK-222.
- Produces: `MatchMode` enum `{ Fuzzy, Suffix }`, exported from `bachuc_client`, for TASK-227.
- Produces: `Client::directory_suggestions_matching(&mut self, cwd: &str, query: &str, include_files: bool, include_directories: bool, limit: u32, mode: MatchMode, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<DirectorySuggestion>>>` for TASK-227.
- Produces: `PrCheck { name: String, status: String, url: Option<String>, workflow: Option<String>, duration: Option<String>, traits: Vec<String> }` for TASK-222 and TASK-224.
- Produces: `WorkspaceScript { name: String, kind: String, lifecycle: String, health: Option<String>, terminal_id: Option<String>, exit_code: Option<i32>, proxy_url: Option<String>, local_proxy_url: Option<String>, public_proxy_url: Option<String> }` for TASK-231.

Expected result:

- After a timeline page loads, `timeline_epoch` returns its epoch; before any page it returns `None`.
- A host without `checkoutDiscardChanges` gives `discard_changes_supported() == false`.

Anti-goal: the old `directory_suggestions` request must stay byte-for-byte the same; tripwire; the existing `tests/rpcs.rs` test near line 2251 passes unchanged before and after.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-client` -> passes, with the 5 new tests.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds.
- Anti-goal: the existing no-`matchMode` test passes with its body unchanged (`git diff` shows no edit to it).
