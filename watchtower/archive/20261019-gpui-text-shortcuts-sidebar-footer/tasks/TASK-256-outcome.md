# TASK-256 Outcome

Status: DONE (screen, defaults, and owner checks DEFERRED)

## Changed

- `packages/desktop-gpui/crates/app/src/main.rs`: `disable_font_smoothing()` writes `AppleFontSmoothing = 0` (CFNumber SInt32) with `CFPreferencesSetAppValue` for `kCFPreferencesCurrentApplication`, then `CFPreferencesAppSynchronize`. `main` calls it first, before `App::run`, on every launch.
- `packages/desktop-gpui/crates/app/Cargo.toml`: `core-foundation-sys = "0.8"` in the macOS target dependencies. `Cargo.lock` resolves it to the existing 0.8.7 (one new dependency line for `bachuc-app`, no second copy).

## Contract

- No Produces/Consumes lines. gpui-pre is not patched.

## Verified

- `./cargo.sh build -p bachuc-app` -> builds.
- `./cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `./cargo.sh clippy -p bachuc-app` -> no warning in `main.rs`.
- DEFERRED (UI checker): `defaults read <check-bundle-id> AppleFontSmoothing` -> `0` after one launch.
- DEFERRED (UI checker): GPUI vs Electron screenshot pair, stroke weight and row positions.
- DEFERRED (owner): GPUI text no longer looks blurry.

## Anti-goal

- Timeline row heights unchanged (0 px): DEFERRED (UI checker). The change touches no font family, size, weight, line height, or layout code; it only sets a preference read by gpui's glyph rasterizer.

## Lessons

- gpui reads `AppleFontSmoothing` as a CFNumber; a string or bool value is ignored (`font_smoothing_allowed_by_user`, `gpui-pre-apple-0.3.9/src/text_system.rs:243-262`).
- No unit test: the call writes the user's preferences domain of the test binary.

## UI Checks

Lead, 2026-10-19, check bundle on live (6768), cua_repl:

- `defaults read io.github.hieppp.bachuc-desktop-check AppleFontSmoothing` -> `0`. PASS.
- Screenshot pair with Electron and the row-height pair: UNVERIFIED (unattended). The owner judges blur by eye.
