# TASK-272 Open a provider subagent from its row

Group: C (the `bachuc-composer` crate and the app shell; shares `subagents_track.rs` and `composer.rs` with TASK-268 and TASK-273, and `shell.rs` with TASK-273)
Class: code

## Brief

Goal: A click on a provider subagent row in the composer track opens a tab with that child's live, read-only timeline.

Change: a provider row opens nothing -> it opens a subagent tab, as Electron's `onOpenProviderSubagent` does.

Design: `SubagentsTrackRow.handlePress` in [packages/app/src/subagents/track.tsx](packages/app/src/subagents/track.tsx) and [packages/app/src/panels/provider-subagent-panel.tsx](packages/app/src/panels/provider-subagent-panel.tsx).

Boundaries:

- Follows [ADR-0051](watchtower/decisions/ADR-0051-read-only-provider-subagent-tab-over-jump-to-parent.md).
- Today only a Paseo row has a click, and the comment says "A provider child opens nothing" (`packages/desktop-gpui/crates/composer/src/subagents_track.rs:238`, click at `:266-269`). Give a provider row a click that emits a new composer event with the parent and child IDs. A Paseo row keeps `ComposerEvent::OpenAgent`.
- `SubagentRow` has no parent ID, but the composer belongs to the parent agent, so the composer's own agent ID is the parent. Add an `open_provider` callback to `SubagentActions` (`composer.rs:2171`) and a `supports_open` input to `subagents_pill`.
- `track_row` (`tracks.rs:222`) takes `pressable`, which also sets `Role::MenuItem` and the pointer cursor. Pass true for a provider row when the host supports it, and wire the dismiss as the Paseo row does (`subagents_track.rs:266-269`).
- The shell opens the tab in the parent's workspace, found as `open_agent_in` does (`packages/desktop-gpui/crates/app/src/shell.rs:2823`). Opening it again focuses the same tab.
- Do not give the tab an agent identity. Every `agent_of_key` use treats its result as a daemon agent ID: copy ID (`shell.rs:1204`), rename (`:1233`), resume (`:1251`), reload (`:1266`), `active_agent` (`:2857`), the tab logo (`:1077-1080`), `PaneContent::kind` (`:150`), and `chrome.rs:475`. Add a new identity prefix and a new `PaneContent` variant (`shell.rs:96`), with its `view`, `owns_focus`, and `kind`. `AgentPane` (`panes.rs:64`) builds a composer, so add a small pane type instead.
- Tab content: a subtitle line with the child's `subtitle` when it has one, as Electron's `subtitleHeader`, over `TimelineView::read_only`. No composer.
- Tab title: the row label, as `row_label` gives it. Tab logo: the child's provider logo, set with `set_item_logo` from the row's `provider`, because `put_pane` finds no logo for a non-agent key (`shell.rs:1077-1115`).
- A host without `projectedSubagentTimeline`: the row opens nothing, as today, and shows the Electron text "Update the host to use this." (`message.actions.forkUnavailable`) as its tooltip.
- Planner call: the shell does not restore subagent tabs after a restart. The child timeline may be gone.
- Non-goal: the nested child track (subagents of the subagent) that Electron shows in the panel. [ADR-0051](watchtower/decisions/ADR-0051-read-only-provider-subagent-tab-over-jump-to-parent.md) records it under Revisit If.

How:

- Add the provider click and the new event in the composer.
- Handle the event in the shell: open or focus the subagent tab.
- Build the tab content from the subtitle and `TimelineView::read_only`.

Files:

- [packages/desktop-gpui/crates/composer/src/subagents_track.rs](packages/desktop-gpui/crates/composer/src/subagents_track.rs) (provider row click)
- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (new event, `SubagentActions` wiring near line 2171)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (open or focus the tab)
- [packages/desktop-gpui/crates/app/src/panes.rs](packages/desktop-gpui/crates/app/src/panes.rs) (the subagent pane content)

Interfaces:

- Consumes: `bachuc_client::provider_subagent_key(parent_agent_id: &str, subagent_id: &str) -> String` from TASK-270
- Consumes: `bachuc_client::parse_provider_subagent_key(key: &str) -> Option<(String, String)>` from TASK-270
- Consumes: `Client::supports_provider_subagent_timeline(&self) -> bool` from TASK-270
- Consumes: `TimelineView::read_only(client: Entity<Client>, key: String, diagrams: Entity<DiagramRenderer>, window: &mut Window, cx: &mut Context<TimelineView>) -> TimelineView` from TASK-271

Expected result:

- A click on a running or finished Claude subagent row opens a tab with its title, logo, and steps.
- While the child runs, new steps show in the tab.
- A second click focuses the same tab.
- A Paseo subagent row still opens its agent.

Anti-goal: the Paseo subagent click and the archive buttons keep working; tripwire; read from the `subagents_track` tests and the screen check, before changes and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer -p bachuc-app` -> every test passes (`bachuc-app` has unit tests only).
- Read check: no `agent_of_key` call can return the subagent tab's identity.
- Pure test in `subagents_track.rs` or `shell.rs`: the click target of a row -> a provider row gives the parent and child IDs; a Paseo row gives its agent ID.
- `cd packages/desktop-gpui && ./cargo.sh build` -> the workspace builds.
- UI checker on live (screen check): ask a live Claude agent to start one Agent tool subagent that runs `sleep 60`. Open the subagents pill and press the row through AX. Expect a new tab with the subagent's label and its steps.
- UI checker on live (screen check): wait until the subagent's next step lands -> the tab shows it with no reload.
- UI checker on live (screen check): press the same row again -> the same tab gets focus, and the tab count stays the same.
- Anti-goal: press a Paseo subagent row through AX -> its agent tab opens, as before.
