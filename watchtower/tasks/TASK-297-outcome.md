# TASK-297 Outcome

## Status

BLOCKED: BUILT, UNVERIFIED (Q-189). Code committed in 04139fc00; see UI Checks.

## Changed

- `packages/desktop-gpui/crates/explorer/src/file_menu.rs`: `FileAction::OpenFileDiff` (group Open, label "Open file diff", icon `IconName::FileDiff`); `(git.uncommitted, OpenFileDiff)` first in the changed file arm of `menu_entries`; menu doc comment; red test plus three updated expected lists.
- `packages/desktop-gpui/crates/explorer/src/explorer.rs`: `run_file_action` maps `(OpenFileDiff, MenuTarget::Change { .. })` to `ExplorerEvent::OpenDiff { cwd, path }`; module comment and `OpenDiff` doc comment name the menu item.

## Contract

- Red test `an_uncommitted_changed_file_offers_open_file_diff_first` added first; red = compile error E0599 (no variant `OpenFileDiff`). Now passes.
- `shell.rs` unchanged. A row click still emits `OpenAllDiffs`. A renamed file uses `path` (new path).

## Verified

- `./cargo.sh test -p bachuc-explorer` -> 57 lib, 38 + 31 integration tests pass (CARGO_BUILD_JOBS=4).
- `./cargo.sh clippy -p bachuc-explorer --no-deps -- -D warnings` -> one error, `file_view.rs:94` `large_enum_variant` (Known Failure, untouched). No error on a line this TASK added. `--no-deps` substituted per run override.
- `git diff --stat -- packages/desktop-gpui/crates/app/src/shell.rs` -> no change.
- Anti-goal (PASS): changed expected lists are `file_menu_of_a_changed_file_ends_with_revert`, `..._drops_what_does_not_apply` (deleted list only; committed list unchanged), `file_menu_hides_discard_on_a_host_without_the_feature`. Each is an uncommitted changed file list gaining "Open file diff" first (deleted list also gains a separator). Files, Changes folder, and Committed lists keep their exact strings. Read from the edits at completion.
- Screen checks (AX ShowMenu, diff tab with `diff-file-toolbar`, untracked double press, row click, Committed no item, TASK-288/291 one-file lines): DEFERRED (UI checker).

## Lessons

- none

## UI Checks

UI checker, 2026-10-22 (Round 1), check bundle rebuilt at `04139fc00` on live (6768), zoom 1.25. Scratch worktree workspace `gray-kangaroo` held a modified `CONTRIBUTING.md`, a new `new-scratch.md`, a 1.8 MB `big-scratch.md`, a binary `binary-scratch.md`, a deleted `README.ko.md`, a modified `code.rs`, and a committed `SECURITY.md` change. Agent and workspace are archived.

Main finding: the Changes row menu cannot be opened without the pointer. The AX row lists only "Pick" as a secondary action; `performSecondaryAction` with "ShowMenu", "Show Menu", and "ShowContextMenu" on the row failed with error -10005 (not a valid secondary action). The app registers `AccessibleAction::ShowContextMenu` at `explorer.rs:2106`, but `accesskit_macos` maps no show-menu action (MEMORY 2026-10-18). The menu key and shift-f10 open the tab menu only (`actions.rs:204-205`). A right-click needs the pointer, which is banned. Every line that needs the "Open file diff" item is UNVERIFIED for that reason. The new item was not observed.

- Screen check, right-click an uncommitted changed `.md` row, press "Open file diff" -> diff tab with `diff-file-toolbar`, "Wrap long lines", "Show rendered": UNVERIFIED (row menu unreachable without the pointer).
- Screen check, untracked `.md` row, press "Open file diff" twice -> one diff tab (`diff_key` reuse): UNVERIFIED (row menu unreachable without the pointer).
- Screen check, AX press on a Changes row opens the all-files diff, as before: PASS. With the Changes diff tab closed, pressing the `CONTRIBUTING.md` row opened one "Changes" tab. Its toolbar was `toolbar Diff` with 5 buttons in order: `Diff mode`, `Collapse all files`, `Hide whitespace`, `Wrap long lines`, `Refresh`. The AX tree had no `diff-file-toolbar` and no "Show rendered".
- Screen check, TASK-288 and TASK-291 one-file lines through the item: UNVERIFIED (row menu unreachable without the pointer). See "Round 2" in both outcomes.
- Screen check, Committed compare shows no "Open file diff" in a file row menu: UNVERIFIED (row menu unreachable without the pointer). Covered only by the unit test `an_uncommitted_changed_file_offers_open_file_diff_first`.

Blocked: BUILT, UNVERIFIED (Q-189). The owner can run the UNVERIFIED lines by hand (right-click a changed file in Changes -> "Open file diff"), or a follow-up TASK can add an AX Click or key path to the row menu (MEMORY 2026-10-18: "give every control and context menu a keyboard or AX Click path").
