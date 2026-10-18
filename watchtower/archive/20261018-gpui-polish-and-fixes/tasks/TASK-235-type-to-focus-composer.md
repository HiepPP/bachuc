# TASK-235 Type anywhere to focus the composer

Group: S (shares [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) with TASK-234)
Class: code

## Brief

Goal: In an agent thread, typing a printable key when no input has focus moves focus to the composer and types that key there, like Electron.

Change: keys typed outside the composer are lost -> they go to the composer.

Boundaries:

- Trigger: a printable character with no Cmd or Ctrl, when no text input, terminal, settings field, or search box has focus. The first key is typed into the composer, not dropped. Owner chose this default on 2026-10-18.
- Today `Shell::on_key_capture` in [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) about line 2159 handles only Escape and plugin shortcuts. The timeline root in [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) about line 2210 handles only Escape. `Composer::focus` is in [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) about line 581.
- GPUI calls subscribers in subscription order; use `cx.defer_in` to read later views (MEMORY 2026-10-18).
- Do not steal keys from IME composition.

How:

- Add a pure `should_redirect_to_composer(key, modifiers, focus_kind) -> bool` with tests.
- In the shell key capture, when it returns true, focus the active pane composer and insert the key.

Files:

- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (key capture redirect)
- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (insert-text entry point if missing)

Expected result:

- Click on the thread, type "hello" -> the composer shows "hello".
- Typing in the terminal or a settings field stays there.

Anti-goal: zero keys lost or doubled when redirected; the first character is the tripwire; read in the UI checker run.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-composer` -> all pass, including the redirect rule test for Cmd, Ctrl, arrows, Enter, Tab, IME composition, a focused terminal, and a surface with no composer (Home, settings, a plugin overlay).
- UI checker: click a thread message, type "abc" -> composer text is "abc", not "bc" or "aabc".
- UI checker: focus the terminal, type "ls" -> the terminal gets "ls" and the composer stays empty.
