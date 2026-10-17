# TASK-170 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/Cargo.toml`: `members = ["crates/*", "plugins/*"]`; the `gpui-kit` features add `tree-sitter-markdown` and `tree-sitter-markdown-inline` (both exist in gpui-kit 0.7.3, and `build -p bachuc-app` passes with them).
- `packages/desktop-gpui/plugins/.gitkeep` (new, empty): cargo rejects the `plugins/*` glob while the folder is missing ("failed to load manifest for workspace member `.../plugins/*`"); with the empty folder, `cargo metadata` exits 0.
- `packages/desktop-gpui/Cargo.lock` (generated): `bachuc-plugin-host` and `tree-sitter-md 0.5.3`.
- `packages/desktop-gpui/crates/plugin-host` (new):
  - `Cargo.toml`: `anyhow`, `bachuc-client`, `bachuc-icons`, `bachuc-theme`, `futures`, `gpui-kit.workspace = true`, `log`, `serde`, `serde_json`.
  - `src/lib.rs`: `NativePlugin`, `Handler`, `ViewFactory`, `PopoverClose`, re-exports.
  - `src/contributions.rs`: `Contributions` and the sidebar, command, page, panel, timeline, and theme kinds.
  - `src/composer.rs`: pill, interceptor, stop button, and attachment kinds; `ComposerDraft`, `pill_applies`, `ComposerAttachment::to_agent_attachment`, `decode_attachment_search`, `search_attachments`.
  - `src/index.rs`: `Contributed<T>`, `ContributionIndex`.
  - `src/host.rs`: `PluginHost`, `PluginHostEvent`, `init`, `host`, `active_plugin_ids`, and the pure `HostModel` (active set, list generation, timeline revision), with unit tests.
  - `src/context.rs`: `PluginContext`, `OpenSurfaceOptions`, `PanelTarget`, `TimelinePassage`, `NewWorkspaceRequest`, `AgentAnchor`, `MessageLevel`.
  - `src/settings.rs`: `PluginSettings`, `SettingsState`, and the pure `SettingsModel`, with unit tests.
  - `src/ui.rs`: `RowText`, `section`, `card`, `switch_row`, `select_row`, `input_row`, `action_row`, `note`, `markdown_style`, `markdown`, plus `divider_above`.
  - `tests/host.rs`: 13 integration tests.

## Contract

- Every Produces line is implemented with its name and type. Each plugin crate exports `pub fn plugin() -> Box<dyn bachuc_plugin_host::NativePlugin>`.
- The tree-sitter markdown feature is enabled in the workspace for TASK-179.
- Additions beyond the spec:
  - `PluginHostEvent::OpenAgentAt { agent_id: String, anchor: AgentAnchor }` carries `open_agent_at`. `OpenAgent` has no anchor field, and the enum is `#[non_exhaustive]`.
  - `ui::divider_above(index) -> bool` is the pure card rule.
  - `decode_attachment_search(Value) -> anyhow::Result<Vec<AttachmentItem>>` is the decode step of `search_attachments`.
  - `ContributionIndex::build(impl IntoIterator<Item = (&'static str, &Contributions)>)` builds the index from plugins in registration order.
- Request routing: `PluginContext` UI requests, `notify`, and `invalidate_timeline` run through `cx.defer` and then update the host. A plugin can call them from `contribute` while the host entity is leased. So `timeline_revision` grows one effect cycle after `invalidate_timeline`, before `TimelineTransformsChanged` is emitted.
- Shared state: `PluginContext::client` and `active_*` read state shared outside the entity. They never read the `PluginHost` entity. Plugins must not read `bachuc_plugin_host::host(cx)` inside `contribute`.
- `show_message` pushes a gpui-kit notification, deferred, to the active window. If no window is active, it uses the first window. If no window exists, it logs.
- `set_client` with the same client entity only lists again, with no deactivation, so pills do not flicker. A different client deactivates every plugin first. Lists run only while the client is connected and again on each `ClientEvent::Connected` and `PluginEvent::CatalogChanged`. A disconnect keeps the active set.
- `PluginSettings`:
  - Loads at creation only when the client is connected; otherwise it waits for `Connected`.
  - With no client, the state is `Error("no active host")`.
  - A `Saved` answer also drops a read that was sent before the save.
  - The work behind `save` and `reset` is held inside the entity, so it finishes even when the caller drops the returned `Task<bool>`. That needs a `futures` oneshot (`futures` is already built by `bachuc-client`).
