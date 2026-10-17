# TASK-140 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/terminal/Cargo.toml`: new crate `bachuc-terminal` (deps: `bachuc-client`, `gpui-kit =0.7.3`, `alacritty_terminal 0.26`, `futures`, `log`, `serde`, `serde_json`).
- `packages/desktop-gpui/crates/terminal/src/lib.rs`: module list and re-exports.
- `packages/desktop-gpui/crates/terminal/src/view.rs`: `TerminalView` (frames, sizing, keys, mouse, wheel, clipboard, pane role and label).
- `packages/desktop-gpui/crates/terminal/src/emulator.rs`: `Emulator` (alacritty term, 10000-line scrollback, selection, damage-based line cache, snapshot apply).
- `packages/desktop-gpui/crates/terminal/src/snapshot.rs`: JSON cell grid to SGR text (port of `renderTerminalSnapshotToAnsi`).
- `packages/desktop-gpui/crates/terminal/src/keys.rs`: key map and paste bytes.
- `packages/desktop-gpui/crates/terminal/src/size.rs`: pane size to rows and columns, position to cell.
- `packages/desktop-gpui/crates/terminal/examples/terminal.rs`: the example.
- `packages/desktop-gpui/Cargo.lock`: updated by cargo (shared; the lead commits it).

## Contract

- `bachuc_terminal::TerminalView::new(client: Entity<Client>, cwd: String, window: &mut Window, cx: &mut Context<TerminalView>) -> TerminalView` and `TerminalView::close(&mut self, cx: &mut Context<TerminalView>)` (kills the daemon terminal and stops reading), as the spec says.
- Extra public items: `TerminalView::attach(client, terminal_id: String, window, cx)`, `terminal_id() -> Option<&str>`, `set_name(name, cx)`, `TerminalEvent { Opened { id }, Exited }` emitted by the view, and the pure helpers `Emulator`, `Snapshot`, `key_bytes`, `paste_bytes`, `grid_size`, `cell_at`.
- The view is a GPUI `Render` that fills its parent (`size_full`). Give it a parent with a size. The pane is a `div` with role `Terminal` and the label `Terminal <name>`; the name is the folder name for `new`, the first 8 characters of the id for `attach`, or what `set_name` sets.
- First size: `terminal_resize(.., claim: true)`; later sizes: `claim: false`. Every `Snapshot` frame resets the emulator to the snapshot's size, replays history and screen, then resizes it to the pane and claims again.
- `TerminalView::new` needs a connected client. A new terminal also needs the client's workspace list (it arrives right after `Connected`, as `AgentsChanged`), or `create_terminal` fails with "workspaceId is required"; the pane then prints the error. The example waits for both.

## Verified

