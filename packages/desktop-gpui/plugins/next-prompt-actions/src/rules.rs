//! The panel rules of `client/panel.tsx` that need no `App`: candidate identity, the card
//! faces, the busy rule, the selection texts, and the order of a Send.

use crate::contracts::{
    Candidate, CandidateState, HostInfo, Scope, SendInput, SendKey, SendOutput,
};
use crate::polling::{JEV_REVIEWING, agent_busy};
use crate::prompts::{PromptBlock, git_action};
use crate::selection::Group;
use bachuc_client::AttentionReason;
use bachuc_icons::IconName;
use regex::Regex;
use std::sync::LazyLock;

/// `width <= 600` of `actions.compact`.
const COMPACT_WIDTH: f32 = 600.;

pub(crate) fn is_compact(width: f32) -> bool {
    width <= COMPACT_WIDTH
}

static RFC3339: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"^([0-9]{4})-([0-9]{2})-([0-9]{2})[Tt ]([0-9]{2}):([0-9]{2}):([0-9]{2})(?:\.([0-9]+))?(?:([Zz])|([+-])([0-9]{2}):([0-9]{2}))$",
    )
    .unwrap()
});

/// Days from 1970-01-01 to a civil date (Howard Hinnant's `days_from_civil`).
fn days_from_civil(year: i64, month: i64, day: i64) -> i64 {
    let year = if month <= 2 { year - 1 } else { year };
    let era = year.div_euclid(400);
    let of_era = year - era * 400;
    let month_index = (month + 9) % 12;
    let of_year = (153 * month_index + 2) / 5 + day - 1;
    let of_cycle = of_era * 365 + of_era / 4 - of_era / 100 + of_year;
    era * 146097 + of_cycle - 719468
}

/// An RFC 3339 timeline timestamp in Unix ms; `None` when it does not parse.
pub(crate) fn timestamp_ms(text: &str) -> Option<i64> {
    let caps = RFC3339.captures(text)?;
    let number = |index: usize| caps.get(index).map(|m| m.as_str().parse::<i64>().unwrap());
    let (month, day) = (number(2)?, number(3)?);
    let (hour, minute, second) = (number(4)?, number(5)?, number(6)?);
    if !(1..=12).contains(&month) || !(1..=31).contains(&day) || hour > 23 || minute > 59 {
        return None;
    }
    let millis = caps.get(7).map_or(0, |m| {
        let digits: String = m.as_str().chars().chain("000".chars()).take(3).collect();
        digits.parse::<i64>().unwrap()
    });
    let offset = match caps.get(9) {
        Some(sign) => {
            let minutes = number(10)? * 60 + number(11)?;
            if sign.as_str() == "-" {
                -minutes
            } else {
                minutes
            }
        }
        None => 0,
    };
    let days = days_from_civil(number(1)?, month, day);
    let seconds = days * 86_400 + hour * 3600 + minute * 60 + second - offset * 60;
    Some(seconds * 1000 + millis)
}

/// The candidate of a prompt (`panel.tsx:661-669`): streamed and stored reply times differ, so
/// identity uses the user-turn bound `after`. A reply without a time matches none.
pub(crate) fn candidate_for<'a>(
    candidates: &'a [Candidate],
    item_ms: Option<i64>,
    block: &str,
    text: &str,
) -> Option<&'a Candidate> {
    let item_ms = item_ms? as f64;
    candidates.iter().find(|candidate| {
        candidate.after.is_some_and(|after| item_ms > after)
            && candidate.block == block
            && candidate.text == text
    })
}

/// `actions.busy`: an action is in flight, a read failed, the agent is not at rest, or Jev
/// reviews the reply.
pub(crate) fn panel_busy(
    pending: bool,
    load_error: bool,
    status: Option<&str>,
    attention: Option<AttentionReason>,
    note: Option<&str>,
) -> bool {
    pending || load_error || agent_busy(status, attention) || note == Some(JEV_REVIEWING)
}

