# TASK-273 Outcome — Status badge on subagent rows and tabs

Status: DONE (code checks; screen and CPU checks deferred)

Changed:

- `packages/desktop-gpui/crates/sidebar/src/ring.rs`: `Knockout` gains `Surface0`, `Surface1`, and `Surface2`; the frame fill is chosen from the token at paint (theme changes follow).
- `packages/desktop-gpui/crates/sidebar/src/badge.rs` (new): `BadgeKind`, pure `badge_kind(bucket)`, and the `StatusBadge` view. The badge sits in the ring's 14 px frame, so the dot (8 px, offset -2 px) and alert (12 px, offset -3 px) keep their Electron offsets inside that frame. The ring entity is created on the first running bucket and dropped when the agent stops.
- `packages/desktop-gpui/crates/sidebar/src/lib.rs`: `pub mod badge` and `pub use badge::StatusBadge`.
- `packages/desktop-gpui/crates/composer/src/subagents_track.rs`: `provider_icon` wraps the logo in a 14 px `relative()` box and draws the row badge; `subagents_pill` takes the badge map.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: `Composer.badges` cache per subagent row key (`Knockout::Surface1`), pruned with the row list and updated through `StatusBadge::set_bucket`.
- `packages/desktop-gpui/crates/workspace/src/tab_menu.rs`: `TabExtras.badges` slot of `AnyView` with `set_badge`, `badge`, `forget`, and `is_empty`.
- `packages/desktop-gpui/crates/workspace/src/workspace.rs`: `set_item_badge`; `tab_glyph` wraps the icon in a 14 px `relative()` box and draws the badge; the drag payload and preview carry the badge.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `Pane.badge`; `agent_bucket` (`bucket_for_agent` from status, permissions, and attention); the badge is set in `put_pane` and updated on `AgentsChanged`.
- `packages/desktop-gpui/crates/workspace/tests/tab_menu.rs`: badge slot tests (clear and forget; unknown key).

Contract:

- Buckets draw: `running` -> ring, `failed` and `attention` -> dot (danger/success), `needs_input` -> `CircleAlert` in `dot_warning`, `done` -> nothing.
- One badge entity per subagent row key and per tab key; a bucket it keeps does not notify.
- The tab badge uses `Knockout::Surface0` (the resting tab bar). The workspace crate holds the badge only as `AnyView`, so it cannot retune the knockout to the hovered/active chip fill (`Electron` resolves that per render); the 1 px halo can differ by one surface step on a hovered or active tab.
- `needs_input` draws the alert glyph in the warning token; the Electron `fill` behind the glyph has no native equivalent in the icon component.

Verified:

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-sidebar -p bachuc-composer -p bachuc-workspace -p bachuc-app` -> PASS (group-level run, exit 0: app 90, composer 124 lib + 4 `text_replacement`, sidebar 14, workspace model 26 + tab_menu 10). Shared with TASK-268.
- Pure test in `badge.rs` (`every_bucket_draws_its_badge`) -> PASS.
- Tab badge slot test in `crates/workspace/tests/tab_menu.rs` set, replace, and clear by key -> UNSATISFIABLE for set/replace: `AnyView` holds an `Entity`, which needs a running `App`, and the gpui-kit/gpui `test-support` feature is forbidden here. Clear and forget PASS (`set_item_badge_clears_and_forgets`, `set_item_badge_on_an_unknown_key_changes_nothing`). Replace with a test under a headless app harness, or a badge slot that accepts a test-constructible handle.
- `cd packages/desktop-gpui && ./cargo.sh build` -> PASS (group-level, exit 0).
- UI checker on live: turning ring on a subagent row and on its agent tab at two angles -> DEFERRED (UI checker).
- UI checker on live: after the agent stops, the tab shows the logo only -> DEFERRED (UI checker).

Anti-goal (GPUI CPU with two running agents and one running subagent row stays at baseline plus 2 points over 60 s; `measure.sh 60` before and after): DEFERRED (UI checker). No CPU read was taken, and no earlier value exists.

Lessons:

- The `ViewElement` of an entity is laid out inline, so an absolute badge root resolves its offsets against the parent `relative()` icon box. Size the root to the ring frame and place the dot and alert inside it.
- A badge entity cannot be built in the plain crate tests. Keep any testable badge rule as a pure function (`badge_kind`).
- `Icon` needs `gpui_kit::component::Sizable as _` in scope for `with_size`.
- `bucket_for_agent` takes the attention reason as the wire string; the shell needs its own `AttentionReason` name map.
- `cargo.sh clean --profile dev` was not run: group A still shares `packages/desktop-gpui/target`, and a clean would delete its build. The lead owns the run-end cleanup.

## UI Checks

Run 2026-10-19 on live (6768) with the check bundle, launched with `BACHUC_NO_ACTIVATE=1` and driven by `cua_repl` AX actions only. Agent `dd68522` ran one Agent-tool subagent (`sleep 150`); agent `be39a90` ran `sleep 240` to keep a second running tab. Both apps opened were quit; the zoom was restored to 1.25; both test agents were archived.

- UI checker on live: turning ring on a subagent row and on its agent tab at two angles -> PASS. Enlarged crops show the row icon ring arc on the right in shot 1 and on the left in shot 2 (region diff 3.15); the tab icon ring also changes (diff 3.18). The row label and tab label are byte-identical (diff 0.0 and 0.008), so the text does not move.
- UI checker on live: after the agent stops -> FAIL as written. The turning ring went, but the tab did not show "logo only": it showed the provider logo plus a green attention dot. The stopped agent is in the attention bucket ("Ready to review"), and the contract draws a success dot for attention (`badge_kind`). The ring-gone intent passes; the literal "logo only" does not.
- Anti-goal: UNVERIFIED (unattended). No pre-change baseline exists. The after value was 35.5% over 60 s with two running agent tabs (`ui268 subagent`, `ui273b runner`) and the subagent row panel open; the subagent `sleep 150` ended near the end of the window.

## Lead Review (2026-10-19)

- UI check "after the agent stops -> the tab shows the logo only": the checker saw the ring go and a green dot stay, because the stopped agent was "Ready to review" (the `attention` bucket). The spec Expected result and the `badge.rs` test map `attention` to a dot and only `done` to the logo. The code follows the spec; the Verify line assumed the `done` bucket. Result: PASS by the spec mapping. A recheck needs an agent in the `done` bucket.
- CPU anti-goal waits for the group B fix round: the 35.5% read included the TASK-266 and TASK-267 motion cost.