- `packages/desktop-gpui/cargo.sh test -p bachuc-terminal` -> 29 tests pass: key map (printable, editing, navigation in both cursor modes, Ctrl+letter, Option, Cmd sends nothing), paste bytes, size math, snapshot to text (SGR colors, soft wrap, no wrap flags), scrollback 10000, view stays when scrolled, snapshot reset, selection by cell, word, and line.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-terminal` -> exit 0.
- `packages/desktop-gpui/cargo.sh clippy -p bachuc-terminal --all-targets` -> no warnings in `bachuc-terminal` (the 2 warnings are in `crates/protocol/build.rs`, not owned here).
- `packages/desktop-gpui/bundle.sh example terminal BachucTerminal io.github.hieppp.bachuc-terminal`, opened with `/tmp/gpui-bench` (see Questions) -> `tput cols` printed 113 at 900 px wide and 326 after the zoom button made the window 2560 px wide; both match (width - 8 px padding) / 7.83 px cell.
- `while true; do date; sleep 0.05; done` with the window zoomed back during the output, then Ctrl+C and `tput cols` -> 113 (the new width); the screenshot during the output showed whole lines and no tears.
- `seq 1 20000`, then a `cua_repl` scroll up 5 pages -> the screen moved from 19968-20000 to 19783-19816. The 10000-line limit is covered by the unit test (`scrollback_keeps_10000_lines`).
- `vim /tmp/gpui-term-check.txt`: typed `hello vim`, two Left arrows, `X`, `Esc`, `:wq`; `cat` -> `hello vXim`. Arrows reach vim. `htop` is not installed on this machine, so `top` replaced it: its full screen drew correctly with colors and inverse headers (UNSATISFIABLE for htop here; replacement check done).
- Drag over `select-me-123`, Cmd+C, then `echo [` Cmd+V `]` -> `echo [select-me-123]` (selection highlight visible; zsh bracketed paste accepted it).
- `echo restore-check`, app killed, relaunched with `--attach <full id>` -> `restore-check` shows. After `seq 1 200`, kill and attach again, scrolling up shows 94-127, so the scrollback comes through the snapshot too.
- `cua_repl` `getAXState` -> `text entry area Terminal gpui-bench` (and `Terminal a4409e34` when attached) in the window tree.
- Cleanup: the terminal was killed (`terminal kill`, `terminal ls --all` empty), every app bundle was quit, and the screen lock was released.
- `npm run format:check:files -- watchtower/tasks/TASK-140-outcome.md` -> exit 0. No JS or TS changed, so no lint run.

## Anti-goal

Gauge: `time yes | head -n 200000` in the pane at most 5 s; then `measure.sh 60 term="BachucTerminal.app"` at most 10.0% CPU and 250 MB phys_footprint. Live daemon on 6768 was running. Window raised before each read.

- Checkpoint 1, after sizing worked (113 and 326 columns verified), 2026-10-15T11:09:05Z to 11:10:05Z: the shell reported `yes` 0.201 s and `head` 0.200 s total; the pane was fully drawn (last line, the prompt) 4 s after Enter. Source: pane screenshot. `measure.sh 60 term=BachucTerminal.app` -> CPU 0.4%, RSS 182 MB, footprint 67 MB. PASS.
- Checkpoint 2, at completion (same binary, attached terminal), 2026-10-15T11:11:44Z to 11:12:45Z: `time` reported 0.221 s total; the pane was fully drawn within 4 s. `measure.sh 60` -> CPU 0.6%, RSS 111 MB, footprint 74 MB. PASS.
- Result: PASS. No tripwire crossed. No re-read was needed.

## Lessons

- The daemon accepts only the full terminal id on subscribe; the 8-character id that `terminal ls` prints gives "Terminal not found". The example prints the full id as `terminal id: <id>`.
- `create_terminal` needs a daemon workspace folder. `/tmp` fails; `/tmp/gpui-bench` works (the spec's "open it in /tmp" checks used it).
- `TerminalView` must be created after the client's `Connected` event, and a new terminal also after the workspace list is loaded. Wait for `ClientEvent::Connected` / `AgentsChanged` like `examples/terminal.rs` does.
- Window resize through `cua_repl`: dragging an edge or corner did nothing. The zoom button (`performSecondaryAction(<zoom button index>, 'zoom the window')`) toggles between the size and the screen size and gave two clean sizes.
- A window that is not raised can keep showing the first blank frame: an AX state and screenshot taken before `Raise` showed an empty pane even though the terminal had opened. Raise first.
- `clear` in zsh on macOS also clears the daemon's scrollback; check scrollback restore with `seq`, not after `clear`.
- GPUI `on_mouse_move` only fires while hovered, so the drag and release listeners are registered with `window.on_mouse_event` in a canvas paint closure; the same canvas reports the pane bounds.
- The view does not report mouse events to apps (vim `mouse=a`, htop clicks); the wheel in the alternate screen sends arrow keys instead. Add mouse reporting if a later TASK needs it.
- `bachuc-composer`'s `test-support` dev-feature briefly broke workspace resolution (`cocoa` conflict) while builders shared the checkout; it was fixed by its owner within minutes.

## Questions

- None blocking. The spec says "open it in `/tmp`"; `/tmp` cannot work for `create_terminal`, so `/tmp/gpui-bench` was used.
