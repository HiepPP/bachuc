# TASK-212 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/pull_request_view.rs` (new): `PullRequestView` and `PullRequestEvent`, with the toolbar, header, Checks section, Activity section, and the loading, error, and empty states. It holds the pure rules `pr_state`, `state_label`, `count_pills`, `visible_activity`, `activity_verb`, `format_age`, `timeline_target`, `classify_check`, `summarize_checks`, and `comment_location`, with 8 tests.
- `packages/desktop-gpui/crates/explorer/src/lib.rs`: `mod pull_request_view;` and `pub use pull_request_view::{PullRequestEvent, PullRequestView};`.

## Contract

- Produces: `PullRequestView::new(client: Entity<Client>, cwd: String, cx: &mut Context<PullRequestView>) -> PullRequestView`.
- Produces: `PullRequestEvent { OpenUrl { url: String } }`, through `impl EventEmitter<PullRequestEvent> for PullRequestView`. View, the header, a check row, and an item's "Open on GitHub" emit it.
- Each "Consumes:" line from TASK-204 is used by its exact name: `PrStatus`, `PrCheck`, `PrStatusReply`, `Client::pr_status`, `PrTimelineItem`, `Client::pr_timeline`, `GitStatusEvent::Changed`, and `GitAction::Refresh` through `Client::git_action`.
- Fetch: on creation, on `ClientEvent::Connected`, after Refresh, and after Retry. A `GitStatusEvent::Changed` for its cwd starts one 500 ms timer through the existing `git_actions::RefreshGate`; more events before it fires add nothing. A generation counter drops older answers. The view has no other timer.
- The timeline loads only when `github_features_enabled` is true and the status has a number, a repo owner, and a repo name (`shouldFetchTimelineFrom`). With no number, the header and checks show, and Activity says "No activity yet".
- Refresh shows only when `server_info.features.checkoutRefresh` is true. It sends `checkout.refresh.request`, shows a spinner while it runs, then fetches.
- States: the skeleton until the first status arrives; an error with Retry when the first status fails; "No pull request yet" when the reply has no status. A failed refetch keeps the PR on screen. A failed timeline load shows "Unable to load pull request activity" in the section.
- Activity drops comments with an empty body and `commented` reviews with an empty body, as `mapActivity` does. The pills count approvals, changes requested, and comments plus `commented` reviews. A pill with zero is hidden.
- Safe default: the client's `PrCheck` has no traits and no duration, so the checks use four kinds (failing, in progress, successful, skipped) and show no timing.
- Safe default: the checks ring is drawn as the outcome icon at 20 px, because GPUI has no segmented SVG arc here.
- Safe default: item bodies show as plain wrapped text, not Markdown. Items have no thread grouping, because `PrTimelineItem` has no thread or review IDs. An inline comment shows its `path:line` above the body.
- Safe default: a failed Refresh shows its message as a red line under the toolbar, because the GPUI client has no toast host yet.
- Out of scope, as the spec says: "Add to chat" and check log details. Items keep Copy and "Open on GitHub" in the kebab menu.

## Verified

- `./cargo.sh test -p bachuc-explorer pull_request` -> 8 passed. `pull_request_coalesces_status_events` gives 1 refetch for 10 events in 90 ms, and 2 for two bursts 600 ms apart.
- `./cargo.sh test -p bachuc-explorer` -> 36 + 38 + 21 passed.
- `./cargo.sh build -p bachuc-explorer` -> exit 0, no warnings.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `./cargo.sh clippy -p bachuc-explorer --tests` -> no warnings in `pull_request_view.rs`.
- `./cargo.sh metadata --format-version 1` -> exit 0.
- `rg "\.timer\(" crates/explorer/src/pull_request_view.rs` -> one timer, in `schedule_fetch`, which only a status event starts.
- UI check, a test repo with a PR shows the header, checks, and activity: DEFERRED (UI checker). TASK-202 opens the tab.
- Anti-goal, no PR status request in the live `daemon.log` over 60 idle seconds: DEFERRED (UI checker).

## Lessons

- `bachuc-app` was not rebuilt; the explorer API only gained exports.
- The item kebab closes on `on_mouse_down_out`, so a second press on the kebab closes the menu and then opens it again. `GitActionsButton`'s caret has the same behavior.
- `Load::Ready` boxes the `PrStatusReply`, because clippy flags `large_enum_variant`.

## UI Checks

Run 2026-10-18 by the UI checker (run 3). Bundle `Bachuc GPUI Check` built from HEAD `2d5734c11` (includes `e9a78e843`, `77a9730cb`, and `22906f2cb`), PID 31420, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The app stayed in the background. The checker forced each frame with the AX "zoom the window" action and read the AX tree after a no-op F15 key. Scratch repo `/tmp/t202-repo` (branches `main` and `feature-x`, script `hello`) was the live workspace `wks_4a3662191ab77c60` with disposable mock agents "t202 scratch", "t200 archive me", "t200 board card", and "t208 cpu stream". The workspace was archived after the run; PID 31420 quit and exited.

- A repo with a PR shows the header, checks, and activity: UNVERIFIED (unattended): no PR. No live workspace the checker may read has a PR: `calendar-ref` on `main` has none (`gh pr view`), and `HiepPP/bachuc` has no open PR. The scratch repo has no remote, so the PR tab did not auto-open.
- Anti-goal, no PR status request over 60 idle seconds: PASS. With the check app idle on the scratch workspace, the 4 metric windows from 18:27:38Z to 18:29:38Z (120 s) show no `checkout_pr_status_request`. The window before (branch switches) had 4, as the git status change asks for it.
