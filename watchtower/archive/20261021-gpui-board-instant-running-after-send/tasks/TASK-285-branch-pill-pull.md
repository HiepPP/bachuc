# TASK-285 Branch pill Pull

Group: D (writes `packages/desktop-gpui/plugins/thread-branch/src/branch.rs` and `store.rs` only)
Class: code

## Brief

Goal: The thread branch pill menu gets a Pull row, so Hiep can pull the branch, for example `main ↓245`, without leaving the thread.

Change: menu `... Fetch, Refresh` -> `... Fetch, Pull, Refresh`; a Pull press runs the daemon pull for the agent's folder.

Boundaries:

- Follows [ADR-0059](watchtower/decisions/ADR-0059-gpui-branch-pill-pull-through-daemon-checkout-pull.md): Pull calls `Client::git_action(cwd, GitAction::Pull, cx)`. Add no server code in `hiep-plugins/plugins/thread-branch`.
- The Pull row shows when the branch has an upstream, as the Fetch row does. It also shows when `behind` is 0, because `git pull` fetches first.
- The label is `Pull`, with `IconName::ArrowDownToLine`. The row sits after Fetch and before Refresh.
- The New workspace pill menu (`draft_menu`) gets no Pull row.
- Before a pull, read the folder again with `Mode::Force`. Block the pull when that fresh info is dirty, and show the explorer's text: "Pull isn't available while you have local changes so commit or stash them first".
- On success, read the folder again with `Mode::Force`, so the pill drops its `↓` count. Show no success message.
- On a failure, show the error text with `error_message`, as Fetch does.
- A second Pull press for the same folder while a pull runs does nothing.
- A running agent in the folder does not block Pull. The dirty check covers its file edits.
- Planner call: when `ctx.client(cx)` is `None`, show "No host connection." and do not pull.
- Planner call: `PULL_NO_UPSTREAM` is "No upstream branch to pull from.". `pull_blocked` checks the upstream first, then dirty.
- Planner call: if the pre-pull `Force` read fails, show its error with `format!("{error:#}")` and do not pull. If the reload after a good pull fails, show its error too, as `spawn_load` does with `report: true`.
- Call the pull as `client.update(cx, |client, cx| client.git_action(&cwd, GitAction::Pull, cx))`, and show a pull `Err` with `format!("{error:#}")`.
- Signatures: `State::begin_pull(&mut self, cwd: &str) -> bool` and `State::end_pull(&mut self, cwd: &str)`.
- This changes the expected row lists of two existing tests in `branch.rs`: `branch_menu_offers_github_issues_and_pull_requests_below_the_open_row` and `menu_rows_visibility`. Their tracked menus gain `"pull"` between `"fetch"` and `"refresh"`. Hiep asked for this row on 2026-10-21. The draft list stays `["copy-branch", "-", "fetch", "refresh"]`.
- Trap: `run_menu` in `store.rs`: the `match action` must stay exhaustive. A draft target never shows Pull, so a draft `MenuAction::Pull` does nothing.
- Trap: `spawn_load` in `store.rs` drops the loaded info. Use `load` for the pre-pull read, because the dirty check needs the `BranchInfo`.

How:

- In `branch.rs`, add `MenuAction::Pull`, the `pull` row in `branch_menu`, the constants `PULL_DIRTY` and `PULL_NO_UPSTREAM`, and `pull_blocked`.
- In `store.rs`, add `begin_pull` and `end_pull` to `State`, backed by a `pulling` set of folders.
- In `run_menu`, handle `MenuAction::Pull`: `begin_pull`, then a `Force` `load`, then `pull_blocked`, then `git_action`, then a `Force` reload, then `end_pull` on every path.

Files:

- [packages/desktop-gpui/plugins/thread-branch/src/branch.rs](packages/desktop-gpui/plugins/thread-branch/src/branch.rs) (`MenuAction::Pull`, Pull row, `pull_blocked`, constants, tests)
- [packages/desktop-gpui/plugins/thread-branch/src/store.rs](packages/desktop-gpui/plugins/thread-branch/src/store.rs) (`State::begin_pull`, `State::end_pull`, the Pull press flow, test)

Expected result:

