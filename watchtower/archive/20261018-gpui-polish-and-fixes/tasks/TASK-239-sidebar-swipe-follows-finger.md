# TASK-239 Sidebar space swipe follows the finger

Group: W ([packages/desktop-gpui/crates/sidebar](packages/desktop-gpui/crates/sidebar), [packages/desktop-gpui/plugins/workspace-spaces](packages/desktop-gpui/plugins/workspace-spaces))
Class: risky

## Brief

Goal: A two-finger horizontal swipe on the sidebar moves the workspace list 1:1 with the fingers and snaps to the next space on release, with no delay and no missed swipe.

Change: threshold-triggered fixed animation that sometimes drops swipes -> list that follows the finger and snaps.

Boundaries:

- Owner chose follow-the-finger on 2026-10-18.
- Today: `Sidebar::on_wheel` in [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) about line 657; `WheelGesture::push` in [packages/desktop-gpui/crates/sidebar/src/plugin.rs](packages/desktop-gpui/crates/sidebar/src/plugin.rs) about line 181 with `QUIET_GAP_MS` 120, `MIN_TRAVEL` 28, `HORIZONTAL_RATIO` 1.5, a 160 ms lockout, and a 65 ms plus 105 ms slide. `swipe` in [packages/desktop-gpui/plugins/workspace-spaces/src/state.rs](packages/desktop-gpui/plugins/workspace-spaces/src/state.rs) about line 248 is ignored while `busy()` ([packages/desktop-gpui/plugins/workspace-spaces/src/state.rs](packages/desktop-gpui/plugins/workspace-spaces/src/state.rs) about line 129). `swipe_target` is in [packages/desktop-gpui/plugins/workspace-spaces/src/model.rs](packages/desktop-gpui/plugins/workspace-spaces/src/model.rs) about line 562.
- Snap rule: release past half the width, or a fast flick, goes to the next space; otherwise snap back. A swipe during a busy save is queued, not dropped.
- Use scroll phase (began, changed, ended) and momentum from GPUI scroll events if available; record what the vendored GPUI gives in the outcome. Planner call: if no phase events exist, end the gesture after a quiet gap with no new delta.
- Keep vertical scroll of the list working.

How:

- Rewrite the gesture as a pure state machine (offset, velocity, phase -> next state) with tests.
- Draw the current and next list side by side at the offset.
- Queue a swipe that arrives while busy.

Files:

- [packages/desktop-gpui/crates/sidebar/src/sidebar.rs](packages/desktop-gpui/crates/sidebar/src/sidebar.rs) (wheel handler, offset drawing)
- [packages/desktop-gpui/crates/sidebar/src/plugin.rs](packages/desktop-gpui/crates/sidebar/src/plugin.rs) (gesture state machine)
- [packages/desktop-gpui/plugins/workspace-spaces/src/state.rs](packages/desktop-gpui/plugins/workspace-spaces/src/state.rs) and [packages/desktop-gpui/plugins/workspace-spaces/src/model.rs](packages/desktop-gpui/plugins/workspace-spaces/src/model.rs) (queued swipe)

Expected result:

- The list moves with the fingers during the swipe and snaps on release.
- A swipe during a save runs after the save.

Anti-goal: sidebar CPU during a swipe stays under 15% with `measure.sh`; a per-frame list rebuild is the tripwire; read before and after.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar -p bachuc-plugin-workspace-spaces` -> all pass, including snap, snap-back, flick, vertical-ignore, busy-queue, two queued swipes, swipe past the first or last space (rubber-band and stay), and direction reversed mid-drag.
- Owner hand test: swipe slowly, then fast, ten times -> each swipe follows and switches, none dropped.
- `packages/desktop-gpui/measure.sh` during swipes -> under 15% CPU.
