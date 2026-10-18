# TASK-213 Explorer file menu

Group: E (shared file [packages/desktop-gpui/crates/explorer/src/explorer.rs](packages/desktop-gpui/crates/explorer/src/explorer.rs) and the explorer crate)
Class: code

## Brief

Goal: A right-click menu on Files tree rows, as Electron's file actions menu, plus Revert on changed files.

Change: no context menu in the explorer -> New file, New folder, Open to side, Copy path, Copy relative path, Add to chat, Rename, Duplicate, Delete, and Revert.

Boundaries:

- Electron: [packages/app/src/components/file-actions-menu.tsx](packages/app/src/components/file-actions-menu.tsx) lines 114-250. Order is create, open, reference, manage, destructive, with separators between groups. Download is out of scope.
- New file and New folder use an inline name row; Rename uses inline edit. Delete asks to confirm first.
- After a create, rename, duplicate, or delete succeeds, re-list the parent folder, as `use-file-explorer-actions.ts` lines 256-328 do.
- Revert: Electron has it only in the diff pane (`git/diff-pane.tsx` line 1710). Owner choice: add it to the menu of a changed file in the Changes tree. It asks to confirm, then runs `discard_changes` with that path.
- Hide create, rename, and delete when `fs_entry_ops_supported` is false; hide Duplicate when `fs_entry_duplicate_supported` is false.
- Open to side and Add to chat are emitted as a new event type, `ExplorerFileEvent`, with `impl EventEmitter<ExplorerFileEvent> for Explorer`. Do not add variants to `ExplorerEvent`: the app matches it exhaustively (`shell.rs` line 711). The app (TASK-203) handles the new type.
- Revert on an untracked file deletes it. Planner call: for an untracked file the confirm says "Delete <name>? This file is not tracked." For a tracked file it says the changes will be discarded.
- The toolbar comment at `explorer.rs` lines 765-768 says create needs a client request. Remove that comment when create works.

How:

- Add `file_menu.rs` with the menu model as a pure function of entry kind, git state, and features, with tests.
- Add the right-click handler, inline rows, and confirm in `explorer.rs`.
- Add the `ExplorerFileEvent` type and its emitter.

Files:

- [packages/desktop-gpui/crates/explorer/src/file_menu.rs](packages/desktop-gpui/crates/explorer/src/file_menu.rs) (new: menu model)
- [packages/desktop-gpui/crates/explorer/src/explorer.rs](packages/desktop-gpui/crates/explorer/src/explorer.rs) (right-click, inline rows, confirm, events)
- [packages/desktop-gpui/crates/explorer/src/lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs) (module line)

Interfaces:

- Consumes: `NewEntryKind` enum `{ File, Directory }` and `Client::create_entry(&mut self, cwd: &str, parent_path: &str, name: &str, kind: NewEntryKind, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` from TASK-206.
- Consumes: `Client::rename_entry(&mut self, cwd: &str, path: &str, name: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>`, `Client::duplicate_entry(&mut self, cwd: &str, path: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>`, and `Client::delete_entry(&mut self, cwd: &str, path: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` from TASK-206.
- Consumes: `Client::fs_entry_ops_supported(&self) -> bool` and `Client::fs_entry_duplicate_supported(&self) -> bool` from TASK-206.
- Consumes: `Client::discard_changes(&mut self, cwd: &str, paths: Vec<String>, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` from TASK-206.
- Produces: `ExplorerFileEvent` enum `{ OpenToSide { cwd: String, path: String }, AddToChat { cwd: String, path: String } }` for TASK-203.

Expected result:

- Right-click a file -> the menu shows the items in Electron's order with separators.
- New file in a folder -> an inline row; Enter creates the file, and it shows in the tree.
- Rename, Duplicate, and Delete (after confirm) change the tree.
- Right-click a changed file in Changes -> Revert; confirm discards its changes.
- Copy path and Copy relative path put the right text on the clipboard.

Anti-goal: Files tree scroll must stay smooth; drift gauge; scroll CPU over a 2,000-entry tree stays at or under the reading before the change plus 1 point, read with `top -l 2 -pid <pid>` before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer file_menu` -> passes, including the untracked and tracked confirm texts.
- `./cargo.sh build -p bachuc-app` -> builds before TASK-203 handles the new event.
- UI check on live in a scratch folder: create, rename, duplicate, and delete a file -> the tree updates each time.
- UI check on live: revert a changed file in a scratch repo -> its change is gone from Changes.
- Anti-goal: scroll CPU over a 2,000-entry tree -> at most the earlier reading plus 1 point.
