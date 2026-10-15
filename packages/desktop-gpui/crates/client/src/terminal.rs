//! Terminal frames routed by slot. Binary frames are `[opcode, slot, payload...]`; see
//! `packages/protocol/src/binary-frames/terminal.ts`.

use futures::channel::mpsc::UnboundedReceiver;
use serde_json::Value;

pub(crate) const OUTPUT: u8 = 0x01;
pub(crate) const INPUT: u8 = 0x02;
pub(crate) const RESIZE: u8 = 0x03;
pub(crate) const SNAPSHOT: u8 = 0x04;
pub(crate) const RESTORE: u8 = 0x05;

/// One frame for a terminal view.
#[derive(Clone, Debug, PartialEq)]
pub enum TerminalFrame {
    /// PTY output bytes.
    Output(Vec<u8>),
    /// The full terminal state as a JSON cell grid (`TerminalStateSchema`). It comes first after
    /// create, attach, and every reconnect, so the view resets its emulator on it.
    Snapshot(Value),
}

/// A subscribed terminal. Frames arrive until the terminal exits or is killed; dropping the
/// receiver unsubscribes on the next frame.
pub struct TerminalHandle {
    pub id: String,
    pub frames: UnboundedReceiver<TerminalFrame>,
}

impl std::fmt::Debug for TerminalHandle {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("TerminalHandle")
            .field("id", &self.id)
            .finish()
    }
}

/// Parses one binary frame into its slot and view frame. Unknown opcodes give `None`.
pub(crate) fn parse_frame(bytes: &[u8]) -> Option<(u8, TerminalFrame)> {
    let (&opcode, rest) = bytes.split_first()?;
    let (&slot, payload) = rest.split_first()?;
    let frame = match opcode {
        // A restore frame is ANSI text that redraws the screen, so it plays as output.
        OUTPUT | RESTORE => TerminalFrame::Output(payload.to_vec()),
        SNAPSHOT => TerminalFrame::Snapshot(serde_json::from_slice(payload).ok()?),
        _ => return None,
    };
    Some((slot, frame))
}

pub(crate) fn encode(opcode: u8, slot: u8, payload: &[u8]) -> Vec<u8> {
    let mut frame = Vec::with_capacity(payload.len() + 2);
    frame.push(opcode);
    frame.push(slot);
    frame.extend_from_slice(payload);
    frame
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn frames_parse_by_opcode() {
        assert_eq!(
            parse_frame(&[OUTPUT, 3, b'h', b'i']),
            Some((3, TerminalFrame::Output(b"hi".to_vec())))
        );
        let snapshot = encode(SNAPSHOT, 1, br#"{"rows":2,"cols":3}"#);
        assert_eq!(
            parse_frame(&snapshot),
            Some((1, TerminalFrame::Snapshot(json!({"rows": 2, "cols": 3}))))
        );
        assert_eq!(parse_frame(&[SNAPSHOT, 1, b'{']), None);
        assert_eq!(parse_frame(&[0x09, 1]), None);
        assert_eq!(parse_frame(&[OUTPUT]), None);
    }
}
