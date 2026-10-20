# TASK-272 outcome

Status: DONE (code checks; screen checks deferred to the UI checker)

Changed:

- `packages/desktop-gpui/crates/composer/src/subagents_track.rs`: `RowClick`, the pure `row_click(row, parent_agent_id, supports_open)`, `SubagentActions.open_provider`, and the `subagents_pill` inputs `parent_agent_id` and `supports_open`. A Paseo row keeps its dismiss press and archive button; a provider row gets a press with the same dismiss. A provider row on a host without the feature opens nothing and shows the tooltip "Update the host to use this."
- `packages/desktop-gpui/crates/composer/src/composer.rs`: `ComposerEvent::OpenProviderSubagent { parent_agent_id, subagent_id }`; the `track_strip` actions read the parent from `Composer.agent_id` and the capability from `Client::supports_provider_subagent_timeline`.
- `packages/desktop-gpui/crates/composer/src/lib.rs`: exports `row_label` (dep fix for the shell's tab title).
- `packages/desktop-gpui/crates/app/src/shell.rs`: `provider_subagent_identity` (`subagent:` prefix), `PaneContent::Subagent`, its `view`, `owns_focus`, `kind` (`TabKind::Other`), and `focus_pane` arm; `open_provider_subagent`, which opens or focuses the tab in the parent's workspace and sets the child provider logo and provider.
- `packages/desktop-gpui/crates/app/src/panes.rs`: `SubagentPane`, the child subtitle header over `TimelineView::read_only`, with no composer.

Contract:

- A Paseo row emits `ComposerEvent::OpenAgent` as before. A provider row emits `ComposerEvent::OpenProviderSubagent` with the composer's agent as parent and the row id as child, only when `supports_provider_subagent_timeline()`.
- The tab identity is `subagent:` + `bachuc_client::provider_subagent_key(parent, child)`, so `agent_of_key` never returns it. It is not an `agent:` key.
- A second press finds the same identity with `find_identity` and activates that tab.
- The timeline key is `bachuc_client::provider_subagent_key(parent, child)`, per TASK-270.
- The tab title is `row_label` of the row at open time; the logo is `provider_logo` of the row's provider, set with `set_item_logo`; the pane holds the provider so a theme change rebuilds it.
- The subtitle header is drawn only when the row's `subtitle` is non-empty, at `FONT_SM` with line height 1.2.

Verified:

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer -p bachuc-app` -> PASS (group-level run for C2, exit 0: app 90, composer 125 lib, 4 `text_replacement`; 0 failed).
- Read check: no `agent_of_key` call can return the subagent tab's identity -> PASS. All 16 call sites in `shell.rs` require the `agent:` prefix; the identity starts with `subagent:`, so each returns `None`. `PaneContent::kind` gives it `TabKind::Other`, and `chat_target_key` excludes it, so no composer command targets it.
- Pure test: provider row -> parent and child IDs, Paseo row -> its agent ID -> PASS (`subagents_track::tests::a_row_press_opens_its_agent_or_its_provider_subagent_tab`). It also checks that a provider row gives `None` without the capability.
- `cd packages/desktop-gpui && ./cargo.sh build` -> PASS (exit 0).
- UI checker on live: press a provider subagent row -> a new tab with the label and steps -> DEFERRED (UI checker).
- UI checker on live: the next child step lands with no reload -> DEFERRED (UI checker).
- UI checker on live: the second press focuses the same tab and the tab count stays -> DEFERRED (UI checker).
- Anti-goal: a Paseo row still opens its agent tab -> DEFERRED (UI checker) for the screen; code read: the Paseo branch of `row_click` is unchanged in effect and emits the same `OpenAgent`.
- `./cargo.sh fmt --check -p bachuc-composer -p bachuc-app` -> PASS (exit 0).

Anti-goal checkpoints:

- 2026-10-19, before changes: composer had 124 lib tests and 4 `text_replacement` tests; app had 90 tests (source: TASK-273 outcome, group-level run).
- 2026-10-19, at completion: composer 125 lib (124 old plus 1 new) and 4 `text_replacement`; app 90. Every old test passes. The Paseo row press and the archive button keep their handlers (`subagents_track`) and their tests pass.
- 2026-10-19, at completion, screen: DEFERRED (UI checker).

Lessons:

- The row has no parent ID, so the shell must take it from the composer's `agent_id`; `row_click` takes it as an input.
- `put_pane` reads a logo only for an `agent:` identity, so a non-agent tab must set its logo after `put_pane` and store the provider on the pane for `refresh_logos`.
- `crates/composer/tests/` holds the owner's uncommitted text replacement test. This TASK added its test inside `subagents_track.rs` and did not touch that folder.

## UI Checks Round 2

Run 2026-10-19 on live (6768) with the check bundle (`BACHUC_NO_ACTIVATE=1`, AX actions only). Parent agent `72f4d2b8` started one Agent-tool child (`sleep 40`, echo, `sleep 40`); its composer track showed the provider row "Run sleeps and echo". All test agents were archived and the app was quit.

- Row press -> tab: PASS. The provider row opened a new tab titled with the row label "Run sleeps and echo"; the pane shows the child subtitle ("general-purpose · Opus 5.5 · Medium · ...") and its steps, and the pane has no composer input.
- Next child step lands live: PASS. The open tab moved from "ran 1 command, used 2 other tools / Running Task notification" to "ran 3 commands, used 19 other tools / Running Agent", then to the finished turn with the child's "done" text, with no reload.
- Second press: PASS. A second AX press on the same row put the same tab back on (`Value: on`) and the tab count stayed 2.
- Plan Verify "a click on a Claude subagent row opens its read-only tab": PASS, from the first press above.
- Paseo subagent row and archive buttons: UNVERIFIED (unattended). The test agents produced only provider rows; no Paseo subagent row existed, and the window stopped rendering before a Paseo child could be started.
