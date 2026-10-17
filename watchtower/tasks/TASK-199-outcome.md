# TASK-199 outcome

## Status

BLOCKED. Reading the code found no break in the app crate. The cause needs a live reproduction, and builders may not run one. No code changed for this TASK.

## Changed

- None.

## Contract

- None. No interface changed.

## Verified

Each suspect of the Brief, checked by reading:

- Suspect 1, `Activated` is not sent: for a mouse click, the event is sent. The group's `capture_any_mouse_down` calls `focus_group` before the tab's `activate_tab` (`crates/workspace/src/workspace.rs:849` and `:989`). A click on any tab of a group without focus changes the active key of the focused group, so `changed` sends `Activated` (`workspace.rs:366-372`). One state does not send it: a click on a lazy tab that is already active in the focused group. Every path found that makes a lazy tab active in the focused group also sends `Activated` for it, or calls `select`, which runs `materialize` (`crates/app/src/shell.rs:672-682`). These paths are `Workspace::open`, `activate_index`, `close` (`workspace.rs:302-316`), `drop_tab`, and the `open_workspace` batch.
- Suspect 2, `put_pane` swaps the item but does not redraw: it redraws. `Workspace::open` calls `changed`, which calls `cx.notify()`. The group body reads `group.active_item().view` on each render (`workspace.rs:795`). No view is cached (`rg "\.cached\("` finds none in the app or workspace crates).
- Suspect 3, `AgentPane` never calls `view_agent`: it does. `TimelineView::new` calls `Client::view_agent` (`crates/timeline/src/lib.rs:366`) and syncs again on `Connected`.
- Lead hint, the TASK-198 lesson: it does not apply to a tab click. A click materializes the tab in the `Activated` handler (`shell.rs:859-866`). Nothing reads the timeline in the same update.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app`: pass, 62 tests. This includes the TASK-198 fix round 1 changes in the same crate.
- UI check, 3 agent tabs each show a timeline: DEFERRED (UI checker).
- UI check, a tab in a second split pane: DEFERRED (UI checker).
- Owner hand test: DEFERRED (owner).
- Anti-goal, idle CPU with 5 tabs: DEFERRED (UI checker). Nothing changed.

## Blocker

- The cause was not found by reading. The next step is a live reproduction that shows which state the empty pane is in:
  - "Loading agent" (`crates/app/src/panes.rs:127-141`): the tab was never materialized, so the break is in the activation path.
  - "Loading timeline..." with a composer (`crates/timeline/src/lib.rs:1988`): the client has no timeline for the agent. One way this happens is two `TimelineView`s of one agent: `Client::view_agent` does not count views (`crates/client/src/core.rs:769-772`), so when one view is released, `unview_agent` drops the timeline the other view uses (`crates/timeline/src/lib.rs:346-350`, `core.rs:811-818`). This cause is in the timeline and client crates.
  - A truly blank group: the group has no active item.
- The UI checker cannot press a tab by AX. The tab chip has `Role::Tab` but no `AccessibleAction::Click` handler (`crates/workspace/src/workspace.rs:973-1000`). CONTEXT says an AX press does nothing without the handler, so the TASK-199 UI checks cannot run through `cua_repl` until the workspace crate adds one. This is in the workspace crate, which no group of this S chain owns.

## Lessons

- In the workspace crate, a group's capture mouse-down focuses the group before the tab's own mouse-down activates the tab, so a tab click sends at most two `Activated` events. The shell handles only the one whose key is still active.
- `Client::view_agent` and `unview_agent` act as an on/off switch per agent, not as a count. Two live views of one agent are not safe.

## Questions

- How does the empty pane look: "Loading agent", "Loading timeline..." with a composer, or blank? Which workspace and which tab layout (one group or a split)? The answer picks the crate that holds the cause.
- May the plan add an `AccessibleAction::Click` handler to the workspace tab chip (`crates/workspace/src/workspace.rs:973`), so the UI checker can press tabs?

## Update from TASK-198 fix round 2

- The workspace tab chip now has an `AccessibleAction::Click` handler (`packages/desktop-gpui/crates/workspace/src/workspace.rs:994`). It focuses the group and activates the tab, as a left mouse-down does. The UI checker can now press a tab by AX to reproduce the empty pane. Not yet run: DEFERRED (UI checker).
