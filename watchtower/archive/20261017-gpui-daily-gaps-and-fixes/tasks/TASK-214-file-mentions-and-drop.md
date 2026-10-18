# TASK-214 File mentions and file drop in the composer

Group: M (standalone: [packages/desktop-gpui/crates/composer](packages/desktop-gpui/crates/composer))
Class: code

## Brief

Goal: Typing `@` in the composer suggests files and folders, and dropping files on the composer attaches images or inserts path mentions.

Change: the placeholder promises "@files" but nothing handles `@`, and there is no drop handler -> both work as in Electron.

Boundaries:

- Mention detection: port `findActiveFileMention` ([packages/app/src/utils/file-mention-autocomplete.ts](packages/app/src/utils/file-mention-autocomplete.ts) lines 20-41). Scan back from the cursor to the last `@`; the query after it has no whitespace or quote. A pick replaces `@query` with the quoted relative path `"path"` (lines 43-52).
- Suggestions: `directory_suggestions` with files and folders on, limit 50, debounced, as `use-agent-autocomplete.ts` lines 427-451 do.
- Reuse the slash popup look and key handling (`composer.rs` `slash_menu` line 1823, `slash_key` line 924). Slash only triggers at the start of the text (`slash.rs` line 122), so put mention logic in a new `mention.rs`.
- Drop: gpui-pre 0.3.9 has `ExternalPaths` and `on_drop::<ExternalPaths>`. Images reuse `load_images`, as paste does (`composer.rs` lines 1378-1383). Owner choice: a file that is not an image inserts a quoted path mention at the cursor. A path inside the workspace is relative; outside it is absolute.
- Ignore drops while the composer submits.
- Planner call: no drag backdrop in this TASK; record it if `drag_over::<ExternalPaths>` is easy and add it.

How:

- Add `mention.rs` with the detection and replacement as pure functions, with tests that use Electron's cases.
- Wire mention detection near `composer.rs` line 836 and key routing near line 924.
- Add the drop handler on the composer root.

Files:

- [packages/desktop-gpui/crates/composer/src/mention.rs](packages/desktop-gpui/crates/composer/src/mention.rs) (new)
- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (popup wiring, drop handler)
- [packages/desktop-gpui/crates/composer/src/lib.rs](packages/desktop-gpui/crates/composer/src/lib.rs) (module line)

Interfaces:

- Consumes: `DirectorySuggestion { path: String, is_directory: bool }` and `Client::directory_suggestions(&mut self, cwd: &str, query: &str, include_files: bool, include_directories: bool, limit: u32, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<DirectorySuggestion>>>` from TASK-206.
- Produces: `Composer::insert_path_mention(&mut self, path: &str, window: &mut Window, cx: &mut Context<Composer>)` for TASK-203 (Add to chat).

Expected result:

- Type `see @src/ma` -> a popup lists matching files; Enter replaces the query with `"src/main.rs"`.
- `@` after a space with no text shows suggestions; a space closes the popup.
- Drop a PNG -> it attaches as an image. Drop a `.rs` file from the workspace -> `"src/x.rs"` is inserted.
- Slash commands still work at the start of the text.

Anti-goal: typing latency must not grow; drift gauge; one suggestion request per pause, never one per key, proven by the daemon log while typing 10 characters fast, read at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer mention` -> passes, including Electron's detection cases.
- UI check on live: type `@` and a query in the composer -> the popup lists files; Enter inserts the quoted path.
- `./cargo.sh test -p bachuc-composer mention` -> a composer with no cwd (a draft before a folder is picked) sends no suggestion request and shows no popup.
- Owner hand test: drag a file and an image from Finder onto the composer -> a path mention and an image attachment.
- Anti-goal: type 10 characters fast after `@`, then count `directory_suggestions_request` in the live `daemon.log` -> 3 or fewer.
