# TASK-238 Slash picker with skills like Electron

Group: C ([packages/desktop-gpui/crates/composer](packages/desktop-gpui/crates/composer); [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) is also touched later by TASK-242)
Class: code

## Brief

Goal: Typing "/" in the GPUI composer opens a picker that lists commands and skills, works in every case Electron does, and looks like Electron.

Change: slash picker that lacks skills, sometimes does not open, and looks different -> Electron parity.

Boundaries:

- The picker exists: [packages/desktop-gpui/crates/composer/src/slash.rs](packages/desktop-gpui/crates/composer/src/slash.rs) and `slash_key`, `slash_matches`, `pick` in [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) about lines 1054-1121. It uses the `list_commands` RPC.
- Owner reports three gaps on 2026-10-18: skills missing, "/" sometimes does not open the picker, and the look differs.
- Electron reference: [packages/app/src/hooks/agent-commands-query.ts](packages/app/src/hooks/agent-commands-query.ts) (draft commands include project skills from the checkout, line 57) and [packages/app/src/utils/agent-command-autocomplete.ts](packages/app/src/utils/agent-command-autocomplete.ts) (`command.kind === "skill"`, inline skill entries at line 79).
- Find each case where "/" does not open the picker (for example a new draft before an agent exists, or "/" after text). Match Electron for each.
- Keep plugin slash commands merged as today (`slash.rs` about line 63).

How:

- Compare the Electron and GPUI command lists for one agent and one draft; record the difference.
- Add skills, fix the open triggers, match rows (name, description, group, key navigation).
- Put filtering and trigger rules in pure functions with tests.

Files:

- [packages/desktop-gpui/crates/composer/src/slash.rs](packages/desktop-gpui/crates/composer/src/slash.rs) (list, filter, skill entries)
- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (open triggers, picker view)

Expected result:

- "/" lists the same commands and skills as Electron for the same agent.
- The picker opens in a new draft and after text, where Electron does.

Anti-goal: picker opens in under 100 ms after "/" with a cached list; a new RPC per key is the tripwire; checked by reading the code and one timed UI check.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer` -> all pass, including skill entry and trigger tests, an empty or failed `list_commands` result, a draft with no checkout, and a skill whose name equals a built-in command.
- UI checker: type "/" in an agent composer -> picker shows skills, for example a project skill from `.claude/skills`.
- UI checker: type "/" in a new-workspace draft -> picker opens.
- Read the code -> no RPC per keystroke.
