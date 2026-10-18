# TASK-238 outcome

## Status

DONE. Skills and the draft command list now reach the composer. Two UI checker lines are deferred.

## Changed

- `packages/desktop-gpui/crates/composer/src/slash.rs`
- `packages/desktop-gpui/crates/composer/src/composer.rs`
- `packages/desktop-gpui/crates/client/src/model.rs`
- `packages/desktop-gpui/crates/client/src/core.rs`
- `packages/desktop-gpui/crates/client/src/lib.rs`
- `packages/desktop-gpui/crates/client/tests/rpcs.rs`

## Contract

- Client: `SlashCommand.skill: bool` (wire `kind == "skill"`). `Core::list_draft_commands(agent_id, &DraftConfig)` and `Client::list_draft_commands(agent_id, &DraftConfig, cx)` send `list_commands_request` with `draftConfig`. The wire protocol is unchanged. The only `SlashCommand` literal was in `core.rs`.
- `slash.rs`: `SlashRange`, `find_active_slash(text, cursor)`, `menu_matches(daemon, plugins, range, draft)`, `replace_slash`, `DaemonCommand { name, description, skill }`, `SlashItem.skill`. `filter_items` ranks like `scoreTextFields`.
- `composer.rs`: the caret decides the command, so "/" opens after text and mid-text. A "/" after text lists skills only. A draft lists no built-in command and opens only with a provider and a folder. `insert_text` refreshes the menu. A draft asks again only when its `DraftConfig` changes (`commands_config`); an agent asks once.

## Difference between Electron and GPUI (the compare step)

- Electron: `kind: "skill"` marks skills; a draft sends `draftConfig`, and the daemon reads project skills from the checkout.
- GPUI before: the client dropped `kind` and a draft never asked the daemon. Now both match Electron.
- No daemon change: `session.ts` `handleListCommandsRequest` already takes `draftConfig`.

## Verified

- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-composer -p bachuc-client` -> all pass (composer 123, client rpcs 60, plus the other client suites). New client tests: `list_commands_parses_the_commands` checks `skill`; `list_draft_commands_sends_the_draft_config` checks the request body.
- `./cargo.sh fmt --check -p bachuc-composer -p bachuc-client` -> exit 0.
- UI checker: "/" in an agent composer shows skills, for example a project skill from `.claude/skills` -> DEFERRED (UI checker).
- UI checker: "/" in a new-workspace draft opens the picker -> DEFERRED (UI checker).

## Anti-goal

- Read the code: `load_commands` sends one RPC per composer, and one more only when the draft config changes. No RPC per keystroke; filtering runs in memory (`composer.rs` `load_commands`, `refresh_slash`; 2026-10-18).
- Timed open under 100 ms: DEFERRED (UI checker).

## Lessons

- Electron ranks an alias match like `q` (for `/exit`) as exact, above prefix matches.
- Electron merges sources first, then hides built-ins in a draft, so a skill named like a built-in stays hidden.
- A fake-server test needs `add_agent` before `connect`, or the connect step panics.
- Not ported by decision (Q-146): Escape on a start-position slash only dismisses the menu.

## UI Checks

- Line 41 (type "/" in an agent composer): PASS. After an AX click on the composer and typeText "/", the picker lists 50 skills and commands (for example gitnexus-debugging, caveman, brandkit) within 3 seconds (`/tmp/wt-ui/chk-238-slash2.png`). The test project has no `.claude/skills` folder, so the user and plugin skills show, not a project skill.
- Line 42 (type "/" in a new-workspace draft): PASS. The same 50 items show (`/tmp/wt-ui/chk-238-newws-slash.png`).
