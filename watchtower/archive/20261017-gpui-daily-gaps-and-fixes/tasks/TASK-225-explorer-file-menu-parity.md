# TASK-225 Explorer file menu matches Electron

Group: E (shared files [file_menu.rs](packages/desktop-gpui/crates/explorer/src/file_menu.rs) and [explorer.rs](packages/desktop-gpui/crates/explorer/src/explorer.rs))
Class: code

## Brief

Goal: The Changes tab file menu, the Changes folder rows, and the Files toolbar match Electron.

Change: the Changes file menu has no Duplicate, Changes folder rows have no menu, and the Files toolbar has no New file or New folder buttons -> Electron's items. Q-132.

Boundaries:

- Changes file menu: Duplicate unless the file is deleted ([packages/app/src/git/file-header.tsx](packages/app/src/git/file-header.tsx) line 178).
- Changes folder row menu: Collapse, Copy path, Copy relative path, Reveal, Duplicate, Revert, each with Electron's condition ([packages/app/src/git/diff-folder-row.tsx](packages/app/src/git/diff-folder-row.tsx) lines 176-187). No New file or New folder there.
- Files toolbar: New file and New folder buttons at the root, shown only when entry ops are supported ([packages/app/src/components/file-explorer-pane.tsx](packages/app/src/components/file-explorer-pane.tsx) lines 1239-1262).
- Keep the TASK-222 gates (Discard, Add to chat). Errors go to `ExplorerNoticeEvent`.
- Every new button has `Role::Button`, an aria label, and an `AccessibleAction::Click` handler.

Files:

- [packages/desktop-gpui/crates/explorer/src/file_menu.rs](packages/desktop-gpui/crates/explorer/src/file_menu.rs)
- [packages/desktop-gpui/crates/explorer/src/explorer.rs](packages/desktop-gpui/crates/explorer/src/explorer.rs)

Expected result:

- Right-click a changed file -> Duplicate is in the menu.
- Right-click a changed folder -> Electron's folder items.
- The Files toolbar shows New file and New folder.

Anti-goal: scrolling a 2,000-entry tree stays as cheap as before; drift gauge; `top` CPU of the GPUI app while scrolling, read at the UI check against the TASK-213 reading; at most 1 point more.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer file_menu` -> passes, with the folder menu and Duplicate cases.
- UI check on live (scratch repo): the toolbar New file button, pressed by AX, shows the inline name row.
- Owner hand test: right-click a changed folder.
