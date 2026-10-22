# TASK-296 Outcome

Status: DONE. Owner check Q-188 PASS (Hiep, 2026-10-22).

## Changed

- `packages/desktop-gpui/plugins/next-prompt-actions/src/panel.rs`
- `packages/desktop-gpui/plugins/next-prompt-actions/src/selection.rs`

## Contract

- `selection::toggles_on_release(moved_px: f32, has_selection: bool) -> bool`: true when `moved_px <= 4.` and no selection.
- `prompt_text` takes `(block, prompt)` first. The prompt line is `SelectableText` id `np-text-<block>-<prompt>`. The reason line is `ui::markdown` id `np-why-<block>-<prompt>`.
- The reason text is built by `reason_markdown`: ASCII punctuation outside `**bold**` runs is backslash-escaped, so `*`, `_`, `#`, and backticks in a reason show as written (same as before the change).
- `selection::row_click(click_count, moved_px, has_selection) -> RowClick {Toggle, Restore, Skip}`: count 2 -> Restore; count 0 or 1 -> Toggle when `toggles_on_release`, else Skip; count 3+ -> Skip.
- The pick row click handler reads distance and `up.click_count` from `ClickEvent::Mouse` (0 px and count 1 for keyboard or touch) and calls `row_click` with `TextSelection::has_selection(window, cx)`. Toggle stores the block and its selection before the toggle in `PanelView.before_click`; Restore puts it back (same block only); Skip clears it. A double-click therefore ends in the original state, also for radio groups.
- `prompt_card` now has id `np-card-<block>-<prompt>`.
- Paragraph gap of the reason markdown is 0; size `SMALL_TEXT`, line height `SMALL_LINE`, color muted.

## Verified

- Red: the new test failed to compile (`unresolved import super::toggles_on_release`) before the function existed.
- Fix round 1: `./cargo.sh test -p bachuc-plugin-next-prompt-actions` -> 62 + 1 passed, 0 failed, including `a_still_click_toggles_and_a_drag_or_selection_does_not` and `a_double_click_restores_and_a_triple_click_does_nothing`. Clippy `--no-deps -D warnings` clean; fmt --check exit 0.
- `./cargo.sh clippy -p bachuc-plugin-next-prompt-actions --no-deps -- -D warnings` -> no warnings. `--no-deps` per AUTORUN; without it clippy lints `bachuc-protocol` and `bachuc-theme`.
- `./cargo.sh fmt --check -p bachuc-plugin-next-prompt-actions` -> exit 0.
- DEFERRED (UI checker): drag-select of prompt and reason, Cmd+C raw copy, pick row click/drag/double-click, newline and `- ` prompt, backtick and star AX text.
- UNVERIFIED (owner): bold part stays bold on screen.

## Anti-goal

Card height within 2 px: tripwire; DEFERRED (UI checker). Not measured. Builders do not touch live (6768), so the before value could not be read. No checkpoint recorded. The card id `np-card-<block>-<prompt>` is in place for the AX read. Risk: the markdown `TextView` may add height versus the old `StyledText` line; the UI checker must read it.

## Lessons

- Markdown strong text in gpui-base draws at `FontWeight::MEDIUM` in the inherited text color (`vendor/gpui-base/src/text/node.rs:2392`). The `TextViewStyle` has no strong color hook, so the reason's bold runs are now MEDIUM weight in the muted color, not BOLD in the foreground color. This differs from the spec (bold weight, foreground color). Fixing it needs a vendor patch, which the spec forbids.
- A double-click: the first click of the pair still toggles the row (no selection exists yet). The second click does not toggle (word selected). Net effect: a double-click leaves the row toggled once. Owner check needed.
- `TextSelectionLayer` is not referenced in the crates; `gpui_kit::base::Root` supplies it, and the existing `np-intro` markdown text already selects through it.
- `gpui_kit::base::{SelectableText, TextSelection}` import works; `TextView::style` takes the style from `markdown_style(cx)`, and `with_paragraph_gap(rems(0.))` removes paragraph margins.

## UI Checks

UI checker, 2026-10-22, check bundle at `74d73ff80` on live (6768), zoom 1.25. Test thread `UI np1` (Claude haiku, cwd `/tmp/wt-ui/np1`, archived) replied with a one-prompt What Next block. Screenshot `/tmp/wt-ui/296-np1.png`.

- Screen check, prompt with a newline and a line starting with `- `: PASS. The card shows three lines: ``Run `cargo test` and check *star* stays literal.``, `- first item line`, `second line with _under_ and # hash`. No list is drawn.
- Screen check, prompt with `` `code` `` and `*star*`: PASS on the screenshot (backticks, stars, `_under_`, `#` show as written). The AX tree does not expose the text: `container What Next` holds only `button Edit` and `button Send`, so an AX text read was not possible.
- Anti-goal, card height within 2 px: UNVERIFIED (no baseline). The AX tree gives no frame for the card. After: about 196 px high in the 2560 px screenshot (card edges y 723 to 919). Reason line draws with bold part "verify" visible in a heavier weight in the muted color (matches the builder lesson).
- Owner checks (drag select and Cmd+C, reason drag, pick-row click, drag, and double-click): UNVERIFIED (owner). Text selection by drag needs a pointer.

Resolved: owner check Q-188 PASS (Hiep, 2026-10-22).
