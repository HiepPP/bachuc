# TASK-308 Outcome

## Status

DONE (code checks). Screen checks are DEFERRED (UI checker).

## Changed

- `crates/client/src/model.rs`: `UsageEntry`, `SkillHookUsage`, `SkillHookUsage::parse`, `chip_label`, `UsageEntry::used_ago`, and 3 unit tests.
- `crates/client/src/core.rs`: parses `skillHookUsage` next to `backgroundProcesses` into the agent extras; `ClientCore::skill_hook_usage`.
- `crates/client/src/lib.rs`: `Client::skill_hook_usage`; exports `SkillHookUsage` and `UsageEntry`.
- `crates/client/tests/rpcs.rs`: the 2 red tests from the spec, unchanged.
- `crates/app/src/chrome.rs`: `HeaderMenu::Usage`, `render_usage_chip` (first in the header buttons), `usage_group`, popover through `header_popover`.

## Contract

- Consumes: agent snapshot `skillHookUsage?: { skills, hooks }` with `{ name, event?, count, errorCount, lastUsedAt }`. Read as given; no new protocol type.
- Produces: `Client::skill_hook_usage(&self, agent_id) -> Option<SkillHookUsage>`; `SkillHookUsage { skills, hooks }`; `UsageEntry { name, event, count, error_count, last_used_at_ms }`; `chip_label() -> Option<String>`.
- A missing field, or one with no entry in both lists, gives `None` and no chip. Entries without a `name` are dropped.

## Verified

- Red: before the code, `cargo test -p bachuc-client --test rpcs --no-run` failed with E0599 (`skill_hook_usage` missing) on 3 lines.
- `./cargo.sh test -p bachuc-client --test rpcs` -> 83 passed, including the 2 new tests.
- `./cargo.sh test -p bachuc-client --lib usage_tests` -> 3 passed. `-p bachuc-app` chrome test -> 1 passed.
- `./cargo.sh build -p bachuc-app` -> ok. `fmt --check` for `bachuc-client` and `bachuc-app` -> exit 0.
- Clippy `--no-deps -D warnings`: only old lines fail, `bachuc-client` `core.rs:2791` (`manual_contains`, commit b5b62fc3aa, 2026-10-19; already in Known Failures) and `bachuc-app` `plugins.rs:10` (`type_complexity`, commit 33e7400add, 2026-10-22; not in my diff). No lint in new code.
- Read check of `core.rs`: `skillHookUsage` parses next to `backgroundProcesses`; no new protocol type.
- Not done: `./cargo.sh clean --profile dev`, because the lead said not to run `cargo clean`. The lead cleans at the end of the run.

## Anti-goal

DEFERRED (UI checker): at 900 px wide, the chip, Scripts, Open in editor, git actions, and Explorer toggle show with no overlap or clip. By reading: the chip is `flex_shrink_0` with `whitespace_nowrap` in the right group, and the title (`min_w_0`, `truncate`) shrinks first.

Also DEFERRED (UI checker): chip shows counts and the popover opens with Skills and Hooks groups; a terminal tab or an agent without use shows no chip.

## Lessons

- GitNexus gave `UNKNOWN` for `render_header` and `HeaderMenu`; a text search showed the only users are in `chrome.rs` and `shell.rs`. No HIGH or CRITICAL risk.
- The `bachuc-app` crate name is `bachuc-app`; `bachuc-desktop` is only its binary.
- The chip shows distinct entries (`len`), not uses. A hook row adds a red `<n> failed` text when its error count is not zero.

## UI Checks

Run on 2026-10-27 with the check bundle on live (6768), fixture agent "wt ui fixture" (Claude haiku, owner hooks), 900x800 window. Screens: `/tmp/wt-ui-check/`.

- C3: PASS. The header chip reads "1 skill · 5 hooks", which matches the 1 skill and 5 distinct hooks in `skillHookUsage` of the agent record. An AX press opens a popover with a Skills group (`feynman ×1 6m ago`) and a Hooks group (`Stop ×10`, `PostToolUse:Bash ×3`, `PreToolUse:Bash ×3`, `UserPromptSubmit ×6`, `SessionStart:startup ×3`, each `6m ago`). The counts and the newest-first order match the record (`c3-chip.png`, `c3-popover.png`).
- C4: PASS. A mock agent tab with no usage ("wt ui nousage") and a New tab screen both show no chip (`c4-nousage-agent.png`, `c4-newtab.png`). The Scripts, Open in editor, and Explorer buttons stay.
- C5: PASS. At 900x800 the header shows the chip, the workspace pill, the actions "..." button, Open in editor with its dropdown, and the Explorer toggle, with no overlap and no clipped button. This was seen at zoom 1.5 and at zoom 1.0, with the sidebar shown and hidden. The title and pill truncate first ("wt..." at zoom 1.5 with the sidebar open). The workspace has no scripts, so no separate Scripts button was drawn; git actions did not show for this non-git folder (`c3-chip.png`, `c5-900-sidebar-z1.0.png`, `c5-900-nosidebar-z1.0.png`).
- Note: a popover click needed a raise first. The popover was opened by a click on the chip; no AX action list is shown for the chip.
