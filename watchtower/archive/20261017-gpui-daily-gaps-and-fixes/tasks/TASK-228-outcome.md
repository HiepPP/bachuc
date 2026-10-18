# TASK-228 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/command/src/center.rs`: `FileResult`, `set_file_rows`, `set_file_error`, the full-list file search (`searches_files`), the Files section in the full list (`with_files`, `file_of`, `layout_of`), selection that survives a file answer (`Selected`), per-file icons, the error line, and 4 new tests.
- `packages/desktop-gpui/crates/command/src/lib.rs`: exports `FileResult`; the crate doc names the full-list file search.

## Contract

- Produces: `FileResult { path: String, icon: Arc<Image> }` (Clone, Debug, PartialEq, Eq), exported from `bachuc_command`.
- Produces: `CommandCenter::set_file_rows(&mut self, rows: Vec<FileResult>, cx: &mut Context<CommandCenter>)`. It shows the rows with their icons and clears the error line.
- Produces: `CommandCenter::set_file_error(&mut self, message: Option<String>, cx: &mut Context<CommandCenter>)`. Some clears the file rows and shows "Error: <message>". None hides the line only.
- Produces: `CommandFileEvent::FileQuery { query }` now also fires in the full list for a non-empty query, with the same 100 ms pause and drop-the-older rule. An empty full-list query sends nothing and clears the files, the error, and the pending query.
- `set_file_results(paths, cx)` keeps its signature and draws the generic `File` icon. The app (TASK-231) moves to `set_file_rows`.
- Full list: the Files section comes after the plugin and app sections, with a divider when other sections show. File `n` uses the layout item index `items.len() + n`, so arrow keys walk the items, then the files. The Files scope keeps index `n`.
- A file answer keeps the highlighted row (by item id or file path) when it is still in the list, as Electron `preserveActiveResultId`. A query change still resets to the first row.
- An answer that arrives after the full-list query was cleared is dropped, because the app sends no `FileQuery` for an empty query and cannot drop it by generation.
- The file search status slot shows in both scopes, as `FileSearchLoadingIndicator`. "No matches" is hidden while a file search runs or after it failed, as Electron `empty`.
- Leaving the Files scope with the chip keeps the file answer when the query is not empty; the query is the same.
- Safe default (spec vs Electron): the spec says "one muted error line in the Files section". Electron draws `errorText` between the header and the list, in `statusDanger` with a bottom border (`command-center.tsx` lines 673-684, 981-988). This TASK follows Electron: `status_danger`, SPACE_4 by SPACE_3 padding, a bottom border, line height 20 px, text "Error: <message>", AX role Alert.
- Fix in passing: `refresh_plugin_rows` read the selected id after `rebuild_items`, through an old layout into new items. It now reads the selection first.
- Impact: GitNexus rates the `CommandCenter` struct LOW (1 direct caller, `shell.rs`); the index is 64 commits stale and lacks the TASK-217 symbols. `rg`: `set_file_results` has 2 callers and `open_files_scope` 1, all in `crates/app/src/shell.rs`; their signatures did not change.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-command` -> exit 0: 43 lib (incl. `full_list_puts_the_files_section_after_the_other_sections`, `full_list_files_alone_have_a_titled_section_without_a_divider`, `files_search_in_the_files_scope_or_for_a_typed_query`, `a_file_answer_keeps_the_highlighted_row`), 5 search (the timing tests passed), 9 shortcuts.
- `./cargo.sh fmt --check -p bachuc-command` -> exit 0.
- `./cargo.sh clippy -p bachuc-command --all-targets` -> no warning in changed code; one existed before, in `search.rs:261`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> exit 0; one warning existed before, in `crates/app/src/state.rs:424`.
- UI check on live: Cmd+K, "main" -> a Files section: DEFERRED (UI checker), after TASK-231 wires `set_file_rows` and `set_file_error`. The current app already answers full-list queries through `set_file_results`, so the section shows with generic icons before TASK-231.
- Anti-goal: Cmd+K to the first row under 50 ms: DEFERRED (UI checker). `new` adds one field init; an empty query sends no file search.

## Lessons

- TASK-231: until it runs, the app answers every full-list keystroke through `query_files`, and its error branch shows a toast per failed search. Move the error to `set_file_error` and drop the toast.
- TASK-231: answer with `set_file_rows`, with icons from `bachuc_explorer::file_icon` on the file name (the last path segment).
- A view crate keeps its layout rules in free functions (`layout_of`, `with_files`, `file_of`) so the full-list cases test without a GPUI context.

## UI Checks

Run 2026-10-18 by the UI checker (run 5). Bundle `Bachuc GPUI Check` built from HEAD `08064a6f3` (includes `626143dc7` and TASK-225 `699395706`), PID 34022, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). Frames were forced with the AX "zoom the window" action; keys went to the check app only. Scratch repo `/tmp/t202-repo` (branches `feature-x` and `main`, new commit `1f8a4a9` with a `web` service script) was the live workspace `wks_9cd2d5d43fcc7b08` with the disposable mock agent "r5 fork src" (`d8ef867`). Request counts come from the live `daemon.log` `ws_runtime_metrics` lines (30 s windows, all clients), parsed by `/tmp/r5-metrics.py`. After the run, PID 34022 exited, the agent and the workspace were archived, and the scratch terminal was killed.

- Cmd+K, "main" -> a Files section with file icons below the other sections: PASS. The command center listed "Workspaces" (4 rows), a divider, then "Files" with "main.rs src" and a Rust file icon. Evidence: `/tmp/r5-228-01-cmdk-main.png`.
- Note: Cmd+K did nothing until the composer had focus; after a branch switcher pick, the window had no focused element.
