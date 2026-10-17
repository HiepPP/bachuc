# TASK-224 PR view matches Electron

Group: E (shared file [packages/desktop-gpui/crates/explorer/src/pull_request_view.rs](packages/desktop-gpui/crates/explorer/src/pull_request_view.rs) and the explorer crate)
Class: code

## Brief

Goal: The PR view shows Markdown bodies, a split checks ring, and check durations, as Electron does.

Change: plain-text bodies, one outcome icon for checks, no timing -> Markdown, ring, durations. Q-124.

Boundaries:

- Electron reference: [packages/app/src/git/pull-request-panel](packages/app/src/git/pull-request-panel): `checks-ring.tsx`, `checks-section.tsx`, `check-status.ts`, `check-group-state.ts`, and the activity files.
- Markdown: use the gpui-base text view the timeline uses for Markdown; do not add a Markdown crate.
- Ring: draw the segments with GPUI paths or canvas from a pure segment function with a test. Kinds follow `check-status.ts`, using `traits` when present.
- Durations: show `PrCheck.duration` as Electron does; `None` shows nothing.
- Threads and review grouping stay out: the client has no thread ids (Q-124).

Files:

- [packages/desktop-gpui/crates/explorer/src/pull_request_view.rs](packages/desktop-gpui/crates/explorer/src/pull_request_view.rs)

Interfaces:

- Consumes: `PrCheck { name: String, status: String, url: Option<String>, workflow: Option<String>, duration: Option<String>, traits: Vec<String> }` from TASK-220.

Expected result:

- A PR body with a list and code renders as Markdown.
- 3 passing, 1 failing, and 1 pending checks give a ring with 3 segments in Electron's colors and order.

Anti-goal: no request while idle, as TASK-212 measured; tripwire; 0 `checkout_pr_status_request` over 60 idle seconds in the live `daemon.log`.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer pull_request` -> passes, with the ring segment and duration cases.
- UI check on live: a repo with a real PR shows the ring and durations; if no PR is readable, record UNVERIFIED (unattended): no PR.
- Anti-goal: 0 `checkout_pr_status_request` in 60 idle seconds.
