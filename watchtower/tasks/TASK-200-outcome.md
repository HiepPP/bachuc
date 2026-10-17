# TASK-200 Outcome

## Status

DONE. The UI checks, the owner hand test, and the anti-goal timing are DEFERRED.

## Changed

- `packages/desktop-gpui/crates/app/Cargo.toml`: depend on `bachuc-history`.
- `packages/desktop-gpui/Cargo.lock`: one added line, `bachuc-history` in the `bachuc-app` dependency list.
- `packages/desktop-gpui/crates/app/src/shell.rs`:
  - `PaneContent::History(Entity<HistoryScreen>)`, added to `view()`. `owns_focus()` stays false for it, so the tab gets a `PaneFrame`. `kind()` and the `focus_pane` match reach it through their existing `_` arms (`TabKind::Other`, frame focus). `HistoryScreen` still has no `Focusable`, and `crates/history` is unchanged.
  - `open_history`: it opens one History tab in the home view (identity `history`) or shows the open one. Then it closes a page surface and selects the home view.
  - A `SidebarNavEvent` subscription in `subscribe_host`, so a host switch subscribes again. `OpenHistory` calls `open_history`. `NewWorkspace { .. }` does nothing, and TASK-203 fills that arm.
  - `open_agent_in`: it opens the agent tab in the given workspace when the tree lists it. Otherwise it opens the tab in the home view (Q-114), closes a page surface, and selects home. It uses the pure rule `agent_target` and clears `pending_prompt`.
  - `open_agent`: on a live hit it calls `open_agent_in` with the tree workspace. If the live lookup fails and `Client::archived_agent` knows the agent, it calls `open_agent_in` with the archived `workspace_id`. If both fail, nothing happens, as before.
  - `agent_tab_title` and `agent_provider` fall back to `Client::archived_agent`. Tab title, tab icon, tab logo, and the resume command then work for archived agents.
- `packages/desktop-gpui/crates/app/src/panes.rs`: `AgentPane` keeps the client and the agent id.
  - While `Client::is_archived` is true, the pane shows the callout in place of the composer. The callout has the text "This agent is archived" and a small "Unarchive" button.
  - The button is disabled while the client is disconnected or a request is in flight (`can_unarchive`). An error shows in `status_danger` under the callout, and the button is enabled again.
  - After a successful reply, the in-flight state stays until the agent is live, as in Electron. The composer comes back on the `agent_update` that clears the archived entry. If the callout had focus, the focus moves to the composer.
  - The layout follows `archived-agent-callout.tsx`: 75 px minimum height, 16 px padding, 820 px maximum width, a `surface1` row with a `border_accent` border and a 16 px radius, and a 12 px gap.

## Contract

