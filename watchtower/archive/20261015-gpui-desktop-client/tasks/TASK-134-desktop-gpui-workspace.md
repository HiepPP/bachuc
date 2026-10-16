# TASK-134 Move the prototype into the desktop-gpui workspace

Group: A (the owner approved one group for the workspace bootstrap with TASK-135)
Class: mechanical

## Brief

Goal: Create the Cargo workspace [packages/desktop-gpui](packages/desktop-gpui), move the GPUI prototype into it as a runnable baseline, and add the shared theme crate and build scripts that every later TASK uses.

Change: untracked [prototypes/bachuc-gpui](prototypes/bachuc-gpui) -> tracked workspace with `crates/prototype` and `crates/theme`.

Boundaries:

- Follows [ADR-0034](watchtower/decisions/ADR-0034-rust-gpui-client-over-swift-and-electron.md): Rust and GPUI client while it beats Swift and Electron on the measured stream.
- Keep `crates/prototype` working and unchanged in behavior. It is the performance baseline. A later plan removes it.
- The workspace `Cargo.toml` lists `members = ["crates/*"]`, so later TASKs add a crate without editing it.
- Commit `Cargo.lock`, because the app pins a GPUI snapshot.
- Theme colors follow [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md) and [ADR-0022](watchtower/decisions/ADR-0022-scheme-label-tones-in-the-theme-over-color-scheme-branches.md). Port the oklch formula from [packages/app/src/styles/turn-label-colors.ts](packages/app/src/styles/turn-label-colors.ts): hues explore 155, edit 55, verify 300; segment oklch(0.74, 0.09, hue); text and border tones per scheme as in `LABEL_TONES`.
- Planner call: `bundle.sh` writes `Info.plist` inside `target/`, so no plist file is tracked.

How:

- Move the prototype files into `crates/prototype`. Fix the mermaid path to reach the repo `node_modules` from the new depth.
- Write the workspace `Cargo.toml` with `resolver = "3"` and the release profile of the prototype.
- Add `cargo.sh`, `bundle.sh`, and move `measure.sh`.
- Add `crates/theme` with the turn label colors and a light and dark scheme for gpui-kit.
- Delete the old untracked folder [prototypes/bachuc-gpui](prototypes/bachuc-gpui) after the moved build passes.
- Add one line for `packages/desktop-gpui` to the repository map in [CLAUDE.md](CLAUDE.md).

Files:

- [packages/desktop-gpui/Cargo.toml](packages/desktop-gpui/Cargo.toml) (new workspace manifest)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (moved from the prototype, then updated by cargo)
- [packages/desktop-gpui/.gitignore](packages/desktop-gpui/.gitignore) (ignore `target/` only)
- [packages/desktop-gpui/cargo.sh](packages/desktop-gpui/cargo.sh) (new toolchain wrapper)
- [packages/desktop-gpui/bundle.sh](packages/desktop-gpui/bundle.sh) (new: `bundle.sh <bin|example> <target> <app-name> <bundle-id>` builds release and wraps the binary in `target/bundles/<app-name>.app`)
- [packages/desktop-gpui/measure.sh](packages/desktop-gpui/measure.sh) (moved unchanged)
- [packages/desktop-gpui/crates/prototype](packages/desktop-gpui/crates/prototype) (moved `Cargo.toml`, `src/main.rs`, `src/daemon.rs`, `src/terminal.rs`)
- [packages/desktop-gpui/crates/theme](packages/desktop-gpui/crates/theme) (new crate `bachuc-theme`)
- [CLAUDE.md](CLAUDE.md) (one repository map line)
- [prototypes/bachuc-gpui](prototypes/bachuc-gpui) (deleted after the move)

Interfaces:

- The scripts `cargo.sh` and `bundle.sh` are shared tools described in [watchtower/CONTEXT.md](watchtower/CONTEXT.md), not code interfaces.
- Produces: `bachuc_theme::init(cx: &mut gpui_kit::App)` for TASK-141
- Produces: `bachuc_theme::Scheme` (enum `Light`, `Dark`) and `bachuc_theme::scheme(cx: &gpui_kit::App) -> Scheme` for TASK-138, TASK-141
- Produces: `bachuc_theme::TurnLabel` (enum `Explore`, `Edit`, `Verify`) and `bachuc_theme::turn_label_colors(label: TurnLabel, scheme: Scheme) -> TurnLabelColors` with fields `segment: Hsla`, `text: Hsla`, `border: Hsla` for TASK-138

Expected result:

- `cargo.sh build --release -p bachuc-gpui` builds the moved prototype.
- The bundled prototype connects to live and streams a mock agent, as before the move.
- `cargo.sh test -p bachuc-theme` passes. A test compares each turn label color with the rgb values that [packages/app/src/styles/turn-label-colors.ts](packages/app/src/styles/turn-label-colors.ts) produces, within 1 unit per channel.
- [prototypes/bachuc-gpui](prototypes/bachuc-gpui) no longer exists.

Anti-goal: the moved prototype stays within 10% CPU and 250 MB phys_footprint on the mock stream; tripwire; read with `measure.sh 60` at completion.

## Verify

- `packages/desktop-gpui/cargo.sh build --release -p bachuc-gpui` -> exit 0
- `packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> all tests pass, including the color comparison test
- `test -d prototypes/bachuc-gpui` -> exit 1
- `git check-ignore packages/desktop-gpui/Cargo.lock` -> exit 1, so the lock file is tracked
- Agent check with a live mock agent: `bundle.sh bin bachuc-gpui BachucPrototype io.github.hieppp.bachuc-prototype`, open it with `--agent <id>`, and take a `cua_repl` screenshot -> the stream, terminal, and mermaid panes render
- Anti-goal: `measure.sh 60 proto="BachucPrototype.app/Contents/MacOS/bachuc-gpui" wk="pids:<WebKit pids>"` -> CPU at most 10.0 and footprint sum at most 250
