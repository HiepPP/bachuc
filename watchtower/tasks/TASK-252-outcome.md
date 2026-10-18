# TASK-252 Outcome

## Status

DONE (code checks pass; every `UI checker:` line and the anti-goal read are DEFERRED).

## Changed

- `packages/desktop-gpui/crates/client/src/model.rs`, `core.rs`, `lib.rs`: `PromptEntry`, `PromptIndex`, `prompt_index_supported` (gate `features.agentTimelinePromptIndex`), and `list_prompts` (`agent.timeline.list_prompts.request` / `.response`).
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: `list_prompts_is_gated_and_parses_the_index`.
- `packages/desktop-gpui/crates/timeline/src/outline.rs` (new): pure rules, `HoverIntent`, and the `OutlineRail` view.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: the rail as a child view, prompt index fetch, reading position report, and the jump with `load_older`.
- `packages/desktop-gpui/crates/timeline/tests/outline.rs` (new): 14 tests.
- `packages/desktop-gpui/crates/settings/src/appearance.rs`, `pane_appearance.rs`: `chat_outline` (key `chatOutline`, default on), a "Chat" section with a "Chat outline" switch in Appearance (no chat pane exists in GPUI).
- `packages/desktop-gpui/crates/theme/src/lib.rs`: `set_chat_outline` and `chat_outline` in the theme global, because the timeline cannot depend on settings.

## Contract

- `bachuc_timeline::outline::prompt_tick_magnification(slot_distance: f32) -> f32` (raised cosine, radius 3).
- `bachuc_timeline::outline::resolve_active_prompt_seq(prompts: &[u64], anchor_seq: Option<u64>) -> Option<u64>`.
- Other pure rules: `pill_size`, `rail_visible`, `accept_prompt_index_epoch`, `reading_anchor`, `plan_prompt_jump`, `HoverIntent`.
- The rail is its own `Entity<OutlineRail>`. A scroll or a hover notifies the rail only, and only when the lit prompt or the hover changes. The timeline never notifies for it.
- The prompt index loads from the daemon RPC. It reloads when the timeline epoch or the latest user prompt seq changes. With the gate off or the setting off, there is no rail.
- A jump: loaded -> `scroll_to` the user row; not loaded -> `Client::load_older` once per sync until the entry loads; no older page -> dropped (Q-167).
- Sizes: all go through `zoomed`; the 918 px threshold is `918 * zoom` device px.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-client -p bachuc-timeline -p bachuc-settings -p bachuc-app` -> exit 0, no failure (outline tests 14, the new rpcs test, and the new appearance test pass).
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> Finished. One warning, `next_theme` in `crates/app/src/state.rs:458`, in a file this TASK did not change.
- `./cargo.sh fmt` then `fmt --check` for `bachuc-client`, `bachuc-timeline`, `bachuc-settings`, `bachuc-theme` -> exit 0.
- Impact analysis (GitNexus): `TimelineView` LOW, `Pending` no callers resolved; all changes are additive.

## Anti-goal

DEFERRED (UI checker). By design nothing runs per frame: no animation, one 150 ms timer only after a hover, notify only on change.

## Lessons

- The list scroll handler fires only on user scroll. A programmatic `scroll_to`, a sync, and tail follow need an explicit reading-position report.
- A row's `anchor` is the entry `seqStart`, which equals the daemon prompt seq for a user message.
- The width threshold needs the timeline bounds from the last paint. The measure canvas flips a flag and defers a rail notify when the width crosses 918 px.
- The 140 ms CSS transition of the bars is not ported. Magnification changes at once. A finite animation would need per-bar previous-value state.
- `window.blur(cx)` takes `&mut App` in gpui-pre 0.3.9.

## UI Checks

Run 2026-10-19 on live, thread `mock TASK-252 outline` (14 prompts), Claude Light, zoom 125%; screenshots in `/tmp/wt-ui/zoom/t252-*.png`.

- 1600x900: the rail shows left of the thread, the last prompt lit -> PASS.
- Hover: the preview showed one clipped line. Fixed in the TASK-252 fix commit (wrapped block, 2 lines, ellipsis) -> PASS after the fix.
- Magnification of nearby bars -> PASS (visible bulge around the hovered bar).
- Click jump to prompts 8, 6, and 1 -> PASS; the lit bar follows.
- Scrolling lights the bar of the prompt being read; at the end bar 14 is lit -> PASS.
- Pointer leave hides the preview -> PASS with a real pointer move. A synthetic scroll left it shown, because it sends no mouse move.
- 1200x800 (pane 880 px): the rail is hidden -> PASS.
- Settings > Appearance > Chat > Chat outline off hides the rail and persists `chatOutline` -> PASS; restored to on.
- Idle CPU, 10 samples over 10 s: 0.0% -> PASS.
- Jump to a prompt that is not loaded: UNVERIFIED (unattended): the 14-prompt thread fits in the first page.
