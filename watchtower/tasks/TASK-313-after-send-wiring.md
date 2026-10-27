# TASK-313 After send wiring

Group: D (shared file packages/desktop-gpui/crates/app/src/shell.rs with TASK-312; also packages/desktop-gpui/crates/app/src/after_send.rs and packages/desktop-gpui/crates/app/src/main.rs)
Class: code

## Brief

Goal: After a good send, the shell applies the After send setting: stay, open the Board, or open the next unread thread.

Change: a send always stays in the thread -> the shell reads `AfterSend` on `ComposerEvent::Delivered` and acts.

Boundaries:

- Follows [ADR-0071](watchtower/decisions/ADR-0071-after-send-setting-over-fixed-jump.md).
- Read the setting with `bachuc_settings::after_send::load()` at each `Delivered`, so a settings change applies at once with no watch.
- `Board` opens the page surface `board` of the plugin `board` through `Shell::open_surface`. If the plugin is not active, stay and show the toast "Board is not installed".
- `NextUnread` calls `open_next_unread` with the sent agent as `exclude`. If it returns `false`, open the Board as in `Board` mode, with the same toast when the Board is not active.
- Check Board activity with `PluginHost::is_active("board")` on `Shell::plugin_host`. Build the `OpenSurface` as the sidebar does: plugin `board`, surface `board`, `SurfacePresentation::Page`, empty params.
- Planner call: keep the decision in a pure `decide_after_send` in a new `after_send.rs`, so the EARS lines get unit tests and `shell.rs` holds only a thin call.
- Act only when the `Delivered` agent is still the viewed agent. If Hiep moved to another thread before the reply came back, do nothing.
- A failed send never emits `Delivered`, so the thread stays with its send error.

How:

- Add `after_send.rs` with `AfterSendStep` and `decide_after_send`; declare it in `main.rs`.
- Handle `ComposerEvent::Delivered` in the composer subscription in `open_agent_pane` in `shell.rs`; add `Shell::after_send`, which calls `decide_after_send` and runs the step.

Files:

- [packages/desktop-gpui/crates/app/src/after_send.rs](packages/desktop-gpui/crates/app/src/after_send.rs) (new: pure decision and tests)
- [packages/desktop-gpui/crates/app/src/main.rs](packages/desktop-gpui/crates/app/src/main.rs) (module declaration)
- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (Delivered handler and `after_send`)

Interfaces:

- Consumes: `ComposerEvent::Delivered { agent_id: String }` from TASK-310
- Consumes: `bachuc_settings::after_send::AfterSend` enum `{ Stay, Board, NextUnread }` with `Default` = `Stay`, from TASK-311
- Consumes: `bachuc_settings::after_send::load() -> AfterSend` from TASK-311
- Consumes: `Shell::open_next_unread(&mut self, exclude: Option<&str>, window: &mut Window, cx: &mut Context<Shell>) -> bool` from TASK-312

Expected result:

- WHILE the mode is Stay, WHEN a send is delivered THE shell SHALL keep the thread open.
- WHILE the mode is Board, WHEN a send is delivered THE shell SHALL open the Board page.
- WHILE the mode is Next unread, WHEN a send is delivered THE shell SHALL open the oldest other unread thread.
- WHILE the mode is Next unread, IF the queue is empty THEN THE shell SHALL open the Board page.
- IF the Board plugin is not active THEN THE shell SHALL keep the thread open and show "Board is not installed".
- IF the send fails THEN THE shell SHALL keep the thread open with its send error.
- IF Hiep left the thread before the reply came back THEN THE shell SHALL not move.

Red tests:

File: [packages/desktop-gpui/crates/app/src/after_send.rs](packages/desktop-gpui/crates/app/src/after_send.rs)

`decide_after_send(mode: AfterSend, still_viewed: bool, has_next: bool, board_active: bool) -> AfterSendStep`, with `AfterSendStep` `{ Stay, OpenNext, OpenBoard, BoardMissing }`. The shell runs `OpenNext` through `open_next_unread`, and shows the toast "Board is not installed" for `BoardMissing`.

```rust
#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_settings::after_send::AfterSend;

    #[test]
    fn stay_never_moves() {
        assert_eq!(decide_after_send(AfterSend::Stay, true, true, true), AfterSendStep::Stay);
    }

    #[test]
    fn board_opens_the_board_or_reports_it_missing() {
        assert_eq!(decide_after_send(AfterSend::Board, true, true, true), AfterSendStep::OpenBoard);
        assert_eq!(decide_after_send(AfterSend::Board, true, true, false), AfterSendStep::BoardMissing);
    }

    #[test]
    fn next_unread_opens_the_next_then_falls_back_to_the_board() {
        assert_eq!(decide_after_send(AfterSend::NextUnread, true, true, false), AfterSendStep::OpenNext);
        assert_eq!(decide_after_send(AfterSend::NextUnread, true, false, true), AfterSendStep::OpenBoard);
        assert_eq!(decide_after_send(AfterSend::NextUnread, true, false, false), AfterSendStep::BoardMissing);
    }

    #[test]
    fn a_thread_left_before_the_reply_never_moves() {
        for mode in [AfterSend::Stay, AfterSend::Board, AfterSend::NextUnread] {
            assert_eq!(decide_after_send(mode, false, true, true), AfterSendStep::Stay);
        }
    }
}
```

Anti-goal: with the default Stay mode, a send changes nothing in the view, 0 navigations; tripwire; read by a screen check of one send with no `after-send.json` before the change is wired and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-app --no-deps -- -D warnings` -> no warnings.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app after_send` -> the 4 decision tests pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app` -> all tests pass.
- Screen check (UI checker, live GPUI app): no `after-send.json`, send a prompt -> the thread stays open.
- Screen check: mode Board, send a prompt -> the Board page opens.
- Screen check: mode Next unread with 1 other unread agent, send a prompt -> that agent opens.
- Screen check: mode Next unread with no other unread agent, send a prompt -> the Board page opens.
- Screen check: mode Next unread, stop the live daemon's agent provider or send to an archived agent so the send fails -> the thread stays with its send error. Mark UNVERIFIED if no failing send can be made on live.
- Screen check: mode Board with the Board plugin disabled on live -> the thread stays and the toast "Board is not installed" shows.
