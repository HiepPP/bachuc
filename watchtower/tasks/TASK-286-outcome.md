# TASK-286 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/composer/src/prompt_history.rs` (new): `PromptHistory`, `MAX_ENTRIES`, `Recall`, `history_key`, `on_first_line`, `on_last_line`, `SharedHistory` (shared cache, off-thread temp-file write), spec red tests.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: `history_key_press` in the keystroke intercept after `mention_key` and `slash_key`; `record_prompt` at the start of `submit`; `recall` reset in `clear_input` and `set_text`.
- `packages/desktop-gpui/crates/composer/src/lib.rs`: `mod prompt_history;`.

## Contract

- Key: `history_key(cwd, projects)` gives the project `root` whose root or workspace cwd equals the cwd (no trailing `/`), else the cwd. No cwd: history off.
- File: `prompt-history.json` in `data_dir()`, `{key: [prompt, ...]}`, 200 newest per key. Read once per process into a gpui global; written on a background thread through a temp file and a rename, newest write wins.
- Up on the first line and Down on the last line recall; menus keep both keys while open; IME composition, modifiers, and no entries go to the textarea. Up at the oldest entry takes the key and keeps the entry.
- A recalled `/name` or `@file` has its menu dismissed so the walk keeps working.
- Submit (after the `processing`/`locked` guard) records the typed text before client, plugin, and interceptor paths. A composer is per agent, so an agent switch is a new walk.

## Verified

- RED: `./cargo.sh test -p bachuc-composer prompt_history` -> compile errors, missing `PromptHistory`, `MAX_ENTRIES` (16:38:52).
- GREEN: same command -> 8 passed.
- `./cargo.sh test -p bachuc-composer` -> 138 passed (130 lib + 8 new, plus 5 integration), 0 failed.
- `./cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- `rg -n "std::fs::read" crates/composer/src/composer.rs` -> no match.
- Screen checks (4 on live) -> PASS; see UI Checks.
- Note: the first builds failed twice in `bachuc-timeline` (group B's `ClientEvent::ProjectRemoveFailed`); the third try after about 5 minutes built.

## Anti-goal

| Checkpoint         | Value                     | Source                             | Time  |
| ------------------ | ------------------------- | ---------------------------------- | ----- |
| baseline           | 0 failed (130 + 5 passed) | `cargo.sh test -p bachuc-composer` | 16:37 |
| after key handling | 0 failed (138 + 5 passed) | same                               | 16:46 |
| completion         | 0 failed (138 + 5 passed) | same, same tree                    | 16:46 |

Tripwire not crossed.

## Lessons

- `InputState::marked_text_range` (trait `EntityInputHandler`, import `gpui_kit::EntityInputHandler as _`) is the public way to detect IME composition; `ime_marked_range` is private.
- A gpui `Global` with `Rc<RefCell<_>>` gives one shared history across composers, so two composers do not overwrite each other's entries.

## UI Checks

Run by the lead with `cua_repl` on the check bundle `Bachuc GPUI Check.app` (release build of 913848389 + a2735a7d6), connected to live (6768), 2026-10-21 17:10-17:25.

- Send two prompts in `wt-ui-proj` main, open the worktree workspace `wt-ui-x`, press Up twice -> PASS. The composer showed "history second", then "history first". Down twice showed "history second", then the empty draft. `prompt-history.json` held `{"/tmp/wt-ui-proj":["history first","history second"]}`.
- Type `/` and press Up and Down -> PASS. The selection moved between `/clear` and `/exit`, and the text stayed `/`. Down on the first item kept the selection there, as the slash menu did before.
- Three lines with the cursor on line three, press Up -> PASS. The cursor moved to line two, and the text did not change.
- Move `prompt-history.json` away, relaunch, press Up in a composer -> PASS. The text stayed empty, and no error showed.
