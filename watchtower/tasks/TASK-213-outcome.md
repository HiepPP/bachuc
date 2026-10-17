# TASK-213 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/file_menu.rs` (new): the menu model `menu_entries(kind, git, features)`, the confirm texts `delete_confirm` and `revert_confirm`, and the path rules `parent_path`, `child_path`, `create_parent`, `absolute_path`, `entry_name`, `discard_paths`, and `draft_slot`, with 9 tests.
- `packages/desktop-gpui/crates/explorer/src/explorer.rs`: `ExplorerFileEvent` and its `EventEmitter`. A right click (or the AX `ShowContextMenu` action) on a Files row or a changed file row selects it and opens a gpui-kit `PopupMenu` at the pointer. Inline name rows for New file, New folder, and Rename. Delete and Discard confirms. An inline error line. The toolbar comment no longer says create needs a client request.
- `packages/desktop-gpui/crates/explorer/src/lib.rs`: `mod file_menu;`, and `ExplorerFileEvent` in the `explorer` export.

## Contract

- Produces: `ExplorerFileEvent { OpenToSide { cwd: String, path: String }, AddToChat { cwd: String, path: String } }`, through `impl EventEmitter<ExplorerFileEvent> for Explorer`. `ExplorerEvent` is unchanged.
- Each "Consumes:" line from TASK-206 is used by its exact name: `NewEntryKind`, `create_entry`, `rename_entry`, `duplicate_entry`, `delete_entry`, `fs_entry_ops_supported`, `fs_entry_duplicate_supported`, and `discard_changes`.
- The Files menu order is Electron's: New file, New folder | Open to the side | Copy path, Copy relative path, Add to chat | Rename, Duplicate | Delete. A separator goes between groups. Open to the side and Add to chat show only on files. Create, Rename, and Delete need `fs_entry_ops_supported`. Duplicate needs `fs_entry_duplicate_supported`.
- The Changes menu, on a changed file only, is: Open to the side | Copy path, Copy relative path, Add to chat | Discard changes. A deleted file drops Open to the side and Add to chat. Discard shows only in the Uncommitted mode (`tree_compare` is `None`), as Electron's `diffMode === "uncommitted"`.
- After a create, rename, duplicate, or delete succeeds, the Explorer lists the parent folder again. A created file is selected and opened with `ExplorerEvent::OpenFile`, as Electron does. A renamed or deleted selection follows the entry.
- New file or New folder opens its folder first, then puts the name row first under it. Enter or a blur commits, an empty name cancels, and Escape cancels. Rename puts a field with the name selected in the row.
- Discard sends the path, plus the old path of a rename. After it succeeds, the Explorer asks for the status again and lists the shown parent folder again.
- Copy path writes `absolute_path(cwd, path)`; Copy relative path writes the path.
- Safe default: the Revert item uses Electron's label "Discard changes" (`workspace.fileActions.revert`).
- Safe default: "untracked" is `is_new`. The server unstages the paths and then cleans untracked files, so a staged new file is deleted too. Its confirm is "Delete <path>?" with "This file is not tracked."; a tracked file gets "Discard changes?" with `Changes to "<path>" will be permanently discarded.`
- Safe default: New file and New folder on a file row create in the file's folder. Electron passes the file path itself as the parent.
- Safe default: the confirms use gpui-kit `open_alert_dialog` with a danger OK button, as the sidebar archive confirm does. A failed operation shows its message as a red line under the tab rail; a press clears it. No toast.
- Out of scope: Download, Reveal, Open in editor, Collapse folder, a menu on Changes folder rows, Duplicate in the Changes menu, and New file and New folder buttons in the Files toolbar.

## Verified

- `./cargo.sh test -p bachuc-explorer file_menu` -> 9 passed, including `file_menu_revert_confirm_texts` (untracked and tracked texts).
- `./cargo.sh test -p bachuc-explorer` -> 45 + 38 + 21 passed.
- `./cargo.sh build -p bachuc-explorer` -> exit 0, no warnings.
- `./cargo.sh build -p bachuc-app` -> exit 0.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `./cargo.sh clippy -p bachuc-explorer --tests` -> no warnings in the changed files. The existing warnings are in `file_view.rs:86` and the `bachuc-protocol` build script.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- UI check, create, rename, duplicate, and delete in a scratch folder update the tree: DEFERRED (UI checker).
- UI check, revert a changed file in a scratch repo: DEFERRED (UI checker).
- Anti-goal, scroll CPU over a 2,000-entry tree: DEFERRED (UI checker). The builder took no "before" reading. Each visible row gains one right-click listener and one AX action closure; the draft slot is computed only while a name row is open.

## Lessons

- `bachuc-app` does not handle `ExplorerFileEvent` yet; TASK-203 subscribes to it.
- The menu is opened at the pointer from the right mouse-down. The AX `ShowContextMenu` action opens it at `window.mouse_position()`, so a UI checker can open it without the pointer.
- A dismiss of an old `PopupMenu` can arrive after a new one opens, so the dismiss handler checks that the entity is still the open menu.
- PopupMenu's dismiss does not take focus back when the item's handler focused something else, so a menu item can focus the inline field directly.

## UI Checks Run 4

Run 2026-10-18 by the UI checker (run 4). Bundle `Bachuc GPUI Check` built from HEAD `60fd3a593`, PID 88032, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The checker forced each frame with the AX "zoom the window" action and sent keys with cua `pressKey`/`typeText` to the check app only. Scratch repo `/tmp/t202-repo` with disposable agents "r4 board card", "r4 remove card", "r4 fork src" (mock ten-second-stream), "r4 fork ok" (Haiku 5.5, one-word reply), and the fork child "Continue." (`af3218a`). After the run, PID 88032 exited, all five agents and the five t202-repo workspaces were archived, and the repo stayed as before (`feature-x`, ` M a.txt`).

- Open the file menu without the pointer: FAIL. The rows of the Files tree expose only the AX action "Pick" (`row (selectable) a.txt, Secondary Actions: Pick`). "AXShowMenu", "ShowMenu", and "Show menu" are rejected as "not a valid secondary action". The cause: `accesskit_macos` 0.26.3 (the version in `Cargo.lock`) has no `accessibilityPerformShowMenu` method; it maps only press, pick, increment, decrement, and value actions, so the `AccessibleAction::ShowContextMenu` handler at `crates/explorer/src/explorer.rs:1515` can never run on macOS. The explorer has no keyboard path to the menu either.
- New file, Rename, Duplicate, Delete with confirm, Discard changes with confirm, tree updates: UNVERIFIED (unattended): needs pointer. The menu opens only from a right mouse-down.
- Revert a changed file in the scratch repo: UNVERIFIED (unattended): needs pointer. `/tmp/t202-repo` keeps its change (` M a.txt`).
- Anti-goal, scroll CPU over a 2,000-entry tree: UNVERIFIED (unattended): needs pointer (scroll).
