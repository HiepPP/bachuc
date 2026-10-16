# TASK-144 Cut timeline and shell render cost on long turns

Group: J (writes `crates/timeline` and `crates/app`; one group because one CPU read covers both)
Class: risky

## Brief

Goal: Keep the Bachuc GPUI app at or below 10% CPU and 250 MB on a long agent turn, so the cost no longer grows with the number of steps.

Change: the app reads 9.7% CPU on a 2-minute mock turn and 13.4% on a 15-minute turn (Q-065) -> at most 10.0% at both ages, with the timeline sync and the shell render doing work in proportion to what changed.

Boundaries:

- Owner answers on 2026-10-16 (Q-065 interview): change `crates/timeline` and `crates/app` only; profile first, then make the work incremental; prove it on the app bundle at two turn ages.
- Keep the 10% CPU and 250 MB budget. Do not raise it.
- Do not slow the stream: keep the 50 ms client flush, and do not throttle redraws in the view (owner answer). See [docs/agent-stream-performance.md](docs/agent-stream-performance.md).
- Do not change `crates/client`, `crates/diagram`, `crates/composer`, `crates/terminal`, `crates/theme`, or `crates/protocol`. If the profile shows the main cost there, record it in the outcome and the questions, and fix only the timeline and shell part.
- Follows [ADR-0019](watchtower/decisions/ADR-0019-always-open-step-list-over-folding-group.md): every step stays visible. Do not fold, cap, or summarize old steps.
- Follows [ADR-0018](watchtower/decisions/ADR-0018-all-trailing-messages-over-last-message-as-answer.md), [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md), and [ADR-0021](watchtower/decisions/ADR-0021-note-opens-its-step-over-tools-before-the-note.md): the rows must look the same as before.
- Keep every role and label. Shell chrome views stay uncached, because a cached GPUI view drops out of the accessibility tree (TASK-141 outcome). Cut render work in a way that keeps the views in the AX tree.
- Keep the one-list-item-per-step rule from [TASK-138 outcome](watchtower/tasks/TASK-138-outcome.md): never put a whole turn in one item.
- Known costs to check first:
  - `TimelineView::sync` in [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (lines 189 to 233) regroups and hashes every entry on each flush, up to 20 times a second.
  - The turn header fingerprint (lines 319 to 335) hashes every entry of the turn, so the header is measured again on each flush.
  - `update_messages` (lines 367 to 417) walks every entry on each flush.
  - `reuse_entries` (line 1140) keys and rebuilds every entry on each flush.
  - `hash_entry` (line 292) hashes `seqEnd`, so any step body change also changes the header fingerprint.
- Planner call: put the incremental grouping in a pure, public type in [packages/desktop-gpui/crates/timeline/src/grouping.rs](packages/desktop-gpui/crates/timeline/src/grouping.rs), with an `update(entries, turn_active) -> Vec<Row>` method that keeps its own cache. `ViewRow`, `push_rows`, and `apply_rows` in `lib.rs` are private and need a GPUI context, so the test compares the pure type with `grouping::group_turns` (public at `grouping.rs:223`; `Row` derives `PartialEq`).
  - The shell renders about 13 times a second during a stream ([TASK-141 outcome](watchtower/tasks/TASK-141-outcome.md) Lessons); `Shell::render` is in [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) line 486.
- Planner call: profile with the macOS `sample` command (`sample <pid> 10 -file /tmp/<name>.txt`). It is a CLI and not UI automation.

How:

- Take a baseline: run the app bundle with a mock `thirty-minute-stream` tab and a terminal tab. At a turn age of about 15 minutes, read `measure.sh 60` and take a `sample` profile. Record the top frames in the outcome.
- Make the timeline sync incremental: regroup only the turns whose entries changed, hash only changed entries, and keep the turn header fingerprint from changing when only a step body changes.
- Cut the shell render work that the profile shows, without cached views.
- Add a test that proves incremental rows equal a full regroup.
- Read again at both turn ages, and profile once more at 15 minutes.

Files:

- [packages/desktop-gpui/crates/timeline/src](packages/desktop-gpui/crates/timeline/src) (sync, rows, header fingerprint)
- [packages/desktop-gpui/crates/timeline/tests](packages/desktop-gpui/crates/timeline/tests) (the incremental-rows test)
- [packages/desktop-gpui/crates/app/src](packages/desktop-gpui/crates/app/src) (shell render work)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (only if cargo updates it)

Interfaces:

- Consumes: `bachuc_timeline::TimelineView` (a GPUI view) with `TimelineView::new(client: gpui_kit::Entity<bachuc_client::Client>, agent_id: String, diagrams: gpui_kit::Entity<bachuc_diagram::DiagramRenderer>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<TimelineView>) -> TimelineView` from TASK-138
- Produces: the same `TimelineView::new` signature, unchanged, for `crates/app`

Expected result:

- On a mock stream with one terminal tab, the app reads at most 10.0% CPU and 250 MB at a turn age of about 2 minutes and about 15 minutes.
- The profile at 15 minutes no longer shows a full regroup or a full hash on every flush.
- Rows, labels, follow-at-bottom, scrolled-up hold, older-page loading, diagrams, and permission buttons work as before.
- Cases:
  - A new entry appended to the open turn changes only that step's row and the header's running state.
  - A new turn starts while an older turn is finished.
  - An older page is prepended and regroups the top turn.
  - A timeline replacement resets the state, and an entry removed in the middle drops its row and its message state.
  - A permission request appears and then resolves; an answered permission clears when the daemon resolves it.
  - The open turn finishes while the view is open: the rule under the last step shows, and the clock stops.
  - An assistant message whose text grows (`seqEnd` changes) shows the new text.
  - Agent status changes with no timeline version change (`AgentsChanged`), and the running state follows it.
  - A diff toggle and a `DiagramReady` event redraw only their row.

Anti-goal: the app stays within 10.0% CPU and 250 MB phys_footprint, WebKit helpers included, with one mock stream tab and one terminal tab, window in front, for 60 seconds at a turn age of about 2 minutes and about 15 minutes; tripwire; read with `measure.sh 60` before the change at about 15 minutes (baseline), after the timeline change, after the shell change, and at completion at both ages.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> all tests pass, including a new `incremental_rows_match_full_regroup` test. It feeds the grouping cases of Expected result (append to the open turn, new turn, older page, replacement, middle removal, turn finish, growing text, status flip with no version change) through the incremental type and compares each step with `grouping::group_turns`
- Agent check on a live mock stream: let the turn finish while the tab is open, then toggle one diff -> the rule under the last step shows, the clock stops, and only the toggled step changes
- `packages/desktop-gpui/cargo.sh test -p bachuc-app` -> all tests pass
- `packages/desktop-gpui/cargo.sh build --release -p bachuc-app` -> exit 0
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-timeline -p bachuc-app` -> exit 0
- Profile: `sample <app pid> 10` at a turn age of about 15 minutes, before and after the change -> the outcome lists the top frames of both; the after profile shows no full regroup or full entry hash per flush
- Agent check: bundle with `packages/desktop-gpui/bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop`, open a mock stream tab -> while at the bottom the view follows new steps; scrolled up it holds still; scrolling back down follows again
- Agent check: open a live agent with more than one page of history and scroll to the top -> the older page loads without a jump
- Agent check: `cua_repl` `getAXState` on the app -> sidebar rows, tabs, the host switcher, timeline steps, and the composer keep their roles and labels
- Anti-goal: `measure.sh 60 app="pids:<app pid>" wk="pids:<WebKit pids>"` at a turn age of about 2 minutes and about 15 minutes -> each at most 10.0 CPU and footprint sum at most 250
