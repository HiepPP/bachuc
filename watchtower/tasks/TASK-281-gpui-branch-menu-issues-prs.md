# TASK-281 GPUI branch menu issues and pull requests rows

Group: A (standalone; writes the thread-branch GPUI plugin crate)
Class: code

## Brief

Goal: The agent branch pill menu in the GPUI client opens the GitHub issues page and the GitHub pull requests page of the `origin` repository.

Change: menu rows Copy, Open repo, divider, Fetch, Refresh -> Copy, Open repo, Issues on GitHub, Pull requests on GitHub, divider, Fetch, Refresh.

Boundaries:

- Titles are "Issues on GitHub" and "Pull requests on GitHub", without the repo name, because the row above already names the repo. Owner choice, 2026-10-21.
- Show the two rows only when `remote_url` contains `github.com`, the same test that picks the GitHub icon for the Open row. Hide them for GitLab and other hosts. Owner choice, 2026-10-21.
- `remote_url` is already a clean web URL such as `https://github.com/o/r`: the server strips `.git` and trailing slashes in [hiep-plugins/plugins/thread-branch/server/git.ts](hiep-plugins/plugins/thread-branch/server/git.ts) `parseRemoteUrl`. Append `/issues` and `/pulls` with no extra normalization.
- Do not change `draft_menu`. The New workspace pill has no remote info.
- Do not touch the server plugin, the React client, or any file under `packages/desktop-gpui/crates/app`; that folder holds unrelated uncommitted work.
- Trap: the existing `menu_rows_visibility` test in `branch.rs` expects `["copy-branch", "open-repo", "-", "fetch", "refresh"]` for a GitHub remote. Update only that full-menu expectation to `["copy-branch", "open-repo", "open-issues", "open-pulls", "-", "fetch", "refresh"]`. This change is Intended by this TASK.
- Planner call: keep the `contains("github.com")` host test as is, to match the Open row icon.
- Trap: `run_menu` in `store.rs` matches `MenuAction` exhaustively. A new variant without an arm fails to compile.

How:

- In `branch.rs`, add `MenuAction::OpenIssues` and `MenuAction::OpenPulls`.
- In `branch_menu`, add rows `open-issues` (icon `IconName::CircleDot`) and `open-pulls` (icon `IconName::GitPullRequest`) right after `open-repo`, visible when `github` is true.
- Add `pub(crate) fn menu_url(remote: &str, action: MenuAction) -> String` in `branch.rs`: `OpenIssues` appends `/issues`, `OpenPulls` appends `/pulls`, every other action returns `remote` unchanged.
- In `store.rs` `run_menu`, merge the three open actions into one arm: read `info.remote_url`, call `cx.open_url(&menu_url(&url, action))`, or show `NO_REMOTE`.
- Update the `branch_menu` doc comment to list the new rows.

Files:

- [packages/desktop-gpui/plugins/thread-branch/src/branch.rs](packages/desktop-gpui/plugins/thread-branch/src/branch.rs) (two `MenuAction` variants, two rows, `menu_url`, tests)
- [packages/desktop-gpui/plugins/thread-branch/src/store.rs](packages/desktop-gpui/plugins/thread-branch/src/store.rs) (open arm in `run_menu`)

Expected result:

- WHEN the origin remote is on github.com THE branch menu SHALL show "Issues on GitHub" and "Pull requests on GitHub" right below the "Open <repo> on GitHub" row.
- WHEN the user presses "Issues on GitHub" THE plugin SHALL open `<remote_url>/issues`.
- WHEN the user presses "Pull requests on GitHub" THE plugin SHALL open `<remote_url>/pulls`.
- IF the remote is not on github.com, or there is no remote, THEN THE branch menu SHALL not show the two rows.

Red tests (add to the `tests` module in `branch.rs`; import `menu_url`):

File: [packages/desktop-gpui/plugins/thread-branch/src/branch.rs](packages/desktop-gpui/plugins/thread-branch/src/branch.rs)

```rust
#[test]
fn branch_menu_offers_github_issues_and_pull_requests_below_the_open_row() {
    let github = BranchInfo {
        remote_url: Some("https://github.com/o/r".into()),
        upstream: Some("origin/feat/board-avatars".into()),
        ..base()
    };
    let menu = branch_menu(&github);
    assert_eq!(
        row_ids(&menu),
        ["copy-branch", "open-repo", "open-issues", "open-pulls", "-", "fetch", "refresh"]
    );
    let MenuEntry::Row(issues) = &menu[2] else {
        panic!("expected the issues row third: {menu:?}");
    };
    assert_eq!(issues.title, "Issues on GitHub");
    assert_eq!(issues.action, MenuAction::OpenIssues);
    let MenuEntry::Row(pulls) = &menu[3] else {
        panic!("expected the pull requests row fourth: {menu:?}");
    };
    assert_eq!(pulls.title, "Pull requests on GitHub");
    assert_eq!(pulls.action, MenuAction::OpenPulls);
}

#[test]
fn branch_menu_hides_issues_and_pull_requests_off_github() {
    for remote_url in [None, Some("https://gitlab.com/o/r".to_string())] {
        let ids = row_ids(&branch_menu(&BranchInfo { remote_url, ..base() })).join(",");
        assert!(!ids.contains("open-issues") && !ids.contains("open-pulls"), "{ids}");
    }
}

#[test]
fn menu_url_appends_the_github_page() {
    let url = "https://github.com/o/r";
    assert_eq!(menu_url(url, MenuAction::OpenIssues), "https://github.com/o/r/issues");
    assert_eq!(menu_url(url, MenuAction::OpenPulls), "https://github.com/o/r/pulls");
    assert_eq!(menu_url(url, MenuAction::OpenRepo), url);
}
```

Anti-goal: existing menu rows keep their order, titles, and visibility; read from the crate tests after the change; tripwire: any existing test fails, or an existing expected value changes other than the one full-menu line of `menu_rows_visibility`.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-thread-branch` -> the three new tests and all existing tests pass.
- `git diff packages/desktop-gpui/plugins/thread-branch/src/branch.rs` -> the only changed existing expected value is the full-menu line of `menu_rows_visibility`.
- Add an assert to new test 1: a GitHub remote with no upstream -> `row_ids` is `["copy-branch", "open-repo", "open-issues", "open-pulls", "-", "refresh"]`.
- `git diff --stat` -> only the two files in Files changed under `packages/desktop-gpui/plugins/thread-branch`; `packages/desktop-gpui/crates/app` diff is the same as before the TASK.
- Manual, needs Hiep and a GPUI build: open the branch pill menu of an agent in a GitHub repo -> the two rows show below the Open row, and each opens the issues or pull requests page in the browser.
- Cleanup: `cd packages/desktop-gpui && ./cargo.sh clean --profile dev` when no other cargo or rustc process runs.
