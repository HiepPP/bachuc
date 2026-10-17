//! The composer's decisions that need no window: what a send does, which primary button shows,
//! and how tall the input grows.

use bachuc_client::TurnBehavior;

/// What a send does. `Send` carries the behavior for a turn that is already running.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SendPlan {
    Send(TurnBehavior),
    Queue,
}

/// Enter steers the running turn (the Electron default `sendBehavior`). Cmd+Enter queues the
/// prompt behind the turn when the daemon supports a queue, and steers when it does not. With
/// no turn running both just send.
pub fn send_plan(running: bool, queue_key: bool, supports_queue: bool) -> SendPlan {
    if running && queue_key && supports_queue {
        SendPlan::Queue
    } else {
        SendPlan::Send(TurnBehavior::Steer)
    }
}

/// The button at the right end of the toolbar.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PrimaryAction {
    Send,
    Stop,
    None,
}

/// `resolvePrimaryActionKind` in `input.tsx`: Send whenever there is something to send, even
/// during a run; Stop only while the agent runs with nothing to send; else nothing.
pub fn primary_action(has_content: bool, running: bool) -> PrimaryAction {
    if has_content {
        PrimaryAction::Send
    } else if running {
        PrimaryAction::Stop
    } else {
        PrimaryAction::None
    }
}

/// The input line height: the content size at a 1.4 line height (21 px at the default 15 px).
pub fn line_height(font_size: f32) -> f32 {
    font_size * 1.4
}

/// The input stops growing at half the window height, and never below 160 px.
pub fn max_input_height(window_height: f32) -> f32 {
    (window_height * 0.5).max(160.)
}

/// How many lines fit in the maximum input height. The input is at least one line.
pub fn max_input_rows(window_height: f32, font_size: f32) -> usize {
    ((max_input_height(window_height) / line_height(font_size)).floor() as usize).max(1)
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_theme::metrics::FONT_CONTENT;

    /// The input height for `lines` lines: one line high, plus one line height per extra line, up
    /// to the maximum. The textarea grows by rows; this is the height those rows make.
    fn input_height(lines: usize, window_height: f32, font_size: f32) -> f32 {
        let line = line_height(font_size);
        (line * lines.max(1) as f32).min(max_input_height(window_height))
    }

    fn default_line_height() -> f32 {
        line_height(FONT_CONTENT)
    }

    #[test]
    fn enter_steers_a_running_turn() {
        assert_eq!(
            send_plan(true, false, true),
            SendPlan::Send(TurnBehavior::Steer)
        );
        assert_eq!(
            send_plan(true, false, false),
            SendPlan::Send(TurnBehavior::Steer)
        );
    }

    #[test]
    fn cmd_enter_queues_only_with_a_queue_on_the_daemon() {
        assert_eq!(send_plan(true, true, true), SendPlan::Queue);
        assert_eq!(
            send_plan(true, true, false),
            SendPlan::Send(TurnBehavior::Steer)
        );
    }

    #[test]
    fn nothing_queues_when_no_turn_runs() {
        assert_eq!(
            send_plan(false, true, true),
            SendPlan::Send(TurnBehavior::Steer)
        );
        assert_eq!(
            send_plan(false, false, true),
            SendPlan::Send(TurnBehavior::Steer)
        );
    }

    #[test]
    fn the_primary_button_follows_content_and_run_state() {
        assert_eq!(primary_action(false, false), PrimaryAction::None);
        assert_eq!(primary_action(true, false), PrimaryAction::Send);
        assert_eq!(primary_action(true, true), PrimaryAction::Send);
        assert_eq!(primary_action(false, true), PrimaryAction::Stop);
    }

    #[test]
    fn one_line_is_21_px_and_each_line_adds_21() {
        assert_eq!(default_line_height(), 21.);
        assert_eq!(input_height(1, 800., FONT_CONTENT), 21.);
        assert_eq!(input_height(2, 800., FONT_CONTENT), 42.);
        assert_eq!(input_height(0, 800., FONT_CONTENT), 21.);
    }

    #[test]
    fn the_input_stops_at_half_the_window_and_never_below_160() {
        assert_eq!(max_input_height(1000.), 500.);
        assert_eq!(max_input_height(400.), 200.);
        assert_eq!(max_input_height(200.), 160.);
        assert_eq!(input_height(100, 1000., FONT_CONTENT), 500.);
        assert_eq!(input_height(100, 200., FONT_CONTENT), 160.);
    }

    #[test]
    fn the_row_limit_fits_in_the_maximum_height() {
        let rows = max_input_rows(1000., FONT_CONTENT);
        assert_eq!(rows, 23);
        assert!(rows as f32 * 21. <= max_input_height(1000.));
        assert_eq!(max_input_rows(200., FONT_CONTENT), 7);
        assert_eq!(max_input_rows(0., 200.), 1);
    }
}
