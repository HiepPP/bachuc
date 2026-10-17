//! The row and settings rules without GPUI: the expand rule of `edit-row.tsx:206`, the locate
//! gate of `useLocatedHunks` (`edit-row.tsx:69-94`), and the states of `client/settings.tsx`.

use crate::diff::{Hunk, Located, Side, needs_locating, side_text};
use crate::edit_item::Status;
use bachuc_plugin_host::SettingsState;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::time::{Duration, Instant};

/// Streaming tool input changes many times a second; locate only once it stops changing.
pub(crate) const SETTLE: Duration = Duration::from_millis(300);

/// `autoExpand` of the `display` settings; the schema default is `true`.
pub(crate) fn auto_expand(values: &Value) -> bool {
    values
        .get("autoExpand")
        .and_then(Value::as_bool)
        .unwrap_or(true)
}

/// A press sets an override; without one the row follows `autoExpand`, and stays collapsed
/// until the settings are `Ready`.
pub(crate) fn is_expanded(toggled: Option<bool>, settings: &SettingsState) -> bool {
    toggled.unwrap_or(match settings {
        SettingsState::Ready { values, .. } => auto_expand(values),
        _ => false,
    })
}

/// The settings values with `autoExpand` replaced, as `{ ...values, autoExpand }`.
pub(crate) fn with_auto_expand(values: &Value, auto_expand: bool) -> Value {
    let mut values = match values {
        Value::Object(object) => object.clone(),
        _ => serde_json::Map::new(),
    };
    values.insert("autoExpand".into(), auto_expand.into());
    Value::Object(values)
}

/// What the "Edit diffs" settings screen shows.
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Screen {
    Note(String),
    Invalid {
        error: String,
        save_error: Option<String>,
        disabled: bool,
    },
    Ready {
        auto_expand: bool,
        save_error: Option<String>,
        disabled: bool,
    },
}

