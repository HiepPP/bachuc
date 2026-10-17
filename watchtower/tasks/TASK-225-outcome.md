# TASK-225 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/file_menu.rs`: `GitState.expanded`; new actions `CollapseFolder` ("Collapse folder", open group) and `Reveal` ("Reveal in Finder", reference group). `menu_entries` gives a Changes folder row (`EntryKind::Directory` with `git`) `DiffFolderRow`'s items, and a changed file gets Duplicate unless it is deleted. 2 new tests; old Changes tests expect Duplicate.
- `packages/desktop-gpui/crates/explorer/src/explorer.rs`: `MenuTarget::ChangeFolder`. Changes folder rows open the menu on a right mouse-down and on the AX `ShowContextMenu` action. Collapse folder, Reveal, Duplicate, and Discard changes run on a folder; Duplicate runs on a changed file. `AfterEntryOp::Status`. The Files toolbar has New file and New folder buttons. An empty root shows the New file row.

## Contract

- No public API change. `ExplorerEvent`, `ExplorerFileEvent`, and `ExplorerNoticeEvent` are unchanged.
- Changes file menu: Open to the side | Copy path, Copy relative path, Add to chat | Duplicate | Discard changes. Duplicate needs `fsEntryDuplicate` and a file that is not deleted (`file-header.tsx` line 178). Committed mode keeps Duplicate, as Electron passes `onDuplicate` in both modes.
- Changes folder menu: Collapse folder (only while open) | Copy path, Copy relative path, Reveal in Finder | Duplicate (`fsEntryDuplicate`) | Discard changes (`checkoutDiscardChanges` and Uncommitted mode). No New file or New folder.
- Collapse folder collapses the folder and every folder under it, as `handleCollapseFolder` in `changed-files-tree.tsx`.
- A folder discard sends the folder path with the tracked confirm ("Discard changes?").
- A Changes Duplicate lists the parent in the Files tree only when it is loaded, then asks for the status again.
- Reveal runs `open -R <absolute path>`. A failure emits `ExplorerNoticeEvent { ok: false, "Failed to reveal entry" }`.
- Files toolbar: `files-new-file` and `files-new-folder` before the hidden files toggle. They show only with `fsEntryOps`. A click and an AX Click both open the inline name row at the root. Each has `Role::Button` and an aria label.
- The TASK-222 gates on Discard and Add to chat are unchanged.
- Safe default: Reveal shows on every Changes folder. Electron shows it only with a local daemon and a file manager target. GPUI has no local-host flag, and its header always offers Finder.
- Safe default: Reveal is only on Changes folder rows, as the spec lists. Electron also shows Reveal and Download on Changes file rows and Files rows. Those stay out of scope.

## Verified

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer file_menu` -> 13 passed. New: `file_menu_of_a_changed_folder_has_the_diff_folder_row_items`, `file_menu_of_a_changed_file_offers_duplicate_unless_deleted`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer` -> 56 + 38 + 21 passed.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `./cargo.sh clippy -p bachuc-explorer --tests` -> no warnings in the changed files. The existing warning is in `file_view.rs:86`.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> exit 0. Its one warning (`next_theme` unused in `crates/app/src/state.rs`) is in the app crate, not this TASK.
- GitNexus impact: the stale index resolved `menu_entries` to `workspace/src/tab_menu.rs` and did not find `GitState`. `rg` shows every changed symbol is used only in the explorer crate.
- UI check, the toolbar New file button pressed by AX shows the inline name row: DEFERRED (UI checker).
- Owner hand test, right-click a changed folder: DEFERRED (owner).
- Anti-goal, scroll CPU over a 2,000-entry tree: DEFERRED (UI checker). Files rows are unchanged. Each visible Changes folder row gains one right-click listener and one AX action closure, the same as a changed file row.

## Lessons

- An empty root renders no list, so a root-level name row needs the list arm to match the empty states while a draft is open.

## UI Checks

Run 2026-10-18 by the UI checker (run B). Bundle `Bachuc GPUI Check` built from HEAD `800604233`, PID 46300, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). Frames were forced with the AX "zoom the window" action; keys went to the check app only. Scratch repo `/tmp/t202-repo` was the live workspace "rb check" (`wks_86e1ae1af8379049`), with a scratch folder `rb-big` of 2,000 files. After the run, PID 46300 exited, the workspace was archived, the scratch files were deleted, and the repo stayed as before (`feature-x`, ` M a.txt`).

- Toolbar New file, pressed by AX, shows the inline name row at the root: PASS. The row "File name" with a focused text field drew above `.dev`. Typing `rb-new.txt` and Enter added `rb-new.txt` to the tree (and on disk), and opened it in a tab. Evidence: `/tmp/rb-225-01-new-file-row.jpg`, `/tmp/rb-225-02-new-file.png`.
- Toolbar New folder, the same way: PASS. The row "Folder name" drew at the root; `rb-newdir` and Enter added the folder to the tree, sorted with the folders. Evidence: `/tmp/rb-225-03-new-folder-row.png`, `/tmp/rb-225-04-new-folder.png`.
- Right-click menus (changed file Duplicate, changed folder items): UNVERIFIED (unattended): needs pointer.
- Anti-goal, `top` CPU while scrolling a 2,000-entry tree: UNVERIFIED (unattended): needs pointer. The Files list is virtual (only about 22 rows in the AX tree), has no keyboard navigation, and exposes no AX scroll action, so only the wheel scrolls it. TASK-213 has no earlier reading. Idle reading with `rb-big` open: 0.0-0.2% (`top -l 6 -s 1`, load 4.4). Evidence: `/tmp/rb-225-05-rb-big-open.png`.
