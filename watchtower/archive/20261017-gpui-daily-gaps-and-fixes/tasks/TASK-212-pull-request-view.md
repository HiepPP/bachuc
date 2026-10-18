# TASK-212 Pull request view

Group: E (shared file [packages/desktop-gpui/crates/explorer/src/lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs) and the explorer crate)
Class: code

## Brief

Goal: A Pull request view that shows the PR header, checks, and activity, as Electron's PR panel does.

Change: no PR view -> a `PullRequestView` entity the app opens as a tab.

Boundaries:

- Electron: [packages/app/src/panels/pull-request-panel.tsx](packages/app/src/panels/pull-request-panel.tsx) and its `pane.tsx` (lines 480-618).
  - Toolbar: "View pull request" (opens the URL) and Refresh. Show Refresh only when `server_info.features.checkoutRefresh` is true; it sends `checkout.refresh.request`.
  - Header: title, `#number`, state icon and label (open, draft, merged, closed), and the repo.
  - Checks: a collapsible section.
  - Activity: a collapsible section with approved, changes-requested, and comment count pills, then the timeline of reviews and comments.
  - States: loading skeleton, error with retry, empty.
- Fetch the timeline only when the status has number, repo owner, and repo name.
- No polling. Refetch on `GitStatusEvent::Changed` for this cwd, on reconnect, and on Refresh. Coalesce a burst of events into one refetch per 500 ms; put that rule in a pure function with a test.
- `PrStatus.number` is optional; with no number, show the header and checks but fetch no timeline. Format `created_at_ms` as time ago.
- Out of scope: "add to chat" on items and check log details. Keep copy and open on items.
- Model the shape on `DiffView` ([packages/desktop-gpui/crates/explorer/src/diff_view.rs](packages/desktop-gpui/crates/explorer/src/diff_view.rs) line 61).

How:

- Add `pull_request_view.rs` with the view and pure functions for the state label and count pills, with tests.
- Export it from `lib.rs`.

Files:

- [packages/desktop-gpui/crates/explorer/src/pull_request_view.rs](packages/desktop-gpui/crates/explorer/src/pull_request_view.rs) (new)
- [packages/desktop-gpui/crates/explorer/src/lib.rs](packages/desktop-gpui/crates/explorer/src/lib.rs) (module line and export)

Interfaces:

- Consumes: `PrStatus { number: Option<u64>, url: String, title: String, state: String, is_merged: bool, is_draft: bool, mergeable: Option<String>, checks: Vec<PrCheck>, review_decision: Option<String>, repo_owner: String, repo_name: String, merge: PrMergeCapability }` and `PrCheck { name: String, status: String, url: Option<String>, workflow: Option<String> }` from TASK-204.
- Consumes: `PrStatusReply { status: Option<PrStatus>, github_features_enabled: bool }` and `Client::pr_status(&mut self, cwd: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<PrStatusReply>>` from TASK-204.
- Consumes: `PrTimelineItem` enum `{ Review { author: String, body: String, created_at_ms: i64, url: Option<String>, state: String }, Comment { author: String, body: String, created_at_ms: i64, url: Option<String>, path: Option<String>, line: Option<u32> } }` and `Client::pr_timeline(&mut self, cwd: &str, number: u64, repo_owner: &str, repo_name: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<Vec<PrTimelineItem>>>` from TASK-204.
- Consumes: `GitStatusEvent` enum `{ Changed { cwd: String }, ScriptsChanged { workspace_id: String } }` emitted by `Client`, from TASK-204.
- Consumes: `GitAction` enum `{ Commit, Pull, Push, PullAndPush, CreatePr, MergePr(MergeMethod), SetAutoMerge { enabled: bool, method: Option<MergeMethod> }, MergeBranch, MergeFromBase, Refresh }` from TASK-204, for Refresh.
- Produces: `PullRequestView::new(client: Entity<Client>, cwd: String, cx: &mut Context<PullRequestView>) -> PullRequestView` for TASK-202.
- Produces: `PullRequestEvent` enum `{ OpenUrl { url: String } }` for TASK-202.

Expected result:

- A workspace with an open PR -> the view shows title, number, state, checks, and activity.
- A merged PR shows "Merged". A draft shows "Draft".
- A failed load shows the error with Retry.

Anti-goal: the view must not poll; tripwire; no `checkout_pr_status_request` in the live `daemon.log` for 60 s while the view is open and idle, read at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-explorer pull_request` -> passes, including a coalesce test: 10 status events in 100 ms give one refetch.
- UI check on live (after TASK-202 opens the tab): a test repo with a PR -> header, checks, and activity show.
- Anti-goal: grep the live `daemon.log` over 60 idle seconds -> no PR status request.
