# TASK-228 Command center Files match Electron

Group: N (standalone: [packages/desktop-gpui/crates/command](packages/desktop-gpui/crates/command))
Class: code

## Brief

Goal: The command center shows a Files section for any typed query, with file icons and an error line, as Electron does.

Change: files show only in the Files scope, with a generic icon and no error state -> a Files section in the full list, per-file icons, and an error line. Q-135.

Boundaries:

- Electron: the file search runs when the scope is Files or the query is not empty ([packages/app/src/command-center/command-center.tsx](packages/app/src/command-center/command-center.tsx) line 262); the Files section is pinned at rank 4 in the full list (lines 294-296). An empty query with no scope shows no Files section.
- Icons: the Material file icons live in the explorer crate (`crates/explorer/src/file_icons.rs`), and `bachuc-command` does not depend on it. The app passes each row's icon image from `bachuc_explorer::file_icon(file_name) -> Arc<Image>` (`file_icons.rs` line 43); the command crate draws it. Planner call.
- Keep `set_file_results(&mut self, paths: Vec<String>, cx)` working; add the new methods beside it.
- The full list is laid out over plugin and app items only (`center.rs` line 328, `plugin_rows::layout_keeping`). Add file rows to that layout as a Files section after the other sections; keep keyboard order and selection working across it.

Files:

- [packages/desktop-gpui/crates/command/src/center.rs](packages/desktop-gpui/crates/command/src/center.rs)
- [packages/desktop-gpui/crates/command/src/lib.rs](packages/desktop-gpui/crates/command/src/lib.rs) (exports)

Interfaces:

- Produces: `FileResult { path: String, icon: Arc<Image> }`, exported from `bachuc_command`, for TASK-231.
- Produces: `CommandCenter::set_file_rows(&mut self, rows: Vec<FileResult>, cx: &mut Context<CommandCenter>)` for TASK-231.
- Produces: `CommandCenter::set_file_error(&mut self, message: Option<String>, cx: &mut Context<CommandCenter>)` for TASK-231.
- Produces: `CommandFileEvent::FileQuery { query: String }` is now also emitted in the full list for a non-empty query (same variant) for TASK-231.

Expected result:

- Cmd+K, type "main" -> a Files section with "src/main.rs" under the other sections.
- An error answer shows one muted error line in the Files section.

Anti-goal: Cmd+K to the first row stays under 50 ms; tripwire; the `search` bench test in `crates/command/tests/search.rs` passes before and after (Known Failure rule applies under load).

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-command` -> passes, with the full-list Files case.
- UI check on live: Cmd+K, "main" -> a Files section.
