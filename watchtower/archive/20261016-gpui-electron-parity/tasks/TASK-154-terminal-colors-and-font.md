# TASK-154 Terminal colors and font from the tokens

Group: J (writes `crates/terminal`)
Class: mechanical

## Brief

Goal: Make the GPUI terminal use the Electron terminal colors, font, and size, and follow the theme.

Change: a fixed dark palette with background `0x0f1115` (`crates/terminal/src/emulator.rs` line 24) and Menlo 13 (`crates/terminal/src/view.rs` lines 14-15) -> the `theme.ts` terminal colors for `light` and `claude` dark, the Electron terminal font family, and the code font size.

Design: [packages/app/src/components/terminal-pane.tsx](packages/app/src/components/terminal-pane.tsx) (line 1046 passes `codeFontSize`), [packages/app/src/terminal/runtime/terminal-font.ts](packages/app/src/terminal/runtime/terminal-font.ts) (font family list at lines 3-21), [packages/app/src/styles/theme.ts](packages/app/src/styles/theme.ts) (terminal colors; see TASK-145 Boundaries for the line ranges)

Boundaries:

- Keep every public item of `bachuc-terminal` and its behavior. Change only colors, font, and size.
- Read colors from `bachuc_theme::tokens(cx)`: `ansi`, `terminal_background`, `terminal_foreground`, `terminal_cursor`, and `terminal_selection`; redraw on a theme change.
- Font size: the code font size, default 12 ([packages/app/src/hooks/use-settings/storage.ts](packages/app/src/hooks/use-settings/storage.ts) line 62), scaled by the code size of `set_font_scale`. Font family: the `terminal-font.ts` list in order, using the first one installed (JetBrains Mono first). Recompute the cell size and the PTY size after a change.

How:

- Replace the fixed palette and font with tokens.
- Subscribe to theme changes.

Files:

- [packages/desktop-gpui/crates/terminal](packages/desktop-gpui/crates/terminal) (`Cargo.toml`, `src`)

Interfaces:

- Consumes: `bachuc_theme::Tokens` (a `Copy` struct of `gpui_kit::Hsla` fields named after the `theme.ts` tokens in snake case: `surface0` to `surface4`, `surface_sidebar`, `surface_sidebar_hover`, `surface_sidebar_selected`, `surface_workspace`, `surface_diff_empty`, `interaction_highlight`, `foreground`, `foreground_muted`, `foreground_extra_muted`, `border`, `border_accent`, `accent`, `accent_bright`, `accent_foreground`, `destructive`, `destructive_foreground`, `ring`, `popover`, `status_success`, `status_danger`, `status_warning`, `status_merged`, `tint_success`, `tint_danger`, `tint_warning`, `tint_merged`, `dot_success`, `dot_danger`, `dot_warning`, `dot_running`, `diff_addition`, `diff_deletion`, `terminal_background`, `terminal_foreground`, `terminal_cursor`, `terminal_selection`, and `ansi: [gpui_kit::Hsla; 16]`) from TASK-145
- Consumes: `bachuc_theme::tokens(cx: &gpui_kit::App) -> Tokens` from TASK-145
- Produces: the same `bachuc_terminal::TerminalView` API, unchanged, for TASK-155

Expected result:

- `ls --color` and `vim` colors match the reference app's terminal in both themes.
- A theme switch recolors an open terminal without a restart.

Anti-goal: `time yes | head -n 200000` stays at or below 5 seconds in the pane, and the example stays at or below 10% CPU for the next 60 seconds; tripwire; read after the change.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-terminal` -> all 29 existing tests pass, plus a palette test that maps each ANSI index to its token
- Agent check: bundle `packages/desktop-gpui/bundle.sh example terminal BachucTerminal io.github.hieppp.bachuc-terminal`; run `ls --color /` and `printf '\e[31mred\e[32mgreen\e[0m'` in `/tmp/gpui-bench` in it and in the reference app on 6770; take `cua_repl` screenshots -> the parity list in the outcome shows the colors, font, and line height match or records each mismatch
- Anti-goal: `time yes | head -n 200000` in the pane -> at most 5 seconds; then `measure.sh 60 term="pids:<pid>"` -> CPU at most 10.0