/// What a prompt card's main button shows and does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) struct Face {
    pub label: &'static str,
    pub icon: IconName,
    /// The press starts a new thread instead of sending here.
    pub new_thread: bool,
    pub ready: bool,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct FaceInput {
    pub state: Option<CandidateState>,
    pub thread_new: bool,
    pub git_label: Option<&'static str>,
    /// The pointer is over the main button.
    pub hovered: bool,
    /// The platform modifier is held.
    pub modifier: bool,
    pub busy: bool,
    pub pending: bool,
}

/// The label and icon rule of `panel.tsx:211-233` and line 317.
pub(crate) fn card_face(input: FaceInput) -> Face {
    let ready_state = input.state == Some(CandidateState::Ready);
    let swapped = input.hovered && input.modifier && ready_state && !input.busy;
    let new_thread = input.git_label.is_none() && (input.thread_new != swapped);
    let sent = input.state == Some(CandidateState::Sent);
    let label = match input.state {
        Some(CandidateState::Unknown) if new_thread => "Check threads",
        Some(CandidateState::Unknown) => "Check chat",
        Some(CandidateState::Sending) if new_thread => "Starting...",
        Some(CandidateState::Sending) => "Sending...",
        Some(CandidateState::Sent) if input.thread_new => "Started",
        Some(CandidateState::Sent) => "Sent",
        _ if new_thread && input.thread_new => "Start in new thread",
        _ if new_thread => "New thread",
        _ => input.git_label.unwrap_or("Send"),
    };
    let icon = if sent {
        IconName::Check
    } else if new_thread {
        IconName::SquarePen
    } else if input.git_label.is_some() {
        IconName::GitCommitHorizontal
    } else {
        IconName::ArrowUp
    };
    Face {
        label,
        icon,
        new_thread,
        ready: ready_state && !input.pending && (new_thread || !input.busy),
    }
}

/// The two faces the main button reserves width for, when it has no Git action.
pub(crate) fn reserved_faces(thread_new: bool) -> [(&'static str, IconName); 2] {
    if thread_new {
        [
            ("Start in new thread", IconName::SquarePen),
            ("Send", IconName::ArrowUp),
        ]
    } else {
        [
            ("Send", IconName::ArrowUp),
            ("New thread", IconName::SquarePen),
        ]
    }
}

/// The Git action label of a candidate, which only a same-thread candidate gets.
pub(crate) fn git_label(candidate: &Candidate) -> Option<&'static str> {
    (!candidate.thread_new())
        .then(|| git_action(&candidate.text).map(|action| action.label))
        .flatten()
}

/// The reply's own fence decides, so earlier replies without candidates keep their badge.
pub(crate) fn is_suggested(
    block: &PromptBlock,
    index: usize,
    candidate: Option<&Candidate>,
) -> bool {
    block
        .suggestions
        .as_ref()
        .is_some_and(|marks| marks.get(index) == Some(&true))
        || candidate.is_some_and(|c| c.suggestion == Some(true))
}

/// A block shows as a selection when it declares relations, has more than one local prompt, a
/// candidate for each, and no Git action.
pub(crate) fn shows_selection(block: &PromptBlock, candidates: &[Option<&Candidate>]) -> bool {
    let local: Vec<usize> = (0..block.prompts.len())
        .filter(|&index| !block.threads[index])
        .collect();
    block.declaration.is_some()
        && local.len() > 1
        && local.iter().all(|&index| candidates[index].is_some())
        && !local
            .iter()
            .any(|&index| git_action(&block.prompts[index]).is_some())
}

pub(crate) fn group_title(group: &Group) -> String {
    match group {
        Group::Choice { .. } => "Choose one".into(),
        Group::Together { set: 0, .. } => "Send together".into(),
        Group::Together { set, .. } => format!("Send together · Set {set}"),
        Group::FollowUp { .. } => "Follow-up".into(),
        Group::Alone { .. } => "Send alone".into(),
    }
}

