# TASK-256 Text drawn like Electron

Group: A (writes [packages/desktop-gpui/crates/app/src/main.rs](packages/desktop-gpui/crates/app/src/main.rs); shared file with TASK-257)
Class: code

## Brief

Goal: GPUI text looks as thin and sharp as Electron text. The owner says it looks blurry and different today.

Change: gpui thickens glyph strokes by text luminance -> no glyph dilation, like Electron's `-webkit-font-smoothing: antialiased`.

Boundaries:

- Cause: both apps use SF at the same sizes. gpui-pre 0.3.9 dilates glyphs to copy CoreGraphics font smoothing (`glyph_dilation_for_color`, `gpui-pre-apple-0.3.9/src/text_system.rs:224-261`). Electron turns smoothing off in [packages/app/public/index.html](packages/app/public/index.html) (lines 31-34).
- gpui skips dilation when the app preference `AppleFontSmoothing` is `0`. It reads the value once, in a `OnceLock`, at the first glyph.
- Fix: write `AppleFontSmoothing = 0` for `kCFPreferencesCurrentApplication` at the start of `main`, before `App::run`. Use `CFPreferencesSetAppValue` from `core-foundation-sys`. Do not patch gpui-pre.
- Planner call: write the value on every launch, so a user `defaults` change cannot bring the dilation back.
- Non-goal: no font family, size, weight, or line-height change. TASK-145 already set those from Electron tokens.
- If the owner still sees a difference after this fix, record each one in the outcome with a screenshot pair. Do not guess more changes.

How:

- Add a small macOS-only function in `main.rs` that sets the preference. Add `core-foundation-sys` to the macOS target dependencies of the app crate. Use the version already in [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock), so no second copy builds.
- Call it first in `main`.

Files:

- [packages/desktop-gpui/crates/app/src/main.rs](packages/desktop-gpui/crates/app/src/main.rs) (set the preference at startup)
- [packages/desktop-gpui/crates/app/Cargo.toml](packages/desktop-gpui/crates/app/Cargo.toml) (add `core-foundation-sys`)

Expected result:

- Light text on dark and dark text on light both draw with thin strokes, like Electron.

Anti-goal: text sizes and layout stay the same; the timeline row heights for one fixed thread differ by 0 px before and after; read from a GPUI screenshot pair of the same thread at `BACHUC_WINDOW=900x800`.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds.
- After one launch of the check bundle, `defaults read <check-bundle-id> AppleFontSmoothing` -> `0`.
- Screen check (cua_repl, AX only): screenshot the same thread in GPUI and in Electron live at the same window size -> stroke weight matches; row positions in the GPUI pair before and after differ by 0 px.
- Owner hand check: GPUI text no longer looks blurry next to Electron.