- Produces: `Shell::open_agent_in(&mut self, agent_id: &str, workspace_id: Option<&str>, window: &mut Window, cx: &mut Context<Self>)`, `pub(crate)`.
- Consumes with exact names: `Shell::open_agent` (signature unchanged), `Client::archived_agent`, `Client::is_archived`, `Client::unarchive_agent`, `SidebarNavEvent::OpenHistory`, `HistoryScreen::new`, and `HistoryEvent::OpenAgent { agent_id, workspace_id }`.
- `agent_target(projects, workspace_id) -> Option<String>`: returns `Some` only for a workspace that the tree lists. `None` means the home view.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> exit 0. The only warning is the pre-existing unused `next_theme` in `state.rs`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> 64 passed, up from 61 at TASK-198 (the counts between are not recorded here). This includes `shell::tests::a_history_row_opens_in_its_tree_workspace_else_the_home_view` (the target rule: tree workspace, missing workspace, no id, empty tree) and `panes::tests::unarchive_needs_a_connection_and_no_request_in_flight`. `state::tests::the_shell_refreshes_logos_only_from_the_theme_observer` passes, and the `ClientEvent::TimelineChanged { .. }` early return is unchanged.
- `./cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `./cargo.sh clippy -p bachuc-app --tests` -> no warnings in `shell.rs` or `panes.rs`.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- GitNexus impact (index 35 commits stale, advisory): `open_agent` LOW and `AgentPane` LOW. `agent_tab_title` is CRITICAL with 20 transitive callers. Its signature and its live-agent result are unchanged; only a lookup miss now tries the archived map.
- UI check, open History and click an archived agent, then Unarchive: DEFERRED (UI checker).
- UI check, archive an agent and then click its Board card: DEFERRED (UI checker).
- Owner hand test, find an old thread by search and open it: DEFERRED (owner).
- Anti-goal, click to first paint under 300 ms at both reads: DEFERRED (UI checker). The live open path adds one `find_workspace` scan in `agent_target` and one hash lookup in `archived_agent`.

## Lessons

- `on_agents_changed` still closes the tab of an agent that leaves `agents()`, so archiving an agent closes its open tab. Reopening it from the Board or History shows the callout.
- An archived agent with no workspace in the tree opens in the home view. After an unarchive restores its workspace, a later `open_agent` opens a second tab in that workspace and leaves the home tab open. Tabs are looked up per target workspace only.
- `AgentPane` still builds its `Composer` for an archived agent and only hides it, so the composer can return without a rebuild.
- The History tab has no focus handle of its own. If a later TASK needs keyboard focus in the search field, add `Focusable` to `HistoryScreen` in `crates/history`. Then add `PaneContent::History` to `owns_focus` and to the `focus_pane` match.

## UI Checks

Run 2026-10-18 by the UI checker (run 3). Bundle `Bachuc GPUI Check` built from HEAD `2d5734c11` (includes `e9a78e843`, `77a9730cb`, and `22906f2cb`), PID 31420, launched with `open -n -g --env BACHUC_NO_ACTIVATE=1` and connected only to live `127.0.0.1:6768` (`lsof`). The app stayed in the background. The checker forced each frame with the AX "zoom the window" action and read the AX tree after a no-op F15 key. Scratch repo `/tmp/t202-repo` (branches `main` and `feature-x`, script `hello`) was the live workspace `wks_4a3662191ab77c60` with disposable mock agents "t202 scratch", "t200 archive me", "t200 board card", and "t208 cpu stream". The workspace was archived after the run; PID 31420 quit and exited.

- History from the sidebar, open an archived row: PASS. The History row is in the nav and opens a "History" tab with "Today / Yesterday / This week" groups and "Archived" pills (`/tmp/r3-02-history.png`). The row "main: t200 archive me" opens the agent in its tree workspace (header "main", t202-repo) with the timeline and the callout "This agent is archived" + "Unarchive" in place of the composer (`/tmp/r3-03-archived-open.png`).
- Unarchive brings the composer back: PASS. After the press the AX tree has `text entry area Message, ID: composer-input` and no callout (`/tmp/r3-06-unarchived.png`); `paseo inspect` shows `Archived false`.
- Archive an agent, then click its Board card: FAIL. "t200 board card" (`d9d6e2d`) was archived by CLI while its card still showed on the Board ("Stale" snapshot). Two AX presses on "Open conversation t200 board card" left the Board open with no tab (`/tmp/r3-08-board-archived-card.png`, `/tmp/r3-09-board-archived-retry.png`). A control press on the live card "t202 scratch" opened its tab at once (`/tmp/r3-10-board-control.png`). Cause not diagnosed. `open_agent` falls back to `Client::archived_agent`, which `upsert_agent` fills from an `agent_update` with `archivedAt` (`crates/client/src/core.rs:3102`); a check of whether that update reached the client was not done. Note: the Board hides archived cards by design (`hiep-plugins/plugins/board/server/snapshot.ts:19`), so the card is only there until the next snapshot; an agent archived earlier ("t200 archive me") never has a card.
- Anti-goal, click to first paint under 300 ms: UNVERIFIED. The app logs no click or paint timestamps, and a background window paints only when forced.

## Fix round 1

### Status

DONE. The Board card of an archived agent now opens its tab. The UI check is DEFERRED (UI checker).

### Cause

Found by reading the code and the daemon. The client subscribes to agent updates with `"subscribe": {}` and no filter (`crates/client/src/core.rs`, `opened`). For a subscription without `includeArchived`, the daemon sends `agent_update { kind: "remove" }` when an agent is archived, not an upsert with `archivedAt` (`packages/server/src/server/session/agent-updates/agent-updates-service.ts:134-137` and `:255-262`). The client then calls `remove_agent`, so `Client::archived_agent` stays empty. `open_agent` found neither a live nor an archived agent and did nothing. The History path passed because a History page fills the archived map (`archive_history_rows`) and the row gives its own `workspace_id`. The test `archived_upserts_move_to_the_archived_map_and_back` used an archived upsert, a message the daemon never sends to this client, so it hid the gap. The shell, the `LatestPrompt` arm, and the Board plugin were ruled out: the press reached `open_agent` (the live card opened with the same path).

The fix fetches the unknown agent instead of subscribing with `includeArchived`. That filter would load every archived agent at each connect.

### Changed

- `packages/desktop-gpui/crates/client/src/core.rs` and `lib.rs`: `fetch_agent(agent_id)` sends `fetch_agent_request` (`Pending::FetchAgent`). An archived answer goes through `upsert_agent`, so it fills the archived map. A live answer is not stored, because the subscription owns live agents. A `null` agent or an error fails the reply. Reason for the client change: the cause is in the client, the fix is small, and no running builder owns the client crate (the other builder owns `crates/command` and `crates/settings`).
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`: `an_archived_remove_needs_a_fetch_to_fill_the_archived_map` (a `remove` leaves the archived map empty; a fetch fills it with the `workspace_id`) and `a_fetch_of_a_missing_agent_fails`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `open_agent` uses the pure rule `agent_open`. When the client knows neither a live nor an archived agent, `open_fetched_agent` fetches it and then opens an archived agent with `open_agent_in` in its own workspace (else the home view, Q-114). A fetch error is logged and nothing opens, as before. Test `an_unknown_agent_is_fetched_before_it_opens`.

### Contract

- Produces: `Client::fetch_agent(&mut self, agent_id: &str, cx: &mut Context<Client>) -> Task<anyhow::Result<()>>`.
- `Shell::open_agent` keeps its signature. It now also opens an agent that was archived after this client last saw it.

### Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-client --test rpcs`: the new test failed first (`no method named fetch_agent`), then passed. All `bachuc-client` suites pass (rpcs 54).
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app`: pass, 75 tests, including `an_unknown_agent_is_fetched_before_it_opens` and `the_shell_refreshes_logos_only_from_the_theme_observer`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app`: exit 0, only the old `next_theme` warning.
- `./cargo.sh fmt --check -p bachuc-app` and `-p bachuc-client`: exit 0.
- `./cargo.sh clippy -p bachuc-app --tests`: no warning in the changed files.
- `./cargo.sh metadata --format-version 1 > /dev/null`: exit 0.
- GitNexus impact (index 54 commits stale): `open_agent` LOW. `composer_response` HIGH (the response dispatcher); only a new `Pending::FetchAgent` arm was added, the other arms are unchanged.
- UI check, archive an agent and then press its Board card: the tab opens with the archived callout: DEFERRED (UI checker).

### Lessons

- An agent update subscription without `includeArchived` gets `remove` on an archive. A client test must send what the daemon sends to that filter, not an archived upsert.
- An archived agent opened from a stale Board card shows the tail, not the latest prompt. The tab exists only after the fetch, so the `LatestPrompt` reveal in the same event finds no tab.