/// The status text of a selection row (`panel.tsx:433-444`).
pub(crate) fn row_status(
    state: Option<CandidateState>,
    allowed: bool,
    combined: bool,
) -> &'static str {
    match state {
        Some(CandidateState::Sent) => "Sent",
        Some(CandidateState::Unknown) => "Check chat",
        Some(CandidateState::Sending) => "Sending...",
        _ if allowed => "",
        _ if combined => "Not with selection",
        _ => "Send alone",
    }
}

/// The footer hint under the selected count (`panel.tsx:537-545`).
pub(crate) fn footer_hint(chosen: usize, allowed: bool) -> &'static str {
    match chosen {
        0 => "Choose a suggestion to continue",
        1 => "One prompt",
        _ if allowed => "One numbered message",
        _ => "Add or remove a suggestion to match a set",
    }
}

/// The Send selected label; `None` states are chosen ids without a candidate.
pub(crate) fn send_selected_label(pending: bool, chosen: &[Option<CandidateState>]) -> String {
    let any = |state: CandidateState| chosen.contains(&Some(state));
    if pending {
        "Sending...".into()
    } else if any(CandidateState::Unknown) {
        "Check chat".into()
    } else if !chosen.is_empty() && chosen.iter().all(|c| *c == Some(CandidateState::Sent)) {
        "Sent".into()
    } else if any(CandidateState::Sending) {
        "Sending...".into()
    } else if !chosen.is_empty() {
        format!("Send selected ({})", chosen.len())
    } else {
        "Send selected".into()
    }
}

/// One step of a Send, in order.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum SendStep {
    /// Mark the agent's turn pending in the client, so the Board shows it running at once.
    MarkPending(String),
    /// Leave for the Board at once; it refreshes when the prompt lands.
    OpenBoard,
    Rpc(SendInput),
}

pub(crate) fn send_plan(back_to_board: bool, scope: Scope, keys: Vec<String>) -> Vec<SendStep> {
    let mark = SendStep::MarkPending(scope.agent_id.clone());
    let rpc = SendStep::Rpc(SendInput {
        scope,
        key: SendKey::of(keys),
    });
    if back_to_board {
        vec![mark, SendStep::OpenBoard, rpc]
    } else {
        vec![mark, rpc]
    }
}

/// Only an acknowledged send keeps the pending mark.
pub(crate) fn send_landed(result: &anyhow::Result<SendOutput>) -> bool {
    matches!(result, Ok(output) if output.sent)
}

/// The toggle command shows only while Jev can judge.
pub(crate) fn command_visible(host: Option<&HostInfo>) -> bool {
    host.is_some_and(|host| host.evaluator)
}

#[cfg(test)]
mod tests {
    use super::{
        Face, FaceInput, SendStep, candidate_for, card_face, command_visible, footer_hint,
        is_compact, panel_busy, row_status, send_landed, send_plan, send_selected_label,
        shows_selection, timestamp_ms,
    };
    use crate::contracts::{Candidate, CandidateState, HostInfo, Scope, SendOutput};
    use crate::polling::JEV_REVIEWING;
    use crate::prompts::parse_prompts;
    use bachuc_client::AttentionReason;
    use bachuc_icons::IconName;

    fn face(state: CandidateState, thread_new: bool, hovered: bool, modifier: bool) -> Face {
        card_face(FaceInput {
            state: Some(state),
            thread_new,
            hovered,
            modifier,
            ..FaceInput::default()
        })
    }

