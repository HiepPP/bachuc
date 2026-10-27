//! What the shell does after a delivered send (ADR-0071). The decision is pure; the shell runs
//! the step.

use bachuc_settings::after_send::AfterSend;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum AfterSendStep {
    /// Keep the thread open.
    Stay,
    /// Open the oldest other unread thread.
    OpenNext,
    OpenBoard,
    /// The Board plugin is not active: stay and say so.
    BoardMissing,
}

/// `still_viewed` is false when the user left the thread before the reply came back. A missing
/// Board never takes the user away from the thread.
pub fn decide_after_send(
    mode: AfterSend,
    still_viewed: bool,
    has_next: bool,
    board_active: bool,
) -> AfterSendStep {
    if !still_viewed {
        return AfterSendStep::Stay;
    }
    let board = if board_active {
        AfterSendStep::OpenBoard
    } else {
        AfterSendStep::BoardMissing
    };
    match mode {
        AfterSend::Stay => AfterSendStep::Stay,
        AfterSend::Board => board,
        AfterSend::NextUnread if has_next => AfterSendStep::OpenNext,
        AfterSend::NextUnread => board,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_settings::after_send::AfterSend;

    #[test]
    fn stay_never_moves() {
        assert_eq!(
            decide_after_send(AfterSend::Stay, true, true, true),
            AfterSendStep::Stay
        );
    }

    #[test]
    fn board_opens_the_board_or_reports_it_missing() {
        assert_eq!(
            decide_after_send(AfterSend::Board, true, true, true),
            AfterSendStep::OpenBoard
        );
        assert_eq!(
            decide_after_send(AfterSend::Board, true, true, false),
            AfterSendStep::BoardMissing
        );
    }

    #[test]
    fn next_unread_opens_the_next_then_falls_back_to_the_board() {
        assert_eq!(
            decide_after_send(AfterSend::NextUnread, true, true, false),
            AfterSendStep::OpenNext
        );
        assert_eq!(
            decide_after_send(AfterSend::NextUnread, true, false, true),
            AfterSendStep::OpenBoard
        );
        assert_eq!(
            decide_after_send(AfterSend::NextUnread, true, false, false),
            AfterSendStep::BoardMissing
        );
    }

    #[test]
    fn a_thread_left_before_the_reply_never_moves() {
        for mode in [AfterSend::Stay, AfterSend::Board, AfterSend::NextUnread] {
            assert_eq!(
                decide_after_send(mode, false, true, true),
                AfterSendStep::Stay
            );
        }
    }
}
