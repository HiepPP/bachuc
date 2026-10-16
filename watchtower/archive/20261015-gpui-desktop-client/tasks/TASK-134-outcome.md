# TASK-134 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/Cargo.toml`: workspace, `resolver = "3"`, `members = ["crates/*"]`, release profile `debug = false`.
- `packages/desktop-gpui/Cargo.lock`: moved from the prototype, updated by cargo (TASK-135 deps too).
- `packages/desktop-gpui/.gitignore`: `target/` only.
- `packages/desktop-gpui/cargo.sh`: rustup toolchain wrapper (`RUSTUP_TOOLCHAIN=1.97.1`).
- `packages/desktop-gpui/bundle.sh`: `bundle.sh <bin|example> <target> <app-name> <bundle-id>`; writes `Info.plist` inside `target/bundles/<app-name>.app`.
- `packages/desktop-gpui/measure.sh`: moved unchanged.
- `packages/desktop-gpui/crates/prototype/`: moved `Cargo.toml` (release profile moved to the workspace), `src/main.rs` (mermaid path now `../../../../node_modules`), `src/daemon.rs`, `src/terminal.rs`. All three sources were reformatted with `cargo.sh fmt`; no behavior change.
- `packages/desktop-gpui/crates/theme/`: new crate `bachuc-theme`.
- `CLAUDE.md`: one repository map line.
- `prototypes/bachuc-gpui`: deleted after the move (the old `target/` folder was moved to `packages/desktop-gpui/target`, so the build cache was reused; `build-app.sh`, `Info.plist`, and the old `.gitignore` were removed, then the empty `prototypes/` folder).

## Contract

- `bachuc_theme::init(cx: &mut gpui_kit::App)`: applies gpui-kit's light or dark theme from the system appearance, and attaches an appearance observer to each window's `gpui_kit::base::Root` as it opens.
- `bachuc_theme::Scheme` (`Light`, `Dark`) and `bachuc_theme::scheme(cx: &gpui_kit::App) -> Scheme` (from `Theme::is_dark`).
- `bachuc_theme::TurnLabel` (`Explore`, `Edit`, `Verify`) and `bachuc_theme::turn_label_colors(label: TurnLabel, scheme: Scheme) -> TurnLabelColors` with `segment`, `text`, `border: Hsla`.

## Verified

- `packages/desktop-gpui/cargo.sh build --release -p bachuc-gpui` -> exit 0 (also rebuilt by `bundle.sh` after formatting).
- `packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> 1 passed (`turn_label_colors_match_the_app`, all 18 colors within 1 unit of the app's rgb values computed on 2026-10-15).
- `test -d prototypes/bachuc-gpui` -> exit 1.
- `git check-ignore packages/desktop-gpui/Cargo.lock` -> exit 1.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-theme -p bachuc-gpui` -> exit 0.
- `npm run format:check:files -- CLAUDE.md` -> exit 0.
- Group-level run (shared with TASK-135): `bundle.sh bin bachuc-gpui BachucPrototype io.github.hieppp.bachuc-prototype` -> exit 0; opened with `--agent 93608375-1318-47d3-9209-6ed7b0ba5ea1` (live mock `thirty-minute-stream`); `cua_repl` screenshot shows the stream (10 events/s, 10 UI updates/s), the terminal (`gpui-bench` prompt), the mermaid diagram, and `connected`. PASS.
- Cleanup: app quit, terminal `80a0e390` killed, agent stopped and archived.

## Anti-goal

- Checkpoint `measure.sh 60` (group-level run), 2026-10-15T09:55:23Z to 09:56:24Z, source `packages/desktop-gpui/measure.sh 60 proto="BachucPrototype.app/Contents/MacOS/bachuc-gpui" wk="pids:30701,30703,30704"` (WebKit GPU, WebContent, Networking pids matched by start time 16:54:55 against app start 16:54:54):
  - proto: CPU 1.4%, footprint 58 MB; WebKit: CPU 0.0%, footprint 57 MB.
  - CPU 1.4 <= 10.0, footprint sum 115 <= 250. PASS.
  - Caveat: the frontmost app during the read was the owner's Arc browser, so the prototype window may have been partly covered. The value is below the 6.5% baseline for that reason; it stays within the limit either way.

## Lessons

- `cargo.sh fmt` reformats the prototype sources; they were never formatted before the move.
- `bachuc_theme::init` must run after `gpui_kit::init`; it calls `Theme::sync_system_appearance`, which needs the theme registry.
- The light and dark schemes are gpui-kit's registry defaults. No Bachuc palette is ported yet.
- Moving the prototype `target/` kept the 1.6 GB cache; a fresh `target/` costs a full GPUI build.

## Questions

- "A light and dark scheme for gpui-kit" was read as gpui-kit's default light and dark themes following the system appearance. If the owner wants Bachuc palette colors, a later TASK must port them.
