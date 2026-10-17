# TASK-208 outcome

## Status

DONE (code). The screen checks are DEFERRED (UI checker). The fork boundary sends no timeline cursor yet, because the client exposes no timeline epoch (see Questions).

## Changed

- `packages/desktop-gpui/crates/timeline/src/lib.rs`:
  - New `pub enum ForkTarget { Tab, Workspace }` and `pub enum TimelineForkEvent { Fork { agent_id: String, target: ForkTarget, boundary: ForkBoundary } }`, with `impl EventEmitter<TimelineForkEvent> for TimelineView`. `TimelineEvent` is unchanged.
  - New `pub fn set_fork_pending(&mut self, pending: bool, cx: &mut Context<TimelineView>)`. Pending closes an open menu and disables every fork trigger (opacity 0.5, no click handlers).
  - `render_footer` draws `render_fork_menu` between the copy button and "Worked for N", as `AssistantTurnFooter`. It shows only when `Client::fork_supported` is true and the response has a boundary. The trigger is the lucide `Split` icon, tooltip and label "Fork chat from here". The menu (220 wide, `deferred(anchored())` like the git actions menu) has "Fork in a new tab" and "Fork in a new workspace". The trigger and both items have `Role::Button` or `Role::MenuItem`, an aria label, `on_click`, and an explicit `AccessibleAction::Click` handler. No hover toggles `display`.
  - A pick closes the menu and emits `TimelineForkEvent::Fork` with the agent id, the target, and the boundary. The app sets pending; the timeline does not.
  - An outside press closes the menu. A trigger click within 500 ms after that close keeps the menu shut, so a press on the trigger of an open menu closes it instead of opening it again.
  - Pure rules with tests: `latest_assistant(entries, start, end)` (the response's latest assistant message, as `findLatestAssistantInResponse`) and `fork_boundary(entry, epoch, cursor_supported)` (`resolveAssistantTurnForkBoundary`; the cursor `seq` is the entry `seqEnd`, as the web hydrates `timelineCursor`).

## Contract

- Produces: `ForkTarget` `{ Tab, Workspace }`, `TimelineForkEvent::Fork { agent_id: String, target: ForkTarget, boundary: ForkBoundary }`, and `TimelineView::set_fork_pending(&mut self, pending: bool, cx: &mut Context<TimelineView>)`, with the exact names and types. `ForkTarget` and `TimelineForkEvent` are exported from the crate root.
- Consumes: `ForkCursor { epoch, seq }`, `ForkBoundary { cursor, message_id }`, `Client::fork_supported`, and `Client::fork_cursor_supported` from TASK-205, unchanged.
- Only completed turns get a footer (the running response gets none), so only they show the button.
- The view passes `epoch = None` to `fork_boundary`, so a boundary is the message id alone. An assistant message without `messageId` shows no fork button, as the web does when the boundary is undefined.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-timeline` -> pass, lib 23 tests, with `fork_boundary_follows_the_turn_boundary_rule` and `the_footer_forks_at_the_latest_assistant_message`. Log: `/tmp/t208-tl.txt`.
- `./cargo.sh fmt --check -p bachuc-timeline` -> exit 0. Build warnings: none from the crate.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> exit 0, so the app builds before TASK-201 subscribes. Log: `/tmp/t208-app.txt`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- GitNexus impact (index 39 commits stale, advisory): `render_footer` LOW (1 caller, `render_row`), `body_line_height` LOW; confirmed with rg.
- UI check: a completed turn footer shows the fork button, and its menu shows both items -> DEFERRED (UI checker).
- Anti-goal: CPU under 10% during the mock stream -> DEFERRED (UI checker). Per footer draw the view adds one feature read and a backward scan to the response's latest assistant message, usually 1 to a few entries; no footer draws for the running turn.

## Lessons

- The client's timeline entries carry `seqStart` and `seqEnd` but no epoch, and `TimelineState` has none, so a view crate cannot build a `ForkCursor`. The epoch lives in the private `TimelineSync.epoch` (`crates/client/src/timeline.rs`).
- GPUI `on_mouse_down_out` runs in the capture phase, before the trigger's click, so a toggle trigger needs a guard or it opens the menu again on the press that closed it.

## Questions

- Add `Client::timeline_epoch(&self, agent_id: &str) -> Option<&str>` (group C, client files) so the timeline can send `boundaryCursor`? The safe default taken: no cursor, message id only. The fork then needs the assistant `messageId`; the Claude provider sets it (`packages/server/src/server/agent/providers/claude/agent.ts:1333`).

## UI Checks

Run 2026-10-18 by the UI checker (run 3). Bundle `Bachuc GPUI Check` built from HEAD `2d5734c11` (includes `e9a78e843`, `77a9730cb`, and `22906f2cb`), PID 31420, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The app stayed in the background. The checker forced each frame with the AX "zoom the window" action and read the AX tree after a no-op F15 key. Scratch repo `/tmp/t202-repo` (branches `main` and `feature-x`, script `hello`) was the live workspace `wks_4a3662191ab77c60` with disposable mock agents "t202 scratch", "t200 archive me", "t200 board card", and "t208 cpu stream". The workspace was archived after the run; PID 31420 quit and exited.

- A completed turn footer shows the fork button: PASS. The footer of a completed turn has "Copy turn" and "Fork chat from here" (a split glyph) before "Worked for 9s" (`/tmp/r3-03-archived-open.png`).
- Its menu shows both items: PASS. The press opened AX `menu Fork chat from here` with "Fork in a new tab" and "Fork in a new workspace" (`/tmp/r3-04-fork-menu.png`). No item was pressed. Escape did not close the menu; a second press on the trigger closed it (`/tmp/r3-05-fork-closed.png`).
- Anti-goal, CPU under 10% during the mock stream: UNVERIFIED (unattended): background frame. With "t208 cpu stream" (five-minute stream) open, `ps` gave 1.7% and 1.2% for PID 31420, but the background window does not paint, so the paint cost is not in this reading (`/tmp/r3-33-cpu-stream.png`).
