# TASK-274 Find the GPUI repaint sources during a running turn

Group: D (standalone; writes only its outcome sidecar)
Class: risky

## Brief

Goal: Name each source that makes the GPUI app redraw while a turn runs, with its share of the 42.6% CPU on main (Q-175), so TASK-275 converts the right sources.

Change: one CPU number per app -> a table of repaint sources, each with a measured CPU share.

Design: [ADR-0052](watchtower/decisions/ADR-0052-one-shared-66ms-frame-ticker-over-per-mark-timers.md) and [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md).

Boundaries:

- Read-only for source code. Change no file under `packages/`. Write results only to the TASK-274 outcome sidecar.
- Candidates found by the planner, each to be confirmed or cleared:
  - stop-orb face: `window.request_animation_frame()` on every render while not cancelling, [packages/desktop-gpui/plugins/stop-orb/src/face.rs](packages/desktop-gpui/plugins/stop-orb/src/face.rs) line 105. The plugin is enabled on live and release.
  - sidebar `RunningRing`: one 33 ms timer per ring, [packages/desktop-gpui/crates/sidebar/src/ring.rs](packages/desktop-gpui/crates/sidebar/src/ring.rs) line 31.
  - board orb: `request_animation_frame` per paint, [packages/desktop-gpui/plugins/board/src/orb_view.rs](packages/desktop-gpui/plugins/board/src/orb_view.rs) line 119.
  - `with_animation(..).repeat()`: board card pulse ([packages/desktop-gpui/plugins/board/src/card.rs](packages/desktop-gpui/plugins/board/src/card.rs) line 148), explorer spinners ([packages/desktop-gpui/crates/explorer/src/pull_request_view.rs](packages/desktop-gpui/crates/explorer/src/pull_request_view.rs) line 760, [packages/desktop-gpui/crates/explorer/src/git_actions_ui.rs](packages/desktop-gpui/crates/explorer/src/git_actions_ui.rs) line 271), watchtower-board ([packages/desktop-gpui/plugins/watchtower-board/src/plan_tree_view.rs](packages/desktop-gpui/plugins/watchtower-board/src/plan_tree_view.rs) line 218), and the command center ([packages/desktop-gpui/crates/command/src/center.rs](packages/desktop-gpui/crates/command/src/center.rs) line 1060).
  - 1 s ticks: the timeline clock ([packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) near line 1782) and the composer live metrics (`LIVE_TICK_MS` = 1000).
  - Vendored gpui-base frames: `request_animation_frame` in the text and scrollbar code, for example the stream fade tick (`STREAM_FADE_TICK`, 33 ms) and `packages/desktop-gpui/vendor/gpui-base/src/text/state.rs` line 1036. Streamed text may request frames during a turn.
  - On this branch only: the timeline `Motion` ticker at 66 ms ([packages/desktop-gpui/crates/timeline/src/motion.rs](packages/desktop-gpui/crates/timeline/src/motion.rs)).
- Measure on live (6768) with the release plugin set, as in Round 4 of the [TASK-266 outcome](watchtower/tasks/TASK-266-outcome.md): Hiep approved front-window runs while away. Launch each app with `open -n <bundle>`, not from the agent shell (a `nice` app never draws). Use the default 1200x800 window, one agent running a foreground `sleep 900`, and `packages/desktop-gpui/measure.sh 60 <label>=<pgrep pattern | pids:PID>`. The script needs a label; give each app its own pattern or `pids:`, because both apps run the same binary name.
- Attribute with two methods:
  - A call-tree sample: `sample <pid> 10 -file /tmp/gpui-sample-<label>.txt` during the running turn. Name the top render and paint frames.
  - A/B reads: with `npm run cli -- plugin disable stop-orb` on live, then enable it again. For each other source, use the screen state that hides it (for example the sidebar collapsed, or the board page closed), not a code change.
- Measure both apps: BASE `74c6266ad` (rebuild per the MEMORY lesson with `git archive` that includes `hiep-plugins`, or reuse `/tmp/wt-base-app/Bachuc GPUI Base.app` if it still exists) and the check bundle of this branch.
- Planner call: if the active shares plus the idle read explain less than the BASE median minus 5 points, list the gap as `unattributed` with the top 5 frames of the sample. Do not stop; TASK-275 still runs on the named sources.
- Planner call: a source counts as active when it requests frames during the running turn with the default layout (sidebar open, one agent tab, composer, no board page). Sources that run only on other pages go in a second list, not in TASK-275.
- Restore at the end: re-enable `stop-orb` on live, quit every app this TASK launched, and archive the mock agent.

How:

- Build or locate the BASE app and the check bundle.
- Start one live agent with a foreground `sleep 900`.
- For each app: three `measure.sh 60` reads and one `sample` during the running turn.
- Repeat the reads with `stop-orb` disabled, and with each other candidate hidden.
- Write the attribution table in the outcome sidecar.

Files:

- [watchtower/tasks/TASK-274-outcome.md](watchtower/tasks/TASK-274-outcome.md) (new; the only write)

Interfaces:

- Produces: `TASK-274 outcome ## Active Sources`, a table with columns Source, path:line, Frame rate, CPU share in points; and `## Other Sources` with the same columns for sources off the default layout.

Expected result:

- The outcome names every source that requests frames during a running turn on the default layout, with a CPU share. The shares plus the idle read explain the 42.6% within 5 points.

Anti-goal: no source file changes; drift gauge `git status --short -- packages hiep-plugins` before and after -> the same output. Also the live plugin state after the run equals the state before it: `stop-orb` enabled.

## Verify

- `git status --short -- packages hiep-plugins` before and after the TASK -> identical output.
- Outcome check: the `## Active Sources` table lists stop-orb face with a measured share, or a measured reason to clear it.
- Outcome check: each read records app, SHA, window size, agent, and the `measure.sh 60` value; three reads per app and state.
- Outcome check: the sum of the active shares plus the idle read is within 5 points of the BASE median, or the gap is listed as `unattributed` with the top 5 sample frames.
- Restore check: `python3 -c "import json;print(json.load(open('.dev/paseo-home/config.json'))['plugins']['stop-orb']['enabled'])"` -> `True`; no app this TASK launched still runs.