    #[test]
    fn card_face_rule() {
        use CandidateState::{Ready, Sending, Sent, Unknown};
        let ready = face(Ready, false, false, false);
        assert_eq!((ready.label, ready.icon), ("Send", IconName::ArrowUp));
        assert!(ready.ready && !ready.new_thread);
        // The modifier alone swaps nothing; held over Send it starts a new thread.
        assert_eq!(face(Ready, false, false, true).label, "Send");
        let swapped = face(Ready, false, true, true);
        assert_eq!(
            (swapped.label, swapped.icon),
            ("New thread", IconName::SquarePen)
        );
        assert!(swapped.new_thread);
        assert_eq!(face(Ready, true, false, false).label, "Start in new thread");
        assert_eq!(face(Ready, true, true, true).label, "Send");
        assert_eq!(face(Sending, false, false, false).label, "Sending...");
        assert_eq!(face(Sending, true, false, false).label, "Starting...");
        let sent = face(Sent, false, false, false);
        assert_eq!((sent.label, sent.icon), ("Sent", IconName::Check));
        assert_eq!(face(Sent, true, false, false).label, "Started");
        assert_eq!(face(Unknown, false, false, false).label, "Check chat");
        assert_eq!(face(Unknown, true, false, false).label, "Check threads");
        assert!(!face(Sent, false, false, false).ready);
        let git = card_face(FaceInput {
            state: Some(Ready),
            git_label: Some("Commit & Push"),
            hovered: true,
            modifier: true,
            ..FaceInput::default()
        });
        assert_eq!(
            (git.label, git.icon, git.new_thread),
            ("Commit & Push", IconName::GitCommitHorizontal, false)
        );
        // Busy keeps Send disabled; a new thread may still start, and pending blocks both.
        let busy = |thread_new| {
            card_face(FaceInput {
                state: Some(Ready),
                thread_new,
                busy: true,
                ..FaceInput::default()
            })
        };
        assert!(!busy(false).ready);
        assert!(busy(true).ready);
        let pending = card_face(FaceInput {
            state: Some(Ready),
            thread_new: true,
            pending: true,
            ..FaceInput::default()
        });
        assert!(!pending.ready);
        assert!(is_compact(0.) && is_compact(600.) && !is_compact(601.));
    }

    fn candidate(block: &str, text: &str, after: Option<f64>) -> Candidate {
        Candidate {
            key: format!("{block}/{text}"),
            block: block.into(),
            text: text.into(),
            after,
            state: CandidateState::Ready,
            ..Candidate::default()
        }
    }

    #[test]
    fn candidate_match_rule() {
        let at = timestamp_ms("2026-10-09T06:00:00.000Z").unwrap();
        assert_eq!(at, 1_791_525_600_000);
        assert_eq!(
            timestamp_ms("2026-10-09T13:00:00.250+07:00"),
            Some(at + 250)
        );
        let candidates = vec![
            candidate("b", "older", Some((at + 1) as f64)),
            candidate("b", "t", None),
            candidate("b", "t", Some((at - 1) as f64)),
        ];
        // Only a candidate whose user turn came before this reply belongs to it.
        assert_eq!(candidate_for(&candidates, Some(at), "b", "older"), None);
        assert_eq!(
            candidate_for(&candidates, Some(at), "b", "t"),
            Some(&candidates[2])
        );
        assert_eq!(candidate_for(&candidates, Some(at), "other", "t"), None);
        assert_eq!(candidate_for(&candidates, Some(at), "b", "x"), None);
    }

    #[test]
    fn missing_timestamp_no_candidate() {
        let candidates = vec![candidate("b", "t", Some(0.))];
        assert_eq!(candidate_for(&candidates, None, "b", "t"), None);
        assert_eq!(timestamp_ms("yesterday"), None);
        assert_eq!(timestamp_ms(""), None);
        assert_eq!(
            candidate_for(&candidates, timestamp_ms("2026-13-01T00:00:00Z"), "b", "t"),
            None
        );
    }

    #[test]
    fn busy_rule() {
        assert!(!panel_busy(false, false, Some("idle"), None, Some("")));
        assert!(!panel_busy(false, false, Some("closed"), None, None));
        assert!(panel_busy(false, false, Some("running"), None, None));
        assert!(panel_busy(
            false,
            false,
            Some("idle"),
            Some(AttentionReason::Permission),
            None
        ));
        assert!(panel_busy(true, false, Some("idle"), None, None));
        assert!(panel_busy(false, true, Some("idle"), None, None));
        assert!(panel_busy(
            false,
            false,
            Some("idle"),
            None,
            Some(JEV_REVIEWING)
        ));
        // A running or permission-waiting agent disables every action button.
        let send = card_face(FaceInput {
            state: Some(CandidateState::Ready),
            busy: panel_busy(false, false, Some("running"), None, None),
            ..FaceInput::default()
        });
        assert!(!send.ready);
    }

