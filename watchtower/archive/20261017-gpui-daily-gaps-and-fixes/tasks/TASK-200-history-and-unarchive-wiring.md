# TASK-200 History screen and unarchive in the app

Group: S (shared file [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs))
Class: code

## Brief

Goal: The sidebar History row opens the History screen, a row opens that agent even when archived, and an archived agent shows an Unarchive callout instead of the composer.

Change: no history and archived agents cannot open -> a History home tab, `open_agent_in`, and an archived mode in the agent pane.

Boundaries:

- History opens in the home view, as `open_new_workspace` does (`shell.rs` lines 1300-1362, `ensure_home`). Add `PaneContent::History` and update `view()`, `owns_focus()`, `kind()`, and the focus match (`shell.rs` lines 72-131, 874).
- A History row: Electron opens the agent tab in the entry's `workspaceId` (`agent-list.tsx` lines 358-374). Add `open_agent_in(agent_id, workspace_id)` that skips the tree lookup. If the workspace is not in the tree, open the agent tab in the home view (Q-114 default).
- `open_agent` (TASK-198) also tries `Client::archived_agent` for the workspace id when the live lookup fails.
- Tab title and icon of an archived agent come from `Client::archived_agent`.
- Archived callout: Electron [packages/app/src/components/archived-agent-callout.tsx](packages/app/src/components/archived-agent-callout.tsx) lines 17-56, placed instead of the composer (`agent-panel.tsx` line 1501). Text "This agent is archived", small button "Unarchive". Disabled while disconnected or in flight. An error shows in red under the callout. On success, the `agent_update` moves the agent to the live directory and the composer returns.
- Subscribe to `SidebarNavEvent` from the sidebar for `OpenHistory`. TASK-203 adds the `NewWorkspace` arm to the same subscription.

How:

- Add the History pane kind and `open_history`.
- Add `open_agent_in` and the archived lookup.
- Add the archived mode to the agent pane in `panes.rs`.

Files:

- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (pane kind, `open_history`, `open_agent_in`, subscriptions)
- [packages/desktop-gpui/crates/app/src/panes.rs](packages/desktop-gpui/crates/app/src/panes.rs) (archived callout instead of the composer)
- [packages/desktop-gpui/crates/app/Cargo.toml](packages/desktop-gpui/crates/app/Cargo.toml) (depend on `bachuc-history`)

Interfaces:

- Consumes: `Shell::open_agent(&mut self, agent_id: &str, window: &mut Window, cx: &mut Context<Self>)` keeps its signature and now closes a `Page` surface, from TASK-198.
- Consumes: `Client::archived_agent(&self, agent_id: &str) -> Option<&AgentSummary>` and `Client::is_archived(&self, agent_id: &str) -> bool` from TASK-205.
- Consumes: `Client::unarchive_agent(&mut self, agent_id: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>` from TASK-205.
- Consumes: `SidebarNavEvent::OpenHistory` from TASK-216.
- Consumes: `HistoryScreen::new(client: Entity<Client>, window: &mut Window, cx: &mut Context<HistoryScreen>) -> HistoryScreen` from TASK-218.
- Consumes: `HistoryEvent::OpenAgent { agent_id: String, workspace_id: Option<String> }` from TASK-218.
- Produces: `Shell::open_agent_in(&mut self, agent_id: &str, workspace_id: Option<&str>, window: &mut Window, cx: &mut Context<Self>)` for later plans; no TASK here consumes it.

Expected result:

- Click History in the sidebar -> the History screen opens in the home view.
- Click a live agent row -> its tab opens in its workspace.
- Click an archived agent row -> its tab opens with the timeline and the "This agent is archived" callout with Unarchive.
- Click Unarchive -> the callout turns into the composer.
- Unarchive fails -> the error shows in red and the button is enabled again.

Anti-goal: opening a live agent must not get slower; drift gauge; Board card click to first timeline paint stays under 300 ms (the TASK-198 limit), read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app` -> passes.
- UI check on live: open History, click an archived agent -> timeline and callout show; click Unarchive -> the composer shows.
- UI check on live: archive an agent, then click its Board card -> its tab opens with the archived callout.
- `./cargo.sh test -p bachuc-app` -> a test of the target rule: a workspace in the tree opens there; a missing workspace or no workspace id opens in the home view (Q-114).
- Owner hand test: find an old thread in History by search and open it.
- Anti-goal: click to first paint from log timestamps -> under 300 ms at both reads.
