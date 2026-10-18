# TASK-223 Branch switcher arrow keys and detached HEAD

Group: E (shared file [packages/desktop-gpui/crates/explorer/src/branch_switcher.rs](packages/desktop-gpui/crates/explorer/src/branch_switcher.rs) and the explorer crate)
Class: code

## Brief

Goal: The branch switcher moves a highlight with the arrow keys, and a detached HEAD shows nothing, as in Electron.

Change: arrows do nothing, Enter always picks the first row (`branch_switcher.rs` line 224), and a detached HEAD leaves a passive "Unknown" label -> Electron behavior. Q-122.

Boundaries:

- Builder Check: a detached HEAD already hides the switcher (`branch_switcher.rs` line 152, `explorer.rs` lines 1895-1898); only the "Unknown" label remains, and it goes. `rank_branches` (lines 37-97) already has the subsequence tier, and Electron pickers leave typo-fuzzy and the compact pass off (`packages/app/src/utils/text-match.ts` line 6, `combobox-options.ts`), so the match tiers stay as they are.
- Arrow Up and Down move the highlight and wrap as Electron does; Enter picks the highlighted row, not always the first.
- Errors already go to the toast after TASK-222; do not add inline errors back.
- Put the highlight move (with wrap) in a pure function with a test.

Files:

- [packages/desktop-gpui/crates/explorer/src/branch_switcher.rs](packages/desktop-gpui/crates/explorer/src/branch_switcher.rs)

Expected result:

- Detached HEAD -> no switcher in the Changes header.
- Down, Down, Enter picks the third row.

Anti-goal: opening the switcher sends one `branch_suggestions_request`, as TASK-211 measured; tripwire; count in the live `daemon.log` per open, read at the UI check.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer branch` -> passes, with the highlight cases.
- UI check on live (scratch repo): arrow keys move the highlight; Enter switches to the highlighted branch.
- Anti-goal: one `branch_suggestions_request` per switcher open in the live `daemon.log`.