    #[test]
    fn selection_footer_texts() {
        use CandidateState::{Ready, Sending, Sent, Unknown};
        assert_eq!(footer_hint(0, false), "Choose a suggestion to continue");
        assert_eq!(footer_hint(1, true), "One prompt");
        assert_eq!(footer_hint(2, true), "One numbered message");
        assert_eq!(
            footer_hint(2, false),
            "Add or remove a suggestion to match a set"
        );
        assert_eq!(send_selected_label(false, &[]), "Send selected");
        assert_eq!(
            send_selected_label(false, &[Some(Ready), Some(Ready)]),
            "Send selected (2)"
        );
        assert_eq!(send_selected_label(true, &[Some(Ready)]), "Sending...");
        assert_eq!(
            send_selected_label(false, &[Some(Sent), Some(Unknown)]),
            "Check chat"
        );
        assert_eq!(
            send_selected_label(false, &[Some(Sent), Some(Sent)]),
            "Sent"
        );
        assert_eq!(
            send_selected_label(false, &[Some(Sent), Some(Sending)]),
            "Sending..."
        );
        assert_eq!(row_status(Some(Sent), true, true), "Sent");
        assert_eq!(row_status(Some(Unknown), true, true), "Check chat");
        assert_eq!(row_status(Some(Sending), true, true), "Sending...");
        assert_eq!(row_status(Some(Ready), true, true), "");
        assert_eq!(row_status(Some(Ready), false, true), "Not with selection");
        assert_eq!(row_status(Some(Ready), false, false), "Send alone");
    }

    #[test]
    fn a_declared_block_with_a_candidate_per_local_prompt_shows_as_a_selection() {
        let text = "## What Next\n```next-prompts\n{\"version\":1,\"prompts\":[{\"id\":\"a\",\"prompt\":\"A.\"},{\"id\":\"b\",\"prompt\":\"B.\"}],\"allowedCombinations\":[[\"a\",\"b\"]]}\n```";
        let block = &parse_prompts(text)[0];
        let one = Candidate::default();
        assert!(shows_selection(block, &[Some(&one), Some(&one)]));
        assert!(!shows_selection(block, &[Some(&one), None]));
        let plain = &parse_prompts("## What Next\n```\nprompt: A.\nprompt: B.\n```")[0];
        assert!(!shows_selection(plain, &[Some(&one), Some(&one)]));
    }

    #[test]
    fn send_marks_the_turn_before_it_opens_the_board() {
        let scope = Scope {
            server_id: "s".into(),
            agent_id: "a".into(),
            workspace_id: "w".into(),
        };
        let on = send_plan(true, scope.clone(), vec!["k".into()]);
        assert_eq!(on.len(), 3);
        assert_eq!(on[0], SendStep::MarkPending("a".into()));
        assert_eq!(on[1], SendStep::OpenBoard);
        assert!(matches!(on[2], SendStep::Rpc(_)));
        let off = send_plan(false, scope, vec!["k".into()]);
        assert!(matches!(&off[..], [SendStep::MarkPending(id), SendStep::Rpc(_)] if id == "a"));
    }

    #[test]
    fn only_an_acknowledged_send_keeps_the_mark() {
        assert!(send_landed(&Ok(SendOutput { sent: true })));
        assert!(!send_landed(&Ok(SendOutput { sent: false })));
        assert!(!send_landed(&Err(anyhow::anyhow!("Agent is busy."))));
    }

    #[test]
    fn command_hidden_without_evaluator() {
        let host = |evaluator| HostInfo {
            server_id: "s".into(),
            evaluator,
        };
        assert!(!command_visible(Some(&host(false))));
        assert!(!command_visible(None), "hidden until the host read lands");
        assert!(command_visible(Some(&host(true))));
        let decoded: HostInfo =
            serde_json::from_value(serde_json::json!({"serverId": "s", "evaluator": false}))
                .unwrap();
        assert!(!command_visible(Some(&decoded)));
    }
}
