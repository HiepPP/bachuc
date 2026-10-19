# TASK-263 Prompt footer with time, rewind, and copy

Group: B (writes [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs), [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs), and [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs); shell.rs is shared with TASK-258)
Class: risky

## Brief

Goal: below each user prompt, GPUI shows the time, a rewind menu, and a copy button, like Electron.

Change: the `"user_message"` arm ([packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) lines 1474-1519) draws only the bubble -> it draws a trailing row in the space `USER_TRAILING_ROW` already reserves.

Design: `UserMessage` in [packages/app/src/components/message.tsx](packages/app/src/components/message.tsx) (lines 429-581) and [packages/app/src/components/rewind/rewind-menu.tsx](packages/app/src/components/rewind/rewind-menu.tsx)

Boundaries:

- Order: time, rewind, copy. Time uses Electron's `formatMessageTimestamp` rule ([packages/app/src/utils/time.ts](packages/app/src/utils/time.ts) line 128): same day shows only the time, within a week shows weekday and time. Read `entry["timestamp"]`.
- The row shows on hover by opacity 0 to 1, and its height is always reserved, so the layout never shifts. Hidden controls take no clicks.
- Copy is a new path for the prompt text. `copy_turn` ([packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) line 1161) copies assistant turns only. Reuse its look: copy icon, then a check for 1,500 ms. The label is "Copy message" (Electron `message.actions.copyMessage`).
- Read the message id from `item.messageId` of the user message entry. If the live wire has none, show no rewind button.
- Rewind: lucide `Undo2` at 16, tooltip "Rewind to this message". The menu shows "This action cannot be undone", then only the modes from `rewind_modes`. Show no button when there is no mode or no `item.messageId`.
- On rewind success for `conversation` or `both`, put the rewound prompt text back in the composer, as `shouldRestoreComposerForRewindMode` does. Add a public `Composer::set_text` (today `set_text` is private, [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) line 1156).
- On failure, show a toast with the error and do not change the composer.
- Build the menu with `PopupMenu`. Give each control an `AccessibleAction::Click` handler.

Files:

- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (trailing row, rewind event)
- [packages/desktop-gpui/crates/composer/src/composer.rs](packages/desktop-gpui/crates/composer/src/composer.rs) (public `set_text`)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (handle the rewind event: call the client, restore the composer, toast)

Interfaces:

- Consumes: `Client::rewind_modes(&self, agent_id: &str) -> RewindModes` (`conversation`, `files`, `both`: bool) from TASK-260
- Consumes: `Client::rewind(&self, agent_id: &str, message_id: &str, mode: RewindMode, cx: &mut App) -> Task<Result<(), String>>` from TASK-260

Expected result:

- Hovering a prompt shows "10:39 AM", the rewind icon, and the copy icon, as in the owner screenshot.
- Rewind conversation removes later turns and puts the prompt back in the composer.

Anti-goal: hover causes 0 px layout shift; the user message row height with and without hover is equal; read in a pure test of the row height and in a screenshot pair.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline -p bachuc-composer` -> new tests pass: timestamp format for same day, same week, and older; no rewind button without modes or `messageId`; row height equal with and without hover.
- Screen check (cua_repl, AX only): the footer controls exist in the AX tree with labels "Rewind to this message" and "Copy message".
- Owner hand check: hover shows the row; copy puts the prompt on the clipboard; rewind conversation on a live test agent removes later turns and restores the prompt; a failed rewind shows a toast and keeps the composer text; an old host shows no rewind button.
