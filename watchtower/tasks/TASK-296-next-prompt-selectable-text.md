# TASK-296 Selectable text in next prompt cards

Group: H (writes the next-prompt-actions GPUI crate only; no other TASK writes it)
Class: code

## Brief

Goal: Let the owner select and copy the prompt text and its reason line in the GPUI next prompt panel, in one-prompt cards and in multi-prompt pick rows.

Change: `prompt_text` draws a plain `div` and `StyledText`, which GPUI cannot select -> both lines use selectable text views; a drag in a pick row selects text and does not toggle the row.

Boundaries:

- Owner choice on 2026-10-22: both the one-prompt card (`prompt_card`) and the pick rows (`np-pick-*`) get selectable text, through the shared `prompt_text` in [packages/desktop-gpui/plugins/next-prompt-actions/src/panel.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/panel.rs).
- Owner choice on 2026-10-22: the prompt line and the reason line are both selectable. The reason keeps its `**bold**` runs in the foreground color and bold weight.
- Keep the look: same font sizes (`FONT_CONTENT`, `SMALL_TEXT`), line heights (`TEXT_LINE`, `SMALL_LINE`), colors, and wrapping as now.
- The prompt text shows exactly as written. Do not render it as markdown: a `*`, `_`, `#`, or backtick in a prompt must not change its look. Copy gives the raw prompt text.
- In a pick row, a plain click still toggles the row. A press that ends with selected text, or that moved more than 4 px, does not toggle. Planner call for the 4 px value.
- Planner call: draw the prompt line with `SelectableText::new(id, text)` from `gpui_kit::base` ([packages/desktop-gpui/vendor/gpui-base/src/selectable_text.rs](packages/desktop-gpui/vendor/gpui-base/src/selectable_text.rs)). It is plain, selectable, and copies the raw text. Gpui-kit has no plain-text `TextView`: `TextViewFormat` is only `Markdown` or `Html`.
- Planner call: draw the reason line with `ui::markdown` from `bachuc_plugin_host` (already imported in `panel.rs`), which keeps `**bold**`. `SelectableText` has no highlight API. If the strong run does not take the foreground color, set it through the markdown text style; do not patch the vendor crate.
- Trap: `TextViewState::has_selection` is `pub(crate)`. Read the selection with `TextSelection::has_selection(window, cx)` (window-wide), together with the moved distance. If a plain click reports a selection, rely on the distance and on an empty `selected_text`.
- The moved distance is the length of `up.position - down.position` from `ClickEvent::Mouse`; a keyboard or touch click counts as 0 px.
- Trap: markdown spacing can make the reason line taller. Match `SMALL_LINE` and remove paragraph margins so the anti-goal holds.
- Trap: text selection runs on a window-level mouse layer that never stops propagation, so the row `on_click` in `panel.rs` fires after a drag too. Gate it with the pure rule below.
- Non-goal: no copy button and no change to the React client, which is frozen.

How:

- Change `prompt_text` in `panel.rs` to build `SelectableText` for the prompt and `ui::markdown` for the reason, with stable element ids per block and prompt index.
- Add `pub(crate) fn toggles_on_release(moved_px: f32, has_selection: bool) -> bool` in [packages/desktop-gpui/plugins/next-prompt-actions/src/selection.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/selection.rs), and call it in the pick row click handler.

Files:

- [packages/desktop-gpui/plugins/next-prompt-actions/src/panel.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/panel.rs) (selectable `prompt_text`; gate the row toggle)
- [packages/desktop-gpui/plugins/next-prompt-actions/src/selection.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/selection.rs) (`toggles_on_release` and its tests)

Expected result:

- WHEN the owner drags across the prompt text of a one-prompt card THE panel SHALL select it, and Cmd+C SHALL copy the raw prompt text.
- WHEN the owner drags across the reason line THE panel SHALL select it, with bold runs kept on screen.
- WHEN the owner clicks a pick row without dragging THE panel SHALL toggle it as before.
- IF a press in a pick row ends with selected text or moved more than 4 px THEN THE panel SHALL not toggle the row.
- WHEN a prompt holds `*`, `_`, `#`, or backticks THE card SHALL show those characters as written.

Red tests:

File: [packages/desktop-gpui/plugins/next-prompt-actions/src/selection.rs](packages/desktop-gpui/plugins/next-prompt-actions/src/selection.rs) (existing test module)

```rust
    #[test]
    fn a_still_click_toggles_and_a_drag_or_selection_does_not() {
        use super::toggles_on_release;
        assert!(toggles_on_release(0., false));
        assert!(toggles_on_release(4., false));
        assert!(!toggles_on_release(4.5, false));
        assert!(!toggles_on_release(0., true));
    }
```

Anti-goal: the card height for the same prompt stays within 2 px of the height before the change; tripwire; read from the AX frame of the card (the parent of the `np-send-*` button; give the card an id `np-card-<block>-<prompt>` if it has none) on live for one known two-line prompt before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-next-prompt-actions` -> all tests pass, including the new test.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-plugin-next-prompt-actions -- -D warnings` -> no warnings.
- Owner check on a live GPUI build: drag across the prompt text of a one-prompt card, press Cmd+C, and paste -> the exact prompt text.
- Owner check: drag across the reason line -> it selects; the bold part stays bold.
- Owner check: in a multi-prompt block, click a row -> it toggles; drag across its text -> it selects and does not toggle; double-click a word -> it selects and does not toggle.
- Screen check: a prompt with a newline and a line starting with `- ` -> both show as written, not as a list.
- Screen check (UI checker, `cua_repl`): a prompt with `` `code` `` and `*star*` -> the AX text shows the backticks and stars as written.
- Anti-goal: AX frame height of the same card before and after -> the change is 2 px or less.
