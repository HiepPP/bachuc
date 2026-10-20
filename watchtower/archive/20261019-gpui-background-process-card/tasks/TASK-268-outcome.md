# TASK-268 Outcome — Running ring on GPUI composer tracks

Status: DONE (code checks; screen and CPU checks deferred)

Changed:

- `packages/desktop-gpui/crates/sidebar/src/lib.rs`: re-exports `FRAME`, `Knockout`, and `RunningRing`.
- `packages/desktop-gpui/crates/composer/src/tracks.rs`: `TrackSegment` gains `ring: Option<Entity<RunningRing>>`; `track_mark` draws the cached ring for `StatusBucket::Running` inside a 12 px box (the 14 px frame offset by -1 px), and no longer draws the still ring. `RING_STROKE` and `RING_TRACK_OPACITY` are gone; `RING` is now `FRAME - 2.`.
- `packages/desktop-gpui/crates/composer/src/subagents_track.rs`: `subagents_pill` takes the composer's ring and attaches it to the `Running` segment; all `TrackSegment` literals set `ring: None`.
- `packages/desktop-gpui/crates/composer/src/task_list.rs`: segment literal only.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: `Composer.rings` cache; `ring(key, knockout, cx)`; `track_strip` is `&mut self` and keeps one ring under the pill key `subagents-track-header`, dropping it when no subagent runs.

Contract:

- `RunningRing::new(knockout, cx)` is created with `Knockout::None`; the running segment receives it before `track_pill` renders.
- The pill text does not move: the wrapper keeps the old 12 px `RING` size, and the 14 px ring view is absolutely offset by `-HALO` (-1 px).
- A `Running` segment without a cached ring falls back to the running dot; the composer always attaches the ring, so this path is unreachable in the app.

Verified:

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer -p bachuc-sidebar` -> PASS, as part of the group-level run `./cargo.sh test -p bachuc-sidebar -p bachuc-composer -p bachuc-workspace -p bachuc-app` (exit 0: composer 124 lib tests + 4 `text_replacement` tests, sidebar 14 tests). The group-level run was shared with TASK-273.
- `cd packages/desktop-gpui && ./cargo.sh build` -> PASS (group-level, exit 0).
- Read check `track_mark` draws `RunningRing` only for `StatusBucket::Running` -> PASS (`tracks.rs`: the `Running` match arm takes `Some(ring)`; the other four arms draw dots); one ring entity is kept per pill key -> PASS (`composer.rs` `rings: HashMap<String, Entity<RunningRing>>`, key `subagents-track-header`).
- UI checker on live: arc at two angles 200 ms apart -> DEFERRED (UI checker).
- UI checker on live: "1 working" text x within 1 px -> DEFERRED (UI checker).

Anti-goal (GPUI CPU stays at baseline plus 1 point over 60 s; `measure.sh 60` before and after): DEFERRED (UI checker). No CPU read was taken, and no earlier value exists.

Lessons:

- `Entity<RunningRing>` in `TrackSegment` compiles with the existing `Debug`/`PartialEq`/`Eq` derives (gpui-pre 0.3.9 `entity_map.rs`).
- A negative margin must not be used to trim the ring halo: the 14 px view sits in an absolute child of a 12 px `relative()` box instead.
- The composer creates the ring inside `track_strip`, which runs from `render`; this follows the sidebar's `Sidebar::ring` pattern and is safe.

## UI Checks

Run 2026-10-19 on live (6768) with the check bundle `packages/desktop-gpui/target/bundles/Bachuc GPUI Check.app`, launched with `BACHUC_NO_ACTIVATE=1` and driven by `cua_repl` AX actions only. A Claude agent (`dd68522`) ran one Agent-tool subagent that ran `sleep 150`. Both apps opened were quit; the zoom was restored to 1.25; every test agent was archived.

- UI checker on live: arc at two angles about 200 ms apart -> PASS. Two screenshots of the composer pill "1 working": the ring region (image 868-902 x 1168-1200) changed (mean diff 2.87) and the enlarged crop shows the arc on the right in shot 1 and on the left in shot 2.
- UI checker on live: "1 working" text x -> UNVERIFIED (unattended). No pre-change build exists to compare. The text started at 914 image px (457 logical px at 2x) in both shots and did not move; the pill text region diff was 0.22 with the ring outside the crop.
- Plan Verify (NEXT.md): "the row and the agent tab show the turning ring while it runs" -> PASS for the ring part. The row and tab ring arcs change between two shots (row diff 3.15, tab diff 3.18). The row-click part of that line is TASK-272 and is not part of this run.
- Anti-goal: UNVERIFIED (unattended). No 60 s read was taken in the single-subagent state and no pre-change baseline exists. A related read with two running agent tabs and the subagent row shown was 35.5% (TASK-273), far above any likely baseline plus 1.
