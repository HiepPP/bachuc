# TASK-252 Chat outline rail

Group: O (timeline crate, client crate, settings crate; runs after group T because it shares the timeline crate)
Class: code

## Brief

Goal: the GPUI thread shows Electron's Chat outline: a rail of prompt bars on the left of the thread. Hover shows a preview, and a click jumps to the prompt.

Change: GPUI has no outline -> a rail with one bar per user prompt. It has dock magnification, hover intent with a preview, the active prompt lit while scrolling, a click or keyboard jump, and a Settings toggle.

Boundaries:

- Q-165 (owner, 2026-10-19): full parity with Electron, not a reduced version.
- Q-166 (owner, 2026-10-19): the prompt list comes from the daemon RPC `agent.timeline.list_prompts.request` / `.response`, gated on `server_info.features.agentTimelinePromptIndex`. With the gate off, show no rail.
- Q-167 (owner, 2026-10-19): a jump to a prompt that is not loaded calls `load_older` until that prompt's seq is loaded, then scrolls to it. Do not add a window fetch or merge non-contiguous windows.
- Port the behavior of `packages/app/src/agent-stream/chat-outline/` (`model.ts`, `hover-intent.ts`, `rail.web.tsx`, `use-chat-outline.ts`) and the rail placement in `packages/app/src/agent-stream/view.tsx`. Read them first; they are the source of truth for every size, delay, and color below.
- Electron values: rail width 36, slot height 8 (slots tile the rail and shrink), placed at left `spacing[2]`, top 10%, bottom 10%. Pills: rest 10x2, active 18 wide, magnified 26x4, radius full, grown to the right from a shared left edge. Magnify radius 3 slots with a raised-cosine falloff. Hover intent: 150 ms first activation, 4 px horizontal transit. Preview: 260x48, 4 px right of the rail, centered on the pill, `lg` radius, 1 px border. Colors: rest `borderAccent`, active `foregroundExtraMuted`, attention `foreground`. Hide the rail below a 918 px panel width and with fewer than 2 prompts. Honor reduced motion (no magnification).
- All sizes go through `bachuc_theme::zoomed` (Zoom Conversion Rule in [watchtower/CONTEXT.md](watchtower/CONTEXT.md)).
- The active prompt is the last indexed prompt at or before the timeline row at the top of the viewport (`resolveActivePromptSeq`). Update it on scroll without re-rendering every timeline row.
- Settings: an on/off "Chat outline" row, "Show an outline for jumping between prompts", default on. Electron keeps it in the chat section of Settings; put it in the GPUI pane that holds chat options, or in Appearance when no chat pane exists. Persist it in `appearance.json` next to the other appearance keys.
- Do not change Electron or the daemon.
- Put the pure rules (magnification, active prompt, hover intent state machine, jump plan) in pure functions, and test them in the crate `tests/` folders.

Files:

- [packages/desktop-gpui/crates/client/src](packages/desktop-gpui/crates/client/src) (the list_prompts request, response, and feature gate)
- [packages/desktop-gpui/crates/timeline/src](packages/desktop-gpui/crates/timeline/src) (a new `outline.rs` for the rail, plus the hook-up in `lib.rs`)
- [packages/desktop-gpui/crates/timeline/tests](packages/desktop-gpui/crates/timeline/tests)
- [packages/desktop-gpui/crates/settings/src](packages/desktop-gpui/crates/settings/src) (the toggle and its persistence)
- [packages/desktop-gpui/crates/theme/src/lib.rs](packages/desktop-gpui/crates/theme/src/lib.rs) only if the toggle's state must live in the theme global

Interfaces:

- Consumes: `Client::load_older(agent_id, cx)` and the timeline `ListState` scrolling in `crates/timeline/src/lib.rs`.
- Produces: `bachuc_timeline::outline::prompt_tick_magnification(slot_distance: f32) -> f32` and `bachuc_timeline::outline::resolve_active_prompt_seq(prompts: &[u64], anchor_seq: Option<u64>) -> Option<u64>`.

Expected result:

- In a thread with 2 or more prompts and a panel of at least 918 px, the rail shows on the left of the thread. Hovering a bar for 150 ms shows the prompt preview, and nearby bars grow. A click scrolls the thread to that prompt.
- Scrolling lights the bar of the prompt being read.
- A click on a prompt older than the loaded window loads older pages and then scrolls to it.

Anti-goal: no frame loop. With the rail showing and the thread idle, CPU stays under 1% over 10 s.

## Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-client -p bachuc-timeline -p bachuc-settings` -> pass, with tests for magnification (1 at 0, 0 at 3, symmetric), the active prompt (none before the first prompt, last at or before the anchor), the hover intent (150 ms first activation, immediate after that), and the jump plan (loaded -> scroll; not loaded -> load older).
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds, no new warnings in changed files.
- `./cargo.sh fmt --check` for each changed crate -> exit 0.
- UI checker on live, with a mock thread of at least 12 prompts (the lead makes it with `npm run cli -- run --provider mock/...` and follow-up prompts): the rail shows, hover preview, magnification, click jump, active bar while scrolling, the jump to an unloaded prompt, hidden below 918 px, the Settings toggle hides it.
- Anti-goal: `ps -o %cpu` of the check bundle, 10 samples over 10 s with the rail showing -> each under 1%.