- WHEN the branch has an upstream THE branch pill menu SHALL show `Pull` after `Fetch` and before `Refresh`.
- IF the branch has no upstream THEN THE branch pill menu SHALL hide `Pull`.
- WHEN Hiep presses Pull on a clean folder THE store SHALL run the daemon pull and then read the folder again.
- IF the fresh read shows local changes THEN THE store SHALL show the dirty message and not pull.
- IF the daemon pull fails THEN THE store SHALL show its error text.
- WHILE a pull runs for a folder, WHEN Hiep presses Pull for that folder again THE store SHALL do nothing.
- THE New workspace pill menu SHALL not show `Pull`.

Red tests:

File: [packages/desktop-gpui/plugins/thread-branch/src/branch.rs](packages/desktop-gpui/plugins/thread-branch/src/branch.rs) (inside the existing `tests` module; add `pull_blocked`, `PULL_DIRTY`, and `PULL_NO_UPSTREAM` to its imports when it does not use `super::*`)

```rust
    #[test]
    fn branch_menu_offers_pull_after_fetch_when_the_branch_has_an_upstream() {
        assert!(!row_ids(&branch_menu(&base())).contains(&"pull"));
        let tracked = BranchInfo {
            upstream: Some("origin/main".into()),
            ..base()
        };
        let menu = branch_menu(&tracked);
        let ids = row_ids(&menu);
        let fetch = ids.iter().position(|id| *id == "fetch").unwrap();
        assert_eq!(ids[fetch + 1], "pull");
        assert_eq!(ids[fetch + 2], "refresh");
        let pull = menu
            .iter()
            .find_map(|entry| match entry {
                MenuEntry::Row(row) if row.id == "pull" => Some(row),
                _ => None,
            })
            .unwrap();
        assert_eq!(pull.title, "Pull");
        assert_eq!(pull.icon, bachuc_icons::IconName::ArrowDownToLine);
        assert_eq!(pull.action, MenuAction::Pull);
        assert!(!row_ids(&draft_menu()).contains(&"pull"));
    }

    #[test]
    fn pull_is_blocked_by_local_changes_or_a_missing_upstream() {
        let tracked = BranchInfo {
            upstream: Some("origin/main".into()),
            ..base()
        };
        assert_eq!(pull_blocked(&tracked), None);
        let dirty = BranchInfo {
            dirty: true,
            ..tracked.clone()
        };
        assert_eq!(pull_blocked(&dirty), Some(PULL_DIRTY));
        assert_eq!(pull_blocked(&base()), Some(PULL_NO_UPSTREAM));
        let both = BranchInfo {
            upstream: None,
            ..dirty
        };
        assert_eq!(pull_blocked(&both), Some(PULL_NO_UPSTREAM));
        assert_eq!(
            PULL_DIRTY,
            "Pull isn't available while you have local changes so commit or stash them first"
        );
        assert_eq!(PULL_NO_UPSTREAM, "No upstream branch to pull from.");
    }
```

File: [packages/desktop-gpui/plugins/thread-branch/src/store.rs](packages/desktop-gpui/plugins/thread-branch/src/store.rs) (inside the existing `tests` module)

```rust
    #[test]
    fn one_pull_at_a_time_per_folder() {
        let mut state = State::default();
        assert!(state.begin_pull("/repo"));
        assert!(!state.begin_pull("/repo"));
        assert!(state.begin_pull("/other"));
        state.end_pull("/repo");
        assert!(state.begin_pull("/repo"));
    }
```

Anti-goal: The existing menu rows keep their ids, order, and visibility rules, and only `pull` is added; tripwire; read from the row lists in the `branch.rs` tests before the change, after the `branch.rs` change, and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-thread-branch` -> all tests pass, including the 3 red tests.
- `git diff main -- packages/desktop-gpui/plugins/thread-branch/src/branch.rs` -> the only changed expected lists add `"pull"` between `"fetch"` and `"refresh"` (anti-goal).
- Read the Pull flow in `store.rs` -> `end_pull` runs on the blocked, failed, no-client, and success paths.
- Owner check on live GPUI: on a clean branch that is behind, open the branch pill menu and press Pull -> the pill's `↓` count goes to 0.
- Owner check on live GPUI: with an uncommitted change, press Pull -> the dirty message shows and nothing is pulled.
