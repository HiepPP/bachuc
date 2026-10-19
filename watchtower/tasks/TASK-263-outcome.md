# TASK-263 Outcome

## Status

DONE (code). The AX check, the screenshot pair, and the owner hand check are DEFERRED (UI checker).

## Changed

- `packages/desktop-gpui/crates/timeline/src/lib.rs`: the `"user_message"` arm draws a trailing row (`render_user_footer`) in the space `USER_TRAILING_ROW` reserves: time, rewind menu (`render_rewind`, `PopupMenu` through `dropdown_menu_with_anchor`), copy button. New `TimelineRewindEvent::Rewind { agent_id, message_id, mode, text }`, `TimelineView::set_rewind_pending`, `copy_user_message`, `rewind`, and the pure rules `format_message_timestamp`, `rewind_choices`, `user_message_id`, `restores_composer` (public), `user_trailing_opacity`.
- `packages/desktop-gpui/crates/composer/src/composer.rs`: `Composer::set_text` is public.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `Shell::rewind` handles the event (calls the client, restores the composer, toasts on failure); `rewind_error`, `REWIND_FAILED`; one subscription beside the fork one.
- `packages/desktop-gpui/crates/timeline/Cargo.toml` (extra, wiring): `chrono` gets the `clock` feature, for the local time zone offset of the prompt time.

## Contract

- Consumes `Client::rewind_modes(&self, agent_id) -> RewindModes` and `Client::rewind(&mut self, agent_id, message_id, mode, cx: &mut Context<Client>) -> Task<Result<(), String>>` exactly as TASK-260 shipped them. The shell calls the second through `self.client.update(cx, |client, cx| client.rewind(...))`.
- Order in the row: time, rewind, copy. The row is drawn at opacity 0 and shows on the message's `group_hover`; `USER_TRAILING_ROW` (8 + 24) is its height in both states. A hidden control gets no pointer click, because the pointer over it is the hover that shows it; an AX press still reaches it.
- Time: en-US `formatMessageTimestamp` — same local day "10:39 AM", the previous six calendar days "Sunday 10:39 AM", older "Oct 12, 2026, 10:39 AM". It reads `entry["timestamp"]` and the local UTC offset.
- Rewind: `Undo2` at 16, tooltip "Rewind to this message", menu "This action cannot be undone" then "Rewind conversation", "Rewind files", "Rewind conversation and files" for the modes the agent takes. No button without `item.messageId` or without a mode. While a rewind runs the icon is dimmed and has no menu.
- On `Ok` for `Conversation` or `Both` the prompt text goes back into the composer through `Composer::set_text` (replacing its text); on `Err` the shell shows the error text, else "Failed to rewind agent", and leaves the composer.
- Copy: `Copy`, then `Check` for 1,500 ms, label "Copy message" ("Copied" while the check shows); it has an `AccessibleAction::Click` handler. The rewind trigger relies on the gpui-kit dropdown press, which has no separate AX handler.

## Verified

- `./cargo.sh test -p bachuc-timeline -p bachuc-composer` -> 124 composer, 30 timeline lib, and the integration suites pass; 0 fail.
- New tests: timestamp for same day, with offset, within the week (1 and 6 days), a week old, 200 days old, and across midnight; no rewind button without a message id or without modes, mode order, empty id; composer restore only for conversation and both; opacity differs while the reserved row height is the same (`USER_TRAILING_ROW` = 32 px).
- `./cargo.sh test -p bachuc-app --bin bachuc-desktop failed_rewind` -> 1 passed.
- `./cargo.sh fmt --check -p bachuc-timeline -p bachuc-composer -p bachuc-app -p bachuc-sidebar` -> exit 0. `./cargo.sh clippy -p bachuc-timeline -p bachuc-composer -p bachuc-app --all-targets` -> no new warning (older in `timeline/src/lib.rs`: `edit_stats` type, `notification` argument count).
- Screen check (AX tree has "Rewind to this message" and "Copy message"): DEFERRED (UI checker).
- Owner hand check (hover shows the row; copy; rewind conversation on a live agent; a failed rewind keeps the composer text; an old host shows no button): DEFERRED (UI checker).

## Anti-goal

Layout shift on hover: the user message is a column of the bubble and a footer with `h(USER_TRAILING_ROW)`; hover changes only the footer opacity. Read in `hovering_a_prompt_changes_the_opacity_and_never_the_row_height` (opacity 0 vs 1, height the same constant 32). The screenshot pair is DEFERRED (UI checker).

## Lessons

- A gpui-kit `PopupMenu` needs a trigger that implements `DropdownMenu`; a small wrapper struct (as in `explorer/src/ui.rs`) is the pattern, and it has no AX press of its own.
- The row used to pad with `pb(USER_TRAILING_ROW)`; a footer element of the same height keeps the row height unchanged.
- The web's negative right margin on the copy button is a left padding only here, because a negative margin on a child of a content-sized flex row collapses its width.
- The bare `Composer::set_text` replaces the whole input; Electron restores the rewound prompt the same way.

## UI Checks

Lead, 2026-10-19, check bundle on live (6768), cua_repl:

- AX tree of a user prompt (thread `mock TASK-263 footer`) has "Rewind to this message" and "Copy message". PASS.
- Hover reveal, copy, rewind on a live agent, failed rewind toast, old host: owner hand check Q-172.

## UI Checks 2 (lead, 2026-10-19 13:50)

- Not run: the display did not repaint and scroll did nothing, so the row was not reachable in the virtual list.

## UI Checks 3 (lead, 2026-10-19 14:00)

- Not run: GPUI draws no frames while another app window covers the check window, so scroll and clicks in the sidebar and timeline did nothing. Run with the check app alone in front.

## UI Checks 4 (lead, 2026-10-19 14:10)

- Not run: Zen (full screen) and the daily Bachuc GPUI window were in front of the check window, so GPUI drew no frames. Clicks, scroll, and Cmd+K changed the AX tree but not the screen.