pub(crate) fn screen(state: &SettingsState, saving: bool, save_error: Option<&str>) -> Screen {
    let save_error = save_error.map(str::to_string);
    match state {
        SettingsState::Loading => Screen::Note("Loading edit diff settings…".into()),
        SettingsState::Error(error) => {
            Screen::Note(format!("Edit diff settings are unavailable: {error}"))
        }
        SettingsState::Invalid { error, .. } => Screen::Invalid {
            error: error.clone(),
            save_error,
            disabled: saving,
        },
        SettingsState::Ready { values, .. } => Screen::Ready {
            auto_expand: auto_expand(values),
            save_error,
            disabled: saving,
        },
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocateHunk {
    pub old_text: String,
    pub new_text: String,
}

/// The `edit-diffs.locate` input.
#[derive(Clone, Debug, PartialEq, Eq, Hash, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LocateRequest {
    pub file_path: String,
    pub cwd: Option<String>,
    pub hunks: Vec<LocateHunk>,
}

impl LocateRequest {
    pub(crate) fn new(file_path: &str, cwd: Option<&str>, hunks: &[Hunk]) -> Self {
        Self {
            file_path: file_path.to_string(),
            cwd: cwd.map(str::to_string),
            hunks: hunks
                .iter()
                .map(|hunk| LocateHunk {
                    old_text: side_text(hunk, Side::Old),
                    new_text: side_text(hunk, Side::New),
                })
                .collect(),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
enum WireSide {
    Old,
    New,
}

#[derive(Clone, Copy, Debug, Deserialize)]
struct WireLocated {
    line: i64,
    side: WireSide,
}

/// The `edit-diffs.locate` output.
#[derive(Clone, Debug, Deserialize)]
pub(crate) struct LocateResponse {
    located: Vec<Option<WireLocated>>,
}

impl LocateResponse {
    pub(crate) fn located(&self) -> Vec<Option<Located>> {
        self.located
            .iter()
            .map(|found| {
                found.map(|found| Located {
                    line: found.line,
                    side: match found.side {
                        WireSide::Old => Side::Old,
                        WireSide::New => Side::New,
                    },
                })
            })
            .collect()
    }
}

/// An answer is kept per status and request, as the React Query key.
pub(crate) type LocateKey = (Status, LocateRequest);

/// What the row does about locating now.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum LocateStep {
    /// Nothing to send.
    Idle,
    /// Ask again after this long; the request has not settled.
    Wait(Duration),
    /// Send this request.
    Send(LocateKey),
}

/// The locate cache of one row.
#[derive(Debug)]
pub(crate) struct Locator {
    request: LocateRequest,
    /// When `request` last changed.
    since: Instant,
    sent: HashSet<LocateKey>,
    answers: HashMap<LocateKey, Vec<Option<Located>>>,
}

impl Locator {
    pub(crate) fn new(request: LocateRequest, now: Instant) -> Self {
        Self {
            request,
            since: now,
            sent: HashSet::new(),
            answers: HashMap::new(),
        }
    }

    /// The current request; a change restarts the settle time.
    pub(crate) fn set_request(&mut self, request: LocateRequest, now: Instant) {
        if request != self.request {
            self.request = request;
            self.since = now;
        }
    }

    /// Sends only when the row is expanded, some hunk lacks a start, the request stayed
    /// unchanged for `SETTLE`, and no answer or request for it exists.
    pub(crate) fn poll(
        &mut self,
        status: Status,
        expanded: bool,
        hunks: &[Hunk],
        now: Instant,
    ) -> LocateStep {
        if !expanded || !needs_locating(hunks) {
            return LocateStep::Idle;
        }
        let key = (status, self.request.clone());
        if self.answers.contains_key(&key) || self.sent.contains(&key) {
            return LocateStep::Idle;
        }
        let elapsed = now.saturating_duration_since(self.since);
        if elapsed < SETTLE {
            return LocateStep::Wait(SETTLE - elapsed);
        }
        self.sent.insert(key.clone());
        LocateStep::Send(key)
    }

    /// Keeps an answer; one for an older request is dropped. Returns whether it was kept.
    pub(crate) fn answer(&mut self, key: LocateKey, located: Vec<Option<Located>>) -> bool {
        if key.1 != self.request {
            return false;
        }
        self.answers.insert(key, located);
        true
    }

    /// The answer for the current request and `status`.
    pub(crate) fn located(&self, status: Status) -> Option<&[Option<Located>]> {
        self.answers
            .get(&(status, self.request.clone()))
            .map(Vec::as_slice)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::diff::{EditSource, apply_locations, build_hunks, parse_unified_diff};
    use serde_json::json;

    fn ready(values: Value) -> SettingsState {
        SettingsState::Ready {
            values,
            revision: "r1".into(),
        }
    }

    #[test]
    fn rows_follow_auto_expand_until_pressed_and_stay_closed_until_ready() {
        assert!(!is_expanded(None, &SettingsState::Loading));
        assert!(!is_expanded(None, &SettingsState::Error("down".into())));
        assert!(!is_expanded(
            None,
            &SettingsState::Invalid {
                error: "bad".into(),
                revision: "r".into()
            }
        ));
        // The schema default of `autoExpand` is true.
        assert!(is_expanded(None, &ready(json!({}))));
        assert!(is_expanded(None, &ready(json!({"autoExpand": true}))));
        assert!(!is_expanded(None, &ready(json!({"autoExpand": false}))));
        // The override wins in every state.
        assert!(is_expanded(Some(true), &SettingsState::Loading));
        assert!(!is_expanded(
            Some(false),
            &ready(json!({"autoExpand": true}))
        ));
        assert!(is_expanded(
            Some(true),
            &SettingsState::Error("down".into())
        ));
    }

    #[test]
    fn the_settings_screen_states() {
        assert_eq!(
            screen(&SettingsState::Loading, false, None),
            Screen::Note("Loading edit diff settings…".into())
        );
        assert_eq!(
            screen(&SettingsState::Error("no active host".into()), false, None),
            Screen::Note("Edit diff settings are unavailable: no active host".into())
        );
        assert_eq!(
            screen(
                &SettingsState::Invalid {
                    error: "autoExpand: expected boolean".into(),
                    revision: "r".into()
                },
                true,
                None
            ),
            Screen::Invalid {
                error: "autoExpand: expected boolean".into(),
                save_error: None,
                disabled: true
            }
        );
        // A save `Conflict` keeps the shown values, so the switch keeps its value and shows
        // the error.
        assert_eq!(
            screen(
                &ready(json!({"autoExpand": false})),
                false,
                Some("settings changed")
            ),
            Screen::Ready {
                auto_expand: false,
                save_error: Some("settings changed".into()),
                disabled: false
            }
        );
    }

    #[test]
    fn a_save_keeps_the_other_values() {
        assert_eq!(
            with_auto_expand(&json!({"autoExpand": true, "extra": 1}), false),
            json!({"autoExpand": false, "extra": 1})
        );
        assert_eq!(
            with_auto_expand(&json!(null), true),
            json!({"autoExpand": true})
        );
    }

    fn codex_hunks() -> Vec<Hunk> {
        parse_unified_diff("@@\n ctx\n-a\n+b\n")
    }

    fn request(hunks: &[Hunk]) -> LocateRequest {
        LocateRequest::new("/w/x.rs", Some("/w"), hunks)
    }

    #[test]
    fn the_locate_request_has_the_rpc_shape() {
        let hunks = codex_hunks();
        assert_eq!(
            serde_json::to_value(request(&hunks)).unwrap(),
            json!({
                "filePath": "/w/x.rs",
                "cwd": "/w",
                "hunks": [{"oldText": "ctx\na", "newText": "ctx\nb"}],
            })
        );
        let null_cwd = LocateRequest::new("/w/x.rs", None, &hunks);
        assert_eq!(serde_json::to_value(null_cwd).unwrap()["cwd"], Value::Null);
        let response: LocateResponse =
            serde_json::from_value(json!({"located": [{"line": 4, "side": "new"}, null]})).unwrap();
        assert_eq!(
            response.located(),
            vec![
                Some(Located {
                    line: 4,
                    side: Side::New
                }),
                None
            ]
        );
    }

    #[test]
    fn an_expanded_bare_diff_sends_one_request_after_300_ms() {
        let hunks = codex_hunks();
        let start = Instant::now();
        let mut locator = Locator::new(request(&hunks), start);
        assert_eq!(
            locator.poll(Status::Completed, true, &hunks, start),
            LocateStep::Wait(SETTLE)
        );
        let at = start + Duration::from_millis(120);
        assert_eq!(
            locator.poll(Status::Completed, true, &hunks, at),
            LocateStep::Wait(Duration::from_millis(180))
        );
        let at = start + SETTLE;
        assert_eq!(
            locator.poll(Status::Completed, true, &hunks, at),
            LocateStep::Send((Status::Completed, request(&hunks)))
        );
        // One request: polling again sends nothing.
        assert_eq!(
            locator.poll(Status::Completed, true, &hunks, at),
            LocateStep::Idle
        );
    }

    #[test]
    fn a_collapsed_row_or_a_located_diff_sends_none() {
        let hunks = codex_hunks();
        let start = Instant::now();
        let late = start + Duration::from_secs(5);
        let mut locator = Locator::new(request(&hunks), start);
        assert_eq!(
            locator.poll(Status::Completed, false, &hunks, late),
            LocateStep::Idle
        );
        let write = build_hunks(&EditSource {
            content: Some("a\n"),
            ..Default::default()
        });
        let mut locator = Locator::new(request(&write), start);
        assert_eq!(
            locator.poll(Status::Completed, true, &write, late),
            LocateStep::Idle
        );
    }

    #[test]
    fn a_changed_request_restarts_the_settle_time() {
        let first = codex_hunks();
        let second = parse_unified_diff("@@\n ctx\n-a\n+c\n");
        let start = Instant::now();
        let mut locator = Locator::new(request(&first), start);
        let at = start + Duration::from_millis(250);
        locator.set_request(request(&second), at);
        assert_eq!(
            locator.poll(Status::Running, true, &second, start + SETTLE),
            LocateStep::Wait(Duration::from_millis(250))
        );
        // The same request again keeps the time.
        locator.set_request(request(&second), at + Duration::from_millis(100));
        assert!(matches!(
            locator.poll(Status::Running, true, &second, at + SETTLE),
            LocateStep::Send(_)
        ));
    }

    #[test]
    fn a_stale_answer_is_dropped_and_answers_are_kept_per_status() {
        let first = codex_hunks();
        let second = parse_unified_diff("@@\n ctx\n-a\n+c\n");
        let start = Instant::now();
        let mut locator = Locator::new(request(&first), start);
        let LocateStep::Send(old_key) = locator.poll(Status::Running, true, &first, start + SETTLE)
        else {
            panic!("expected a send");
        };
        let found = vec![Some(Located {
            line: 9,
            side: Side::Old,
        })];
        locator.set_request(request(&second), start + SETTLE);
        assert!(!locator.answer(old_key, found.clone()));
        assert_eq!(locator.located(Status::Running), None);

        let at = start + SETTLE * 2;
        let LocateStep::Send(key) = locator.poll(Status::Running, true, &second, at) else {
            panic!("expected a send");
        };
        assert!(locator.answer(key, found.clone()));
        assert_eq!(locator.located(Status::Running), Some(found.as_slice()));
        let located = apply_locations(&second, locator.located(Status::Running).unwrap());
        assert_eq!(
            (located[0].old_start, located[0].new_start),
            (Some(9), Some(9))
        );
        // A new status is a new key: it asks again, and the running answer stays kept.
        assert_eq!(locator.located(Status::Completed), None);
        assert!(matches!(
            locator.poll(Status::Completed, true, &second, at),
            LocateStep::Send(_)
        ));
        assert_eq!(locator.located(Status::Running), Some(found.as_slice()));
    }
}
