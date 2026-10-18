# TASK-235 Outcome

## Status

Done: owner hand test passed (Q-153).

DONE, with both UI checker lines DEFERRED.

## Changed

- `packages/desktop-gpui/crates/app/src/shell.rs`: `FocusKind`, `focus_kind`, `should_redirect_to_composer`, and `Shell::on_key_type`. The handler runs on the shell root's `on_key_down` (bubble phase), next to the existing `capture_key_down`. Inline tests were added to the shell test module, which imports names one by one.
- `packages/desktop-gpui/crates/app/src/actions.rs`: `TEXT_CONTEXTS` is now `pub(crate)`.
- `composer.rs` is not edited. The lead's `Composer::insert_text` is used.

## Contract

- Consumes `Composer::insert_text(&mut self, text: &str, window: &mut Window, cx: &mut Context<Composer>)`.
- Produces `pub(crate) fn should_redirect_to_composer(key: &str, key_char: Option<&str>, modifiers: &Modifiers, focus: FocusKind) -> bool` and `pub(crate) fn focus_kind(contexts: &[&str], overlay: bool) -> FocusKind`. They live in the app crate, per the lead note, not in `bachuc-composer`.
- Rule: the focus is a thread only when the innermost key context is `BachucAgent` or `Timeline` inside an agent pane, and no text context (`Input`, `Terminal`, `NumberInput`, `TimeField`, `Combobox`) is on the path. Settings, the command center, the shortcuts dialog, and a plugin surface count as overlays. The key must be one printable character, Shift allowed, with no Cmd, Ctrl, Alt, or Fn. Space, named keys, and macOS private-use function characters are not moved. An archived agent is skipped.
- The key goes through `insert_text` and the handler stops propagation. GPUI runs key bindings before key listeners and returns when one handles the key (`gpui-pre-0.3.9/src/window.rs:6009`). So a bound key such as `?` is never typed twice.

## Verified

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-settings -p bachuc-command -p bachuc-workspace -p bachuc-theme -p bachuc-app -p bachuc-icons` -> PASS, exit 0. `a_printable_key_in_a_thread_moves_to_the_composer` covers Cmd, Ctrl, Alt, Fn, arrows, Enter, Tab, Space, Escape, Backspace, and a dead key with no character (IME composition). `only_the_timeline_or_the_agent_pane_counts_as_a_thread` covers the composer, the composer with an IME composition, the find bar, a focused terminal, Settings or an overlay, Home, a tab with no agent pane, and a menu inside the pane.
- Verify command replaced: the spec named `-p bachuc-composer`. The rule lives in `bachuc-app` (lead note), so the command above runs it.
- `CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> PASS. `fmt --check -p bachuc-app` -> exit 0.
- UI checker: click a thread message, type "abc" -> DEFERRED (UI checker).
- UI checker: focus the terminal, type "ls" -> DEFERRED (UI checker).

## Anti-goal

- Checkpoint "zero keys lost or doubled; the first character is the tripwire": DEFERRED (UI checker). Read check, 2026-10-18: the first key's own character is inserted, and propagation stops, so no input handler types it again. A timeline with focus has no platform input handler, so nothing else takes the key.

## Lessons

- Safe default taken: Alt is excluded, as Electron's `isTypeToFocusKey` excludes it. On macOS, Alt starts dead-key compositions. The spec named only Cmd and Ctrl.
- The handler runs in the bubble phase, not in the shell capture listener the spec named, so views keep their own key handling first, as in Electron.

## UI Checks

- Line 39 (click a thread message, type "abc"): UNVERIFIED (unattended): needs foreground. The AX click on a message does not give the thread GPUI key focus. Both typeText "abc" and pressKey a, b, c left the composer empty (`/tmp/wt-ui/chk-235-abc2.png`). Typing "abc" after an AX click on the composer works, so the harness reaches the window but cannot place focus on the timeline. This is not proof of a defect.
- Line 40 (focus the terminal, type "ls"): PASS. A Terminal tab got "ls" twice (`/tmp/wt-ui/chk-235-term2.png`) and the composer of a New agent tab in the same workspace stayed empty (`/tmp/wt-ui/chk-235-agent-after.png`).
