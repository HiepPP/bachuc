# TASK-218 History screen

Group: H (standalone: new crate [packages/desktop-gpui/crates/history](packages/desktop-gpui/crates/history))
Class: code

## Brief

Goal: A History screen that lists past agents, archived ones too, with search, date groups, and Load more, as Electron's Sessions screen does.

Change: no history in GPUI -> a `bachuc-history` crate with a `HistoryScreen` view.

Boundaries:

- Electron: [packages/app/src/screens/sessions-screen.tsx](packages/app/src/screens/sessions-screen.tsx), [packages/app/src/hooks/use-agent-history.ts](packages/app/src/hooks/use-agent-history.ts), and rows from [packages/app/src/components/agent-list.tsx](packages/app/src/components/agent-list.tsx).
  - Search debounces 200 ms (`sessions-screen.tsx` line 24). Hide the field when `history_search_supported` is false.
  - Sort by last activity, newest first (`use-agent-history.ts` line 118). Group by today, yesterday, this week (7 days or less), this month (30 days or less), older (`agent-list.tsx` lines 55-81, 409-430).
  - A row shows the workspace name (or the project name), a chevron, the provider icon, and the title; then the project, the branch, and the time ago. Badges: Archived and "N pending" (lines 222-322). Highlight search matches in the title.
  - "Load more" button, not infinite scroll (`sessions-screen.tsx` lines 156-165).
  - Empty and no-match texts, and "Too many matches" for `search_truncated`: copy the exact strings from [packages/app/src/i18n/resources/en.ts](packages/app/src/i18n/resources/en.ts) lines 280-290. The error text "Unable to load sessions", the button "Try again", and "Loading..." are hard-coded in `sessions-screen.tsx` lines 162 and 202-204; copy them from there.
- Out of scope: the import sheet, the host column, and long-press archive.
- One host only ([ADR-0040](watchtower/decisions/ADR-0040-one-host-plain-ws-over-merged-hosts-and-relay.md)).
- The crate depends only on `bachuc-client`, `bachuc-theme`, and `bachuc-icons` as path dependencies (as [packages/desktop-gpui/crates/explorer/Cargo.toml](packages/desktop-gpui/crates/explorer/Cargo.toml) lines 9-12 do), and on `gpui-kit` with `workspace = true`. The workspace member glob `crates/*` picks it up.
- A click emits an event; the app (TASK-200) opens the agent.

How:

- Create the crate with `Cargo.toml`, `lib.rs`, and `screen.rs`.
- Put the date grouping, sort, and match highlight in pure functions with tests.

Files:

- [packages/desktop-gpui/crates/history/Cargo.toml](packages/desktop-gpui/crates/history/Cargo.toml) (new)
- [packages/desktop-gpui/crates/history/src/lib.rs](packages/desktop-gpui/crates/history/src/lib.rs) (new)
- [packages/desktop-gpui/crates/history/src/screen.rs](packages/desktop-gpui/crates/history/src/screen.rs) (new)

Interfaces:

- Consumes: `HistoryEntry { agent_id: String, title: String, provider: String, workspace_id: Option<String>, workspace_name: Option<String>, project_name: String, branch: Option<String>, cwd: String, archived: bool, last_activity_at: Option<String>, pending_permissions: usize }` from TASK-205.
- Consumes: `HistoryPage { entries: Vec<HistoryEntry>, next_cursor: Option<String>, has_more: bool, search_truncated: bool }` and `Client::fetch_agent_history(&mut self, search: Option<&str>, cursor: Option<&str>, cx: &mut Context<Client>) -> Task<anyhow::Result<HistoryPage>>` from TASK-205.
- Consumes: `Client::history_search_supported(&self) -> bool` from TASK-205.
- Produces: `HistoryScreen::new(client: Entity<Client>, window: &mut Window, cx: &mut Context<HistoryScreen>) -> HistoryScreen` for TASK-200.
- Produces: `HistoryEvent::OpenAgent { agent_id: String, workspace_id: Option<String> }` for TASK-200.

Expected result:

- The screen lists agents in date groups, newest first, with Archived badges.
- Typing a search refetches after 200 ms and highlights matches.
- "Load more" appends the next page; it hides when `has_more` is false.
- A truncated search shows the "Too many matches" footer.
- A row click emits `HistoryEvent::OpenAgent`.

Anti-goal: workspace build time must not grow much; drift gauge; a clean `./cargo.sh build -p bachuc-app` takes at most 10% longer, read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-history` -> passes, including date grouping and highlight tests.
- `./cargo.sh build -p bachuc-history` -> builds.
- `./cargo.sh test -p bachuc-history` -> a test for the view state picks the empty text for no entries, the no-match text for a search with no entries, and the error text for a failed load.
- UI check on live (after TASK-200): open History -> rows in date groups; search narrows them.
- Anti-goal: build time of `bachuc-app` -> at most 110% of the earlier reading.
