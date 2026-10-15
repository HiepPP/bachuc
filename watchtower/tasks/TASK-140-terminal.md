# TASK-140 Terminal with resize, scrollback, and selection

Group: F (writes only `crates/terminal`)
Class: code

## Brief

Goal: Add the crate `bachuc-terminal` with a terminal view on `alacritty_terminal` that fits its pane, restores history, keeps scrollback, and supports selection, copy, and paste.

Change: the prototype's fixed 60x24 grid with no scrollback -> a daily-use terminal pane.

Boundaries:

- Measure the cell size from the monospace font, and compute rows and columns from the pane bounds. On the first size, call `terminal_resize` with `claim: true`; on later size changes, call it with `claim: false`, and resize the emulator too.
- The first frame after create or attach is `TerminalFrame::Snapshot`, a JSON cell grid (`TerminalStateSchema` in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts)). Planner call: write the grid into the emulator by turning each row into text with SGR color codes, then feed it, before any `Output` frame.
- Keep 10000 lines of scrollback (the alacritty default). The mouse wheel scrolls it. New output returns to the bottom only when the view is at the bottom.
- Mouse drag selects text; Cmd+C copies; Cmd+V pastes with bracketed paste when the app asks for it.
- Keys: printable keys, Enter, Backspace, Tab, Escape, arrows, Home, End, Page Up and Down, and Ctrl+letter. Option sends Escape plus the key.
- Lead note from TASK-136 (Q-059): the client subscribes with no restore mode, so the first frame is the `0x04` JSON grid as `TerminalFrame::Snapshot`. A `Snapshot` also arrives after each reconnect; reset the emulator on every one. A `create_terminal` folder must be a daemon workspace: `/tmp` fails with "workspaceId is required", `/tmp/gpui-bench` works.
- Draw cells with one text run per color span per line, as the prototype does, and cache unchanged lines.
- Give the pane a role and a label with the terminal name.

How:

- Move the prototype terminal code in as a start, then add sizing, snapshot restore, scrollback, selection, and the key map.
- Add `examples/terminal.rs`. It opens a new terminal in a given folder, or attaches to a terminal id given with `--attach`.

Files:

- [packages/desktop-gpui/crates/terminal/Cargo.toml](packages/desktop-gpui/crates/terminal/Cargo.toml) (new crate `bachuc-terminal`)
- [packages/desktop-gpui/crates/terminal/src](packages/desktop-gpui/crates/terminal/src) (emulator view, snapshot restore, input, selection)
- [packages/desktop-gpui/crates/terminal/examples/terminal.rs](packages/desktop-gpui/crates/terminal/examples/terminal.rs) (example)

Interfaces:

- Consumes: `bachuc_client::Client` (a GPUI entity) with `Client::connect(host: HostConfig, cx: &mut gpui_kit::App) -> gpui_kit::Entity<Client>` from TASK-136
- Consumes: `Client::create_terminal(&mut self, cwd: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<TerminalHandle>>`, `Client::attach_terminal(&mut self, terminal_id: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<TerminalHandle>>`, `TerminalHandle { id: String, frames: futures::channel::mpsc::UnboundedReceiver<TerminalFrame> }`, `TerminalFrame` (enum `Output(Vec<u8>)`, `Snapshot(serde_json::Value)`), `Client::terminal_input(&mut self, terminal_id: &str, bytes: &[u8])`, `Client::terminal_resize(&mut self, terminal_id: &str, rows: u16, cols: u16, claim: bool)`, and `Client::kill_terminal(&mut self, terminal_id: &str, cx: &mut gpui_kit::Context<Client>)` from TASK-136
- Produces: `bachuc_terminal::TerminalView` (a GPUI view) with `TerminalView::new(client: gpui_kit::Entity<bachuc_client::Client>, cwd: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<TerminalView>) -> TerminalView` and `TerminalView::close(&mut self, cx: &mut gpui_kit::Context<TerminalView>)` for TASK-141

Expected result:

- Resizing the window changes the column count; `tput cols` in the terminal prints the new width.
- `seq 1 20000` runs, and the wheel scrolls back through the kept 10000 lines.
- `vim` and `htop` draw correctly and take arrow keys.
- Selecting text and pressing Cmd+C copies it. Cmd+V pastes.
- Attaching to an existing terminal shows its earlier output from the snapshot.

Anti-goal: `time yes | head -n 200000` finishes in at most 5 seconds in the pane, and the example then stays within 10% CPU and 250 MB phys_footprint for the next 60 seconds; tripwire; read after sizing works and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-terminal` -> key map, size math, snapshot-to-text, and selection tests pass
- Agent check: `packages/desktop-gpui/bundle.sh example terminal BachucTerminal io.github.hieppp.bachuc-terminal`, open it in `/tmp`, type `tput cols` with `cua_repl`, resize the window, type it again -> the two numbers differ and match the pane width
- Agent check: run `while true; do date; sleep 0.05; done`, resize the window with `cua_repl` during the output, stop it with Ctrl+C, then run `tput cols` -> the value matches the new pane width and the screen has no torn lines
- Agent check: run `seq 1 20000`, scroll up with `cua_repl` -> earlier numbers show; take a screenshot
- Agent check: run `vim /tmp/gpui-term-check.txt`, type text, save with `:wq`, then `cat` the file -> the text is there
- Agent check: type `echo restore-check`, quit the example, open it again with `--attach <terminal id>` -> `restore-check` shows from the snapshot
- Agent check: `cua_repl` `getAXState` -> the terminal pane appears with its label
- Anti-goal: type `time yes | head -n 200000` in the pane -> real time at most 5 seconds; then `measure.sh 60 term="BachucTerminal.app"` -> CPU at most 10.0 and footprint at most 250