- Not wired into `bachuc-app`; the app is outside this TASK's Files. With no plugin registered, nothing changes.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-host` -> exit 0. Unit: 12 passed (host model, settings model, card divider). Integration: 13 passed. Cases covered:
  - Object safety.
  - Gating: `enabled: false`, `Disabled`, `Failed`, `Unknown`, listed but not registered, registered but not listed, and registration order.
  - A list `Err` leaves no plugin active.
  - A stale list answer or a host switch is dropped.
  - Only changed plugins contribute.
  - Index order across two plugins; `transformers_for` by type, and an unknown type gives an empty slice.
  - `renderer_for`: version 2 is missing, and of two plugins with the same kind and version the first wins.
  - `surface` found and missing.
  - `pill_applies`: draft with `No`, `Yes`, and `Only`; scoped pills on a draft; `Only` on an agent composer; another agent; another workspace.
  - `ComposerDraft::environment` keys and JSON values.
  - `to_agent_attachment` with a comment, with a blank comment, with no comment, and with `commentable: false`.
  - The search payload decode, including bad payloads.
  - Settings: `Ready`, `Invalid`, and `Err` reads; a stale read; `Saved`; `Conflict`, `Invalid`, and `Err` keep the shown values and set `save_error`.
  - `timeline_revision` grows on a contribution change, a host switch, and `invalidate_timeline`, but not on `notify`, an unchanged set, or another event.
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)" packages/desktop-gpui/crates/plugin-host/Cargo.toml` -> no match (exit 1).
- `rg -n "gpui-kit" packages/desktop-gpui/crates/plugin-host/Cargo.toml` -> one line, `15:gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-host` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 with the markdown features (first build 18:55, second build 19:06).
- `ls packages/desktop-gpui/target/debug/deps/libgpui_kit-*.rlib | wc -l` -> UNSATISFIABLE as written. The count was 4 after the first app build and 5 after `test -p bachuc-plugin-host`, then stayed 5 after the second app build and a repeat test.
  - Reason: cargo unifies features per selected package set. App-only crates (`alacritty_terminal` from the terminal crate, and the `objc2` and `objc2-core-foundation` features) change the features of gpui-kit's own dependencies. So any crate built alone gets its own gpui-kit build. The 3 rlibs that existed before this TASK came from TASK-168's `test -p bachuc-client`, `test -p bachuc-timeline -p bachuc-sidebar`, and app builds.
  - `cargo tree -e features -i gpui-component` gives the same 19 feature lines for `-p bachuc-app` and `-p bachuc-plugin-host`, so the new crate adds no gpui-kit feature.
  - `cargo tree -e features -p bachuc-client` and `-p bachuc-plugin-host` match for every non-bachuc package.
  - `test -p bachuc-client --no-run` compiled no gpui-kit and left the count at 5. So plugin-host reuses the build that the client's own tests already need.
  - Replacement check: the count stays the same across a repeat `test -p bachuc-plugin-host` and a second `build -p bachuc-app` (5, 5, 5). Also, `cargo tree -e features -i gpui-component` must match for `-p bachuc-app` and `-p bachuc-plugin-host`.
- UI checks: none in this spec. The `ui` helpers need `App`, so they were checked by reading only. Their look is UNVERIFIED (unattended): code tests only.

## Anti-goal

- Checkpoint 0, before any change: 3 `libgpui_kit-*.rlib` (`1de61d1f`, `3166ee62`, `951b0301`, from TASK-168 runs); source `ls target/debug/deps`; 2026-10-16 18:54.
- Checkpoint 1, after the workspace `Cargo.toml` change and the first `build -p bachuc-app`: 4 (new `851407a4`, the markdown feature set); source `/tmp/wt-b-build1.txt`; 2026-10-16 18:55.
- Checkpoint 2, after `test -p bachuc-plugin-host`: 5 (new `887e12ac`, the per-selection variant that `bachuc-client` alone also resolves to); source `/tmp/wt-b-test1.txt`, `/tmp/wt-b-client-norun.txt`; 2026-10-16 19:04-19:05.
- Checkpoint 3, at completion, after the second `build -p bachuc-app` and a repeat test: 5; source `/tmp/wt-b-build2.txt`, `/tmp/wt-b-test2.txt`; 2026-10-16 19:06.
- Tripwire: the literal check tripped at checkpoint 2, but by the cause above, not by a feature this crate adds. The count was stable from checkpoint 2 on.

## Lessons

- In a test module, `use super::*` in a file that has `use gpui_kit::*` imports gpui's `test` macro. That macro shadows `#[test]` and fails with "recursion limit reached while expanding `#[test]`". Import the tested names one by one.
- A `plugins/*` workspace glob needs the folder to exist; an empty folder with a `.gitkeep` is enough.
- `libgpui_kit-*.rlib` counts grow by one per distinct package selection (app, a crate's tests alone), because cargo unifies features per selection. To check that a crate adds no GPUI build, use `cargo tree -e features` against the app instead of a raw count.
- GPUI cancels a dropped `Task`. When a method's side effect must finish, as with settings save state, hold the work in the entity and return a task that awaits a oneshot.
