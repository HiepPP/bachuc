# TASK-154 outcome

Status: DONE (screen checks deferred)

## Changed

- `packages/desktop-gpui/crates/terminal/Cargo.toml`: add `bachuc-theme`.
- `src/emulator.rs`: new `Palette` (ANSI 16, background, foreground, cursor, selection) from `bachuc_theme::Tokens`; `Emulator::set_palette` and `palette`; color replies (OSC 4/10/11) use the palette; `reset` keeps the palette. The fixed `BACKGROUND`, `FOREGROUND`, `SELECTION` constants and the fixed ANSI table are gone.
- `src/view.rs`: font family is the first installed of the `terminal-font.ts` list (fallback Menlo); size is `code_font_size(FONT_CODE)`; `TerminalView` observes the gpui-kit `Theme` global and recolors, remeasures, and lets the next paint resize the grid and PTY.
- `src/lib.rs`: export `Palette`.

## Contract

`TerminalView`, `TerminalEvent`, and every earlier public item are unchanged. Added: `Palette`, `Emulator::palette`, `Emulator::set_palette`.

## Verified

- `cargo.sh test -p bachuc-terminal` -> 31 pass (29 old + palette-per-index test + recolor-survives-reset test).
- `cargo.sh fmt --check -p bachuc-terminal` -> exit 0.
- Screen checks, parity list, anti-goal: DEFERRED (UI checker).

## Anti-goal

Not measured: `time yes | head -n 200000` and `measure.sh 60` need the screen (DEFERRED). Per-cell work is unchanged (Hsla copy instead of a u32 convert).

## Lessons

- Theme and font-scale changes reach the view through `observe_global::<gpui_kit::component::theme::Theme>`, because `bachuc_theme::set_theme` and `set_font_scale` write that global.
- Terminal selection tokens carry alpha (0.15 light, 0.2 dark); they draw over the pane background.
- Indexes 16-255 keep the standard xterm cube and gray ramp; only 0-15 come from tokens.

## UI Checks

Run by the UI checker on commit 94075ca5b, 2026-10-16, light theme only (the dark theme was not run). Screenshots are in `/tmp/gpui-ui-check/J/`: `gpui-colors-light.png`, `ref-colors-light.png`, `gpui-flood-after.png`, `gpui-flood-relaunch.png` (the files are JPEG data with a `.png` name).

### Agent check: parity list (GPUI example vs reference app)

Reference: the in-place Electron build on 6770. Its daemon had no `/tmp/gpui-bench` workspace, so the checker ran `paseo --host localhost:6770 run --provider mock ...`; that failed with "Provider mock is not configured" but still created the workspace. The terminal was opened in the reference app on that workspace. The `~/.paseo-dev` home keeps the `gpui-bench` project (terminal killed, app quit).

| Element                           | GPUI                                                    | Reference                     | Result                                                                                                                           |
| --------------------------------- | ------------------------------------------------------- | ----------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Normal red (`\e[31m`)             | `#943e49` sampled (anti-aliased)                        | `#9c3b42` sampled             | match (token red `#dc2626`)                                                                                                      |
| Normal green (`\e[32m`)           | `#3a8a57` sampled                                       | `#44875a` sampled             | match (token green `#16a34a`)                                                                                                    |
| Foreground and cursor             | `#161618`, cursor `#151515`                             | `#1b1b1b`, cursor `#191919`   | match                                                                                                                            |
| Background                        | `#ffffff`                                               | `#ffffff`                     | match                                                                                                                            |
| Font                              | JetBrains Mono look, same glyph shapes                  | JetBrains Mono                | match                                                                                                                            |
| Line height                       | 16 px rows                                              | 16 px rows                    | match                                                                                                                            |
| Char advance                      | about 7.15 px                                           | about 7.0 px                  | match within screenshot scale                                                                                                    |
| Bold cyan (`ls` dirs, prompt dir) | `#2a808d` sampled (cyan `#0891b2`, no bold brightening) | `#29a4a9` sampled, lighter    | MISMATCH: the reference draws bold text in the bright color (xterm.js `drawBoldTextInBrightColors`); GPUI keeps the normal color |
| Bold green prompt arrow           | `#1a9d4d` sampled                                       | `#3bb272` sampled, lighter    | MISMATCH, same cause                                                                                                             |
| Left padding                      | 4 px                                                    | 0 px                          | MISMATCH (the reference capture was clipped to the left 350 px, so the pane edge is not certain)                                 |
| Magenta `ls` entries              | `#7d5997`                                               | not visible (capture clipped) | NOT COMPARED                                                                                                                     |

Notes:

- The reference window repainted only its left 350 px in every capture (the compositor gap in [docs/browser-capture-harness.md](docs/browser-capture-harness.md)); the width limits the `ls` column comparison.
- Sampled colors come from JPEG screenshots of thin glyph strokes, so only the bold-versus-normal difference is clear. Token values are in `packages/app/src/styles/theme.ts` lines 245-257.
- Fix suggestion (not done, no source edit): brighten bold ANSI 0-7 to 8-15 in `crates/terminal/src/emulator.rs`.

### Anti-goal

- `time yes | head -n 200000` in the pane: the shell reports 0.191 to 0.226 s total on four runs; the pane showed the final state and the prompt within 4 s of the keystroke. Limit: 5 s. PASS.
- `measure.sh 60 term="pids:<pid>"`: first run 0.8 CPU%, 115 MB RSS, 73 MB footprint, but that window drew only its left 328 px (Q-069), so the read is not valid. After quitting and relaunching the bundle, the window drew full width and the second run gave 0.8 CPU%, 86 MB RSS, 72 MB footprint. Limit: 10.0. PASS.

### Cleanup

Both terminals on live were killed (0 left in `/tmp/gpui-bench`); the bundle and the reference app were quit; no release (6767) call; the `gpui-age` agent was not touched; the screen lock was released.

## Fix Round 1

Status: DONE (screen recheck deferred to the UI checker).

Changed:

- `crates/terminal/src/emulator.rs`: bold cells draw ANSI 0-7 (named or `38;5;0-7`) as 8-15, like xterm.js `drawBoldTextInBrightColors`. The Electron runtime (`packages/app/src/terminal/runtime/terminal-emulator-runtime.ts` line 441) does not set the option, and its default is true. As in xterm.js, the brightened color is the one drawn as text, so with inverse the cell background value brightens. New test `bold_text_draws_ansi_0_to_7_in_the_bright_color`.
- `crates/terminal/src/size.rs`: `PADDING` is 0. The Electron chain has no padding: `HOST_DIV_STYLE` sets 0 on every side (`terminal-emulator.tsx` lines 69-72), and `outputContainer` and `terminalGestureContainer` in `terminal-pane.tsx` set none. The 16 px in `terminal-panel.tsx` is only for the centered empty and error states. The two size tests now assert the no-padding values.

Verified:

- GitNexus impact: `build_line` HIGH (5 upstream, the view paint path; the change only alters bold cell colors). `PADDING` UNKNOWN; text search finds uses only in the terminal crate.
- `cargo.sh test -p bachuc-terminal` -> 32 pass (31 earlier + 1 new).
- `cargo.sh fmt --check -p bachuc-terminal` -> exit 0. `cargo.sh build -p bachuc-terminal --examples` -> no warning in the terminal crate.
- Bold cyan and green brightness and left padding vs the reference app: DEFERRED (UI checker).

### Re-check 1

Run by the UI checker on commit a59d730c6 (bundle `BachucTerminal` built 2026-10-16 10:59), 2026-10-16, light theme. Screenshots: `/tmp/gpui-ui-check/J/gpui-colors-recheck.jpg`, `/tmp/gpui-ui-check/J/ref-colors-recheck.jpg`. Both ran `printf '\e[1;36mbold-cyan \e[1;32mbold-green \e[31mred\e[0m'` and `ls --color /` in `/tmp/gpui-bench`. The reference app was relaunched fresh (the restored one had no `git` on its PATH and its shells exited at once) and its full window painted, so the capture is not clipped.

| Element                   | GPUI (sampled glyph core)      | Reference (sampled glyph core) | Result                                          |
| ------------------------- | ------------------------------ | ------------------------------ | ----------------------------------------------- |
| Bold cyan                 | `#3ba2b4` (darkest `#19a9bc`)  | `#449dae` (darkest `#3f97aa`)  | match; both lighter than normal cyan `#0891b2`  |
| Bold green                | `#4fab71`                      | `#55a874`                      | match; both lighter than normal green `#16a34a` |
| Red (`\e[31m`)            | `#c2595a`                      | `#bb5d60`                      | match                                           |
| Prompt arrow (bold green) | `#47b16d`                      | `#53b17a`                      | match                                           |
| Magenta `ls` entries      | purple, same hue               | purple, same hue               | match                                           |
| Left padding              | text starts at pane edge, 0 px | text starts at pane edge, 0 px | match                                           |

Sampled values come from JPEG glyph strokes, so both apps differ by a few units. The earlier mismatches (bold brightening, 4 px padding) are fixed. PASS.

Setup notes: `/tmp/gpui-bench` did not exist after the crash; the checker recreated it as an empty folder. The reference daemon keeps one dead `Terminal 1` record that `terminal kill` reports as killed but still lists; it has no process.
