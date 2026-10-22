# TASK-292 outcome

Status: DONE (owner card check pending)

## Changed

- `packages/desktop-gpui/crates/client/src/core.rs`: `Core::agent_thinking_option_id`.
- `packages/desktop-gpui/crates/client/src/lib.rs`: `Client::agent_thinking_option_id`.
- `packages/desktop-gpui/plugins/watchtower-board/src/model/actions.rs`: `AgentCandidate.model`, `.thinking`.
- `packages/desktop-gpui/plugins/watchtower-board/src/store.rs`: `candidates` fills them.
- `packages/desktop-gpui/plugins/watchtower-board/src/model/builders.rs`: `TaskBuilder.provider/model/thinking`, copied in `match_builders`, one new test.
- `packages/desktop-gpui/plugins/watchtower-board/src/model/overview_view.rs`: `secondary_line` appends ` · <run part>` via `run_part`; `builder` helper; red tests; 3 existing assertions gained ` · claude`.
- `packages/desktop-gpui/crates/client/tests/composer.rs`: `the_stored_thinking_option_id_needs_no_loaded_model_options`.

## Contract

- `Core/Client::agent_thinking_option_id(&self, &str) -> Option<String>`: None for an unknown agent or no stored id.
- `TaskBuilder { provider: String, model: Option<String>, thinking: Option<String> }`.
- `secondary_line`: `<line> · <provider>, <model>, <effort>`; parts trimmed and skipped when empty; line empty -> run part alone; rows with no builder unchanged.

## Verified

- Red: with only the red tests added, the crate failed to compile (E0560, no field `provider`/`model`/`thinking`), 2026-10-22T04:3x.
- `./cargo.sh test -p bachuc-plugin-watchtower-board`: 135 pass, 1 fail = Known Failure `one_thousand_derives...` (alone: fails too under load avg ~10; timing only). Integration test `tests/plugin.rs` passes with `--no-fail-fast`.
- `./cargo.sh test -p bachuc-client`: all pass except `sync::one_thousand_workspace_updates_stay_cheap` (58-111 ms vs 45 ms bound, load avg ~10-15; the path does not touch the change). Not in AUTORUN Known Failures; baseline not A/B-able without stash.
- `fmt --check` both crates: exit 0.
- Clippy (`--no-deps -- -D warnings`) fails on code outside this TASK: `plugins/watchtower-board/src/model/page.rs:66` (filter_next) and `crates/client/src/core.rs:2711` (manual contains). Both are untouched lines; clippy on newer toolchain (1.97). Not fixed.
- `git diff` of `crates/client/src/model.rs` and `hiep-plugins/plugins/watchtower-board`: empty.
- Owner check on live GPUI build: DEFERRED (UI checker).

## Anti-goal

Tripwire: existing `secondary_line_covers_each_row_state` strings.

- Before (2026-10-22T04:30:50Z, cargo test watchtower-board): 132 pass, 1 known timing fail.
- After `secondary_line` change (04:36:45Z): 135 pass (132 + 3 new), same 1 known timing fail; `secondary_line_covers_each_row_state` ok.
- Completion: same result on the `--no-fail-fast` rerun.
- Test diff: only 3 expected strings changed, each only appends ` · claude`: "Builder A asks permission", "Builder A, active 5m ago", "Builder, idle". PASS.

## Lessons

- `Client::agent_thinking` needs loaded model options; use `agent_thinking_option_id` for the raw id.
- Clippy 1.97 reports two lints in untouched code (`page.rs:66`, `core.rs:2711`), so `-D warnings` clippy fails for both crates today.

## UI Checks

UI checker, 2026-10-22. Owner check on a running watchtower card: UNVERIFIED (owner). No builder agent was running on live (6768): `npm run cli -- ls -a` listed only mock and closed check agents, and builders of this run use the release daemon. The Board was not opened.
