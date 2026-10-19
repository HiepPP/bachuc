# TASK-274 outcome

Status: DONE (every active source measured; two plan-level findings need the lead)

Changed:

- `watchtower/tasks/TASK-274-outcome.md` (new). No file under `packages/` or `hiep-plugins/` changed.

Contract:

- `## Active Sources` and `## Other Sources` below, with columns Source, path:line, Frame rate, CPU share in points.
- Share rule: the drop in the median 60 s read when the source is hidden and the other sources stay as they are. Frames of different sources coalesce, so the shares depend on the order of the ladder and are not independent. The standalone reads are in `## Reads`.
- Frame rates come from the code. No frame counter exists, so no rate is a measured number. The display refresh rate was not read, so "every vsync" is UNVERIFIED as a number (the 2560x1664 Retina display is assumed to run at 60 Hz).

## Active Sources

Default layout: sidebar open, one agent tab, composer, no board page; the agent runs a foreground `sleep`.

BASE `74c6266ad` (the 42.6% of Q-175; median read 42.4%):

| Source                                                        | path:line                                                                                                                      | Frame rate                                                               | CPU share in points                                                               |
| ------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------ | --------------------------------------------------------------------------------- |
| Stop-orb face                                                 | `packages/desktop-gpui/plugins/stop-orb/src/face.rs:105`                                                                       | `request_animation_frame` on every render, so every vsync (about 60 fps) | 17.4 (42.4 -> 25.0 with `stop-orb` disabled); alone with the sidebar hidden: 37.7 |
| Sidebar `RunningRing` (one ring on the running workspace row) | `packages/desktop-gpui/crates/sidebar/src/ring.rs:31` (`FRAME_INTERVAL` 33 ms; the row at `crates/sidebar/src/sidebar.rs:797`) | one 33 ms timer, up to about 30 fps                                      | 22.1 (25.0 -> 2.9 with the sidebar hidden and `stop-orb` off)                     |
| Timeline clock and composer live metrics                      | `packages/desktop-gpui/crates/timeline/src/lib.rs:1782`; `crates/composer/src/composer.rs:866` (`LIVE_TICK_MS` 1000)           | 1 fps each                                                               | 1.6 (2.9 -> 1.3 with the timeline hidden; the two are not separable)              |
| Idle floor                                                    | none                                                                                                                           | none                                                                     | 1.3 (new tab, sidebar hidden, no agent tab)                                       |
| Sum                                                           |                                                                                                                                |                                                                          | 42.4 (BASE median 42.4; no gap, so no `unattributed` row)                         |

CHECK `083620d10` (this branch; median read 31.3%, range 31.1 to 35.6):

| Source                                                                                            | path:line                                                                                                                                                                                        | Frame rate                                                              | CPU share in points                                                                                         |
| ------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- |
| Stop-orb face                                                                                     | `packages/desktop-gpui/plugins/stop-orb/src/face.rs:105`                                                                                                                                         | every vsync (about 60 fps)                                              | 4.3 (31.3 -> 27.0 with `stop-orb` disabled)                                                                 |
| Sidebar `RunningRing`                                                                             | `packages/desktop-gpui/crates/sidebar/src/ring.rs:31`                                                                                                                                            | one 33 ms timer, up to about 30 fps                                     | 7.2 (27.0 -> 19.8 with the sidebar hidden); alone: 14.8 above the floor (16.1 read)                         |
| Tab-bar `StatusBadge` ring (branch only)                                                          | `packages/desktop-gpui/crates/app/src/shell.rs:1118` (builds `StatusBadge`; ring at `crates/sidebar/src/badge.rs:66` and `crates/sidebar/src/ring.rs:31`)                                        | one 33 ms timer, up to about 30 fps                                     | 9.4 (10.7 -> 1.3 with the agent tab closed); alone: 9.4 above the floor                                     |
| Timeline marks: running loader, pulse, shimmer, Background card, plus the 1 fps clock and metrics | `packages/desktop-gpui/crates/timeline/src/motion.rs:19` (`FRAME_INTERVAL` 66 ms, one shared ticker, quantized states); `crates/timeline/src/lib.rs:1782`; `crates/composer/src/composer.rs:866` | one ticker, at most about 15 fps, fewer where the state does not change | 9.1 (19.8 -> 10.7 with the timeline hidden through a new tab; Motion and the 1 fps ticks are not separable) |
| Idle floor                                                                                        | none                                                                                                                                                                                             | none                                                                    | 1.3                                                                                                         |
| Sum                                                                                               |                                                                                                                                                                                                  |                                                                         | 31.3 (CHECK median 31.3)                                                                                    |

