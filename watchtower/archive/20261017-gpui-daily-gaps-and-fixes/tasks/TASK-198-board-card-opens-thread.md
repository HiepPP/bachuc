# TASK-198 Board card opens its thread

Group: S (shared file [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs))
Class: risky

## Brief

Goal: A click on any Board card shows that agent's thread, as Electron does.

Change: the agent tab opens behind the Board page -> the Board page closes and the agent tab shows in front.

Boundaries:

- Root cause found by reading: the Board is a Page surface (`SurfacePresentation` default `Page`, [packages/desktop-gpui/crates/plugin-host/src/contributions.rs](packages/desktop-gpui/crates/plugin-host/src/contributions.rs) lines 149-152; Board sets none, [packages/desktop-gpui/plugins/board/src/lib.rs](packages/desktop-gpui/plugins/board/src/lib.rs) lines 55-62). `shell.rs` lines 2605-2607 draw the page instead of the workspace. `open_agent` (line 2044), `open_workspace` (line 607), and `select` (line 655) never clear `self.surface`; only `close_surface` (line 1602) does.
- The click chain itself works: `card.rs` line 325 `CardAction::Open` -> `store.rs` line 464 `open_agent_at` -> `PluginHostEvent::OpenAgentAt` -> `shell.rs` line 1478 -> `Shell::open_agent`. Hit-testing is fine: the content `v_flex` inserts no hitbox.
- Second guard: `open_agent` returns with no effect when no listed workspace holds the agent (`shell.rs` lines 2056-2061). Add a fallback: use `AgentSummary.workspace_id` from the directory when the tree lookup fails. If the agent is still not found, do nothing (TASK-200 adds archived agents).
- Electron opens with anchor `latest-prompt`. Owner answer Q-110: do the same. For `OpenAgentAt` with `AgentAnchor::LatestPrompt`, find the last user message id of the agent and call `TimelineView::reveal_passage` ([packages/desktop-gpui/crates/timeline/src/plugin_view.rs](packages/desktop-gpui/crates/timeline/src/plugin_view.rs) line 45), as the `RevealTimelinePassage` arm does (`shell.rs` lines 1486-1496). With no user message, keep the tail view.
- Close only a `Page` surface. Leave other surface kinds as they are.

How:

- Run `/solve` first: reproduce on live, open Board, click a card, confirm the tab opens behind the page.
- In `open_agent` and `open_workspace`, close an open `Page` surface before selecting the workspace.
- Add the `workspace_id` fallback in `open_agent`.
- Put the "close the page on open" rule in a pure function, for example `fn closes_surface_on_open(presentation: SurfacePresentation) -> bool`, with a unit test.

Files:

- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (close the page surface in `open_agent` and `open_workspace`; `workspace_id` fallback)

Interfaces:

- Produces: `Shell::open_agent(&mut self, agent_id: &str, window: &mut Window, cx: &mut Context<Self>)` keeps its signature and now closes a `Page` surface, for TASK-200.

Expected result:

- Board open, click a running card -> Board closes, that agent's tab is active and shows its timeline.
- Board open, click a "Just finished" card -> same result.
- The opened timeline shows the latest user prompt at the top of the view, not the tail.
- A click on the star, pencil, or Remove of a card does not open the agent.
- Sidebar workspace click while the Board is open -> Board closes and the workspace shows.
- A notification click and a command center agent pick still open the agent.

Anti-goal: agent-open latency must not grow; drift gauge; time from card click to the first painted timeline row stays under 300 ms, read on live with a log timestamp before the change, after it, and at completion.

Prompt:

```text
Use /solve to reproduce the Board card click on live, then fix it as the Brief says.
```

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app` -> passes, including the new surface rule test.
- UI check on live: open Board, click 2 cards (one running, one finished) -> each time the Board closes and the clicked agent's timeline shows.
- UI check on live: click the star and Remove of a card -> the Board stays open and no agent tab opens.
- UI check on live: with Board open, click a sidebar workspace -> the workspace shows.
- Owner hand test: click Board cards in daily use -> the thread opens every time.
- Anti-goal: log timestamps of click and first timeline paint -> under 300 ms at all 3 reads.
