# TASK-224 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/explorer/src/pull_request_view.rs`: item bodies render as Markdown; the checks header draws a split ring; check rows show their duration; checks use all 7 `CheckPresentation` kinds with traits. New pure rules `is_pending_status`, `check_timing`, `ring_segments`, `ring_color`; `classify_check` now takes a `PrCheck`. 3 new tests.

## Contract

- Consumes: `PrCheck { name, status, url, workflow, duration: Option<String>, traits: Vec<String> }` from TASK-220, by those names.
- Kinds and order follow `check-presentation.ts` and `checks-summary.ts`: needs action, warning, failing, in progress, manual, successful, skipped. A trait `action_required`, `warning`, or `manual` wins over the status; an unknown trait is ignored.
- Outcome follows `selectOutcome`: needs action, then failure or warning, then any status `mapCheckStatus` reads as pending, then success. The new headline is "Some checks need your attention".
- Ring: `ring_segments` is `buildSegments` of `checks-ring.tsx` (20-unit box, stroke 3, gap 2, minimum 1.5, one group owns the whole circle). `checks_ring` paints the `surface3` track as a bordered quad and each arc with `PathBuilder::stroke`, clockwise from twelve, at `ICON_LG`. Colors: warning for needs action, warning, and pending; danger for failing; muted for manual; success for successful; extra muted for skipped. With no checks only the track shows, as Electron does.
- Duration: `check_timing` is `formatCheckTiming`: `None` or empty shows nothing; a pending status shows "running {duration}". The row shows it at the trailing edge in `FONT_SM` muted.
- Markdown: `gpui_kit::component::text::TextView::markdown` with the theme's installed style, as the timeline's plan card uses it, inside a div with an explicit line height. A link click emits `PullRequestEvent::OpenUrl`. No new crate dependency; `Cargo.lock` is unchanged.
- Safe default: the manual and needs-action check icons draw `IconName::Settings`, because the icon set has no `ManualStatusIcon` gear and `crates/icons` is outside this TASK's files.
- Unchanged: `PullRequestEvent`, `PullRequestNoticeEvent`, the first-load error state with Retry, the fetch triggers, and the single timer in `schedule_fetch`. Threads and review grouping stay out (Q-124).

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer pull_request` -> 11 passed, with `pull_request_ring_segments_follow_the_groups` (3 passing, 1 failing, 1 pending -> 3 segments, failing, pending, successful, in danger, warning, success), `pull_request_check_duration`, and `pull_request_checks_traits_win_over_the_status`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-explorer` -> 54 + 38 + 21 passed.
- `./cargo.sh fmt --check -p bachuc-explorer` -> exit 0.
- `./cargo.sh clippy -p bachuc-explorer --tests` -> no warnings in `pull_request_view.rs`.
- `./cargo.sh metadata --format-version 1` -> exit 0; `Cargo.lock` unchanged.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds; its one warning (`next_theme` never used, `crates/app/src/state.rs`) is not from this TASK.
- `rg -c "\.timer\(" crates/explorer/src/pull_request_view.rs` -> 1, the existing `schedule_fetch` timer.
- GitNexus impact for `summarize_checks` and `classify_check` -> UNKNOWN (stale index, target not found). `rg` shows every changed symbol is private to `pull_request_view.rs`.
- UI check, a repo with a real PR shows the ring, durations, and a Markdown body with a list and code: DEFERRED (UI checker).
- Anti-goal, 0 `checkout_pr_status_request` over 60 idle seconds in the live `daemon.log`: DEFERRED (UI checker). The fetch code did not change.

## Lessons

- The timeline's own `markdown_style` lives in `bachuc-timeline` and `bachuc-plugin-host`. The explorer cannot depend on the timeline (the timeline depends on the explorer), so the PR view uses the theme's installed `TextView` style, as `card_view.rs` does.
- In a `canvas` closure that takes a `size: f32` parameter, call `gpui_kit::size(..)` by path; the glob `size` is shadowed.

## UI Checks

Run 2026-10-18 by the UI checker (run 5). Bundle `Bachuc GPUI Check` built from HEAD `08064a6f3` (includes `626143dc7` and TASK-225 `699395706`), PID 34022, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). Frames were forced with the AX "zoom the window" action; keys went to the check app only. Scratch repo `/tmp/t202-repo` (branches `feature-x` and `main`, new commit `1f8a4a9` with a `web` service script) was the live workspace `wks_9cd2d5d43fcc7b08` with the disposable mock agent "r5 fork src" (`d8ef867`). Request counts come from the live `daemon.log` `ws_runtime_metrics` lines (30 s windows, all clients), parsed by `/tmp/r5-metrics.py`. After the run, PID 34022 exited, the agent and the workspace were archived, and the scratch terminal was killed.

- A repo with a real PR shows the ring, durations, and Markdown bodies: UNVERIFIED (unattended): no PR. No live workspace has a PR: only `calendar-ref` has a remote, on `main`, and `gh pr list --head main` returns none. The checker did not add a real repo to live.
- Anti-goal, 0 `checkout_pr_status_request` over 60 idle seconds: PASS. With the scratch workspace open and idle, the windows 04:00:38, 04:01:08, 04:01:38, 04:02:08, 04:02:38, 04:03:08, and 04:03:38 (3.5 min, all clients) had 0. Note: the workspace has no PR, so no Pull request tab was open.
