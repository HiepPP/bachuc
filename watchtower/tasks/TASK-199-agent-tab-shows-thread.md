# TASK-199 Agent tab shows its thread on click

Group: S (shared file [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs))
Class: risky

## Brief

Goal: A click on an agent tab in the workspace screen shows that agent's thread, not an empty pane.

Change: a clicked agent tab shows empty content -> it shows the timeline and the composer.

Boundaries:

- The cause is not known yet. A read found no break in the wiring:
  - `open_workspace` makes lazy tabs with `open_lazy_agent` (`shell.rs` line 1822, `PaneContent::LazyAgent`).
  - A tab click: `workspace.rs` lines 987-990 `activate_tab` -> `layout.activate` -> `changed()` emits `Activated` only when the active key changes (`workspace.rs` lines 366-372).
  - The shell handles `PaneEvent::Activated` at `shell.rs` line 845 and calls `materialize` (line 1847) -> `open_agent_pane` (line 1764) -> `put_pane` (line 896), which replaces the item by key in place (`model.rs` lines 444-451) and does not re-emit.
  - A lazy tab gets a `PaneFrame` that shows "Loading agent" (`panes.rs` lines 127-141).
- Suspects to test, in order: `Activated` not emitted when the clicked tab is already the active key of another pane; `put_pane` swapping the item but not redrawing the pane; the materialized `AgentPane` never calling `Client::view_agent`, so `TimelineView` ([packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) line 293) stays empty.
- Fix the cause in the app crate. If the cause is in [packages/desktop-gpui/crates/workspace](packages/desktop-gpui/crates/workspace) or the timeline crate, stop and record the finding as a blocker with path:line; those crates belong to no group of this plan's S chain except by a plan change.

How:

- Run `/solve`. Reproduce on live: open a workspace with 3 or more agents, click each tab.
- Log the event path to find where it stops.
- Fix it, and put any new rule in a pure function with a test.

Files:

- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (fix the lazy tab path)
- [packages/desktop-gpui/crates/app/src/panes.rs](packages/desktop-gpui/crates/app/src/panes.rs) (only if the cause is in the pane frame)

Expected result:

- Open a workspace with 3 agents, click each tab -> each shows its timeline and composer within 1 second.
- Click a tab in a second split pane -> same result.
- Switch away and back to a materialized tab -> it keeps its scroll position.

Anti-goal: idle app CPU must not grow; drift gauge; with 5 materialized agent tabs and no stream, CPU stays at or under the reading before the change plus 1 point, read with `top -l 2 -pid <pid>` before the change and at completion.

Prompt:

```text
Use /solve to reproduce the empty agent tab on live, find the cause, and fix it as the Brief says.
```

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app` -> passes.
- UI check on live: open a workspace with 3 agents and click each tab -> every tab shows a timeline with rows.
- UI check on live: split the pane, click an agent tab in the new pane -> it shows the timeline.
- Owner hand test: click agent tabs in daily workspaces -> the thread shows every time.
- Anti-goal: `top` CPU with 5 idle materialized tabs -> at most the earlier reading plus 1 point.
