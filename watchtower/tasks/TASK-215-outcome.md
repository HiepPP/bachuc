# TASK-215 Outcome

## Status

DONE. The screen checks and the flood timing are deferred to the UI checker.

## Changed

- `packages/desktop-gpui/crates/terminal/src/find.rs` (new): a search without case over the buffer lines, `buffer_lines` (history plus screen, view kept), the step, reveal, and status rules, and `paint` to lay highlight spans over a row's runs. 10 tests.
- `packages/desktop-gpui/crates/terminal/src/links.rs` (new): URL and `path:line` detection on a row, cell-to-byte mapping across wide characters, `lookup` (where to ask the daemon), and `resolve` (the Q-112 rule). 8 tests.
- `packages/desktop-gpui/crates/terminal/src/view.rs`: `TerminalEvent::OpenFile`, the find bar, Cmd+F, Cmd-click, and the Cmd-hover underline.
- `packages/desktop-gpui/crates/terminal/src/lib.rs`: the two module lines.
- `packages/desktop-gpui/crates/terminal/examples/terminal.rs`: one `OpenFile` match arm. This file is not in the spec's Files, but its exhaustive `match` on `TerminalEvent` broke the crate test build after the new variant was added.

## Contract

- Produces `TerminalEvent::OpenFile { cwd: String, path: String, line: Option<u32> }`. `cwd` is the folder the terminal opened in. `path` is relative to `cwd`, as the daemon returns it, or absolute for a file outside `cwd`. A line of 0 gives `None`.
- Consumes `Client::directory_suggestions(cwd, query, true, false, 100, cx)` and `DirectorySuggestion` from TASK-206, as specified.
- Q-112 rule (`links::resolve`): open the file whose path equals the query. If no path is equal, open the only file whose path ends with `/query`. If there are 0 such files, 2 or more, or the lookup fails, open nothing.
- A path under `cwd` is searched in `cwd` as a relative path. Any other absolute path is searched by name in its parent folder. A `~` path, or a relative path in an attached terminal (no `cwd`), has no lookup and opens nothing.
- `http` and `https` links open with `cx.open_url`. A Cmd-click on a link does not start a selection. A Cmd-click elsewhere selects as before.
- Find: Cmd+F opens or refocuses the bar. Typing searches the history and the screen without case, then shows the first match from the screen top down. Enter goes to the next match and Shift+Enter to the previous one. Escape in the field or the close button closes the bar and focuses the terminal. Matches are yellow at 35%, the current match is blue at 60%, and the search stops at 20,000 matches with a "+" in the count. The status text is "No matches", "3 of 12", or "12 matches".
- Output that arrives while the bar is open sets a `stale` flag only. The next step searches again, so output frames never search. The marks are found again in each drawn row, so they follow new output.
- The bar is a sibling of the terminal element, so keys typed in its field never reach the PTY. The bar uses `occlude()`.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-terminal` -> passed, 50 tests (32 before plus 18 new). The new tests include find, links, and `a_link_opens_only_an_exact_or_single_match` for 0, 1, and 2 matches.
- `./cargo.sh fmt --check -p bachuc-terminal` -> exit 0.
- `./cargo.sh clippy -p bachuc-terminal --tests --examples` -> no warnings in the terminal crate. The 2 build-script warnings come from `bachuc-protocol`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- `./cargo.sh build -p bachuc-app` -> exit 0. Its one warning (`next_theme` unused, `crates/app/src/state.rs:424`) was there before this TASK.
- UI check: `echo hello https://example.com src/main.rs:1`, then Cmd+F `hello` -> DEFERRED (UI checker).
- Owner hand test: Cmd-click a URL and a file path -> DEFERRED (owner). The file tab opens only after TASK-203 wires `OpenFile`.
- Anti-goal: the 200,000-line flood timing -> DEFERRED (UI checker). The baseline was not read, because builders open no app window. Lead: take the before reading from a bundle built at `59a040442`. By reading the code: with the bar closed, a feed adds one `Option` check, and a draw adds one empty overlay check per row.

## Lessons

- `TerminalView::attach` has no `cwd`, so an attached terminal opens only absolute file links. The app opens terminals with `TerminalView::new` only.
- TASK-206 sends no `matchMode`, so the daemon matches fuzzily. `resolve` applies the suffix match on the client side. Up to 100 results are read, so a second suffix match past result 100 would not be seen.
- `Emulator` reads only screen rows. `find::buffer_lines` scrolls through the history to read it, then restores the offset. That marks the screen fully damaged once per search.
- Link detection reads one row. A path that wraps onto the next row is cut, unlike Electron, which joins wrapped rows.
- The terminal crate does not depend on `bachuc-icons`. The bar draws the Lucide SVGs by asset path (`lucide/arrow-up.svg`), which the app's asset source serves.
- Electron keeps the last query when the bar closes and searches it again on reopen. This bar starts empty.

## UI Checks Run 4

Run 2026-10-18 by the UI checker (run 4). Bundle `Bachuc GPUI Check` built from HEAD `60fd3a593`, PID 88032, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The checker forced each frame with the AX "zoom the window" action and sent keys with cua `pressKey`/`typeText` to the check app only. Scratch repo `/tmp/t202-repo` with disposable agents "r4 board card", "r4 remove card", "r4 fork src" (mock ten-second-stream), "r4 fork ok" (Haiku 5.5, one-word reply), and the fork child "Continue." (`af3218a`). After the run, PID 88032 exited, all five agents and the five t202-repo workspaces were archived, and the repo stayed as before (`feature-x`, ` M a.txt`).

- Run `echo hello https://example.com a.txt:1`, Cmd+F, type hello -> match count and highlight: PASS. Cmd+Shift+T opened "Terminal t202-repo"; the line ran. Cmd+F opened the Find bar (`Find in pane`, Previous, Next, Close). "hello" gave "1 of 2": the command line and the output both contain hello, so 2 is right. The current match is drawn blue and the other tan. Evidence: `/tmp/r4-215-04-find-hello.png`.
- Escape closes the Find bar: PASS. The AX tree has no `Find` container after Escape. Evidence: `/tmp/r4-215-05-find-closed.png`.
- Cmd-click a URL and a file path: UNVERIFIED (unattended): needs pointer.
- Anti-goal, 200,000-line flood timing: UNVERIFIED. Not run; it needs the baseline bundle at `59a040442`.
- Observation: the first terminal row (the prompt line) draws half under the tab bar edge in the same shot.
