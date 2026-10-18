# TASK-215 Terminal find and links

Group: R (standalone: [packages/desktop-gpui/crates/terminal](packages/desktop-gpui/crates/terminal))
Class: code

## Brief

Goal: The terminal has a find bar, and Cmd-click opens URLs and file paths, as Electron does.

Change: the GPUI terminal only selects, copies, and pastes ([packages/desktop-gpui/crates/terminal/src/view.rs](packages/desktop-gpui/crates/terminal/src/view.rs)) -> find and clickable links.

Boundaries:

- Find: Electron `TerminalFind` ([packages/app/src/components/terminal-pane.tsx](packages/app/src/components/terminal-pane.tsx) line 1071). Cmd+F opens a bar with next, previous, and a match count; Escape closes it. Search the visible grid and the scrollback the view holds.
- URLs: `http` and `https` links open in the default browser with `cx.open_url`.
- File links: Electron resolves a path with `classifyForResolution`, then `directory_suggestions` when unsure (`terminal-pane.tsx` lines 883-908). A click opens a file tab at the line when the text has `:line` (lines 909-918).
- Owner answer Q-112: open a file link only when the path exists exactly, or `directory_suggestions` returns a single match. With 0 or 2 or more matches, do nothing.
- The view emits events; the app (TASK-203) opens the file tab.
- Hover underline only while Cmd is held.

How:

- Add `find.rs` with the search over cells as a pure function, with tests.
- Add `links.rs` with URL and `path:line` detection on a row as a pure function, with tests.
- Wire Cmd+F in `key_down` (line 335), the bar in `render` (line 472), and Cmd-click in `mouse_down` (line 407).

Files:

- [packages/desktop-gpui/crates/terminal/src/find.rs](packages/desktop-gpui/crates/terminal/src/find.rs) (new)
- [packages/desktop-gpui/crates/terminal/src/links.rs](packages/desktop-gpui/crates/terminal/src/links.rs) (new)
- [packages/desktop-gpui/crates/terminal/src/view.rs](packages/desktop-gpui/crates/terminal/src/view.rs) (bar, keys, clicks, events)
- [packages/desktop-gpui/crates/terminal/src/lib.rs](packages/desktop-gpui/crates/terminal/src/lib.rs) (module lines)

Interfaces:

- Consumes: `DirectorySuggestion { path: String, is_directory: bool }` and `Client::directory_suggestions(&mut self, cwd: &str, query: &str, include_files: bool, include_directories: bool, limit: u32, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<DirectorySuggestion>>>` from TASK-206.
- Produces: `TerminalEvent::OpenFile { cwd: String, path: String, line: Option<u32> }` for TASK-203.

Expected result:

- Cmd+F, type a word -> matches highlight and the count shows; Enter goes to the next match.
- Cmd-click `https://example.com` -> the browser opens it.
- Cmd-click `src/main.rs:42` that exists -> `TerminalEvent::OpenFile` with line 42.
- Cmd-click a path that does not exist, or one that matches 2 or more files -> nothing happens.

Anti-goal: terminal output throughput must not drop; tripwire; the 200,000-line flood check from [packages/desktop-gpui/docs/parity-audit.md](packages/desktop-gpui/docs/parity-audit.md) line 174 (`time seq 1 200000` in a GPUI terminal tab, until the last line shows) stays within 5% of the reading before the change, read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-terminal` -> passes, including find and link tests, and a resolve rule test for 0, 1, and 2 matches.
- UI check on live: in a terminal tab, run `echo hello https://example.com src/main.rs:1`, then Cmd+F `hello` -> one match highlighted.
- Owner hand test: Cmd-click a URL and a file path in a terminal -> the browser opens and the file tab opens at the line.
- Anti-goal: the 200,000-line flood timing -> within 5% at both reads.