Findings the shares show:

- A frame costs the same whichever source asks for it, because GPUI marks the whole window dirty (the TASK-266 lesson). Both samples show the draw is the whole main-thread cost (`Window::draw` 46% of BASE main-thread samples, 85% of CHECK). So every source that asks for a frame at 30 fps or more is expensive, and the orb and the 33 ms rings are the ones that do.
- BASE: two sources explain 39.5 of the 42.4 points (the orb and the sidebar ring).
- CHECK reads 11.1 points lower than BASE (31.3 against 42.4), even with the added tab ring and Motion ticker. Frames of the orb and the rings overlap, so the orb adds only 4.3 points on CHECK against 17.4 on BASE.
- The 10% cap is not reachable by converting only the orb. On CHECK, the ladder without the orb stays at 27.0; the rings (sidebar 7.2, tab 9.4) and the timeline marks (9.1) together are 25.7 of the 31.3.

## Other Sources

Sources that request frames only off the default layout, or only in a state the foreground-`sleep` turn did not reach. They stay out of TASK-275.

| Source                                   | path:line                                                                                                         | Frame rate                                                       | CPU share in points                                                                                                                       |
| ---------------------------------------- | ----------------------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------- |
| Board page: orb per paint and card pulse | `packages/desktop-gpui/plugins/board/src/orb_view.rs:119`; `plugins/board/src/card.rs:148`                        | every vsync while the board page is painted                      | 17.1 together (Board page read 33.2 against 16.1 for the sidebar-ring-only state, both on CHECK; the two are not separable)               |
| Explorer spinners                        | `packages/desktop-gpui/crates/explorer/src/pull_request_view.rs:760`; `crates/explorer/src/git_actions_ui.rs:271` | `with_animation(..).repeat()`, every vsync while a spinner shows | UNVERIFIED: the Explorer was not opened                                                                                                   |
| Watchtower-board spinner                 | `packages/desktop-gpui/plugins/watchtower-board/src/plan_tree_view.rs:218`                                        | every vsync while a task runs on that page                       | UNVERIFIED: the page was not opened                                                                                                       |
| Command center file spinner              | `packages/desktop-gpui/crates/command/src/center.rs:1060`                                                         | every vsync while a file search spins                            | UNVERIFIED: the command center was not opened                                                                                             |
| Stream fade tick and text reveal         | `packages/desktop-gpui/vendor/gpui-base/src/text/state.rs:46` (`STREAM_FADE_TICK` 33 ms) and `:1036`              | 33 ms and one frame per reveal, only while text streams          | UNVERIFIED: a foreground `sleep` turn streams no text. This one can be active in a real streaming turn; measure it with a streaming agent |
| Composer file-drop backdrop              | `packages/desktop-gpui/crates/composer/src/composer.rs:1444`                                                      | every vsync while the backdrop fades                             | not active: a file drag only                                                                                                              |
| Sidebar swipe slide                      | `packages/desktop-gpui/crates/sidebar/src/sidebar.rs:2816`                                                        | every vsync while a swipe slides                                 | not active: a swipe only                                                                                                                  |
| Next-prompt panel resize                 | `packages/desktop-gpui/plugins/next-prompt-actions/src/panel.rs:971`                                              | one frame when the width crosses the compact threshold           | not active: a resize only                                                                                                                 |

## Method

