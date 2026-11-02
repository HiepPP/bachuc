# TASK-326 Outcome

## Status

DONE. The two screen checks are DEFERRED (UI checker). `cargo.sh clean` is left to the lead.

## Changed

- New crate `packages/desktop-gpui/plugins/ponytail/` (`bachuc-plugin-ponytail`): `Cargo.toml`, `src/lib.rs`, `src/mode.rs`, `tests/plugin.rs`.
- `packages/desktop-gpui/crates/app/Cargo.toml`: dependency added.
- `packages/desktop-gpui/crates/app/src/plugins.rs`: `PLUGINS` and test `IDS` raised to 17 with `ponytail` between `plugins-nav` and `prompt-translate`; test renamed `the_registered_ids_are_the_seventeen_in_catalog_order`.
- `packages/desktop-gpui/Cargo.lock`: the new crate entry only.

## Contract

- Consumes `ponytail.mode.read` and `ponytail.mode.write` with the shapes of `shared/contracts.ts`; the draft value under plugin id `ponytail` as `{ mode }`.
- One toolbar Select pill, id `ponytail-mode`, title `Ponytail mode`, label `Ponytail: <Label>`, `ShowOnDraft::Yes`. An unknown mode shows `Ponytail: Full`.
- An agent pick shows at once and sends `ponytail.mode.write`. A draft pick stores `{ mode }` and sends no RPC.

## Verified

- Red run: `./cargo.sh test -p bachuc-plugin-ponytail --lib` -> E0432 unresolved imports of `Mode`, `Modes`, `draft_mode`, `draft_payload`, `mode_options`, `pill_label` (expected red).
- `./cargo.sh test -p bachuc-plugin-ponytail` -> 4 unit tests and 1 integration test pass, including the three red tests.
- `./cargo.sh test -p bachuc-app plugins` -> 10 pass, 17 entries.
- `./cargo.sh clippy -p bachuc-plugin-ponytail --no-deps -- -D warnings` -> no warnings, finished.
- `./cargo.sh fmt --check` for `bachuc-plugin-ponytail` and `bachuc-app` -> exit 0.
- `rg -n features plugins/ponytail/Cargo.toml` -> no match.
- GitNexus impact on `PLUGINS`: risk UNKNOWN (no edges). `rg` confirms three uses, all in `crates/app/src/plugins.rs` (definition, `registered_plugins`, one test). Low risk.
- DEFERRED (UI checker): agent composer shows `Ponytail: Full`, picking Ultra shows `Ponytail: Ultra`.
- DEFERRED (UI checker): New workspace draft picks Off, the new thread's pill shows `Ponytail: Off`.

## Anti-goal

At most one `ponytail.mode.read` per agent between reconnects, read from `read_once_per_agent_and_no_loop_after_failure`:

- Before changes: cannot compile (E0432); red.
- After the store was written (`Modes` in `src/mode.rs`): passes.
- At completion: passes.

## Lessons

- The spec's dependency list omits `bachuc-client`. `plugin-host` does not re-export `Client`, and the reconnect clear needs `ClientEvent::Connected`, so the crate adds `bachuc-client` like prompt-translate does.
- `cx.new` on `&mut App` needs `use gpui_kit::AppContext as _`.
- A pick on an agent whose mode is still unknown publishes at once (no `begin_save`), because an unknown mode shows `Full`.
- The first cargo build of a new crate waits on the target lock when another group builds.

## UI Checks

UI checker run 2026-11-02, check bundle `Bachuc GPUI Check` against live (6768), AX actions only. Limit: macOS Stage Manager held the background window in the side strip, so every screenshot is a 178x182 thumbnail and the pill label text is not readable. The AX name of a pill is its title (`Ponytail mode`), not its label. The pill state is proven by the pill menu's `(selected)` row and by the daemon file. Label text `Ponytail: <Label>` is UNVERIFIED by sight.

- C1 PASS (label text UNVERIFIED by sight): agent `ponytail check claude` (`d1e0f51e-0427-4647-b01d-729d7b069c09`) shows a `Ponytail mode` pill before the `Caveman mode` pill in the toolbar. Its menu lists Off, Lite, Full, Ultra with `(selected)` on Off, which matches the saved `off`. After picking Ultra the menu reopens with `(selected)` on Ultra, and `.dev/paseo-home/plugin-data/ponytail/agents/d1e0f51e-0427-4647-b01d-729d7b069c09/mode.json` holds `{"mode":"ultra"}` (modified 11:00:19, at the pick).
- C2 PASS (label text UNVERIFIED by sight): agent `ponytail check codex` (`22933707-6660-4384-8a13-5a962f7592d3`) shows the pill with `(selected)` on Full. Its `mode.json` already held `{"mode":"full"}` (written by the server at session open, so not strictly "no saved pick").
- Screenshots (thumbnails): `/tmp/c1-claude-before.png`, `/tmp/c1-claude-after-ultra.png`, `/tmp/c2-codex-full.png`.
- Cleanup: check app quit, `sidebar.json` and `prompt-history.json` restored from `/tmp/wt-ui326/`. App zoom stayed 1.75 (`appearance.json` unchanged; the AX window zoom was toggled twice, so it is back). `cargo clean` not run. `Bachuc GPUI Check` was the frontmost app for a moment after a Window-menu `makeKeyAndOrderFront` click; the pointer was never moved.
