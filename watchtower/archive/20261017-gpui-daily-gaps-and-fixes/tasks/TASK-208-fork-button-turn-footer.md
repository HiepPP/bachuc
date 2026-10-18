# TASK-208 Fork button in the turn footer

Group: T (shared files [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) and its modules)
Class: code

## Brief

Goal: Each turn footer has a Fork button with two choices, "Fork in a new tab" and "Fork in a new workspace", as Electron has.

Change: "Fork is out of scope" in `render_footer` (`lib.rs` lines 1504-1514) -> a fork button that emits a fork event.

Boundaries:

- Electron: [packages/app/src/components/assistant-fork-menu.tsx](packages/app/src/components/assistant-fork-menu.tsx), placed in the turn footer next to Copy and "Worked for N" (`agent-stream/turn-footer.tsx` lines 160-210). Split icon, tooltip "Fork chat from here". It locks while a fork runs.
- Boundary: for an assistant message, use its timeline cursor when the item has one and `Client::fork_cursor_supported` is true, plus its message id; else the message id alone (`turn-boundary.ts` lines 7-23). A running turn has no footer in GPUI (`lib.rs` line 560), so only completed turns get the button.
- Hide the button when `Client::fork_supported` is false.
- The timeline only emits the event. The app (TASK-201) calls `fork_context` and opens the draft.

How:

- Add the button and its two-item menu to the footer.
- Add a new event type `TimelineForkEvent` with `impl EventEmitter<TimelineForkEvent> for TimelineView`. Do not add a variant to `TimelineEvent`: the app reads it with an irrefutable `let TimelineEvent::OpenFile { .. } = event;` (`shell.rs` line 1787), so a new variant breaks the app build.
- Put the boundary rule in a pure function with tests.

Files:

- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (footer button, menu, event)

Interfaces:

- Consumes: `ForkCursor { epoch: String, seq: u64 }` and `ForkBoundary { cursor: Option<ForkCursor>, message_id: Option<String> }` from TASK-205.
- Consumes: `Client::fork_supported(&self) -> bool` and `Client::fork_cursor_supported(&self) -> bool` from TASK-205.
- Produces: `ForkTarget` enum `{ Tab, Workspace }` and `TimelineForkEvent::Fork { agent_id: String, target: ForkTarget, boundary: ForkBoundary }` for TASK-201.
- Produces: `TimelineView::set_fork_pending(&mut self, pending: bool, cx: &mut Context<TimelineView>)` for TASK-201.

Expected result:

- A completed turn shows the fork button; its menu has the two items.
- A pick emits `TimelineForkEvent::Fork` with the right boundary.
- The app still builds before TASK-201 subscribes to the new event.
- While pending, the button is disabled.
- On a host without `agentForkContext`, no fork button shows.

Anti-goal: timeline CPU while a turn streams must stay under 10% ([ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md)); tripwire; read on live with the mock `five-minute-stream` model before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> passes, including the boundary rule tests.
- UI check on live: a completed turn footer shows the fork button, and its menu shows both items.
- Anti-goal: CPU during the mock stream -> under 10% at both reads.