- Apps: BASE `74c6266ad` is `/tmp/wt-base-app/Bachuc GPUI Base.app` (md5 `324b683285f877047c9030a797b607b8`). CHECK is the bundle built from this branch (md5 `4591b4d879734818e17b51639ac761fa`, HEAD `a958027e3`, last code commit `083620d10`, with Hiep's uncommitted `gpui-base` input changes in the tree). Each ran as one `open -n` instance on live `127.0.0.1:6768`, default 1200x800 window.
- Agent `0ed08ee0` (Claude Haiku, `/tmp/wt-mock-274`), foreground `sleep`. Claude caps one foreground shell call at 10 minutes, so `sleep 900` ended at 10m15s. The turns used `sleep 580` repeated 3 times, then 2 times. This changes the step count in the timeline and nothing else. `stop-orb` was toggled with `npm run cli -- plugin disable|enable stop-orb`.
- Reads: `packages/desktop-gpui/measure.sh 60 <label>=pids:<pid>`, pid from `pgrep -f` on the bundle path. Wrapper `/tmp/t274-read.sh` runs `open <bundle>` every 3 s during each read. Without it the window lost the front within about 20 s and read 0.2% to 0.3%; with it the clock advanced and the reads were stable. Every read below was taken during a running turn, at the times shown.
- Screen states came from AX actions through `cua_repl` (Hide sidebar, New tab, Close tab, Board). No AppleScript, `osascript` or `screencapture`.
- Samples: `sample <pid> 10 -file /tmp/gpui-sample-base.txt` (BASE, 00:04:30) and `/tmp/gpui-sample-checkn.txt` (CHECK, 00:34:05), during the running turn with `stop-orb` on. All main-thread work is `gpui_macos::window::step` -> `Window::draw`. Top non-idle self frames, BASE: `_platform_memmove` 297, taffy `compute_flexbox_layout` 149, `_xzm_free_main` 110, `_xzm_xzone_malloc_tiny` 90, `<deduplicated_symbol>` 73. CHECK: `_platform_memmove` 405, `compute_flexbox_layout` 225, `iokit_user_client_trap` 167, `_xzm_free_main` 153, `<deduplicated_symbol>` 138. Shares of `Window::draw`: sidebar 29% (BASE) and 24% (CHECK); timeline 13% and 22%; composer 17% and 16%. The samples cannot name who asked for a frame. They show only the cost per frame, so the sources come from the A/B reads and from the code. The CHECK sample ran the app at 88% main-thread busy, above the 31% read, so sampling slows it. Use only its proportions.
- Hidden states: orb = `stop-orb` disabled; sidebar hidden = "Hide sidebar"; timeline hidden = a "New tab" selected; agent tab closed = "Close t274 mock". The sidebar state is saved in the app settings, so the apps showed the sidebar hidden at the next launch; it was shown again.

## Reads

Window 1200x800, agent `0ed08ee0`, live. All values are `measure.sh 60` CPU%, one core = 100%.

BASE `74c6266ad`, pid 59475, started from the `Bachuc GPUI Check.app` path (see Findings: that bundle held the BASE binary):

| State                                       | Reads (start time)                                | Median |
| ------------------------------------------- | ------------------------------------------------- | ------ |
| default: orb on, sidebar open               | 42.4 (00:01:15), 42.4 (00:02:17), 41.9 (00:03:19) | 42.4   |
| orb off                                     | 22.6 (00:05:23), 25.0 (00:10:38), 25.5 (00:11:40) | 25.0   |
| orb off, sidebar hidden                     | 3.4 (00:13:34), 2.9 (00:14:36), 2.9 (00:15:38)    | 2.9    |
| orb on, sidebar hidden                      | 36.8 (00:20:13), 37.7 (00:21:15), 38.0 (00:22:17) | 37.7   |
| floor: new tab, sidebar hidden, no timeline | 1.3 (00:23:37), 1.3 (00:24:39), 1.3 (00:25:41)    | 1.3    |

Left out of the BASE medians: 6.7, 0.3, 0.2 (23:57 to 23:59, window not in front, before the `open` loop); 9.3 and 2.5 (00:06:26, 00:07:28, the first turn ended at about 00:06:49); 29.0, 35.3, 37.2 (00:17:00 to 00:19:05, the plugin had just been enabled and a step ended at 00:18:27). Including them: orb on with the sidebar hidden has the median 37.0, nothing else changes.

CHECK `083620d10`, pid 78352 (second launch 00:30:20; `stop-orb` on in the first and last group):

| State                                                     | Reads (start time)                                                                  | Median |
| --------------------------------------------------------- | ----------------------------------------------------------------------------------- | ------ |
| default: orb on, sidebar open                             | 31.3 (00:30:59), 35.6 (00:32:01), 35.0 (00:33:03), 31.1 (00:54:53), 31.1 (00:55:55) | 31.3   |
| orb off                                                   | 27.0 (00:34:31), 26.8 (00:35:34), 27.5 (00:36:36)                                   | 27.0   |
| orb off, sidebar hidden                                   | 19.8 (00:38:50), 19.9 (00:39:52), 19.2 (00:40:54)                                   | 19.8   |
| orb off, sidebar hidden, timeline hidden (tab ring stays) | 10.8 (00:42:09), 10.6 (00:43:11), 10.6 (00:44:14)                                   | 10.6   |
| orb off, sidebar hidden, agent tab closed (floor)         | 1.3 (00:45:29), 1.3 (00:46:31), 1.3 (00:47:33)                                      | 1.3    |
| orb off, sidebar open, new tab only (sidebar ring alone)  | 16.2 (00:48:54), 16.0 (00:49:56)                                                    | 16.1   |
| orb off, sidebar open, Board page                         | 33.1 (00:51:07), 33.2 (00:52:09)                                                    | 33.2   |
| idle: turn done, `stop-orb` on, sidebar open              | 2.7 (00:58:23), 2.7 (00:59:25), 2.7 (01:00:27)                                      | 2.7    |

## Findings

- The `Bachuc GPUI Check.app` bundle held the BASE binary at the start of this run (md5 `324b6832…` equal to the BASE app). `bundle.sh` copies `target/release/bachuc-desktop`, and the BASE build in `/tmp/wt-base` shares the same cargo unit hashes and output path. Cargo said `Finished ... in 0.80s` and left the BASE binary in place. After `cargo clean --release -p` for the 7 changed packages, the rebuild (1m09s) gave a different binary that contains `crates/timeline/src/motion.rs`. The first reads of this run (42.4, 25.0, 2.9, 37.7, 1.3) therefore belong to BASE and are filed as BASE.
- TASK-266 Round 4 used the same bundle path. Its CHECK reads (44.2, 44.1, 43.9) may have been the BASE binary. That bundle was overwritten before its md5 was recorded, so this is UNVERIFIED. This run measures a real CHECK: median 31.3 against BASE 42.4, which is 11.1 points lower, so baseline + 2 points holds. The 10% cap still fails on both.
- The tab-bar ring (`crates/app/src/shell.rs:1118`) and the Motion ticker exist only on this branch. The tab ring alone costs 9.4 points; the sidebar ring costs 14.8 points when it is the only source.
- On BASE the tab shows no ring and the sidebar ring and the orb carry the whole cost. On CHECK four sources each cost 4 to 9 points.

Verified:

- `git status --short -- packages hiep-plugins` before and after: identical (the 5 modified and 2 untracked paths of Hiep's input work only).
- Restore: `stop-orb` `enabled` is `True` in `.dev/paseo-home/config.json`; no `Bachuc GPUI Check` or `Base` process runs; agent `0ed08ee0` is archived; the sidebar is shown again. The mock workspace `wt-mock-274` stays on live, like the earlier mock workspaces.
- Outcome check: `## Active Sources` lists stop-orb face with a measured share on both apps.
- Outcome check: each state has three reads, except the sidebar-ring-alone and Board states on CHECK (two reads each, both within 0.2 points).
- Outcome check: the shares plus the idle floor equal the BASE median (42.4) and the CHECK median (31.3), so there is no gap and no `unattributed` row.
- Release daemon on 6767 and live untouched: no stop, no restart, no reload.

Anti-goal: PASS. No source file changed, the drift gauge is identical, and `stop-orb` is enabled on live after the run.

Lessons:

- To tell whether a check bundle holds new code, compare its md5 with the BASE binary and `strings -a` for a name that exists only in the new code. `bundle.sh` plus a shared `target/release` can copy the BASE binary after a `git archive` build.
- A GPUI window drops out of drawing within seconds after another app takes the front. A loop of `open <bundle>` every 3 s during a read kept it drawing; check that the `Working` clock advances before you trust a read.
- A Claude agent stops a foreground shell call after 10 minutes. Use `sleep 580` steps for a long mock turn.
- A loop in `measure.sh` labels must be a shell identifier: `a-b` fails with `command not found`.
- Hiding a source shows its cost together with the frames it shares. For TASK-275, convert the orb, the 33 ms rings (sidebar and tab) and the timeline marks to one shared ticker together, because removing one leaves the others' frames at the same cost.
