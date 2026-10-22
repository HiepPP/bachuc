# TASK-297 Open file diff from a Changes row menu

Group: I (writes file_menu.rs and explorer.rs; no other open TASK writes them)
Class: code

## Brief

Goal: Give the owner a way to open the one-file uncommitted diff, so the one-file wrap toolbar (TASK-288) and the Rendered markdown diff (TASK-291) can be used. Answers [Q-186](watchtower/QUESTIONS.md).

Change: no app path emits `ExplorerEvent::OpenDiff` -> the right-click menu of a changed file in the Changes tab has an "Open file diff" item that emits it, and the app opens the one-file diff tab through the existing `open_diff` in [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs).

Boundaries:

- Owner choice on 2026-10-22: a menu item on the Changes file row. A plain click on a Changes row keeps emitting `ExplorerDiffEvent::OpenAllDiffs`. Do not change the click.
- Show the item only for a changed file row (`MenuTarget::Change`) whose `GitState::uncommitted` is true. The one-file diff always loads the uncommitted compare, so a Committed tree (`tree_compare` set) shows no item. A Changes folder row and a Files tree row show no item.
- Show it also for a deleted file: the one-file diff shows its removed lines, and TASK-291 hides the Rendered toggle there.
- Planner call: label "Open file diff", icon `IconName::FileDiff` (the icon of the diff tab in `open_diff`). Put it in the Open group, before "Open to the side".
- The item emits `ExplorerEvent::OpenDiff { cwd, path }` with the Explorer `cwd` and the row path. The app side already handles it at [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (`ExplorerEvent::OpenDiff` arm); do not change `shell.rs`.
- Update the module comment at the top of `explorer.rs` and the doc comment of `ExplorerEvent::OpenDiff` ("A changed file of the Changes tree was picked") so they name the new menu item.
- Planner call: a renamed file opens the diff of its new path (`MenuTarget::Change::path`); `old_path` is not used.
- Adding the variant needs new arms in `group`, `label`, and `icon` in `file_menu.rs`; `file_menu_item` in `explorer.rs` needs no change.
- Existing menu tests change their expected lists for uncommitted changed files: each gains "Open file diff" first (the Builder Check found `file_menu_of_a_changed_file_ends_with_revert`, `file_menu_of_a_changed_file_drops_what_does_not_apply`, and `file_menu_hides_discard_on_a_host_without_the_feature`). A deleted file list gains "Open file diff" and a separator, because Open to the side is hidden there. This is the intended behavior change of Q-186. Lists for Files rows, Changes folders, and Committed files keep their exact strings.

How:

- Add `FileAction::OpenFileDiff` in `file_menu.rs`, with its group, label, and icon, and add `(git.uncommitted, FileAction::OpenFileDiff)` first in the changed file arm of `menu_entries`.
- In `run_file_action` in `explorer.rs`, map `(FileAction::OpenFileDiff, MenuTarget::Change { .. })` to `cx.emit(ExplorerEvent::OpenDiff { cwd, path })`.

Files:

- [packages/desktop-gpui/crates/explorer/src/file_menu.rs](packages/desktop-gpui/crates/explorer/src/file_menu.rs) (the action, its placement in `menu_entries`, the tests)
- [packages/desktop-gpui/crates/explorer/src/explorer.rs](packages/desktop-gpui/crates/explorer/src/explorer.rs) (run the action; module comment)

Expected result:

- WHEN the owner right-clicks an uncommitted changed file in Changes THE menu SHALL show "Open file diff" first.
- WHEN the owner picks "Open file diff" THE app SHALL open the one-file diff tab of that file, with the `diff-file-toolbar`.
- IF the Changes tab shows a Committed compare, or the row is a folder, or the menu is from the Files tree THEN THE menu SHALL show no "Open file diff".
- WHEN the owner clicks a Changes row THE app SHALL still open the all-files diff.

Red tests:

File: [packages/desktop-gpui/crates/explorer/src/file_menu.rs](packages/desktop-gpui/crates/explorer/src/file_menu.rs) (existing `tests` module)

```rust
    #[test]
    fn an_uncommitted_changed_file_offers_open_file_diff_first() {
        let git = GitState {
            untracked: false,
            deleted: false,
            uncommitted: true,
            expanded: false,
        };
        let entries = menu_entries(EntryKind::File, Some(git), ALL);
        assert_eq!(entries.first(), Some(&FileMenuEntry::Item(FileAction::OpenFileDiff)));
        assert_eq!(FileAction::OpenFileDiff.label(), "Open file diff");
        let deleted = GitState { deleted: true, ..git };
        assert_eq!(
            render(&menu_entries(EntryKind::File, Some(deleted), ALL)),
            vec!["Open file diff", "---", "Copy path", "Copy relative path", "---", "Discard changes"]
        );
        let committed = GitState { uncommitted: false, ..git };
        assert!(!menu_entries(EntryKind::File, Some(committed), ALL)
            .contains(&FileMenuEntry::Item(FileAction::OpenFileDiff)));
        let folder = GitState { expanded: true, ..git };
        assert!(!menu_entries(EntryKind::Directory, Some(folder), ALL)
            .contains(&FileMenuEntry::Item(FileAction::OpenFileDiff)));
        assert!(!menu_entries(EntryKind::File, None, ALL)
            .contains(&FileMenuEntry::Item(FileAction::OpenFileDiff)));
    }
```

Anti-goal: the menus of Files rows, Changes folders, and Committed changed files keep their exact item lists; tripwire; read from the expected lists of the existing `file_menu` tests in `git diff` at completion (only uncommitted changed file lists may gain "Open file diff" first, plus a separator when no other Open item follows).

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer` -> all tests pass, including `an_uncommitted_changed_file_offers_open_file_diff_first`.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh clippy -p bachuc-explorer --no-deps -- -D warnings` -> no error on a line this TASK added (the `enum State` lint in `file_view.rs` is a Known Failure).
- `git diff --stat -- packages/desktop-gpui/crates/app/src/shell.rs` -> no change.
- Anti-goal: in `git diff -- packages/desktop-gpui/crates/explorer/src/file_menu.rs`, every changed expected list is an uncommitted changed file list that gains "Open file diff" first, plus a separator when no other Open item follows.
- Screen check (UI checker, `cua_repl`, check bundle on live): in a scratch worktree, change a tracked `.md` file; open Changes, open the row menu with the AX `ShowMenu` action, and press "Open file diff" -> a diff tab opens whose AX tree has `diff-file-toolbar` with "Wrap long lines" and "Show rendered".
- Screen check: on a new untracked `.md` file row, press "Open file diff" twice -> one diff tab, activated again on the second press (`diff_key` reuse).
- Screen check: press a Changes row with the AX press action -> the all-files diff opens, as before.
- Screen check: with the one-file diff open, run the one-file lines of TASK-288 and TASK-291 `## Verify` that the UI checker left UNVERIFIED (wrap toggle, red and green blocks, new untracked file all green, append while Rendered, placeholder with no toggle, deleted file with no toggle, the daemon log anti-goal) -> record each result in the TASK-288 and TASK-291 outcome sidecars.
- Screen check: switch Changes to a Committed compare and open a file row menu -> no "Open file diff".
