# TASK-268 Running ring on GPUI composer tracks

Group: C (the `bachuc-composer` crate and the `bachuc-sidebar` export; shares files with TASK-272 and TASK-273, none with Group B)
Class: code

## Brief

Goal: The "N working" mark on the GPUI composer tracks turns like the Electron `StatusRing`, as the sidebar ring already does.

Change: a static ring in `track_mark` -> the sidebar `RunningRing` view.

Design: `ComposerTrackMark` in [packages/app/src/composer/tracks.tsx](packages/app/src/composer/tracks.tsx) (line 310) draws `StatusRing`. GPUI already ports it as `RunningRing` in [packages/desktop-gpui/crates/sidebar/src/ring.rs](packages/desktop-gpui/crates/sidebar/src/ring.rs).

Boundaries:

- `track_mark` (`packages/desktop-gpui/crates/composer/src/tracks.rs:77`) draws the `Running` bucket as a still ring with a dot (lines 87-97). Use `RunningRing` with `Knockout::None` instead. Keep the other buckets as they are.
- `ring` is a private module of `bachuc-sidebar` (`packages/desktop-gpui/crates/sidebar/src/lib.rs:14`). Export `RunningRing`, `Knockout`, and `FRAME` from it. `bachuc-composer` already depends on `bachuc-sidebar`.
- `track_mark` runs inside the `RenderOnce` pill trigger, which cannot keep an entity. The `Composer` view (an entity, `composer.rs:3066`) keeps one `Entity<RunningRing>` per track pill, cached by key, as `Sidebar::ring` does (`packages/desktop-gpui/crates/sidebar/src/sidebar.rs:794`). `track_strip` takes `&self` (`composer.rs:2148`), so give it `&mut self` or keep the cache in a `RefCell`.
- `pill_segments` and `task_pill` are pure and have no `cx`. Add `ring: Option<Entity<RunningRing>>` to `TrackSegment`, set to `None` where segments are built, and attach the cached ring afterwards in `Composer`. Tests keep `None` and need no GPUI context. `Entity<T>` has `Debug`, `PartialEq`, and `Eq` (gpui-pre 0.3.9 `entity_map.rs:542-565`), so the derives stay.
- There are 4 `TrackSegment` literals: `subagents_track.rs:101`, `:114`, `:367`, and `task_list.rs:49`. Run `rg "TrackSegment \{"` before you change the struct, and update every literal.
- The ring box must keep the size of the old ring, so the pill text does not move. The old ring is `RING` (12 px) in `tracks.rs:25`. `RunningRing` draws in a 14 px `FRAME` box; Electron gives the ring a margin of minus the 1 px halo (`ringMark`, `packages/app/src/composer/tracks.tsx:422`). Do the same, and remove the `RING` constants that become unused.
- The task list mark `TaskMark::Running` (`task_list.rs:69`) stays a still `CircleDot`. Electron draws it still too (`packages/app/src/components/task-list-row.tsx`).

How:

- Export `RunningRing`, `Knockout`, and `FRAME` from `bachuc-sidebar`.
- Cache the ring entities in the view that builds the track strip, and pass each one into its running segment.
- Draw the cached ring in `track_mark` for the `Running` bucket.
- Update every `TrackSegment` literal.

Files:

- [packages/desktop-gpui/crates/sidebar/src/lib.rs](packages/desktop-gpui/crates/sidebar/src/lib.rs) (export the ring)
- [packages/desktop-gpui/crates/composer/src/tracks.rs](packages/desktop-gpui/crates/composer/src/tracks.rs) (`track_mark` and `TrackSegment`)
- [packages/desktop-gpui/crates/composer/src/subagents_track.rs](packages/desktop-gpui/crates/composer/src/subagents_track.rs) (running segment gets its ring; test literals)
- [packages/desktop-gpui/crates/composer/src/task_list.rs](packages/desktop-gpui/crates/composer/src/task_list.rs) (segment literal only)
- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (ring cache in the view that builds the strip, near `track_strip` at line 2148)

Expected result:

- While a subagent runs, the mark before "N working" shows a quarter arc that turns once every 900 ms, in step with the sidebar rings.
- When no subagent runs, the mark is the still dot of its bucket, as today.
- The pill text does not move when the mark changes from the old ring to `RunningRing`.

Anti-goal: GPUI app CPU while one subagent runs and the composer shows its track stays at or below the baseline plus 1 point over 60 s; drift gauge; read with `packages/desktop-gpui/measure.sh 60 gpui=<check bundle binary>` before changes and at completion. `RunningRing` sleeps when it is not painted, so the extra cost should be one small view.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer -p bachuc-sidebar` -> every test passes, including the updated `subagents_track` tests.
- `cd packages/desktop-gpui && ./cargo.sh build` -> the workspace builds, so no `TrackSegment` literal broke.
- Read check: `track_mark` draws the cached `RunningRing` only for `StatusBucket::Running`, and one ring entity is kept per pill key.
- UI checker on live (screen check): ask a live agent to start one subagent that runs `sleep 60`. Raise the GPUI window and take two screenshots about 200 ms apart. Expect the arc of the "1 working" ring at a different angle in each shot.
- UI checker on live (screen check): measure the x of the "1 working" text in both builds -> the same x within 1 px.
- Anti-goal: `measure.sh 60` before and after, with the same running subagent -> after is at most baseline + 1 point; record both values.
