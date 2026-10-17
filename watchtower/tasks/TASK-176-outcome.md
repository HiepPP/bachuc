# TASK-176 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/command/Cargo.toml`: adds `bachuc-plugin-host`.
- `packages/desktop-gpui/Cargo.lock` (generated): the new `bachuc-command` dependency. The file also holds the other builder's changes.
- `packages/desktop-gpui/crates/command/src/search.rs`: `SearchIndex::with_keywords`; a third rank for keyword matches; 4 unit tests.
- `packages/desktop-gpui/crates/command/src/plugin_rows.rs` (new): the pure rules and their 10 tests.
- `packages/desktop-gpui/crates/command/src/center.rs`: plugin rows, `set_plugin_context`, host subscription, run order.
- `packages/desktop-gpui/crates/command/src/lib.rs`: private `plugin_rows` module and doc lines.
- `packages/desktop-gpui/crates/command/tests/search.rs`: the 600-item keyword timing test.

## Contract

- `CommandCenter::set_plugin_context(&mut self, workspace_id: Option<String>, agent_id: Option<String>, cx: &mut Context<CommandCenter>)` is public. It does nothing when both ids are unchanged.
- `CommandItem`, `CommandEvent`, `CommandCenter::new`, `set_items`, and `SearchIndex::new` keep their signatures. `crates/app` needed no edit.
- Plugin rows:
  - `CommandCenter::new` reads `bachuc_plugin_host::host(cx)` and `index().command_items()`, and puts the rows before the caller's items.
  - Each plugin has one section titled with the plugin id, in index order.
  - The row id is `plugin:<plugin_id>:<item_id>`. Rows show with an empty query.
  - Gate order: context first, then `visible`, so `visible` never runs for an item the context hides.
  - Before `set_plugin_context`, only `Global` items show.
- Keywords:
  - Kept in the private `PluginRow` side table. `CommandItem` has no keyword field.
  - `SearchIndex::with_keywords(impl IntoIterator<Item = (&str, &str, Vec<&str>)>)` is the new constructor; `new` calls it with no keywords.
  - Rank order: title match, then group match, then keyword match. Before, a group match was the lowest rank; the keyword rank is a new one below it.
  - Equal scores keep index order. A keyword is never drawn.
- Run: Enter or click of a plugin row calls `on_select(CommandInput { workspace_id, agent_id }, window, cx)`, then emits one `CommandEvent::Run` with the `plugin:` id. `run_row` fixes this order.
- Rebuild: the view subscribes to the host entity and refreshes on `ContributionsChanged` and `PluginStateChanged`.
  - It keeps the query and the selected row id when that row still exists; otherwise it highlights the first row.
  - It resets the list only when a row's id, title, icon, or keywords changed. Handlers are always replaced.
  - A host created after the view opened is not subscribed. The app creates the host first (TASK-191).
- Slash commands: no code and no composer edit, as the spec says.
- Safe defaults taken (no question needed):
  - With a query, sections sort by best score across plugin and app sections, as `search.rs` does today. Plugin sections come first only on ties and with an empty query.
  - `PluginStateChanged` refreshes every plugin's rows, not only the named plugin, because a `visible` result may depend on another plugin.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-command` -> pass: lib 31 passed, `tests/search.rs` 5 passed, `tests/shortcuts.rs` 9 passed. The existing search and shortcut tests are unchanged. New pure tests cover each expected case:
  - no plugin item gives today's list;
  - the context gate for `Global`, `Workspace`, `Agent`;
  - `visible` false, and the gate-before-`visible` order;
  - title match above keyword match, and the keyword not drawn;
  - keyword below title and group matches, in `search.rs` and against an app item's group;
  - two plugins give two sections in index order before "Actions";
  - a rebuild with query "ed" keeps the query and the selected id, and falls back to the first row;
  - `same_rows` ignores a handler but not the look;
  - the run order, handler then one `Run` with the `plugin:` id;
  - the 600-item keyword timing test.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-command` -> exit 0 (after `fmt -p bachuc-command`).
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0. `git status` shows no edit outside `crates/command` except `Cargo.lock`.
- The view itself (host subscription, click and Enter run, list reset) has no headless test and is checked by reading only: UNVERIFIED (unattended): code tests only.
- UI check: no UI check in this spec. Plugin rows get their screen check after TASK-191: DEFERRED (UI checker).

## Anti-goal

- Checkpoint 0, before any change: `search_500_items_stays_under_16_ms_per_keystroke` worst 1.08 ms; source `/tmp/t176-base.txt`; 2026-10-16 23:06.
- Checkpoint 1, after keyword search landed: 500 items 3.08 ms, 600 items with keywords 3.64 ms; source `/tmp/t176-s1.txt`; 2026-10-16 23:09. Load average was about 3.7 with the other builder compiling.
- Checkpoint 2, at completion: 500 items 1.39 ms, 600 items with keywords 1.87 ms; both under 16 ms; source `test -p bachuc-command --test search -- --nocapture`; 2026-10-16 23:14.
- The 600-item test mixes 100 keyword items with the 500 app items and types three queries: one matches by title, one only by keyword, one nowhere. The tripwire did not trip.

## Lessons

- A `Handler` is an `Rc<dyn Fn(I, &mut Window, &mut App)>`, so a pure test builds a row with `Rc::new(|_, _, _| {})`. Import tested names one by one in the test module of a file that uses gpui, to avoid the `test` macro clash.
- `layout.order` lists items by section, so a ranking test that puts items in one group sees the group order, not the global score order. Put each ranked item in its own group.
- A closure that must call a handler and then emit, both needing `cx`, fails the borrow check as two closures. `run_row` passes `cx` through as an argument.
