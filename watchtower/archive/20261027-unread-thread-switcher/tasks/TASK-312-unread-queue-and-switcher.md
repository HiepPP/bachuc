# TASK-312 Unread queue and Mod+J switcher

Group: D (shared file packages/desktop-gpui/crates/app/src/shell.rs with TASK-313)
Class: risky

## Brief

Goal: From any thread, Hiep presses Mod+J to reach the oldest unread finished thread, Cmd+Tab style, and Mod+Shift+J to return to the thread just left.

Change: no unread navigation -> a pure queue model, a switcher popover, and two new shortcut actions.

Boundaries:

- Follows [ADR-0070](watchtower/decisions/ADR-0070-host-unread-switcher-over-board-mini-board.md): host code, unread-only FIFO queue, popover, Mod+J keys.
- Queue rule and host scope are in [watchtower/CONTEXT.md](watchtower/CONTEXT.md). The shell fills `attention_at` from `agent_attention_timestamp`, else `agent_updated_at`. The queue sorts by `attention_at`, then agent id. ISO timestamps compare as strings. An entry with neither timestamp sorts last.
- Keys, Cmd+Tab style. Mod+J (`Cmd+J`) opens the switcher with the oldest row selected. While Cmd is held, each J press moves down and Shift+J moves up, with wrap. Releasing Cmd opens the selected row. Enter opens it too. Arrows move. Esc closes and keeps the view.
- A quick tap opens the oldest thread with no flash: the popover draws only when Cmd is still held 150 ms after the press.
- Mod+Shift+J (`Cmd+Shift+J`) opens the most recent viewed agent that is not the current view. On a thread, that is the thread before it, so two presses toggle. On the Board or another page, that is the last thread. A page visit does not count as a view. With no such agent, or an agent that left `Client::agents`, it does nothing.
- Planner call: keep this in a pure `ViewHistory` in `unread.rs` with `visit(&mut self, viewed: Option<&str>)` and `back(&self, current: Option<&str>) -> Option<String>`. `sync_viewed_agent` calls `visit` with the new `viewed_agent`.
- While the switcher shows, the actions route to it: `agent.unread.switcher` (Cmd+J) moves the selection down and `agent.previous` (Cmd+Shift+J) moves it up. Neither opens a thread nor runs Back.
- An empty queue on Mod+J shows the toast "No unread threads" and opens no popover.
- A row shows the agent title, the workspace name, the reason (finished or error), and the age. No reply preview, per Q-195.
- Opening a row goes through `Shell::open_agent`. That path already clears attention through `sync_viewed_agent`.
- Trap: `CommandCenter` in [packages/desktop-gpui/crates/command/src/center.rs](packages/desktop-gpui/crates/command/src/center.rs) is the popover pattern to copy, for focus, Esc, dismissal, and the `Shell::command` slot. Do not put the switcher rows into the command center list.
- Trap: GPUI reports Cmd release through `on_modifiers_changed` with `ModifiersChangedEvent`, as in [packages/desktop-gpui/crates/terminal/src/view.rs](packages/desktop-gpui/crates/terminal/src/view.rs). The popover must hold focus, or it never sees the release.
- Trap: `binding_specs` in `actions.rs` builds the keymap from the shortcut table. Add the two actions to `DEFAULTS` in `shortcuts.rs`, not as hand-made `KeyBinding`s, so the settings pane can rebind them. Put them in the `Workspaces` section with labels "Open unread thread" and "Back to previous thread".
- Trap: the timeline and the composer must not take Cmd+J. Check that the bindings fire while the composer has focus. Use the shell context, as `command-center.toggle` does.
- The switcher closes on a host switch: clear its slot and reset `ViewHistory` in `reconnect`, where `self.command` is already cleared. It drops a row whose agent leaves `Client::agents`, and it closes when no rows are left.
- Trap: `CommandCenter` owns an `InputState`; the switcher has no input. Give it a plain `FocusHandle`, focus it on open, and put `on_modifiers_changed` and `on_key_down` on its root element.

How:

- Add `unread.rs` with `QueueEntry`, `unread_queue`, and `ViewHistory`; build entries from `Client` in the shell.
- Add `unread_switcher.rs` with the `UnreadSwitcher` view and its `UnreadSwitcherEvent`.
- In `shell.rs`, handle the actions `agent.unread.switcher` and `agent.previous` in `run_action`; track `previous_agent` where `sync_viewed_agent` swaps `viewed_agent`; add `open_next_unread`.
- Add the two `DEFAULTS` rows in `shortcuts.rs` and the modules in `main.rs`.

Files:

- [packages/desktop-gpui/crates/app/src/unread.rs](packages/desktop-gpui/crates/app/src/unread.rs) (new: queue model and tests)
- [packages/desktop-gpui/crates/app/src/unread_switcher.rs](packages/desktop-gpui/crates/app/src/unread_switcher.rs) (new: popover view)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (actions, previous agent, `open_next_unread`, popover slot)
- [packages/desktop-gpui/crates/app/src/main.rs](packages/desktop-gpui/crates/app/src/main.rs) (module declarations)
- [packages/desktop-gpui/crates/command/src/shortcuts.rs](packages/desktop-gpui/crates/command/src/shortcuts.rs) (two `DEFAULTS` rows)
- [packages/desktop-gpui/crates/app/src/actions.rs](packages/desktop-gpui/crates/app/src/actions.rs) (binding test only, if a context change is needed)

Interfaces:

- Consumes: `Client::agent_attention_timestamp(&self, agent_id: &str) -> Option<String>` from TASK-309
- Produces: `Shell::open_next_unread(&mut self, exclude: Option<&str>, window: &mut Window, cx: &mut Context<Shell>) -> bool` for TASK-313. It opens the oldest queue entry other than `exclude` and returns `false`, with no toast, when there is none.

Expected result:

- WHEN Hiep taps Mod+J and the queue has entries THE shell SHALL open the oldest unread finished or error thread.
- WHILE Cmd is held after Mod+J, WHEN 150 ms pass THE switcher SHALL show the queue with the oldest row selected.
- WHILE the switcher shows, WHEN Hiep presses J, Shift+J, or an arrow THE switcher SHALL move the selection, with wrap.
- WHEN Hiep releases Cmd or presses Enter THE shell SHALL open the selected thread and close the switcher.
- WHEN Hiep presses Esc THE switcher SHALL close and the view SHALL not change.
- IF the queue is empty on Mod+J THEN THE shell SHALL show the toast "No unread threads" and open nothing.
- WHEN Hiep presses Mod+Shift+J THE shell SHALL open the thread the user left last.
- IF there is no previous thread THEN THE shell SHALL do nothing on Mod+Shift+J.
- IF an agent is a subagent, has reason `Permission`, is not flagged, or is the viewed agent THEN THE queue SHALL leave it out.
- IF the host switches while the switcher shows THEN THE switcher SHALL close.
- WHILE the switcher shows, WHEN Hiep presses Cmd+J or Cmd+Shift+J THE switcher SHALL move down or up, and SHALL not open a thread.
- IF the last row drops while the switcher shows THEN THE switcher SHALL close.
- WHEN Hiep presses Mod+Shift+J on the Board page THE shell SHALL open the last viewed thread.

Red tests:

File: [packages/desktop-gpui/crates/app/src/unread.rs](packages/desktop-gpui/crates/app/src/unread.rs)

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_client::AttentionReason;

    fn entry(id: &str, reason: Option<AttentionReason>, at: Option<&str>) -> QueueEntry {
        QueueEntry {
            agent_id: id.into(),
            requires_attention: true,
            reason,
            is_subagent: false,
            attention_at: at.map(String::from),
        }
    }

    #[test]
    fn the_oldest_finished_or_error_thread_comes_first() {
        let entries = [
            entry("new", Some(AttentionReason::Finished), Some("2026-10-27T05:00:00.000Z")),
            entry("old", Some(AttentionReason::Error), Some("2026-10-27T04:00:00.000Z")),
            entry("none", Some(AttentionReason::Finished), None),
        ];
        assert_eq!(unread_queue(&entries, None), ["old", "new", "none"]);
    }

    #[test]
    fn permission_subagents_read_threads_and_the_viewed_thread_stay_out() {
        let mut sub = entry("sub", Some(AttentionReason::Finished), Some("1"));
        sub.is_subagent = true;
        let mut read = entry("read", Some(AttentionReason::Finished), Some("2"));
        read.requires_attention = false;
        let entries = [
            entry("perm", Some(AttentionReason::Permission), Some("0")),
            sub,
            read,
            entry("viewed", Some(AttentionReason::Finished), Some("3")),
            entry("keep", Some(AttentionReason::Finished), Some("4")),
            entry("no-reason", None, Some("5")),
        ];
        assert_eq!(unread_queue(&entries, Some("viewed")), ["keep"]);
    }

    #[test]
    fn equal_timestamps_sort_by_agent_id() {
        let entries = [
            entry("b", Some(AttentionReason::Finished), Some("1")),
            entry("a", Some(AttentionReason::Finished), Some("1")),
        ];
        assert_eq!(unread_queue(&entries, None), ["a", "b"]);
    }

    #[test]
    fn no_entries_give_an_empty_queue() {
        assert!(unread_queue(&[], None).is_empty());
    }

    #[test]
    fn back_toggles_between_two_threads() {
        let mut history = ViewHistory::default();
        history.visit(Some("a"));
        history.visit(Some("b"));
        assert_eq!(history.back(Some("b")).as_deref(), Some("a"));
        history.visit(Some("a"));
        assert_eq!(history.back(Some("a")).as_deref(), Some("b"));
    }

    #[test]
    fn a_page_visit_keeps_the_last_thread() {
        let mut history = ViewHistory::default();
        history.visit(Some("a"));
        history.visit(Some("b"));
        history.visit(None);
        assert_eq!(history.back(None).as_deref(), Some("b"));
    }

    #[test]
    fn no_history_gives_no_back() {
        let mut history = ViewHistory::default();
        assert_eq!(history.back(None), None);
        history.visit(Some("a"));
        assert_eq!(history.back(Some("a")), None);
    }
}
```

`unread_queue` returns `Vec<String>`. `QueueEntry` has the fields `agent_id: String`, `requires_attention: bool`, `reason: Option<AttentionReason>`, `is_subagent: bool`, and `attention_at: Option<String>`. The shell fills `attention_at` from `agent_attention_timestamp`, else `agent_updated_at`.

Anti-goal: the existing shortcut chords keep their keys, 0 changed or lost bindings; tripwire; read from `every_assigned_shortcut_binds_and_digits_bind_nine_tabs` in `actions.rs` and a diff of `DEFAULTS` rows before the change, after the shortcut rows, and at completion. Only the two new rows may differ.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app unread` -> the 7 queue and history tests pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app -p bachuc-command` -> all tests pass, including `every_assigned_shortcut_binds_and_digits_bind_nine_tabs`.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-app --no-deps -- -D warnings` -> no warnings.
- `git diff packages/desktop-gpui/crates/command/src/shortcuts.rs` -> only two added `DEFAULTS` rows in the `Workspaces` section (titled "Projects & Workspaces"), with `Cmd+J` and `Cmd+Shift+J`.
- Screen check (UI checker, live GPUI app, AX actions only): with 2 unread finished agents, focus a third thread's composer and send Cmd+J as a tap -> the older unread thread opens.
- Screen check: hold Cmd, press J twice -> the switcher shows, the second row is selected; release Cmd -> that thread opens.
- Screen check: Cmd+J with no unread agents -> the toast "No unread threads" shows and the view does not change.
- Screen check: Cmd+Shift+J -> the thread left last opens; a second Cmd+Shift+J returns.
- Screen check: open thread A, open the Board, press Cmd+Shift+J -> thread A opens.
- Screen check: hold Cmd, press J, then Shift+J -> the selection moves down, then up, and no thread opens until Cmd is released.
- Screen check: with 1 unread agent that is the viewed thread, press Cmd+J -> the toast "No unread threads" shows.
- Screen check: open the switcher, then archive one listed agent from the CLI -> its row drops; Esc closes with no view change.
- Screen check: open the switcher, then press Cmd+` to switch hosts -> the switcher closes.
