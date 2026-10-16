//! The RPC types and pure rules of the jev-orchestrator panels. The jobs rules follow
//! `client/panel.tsx`; the direct-route rules follow `client/direct-panel.tsx` and
//! `shared/direct.ts`. The views only draw what these functions return.

use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::collections::BTreeMap;

pub(crate) const JOBS_LIST: &str = "jobs.list";
pub(crate) const JOBS_CANCEL: &str = "jobs.cancel";
pub(crate) const DIRECT_PROFILES: &str = "direct.profiles";
pub(crate) const DIRECT_RUN: &str = "direct.run";
pub(crate) const DIRECT_STATUS: &str = "direct.status";

/// `refetchInterval` of the jobs query.
pub(crate) const JOBS_POLL_MS: i64 = 5_000;
/// The tails `check.output.slice(-1200)` and `a.output.slice(-2500)` keep, counted in chars.
pub(crate) const CHECK_TAIL: usize = 1_200;
pub(crate) const ATTEMPT_TAIL: usize = 2_500;
pub(crate) const MAX_PROFILES: usize = 8;
pub(crate) const MIN_PROMPT: usize = 8;
pub(crate) const MAX_PROMPT: usize = 12_000;

pub(crate) const TITLE_TEXT: &str = "Delegated work";
pub(crate) const HINT_TEXT: &str = "Use delegate_task in a new agent to start. Checks, model judgments, and child claims remain separate.";
pub(crate) const FAILED_TEXT: &str = "Request failed. Refresh and inspect the job before retrying.";
pub(crate) const LOADING_TEXT: &str = "Loading jobs…";
pub(crate) const EMPTY_TEXT: &str = "No delegated jobs for this agent.";
pub(crate) const CANCEL_TEXT: &str = "Cancel and archive children";
pub(crate) const REFRESH_TEXT: &str = "Refresh";

// ---------------------------------------------------------------- Delegated work

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScopeInput {
    pub parent_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CancelInput {
    pub parent_id: String,
    pub id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct Cancelled {
    #[allow(dead_code)] // any answer is a success, as `onSuccess` of the mutation
    pub cancelled: bool,
}

/// `jobs.list` output. `history` is ignored, as Electron does.
#[derive(Debug, Deserialize)]
pub(crate) struct JobsPayload {
    pub jobs: Vec<Value>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct CheckView {
    #[serde(rename = "exitCode")]
    pub exit_code: Option<i64>,
    pub output: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileRef {
    pub name: String,
    pub model: String,
    pub thinking_option_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AttemptView {
    pub child_id: String,
    pub phase: String,
    pub profile: ProfileRef,
    pub status: Option<String>,
    pub output: Option<String>,
    pub checks: Option<Vec<CheckView>>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct TaskRef {
    pub id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JobView {
    pub task: TaskRef,
    pub status: String,
    pub message: String,
    pub notify_error: Option<String>,
    pub attempts: Vec<AttemptView>,
}

/// Decodes each job on its own and skips one that does not fit `JobView`.
pub(crate) fn decode_jobs(payload: JobsPayload) -> Vec<JobView> {
    payload
        .jobs
        .into_iter()
        .filter_map(|job| match serde_json::from_value::<JobView>(job) {
            Ok(job) => Some(job),
            Err(error) => {
                log::warn!("jev-orchestrator: skipped a job that does not fit: {error}");
                None
            }
        })
        .collect()
}

/// Cancel shows only for a job that has not ended.
pub(crate) fn cancellable(status: &str) -> bool {
    matches!(status, "queued" | "running" | "needs_input" | "interrupted")
}

/// The last `max` chars of `text`.
pub(crate) fn tail_chars(text: &str, max: usize) -> &str {
    let count = text.chars().count();
    if count <= max {
        return text;
    }
    let start = text
        .char_indices()
        .nth(count - max)
        .map_or(text.len(), |(index, _)| index);
    &text[start..]
}

pub(crate) fn job_title(job: &JobView) -> String {
    format!("{} · {}", job.task.id, job.status)
}

pub(crate) fn attempt_title(attempt: &AttemptView) -> String {
    format!(
        "{} · {} · {} · effort {}",
        attempt.phase,
        attempt.profile.name,
        attempt.profile.model,
        attempt
            .profile
            .thinking_option_id
            .as_deref()
            .unwrap_or("default")
    )
}

pub(crate) fn attempt_child_line(attempt: &AttemptView) -> String {
    format!(
        "{} · {}",
        attempt.child_id,
        attempt.status.as_deref().unwrap_or("running")
    )
}

/// A check with a stable key. Checks carry no id, so the key is their content plus a counter
/// for identical repeats (`keyedChecks`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct KeyedCheck<'a> {
    pub number: usize,
    pub key: String,
    pub check: &'a CheckView,
}

pub(crate) fn keyed_checks(checks: &[CheckView]) -> Vec<KeyedCheck<'_>> {
    let mut seen: Vec<(String, usize)> = Vec::new();
    checks
        .iter()
        .enumerate()
        .map(|(index, check)| {
            let exit = check
                .exit_code
                .map_or_else(|| "null".to_string(), |code| code.to_string());
            let content = format!("{exit}:{}", check.output);
            let repeat = match seen.iter_mut().find(|(seen, _)| *seen == content) {
                Some((_, count)) => {
                    *count += 1;
                    *count - 1
                }
                None => {
                    seen.push((content.clone(), 1));
                    0
                }
            };
            KeyedCheck {
                number: index + 1,
                key: format!("{content}:{repeat}"),
                check,
            }
        })
        .collect()
}

pub(crate) fn check_text(number: usize, check: &CheckView) -> String {
    let exit = check
        .exit_code
        .map_or_else(|| "unknown".to_string(), |code| code.to_string());
    format!(
        "Check {number}: exit {exit}\n{}",
        tail_chars(&check.output, CHECK_TAIL)
    )
}

/// An attempt's output, or `None` for an empty one (`a.output &&`).
pub(crate) fn attempt_output(attempt: &AttemptView) -> Option<&str> {
    attempt
        .output
        .as_deref()
        .filter(|output| !output.is_empty())
        .map(|output| tail_chars(output, ATTEMPT_TAIL))
}

/// One row group of the Delegated work list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum JobsItem {
    Title,
    Hint,
    Failed,
    Loading,
    Empty,
    Job(JobView),
    Refresh,
}

/// What the Delegated work panel shows: the last good job list, and the failures of the list
/// query and the cancel mutation (react-query keeps old data after an error).
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct JobsState {
    pub jobs: Option<Vec<JobView>>,
    pub list_failed: bool,
    pub cancel_failed: bool,
    pub cancelling: bool,
}

impl JobsState {
    pub(crate) fn listed(&mut self, result: Result<Vec<JobView>, ()>) {
        match result {
            Ok(jobs) => {
                self.jobs = Some(jobs);
                self.list_failed = false;
            }
            Err(()) => self.list_failed = true,
        }
    }

    /// `false` when a cancel already runs; the button is disabled then.
    pub(crate) fn begin_cancel(&mut self) -> bool {
        if self.cancelling {
            return false;
        }
        self.cancelling = true;
        self.cancel_failed = false;
        true
    }

    pub(crate) fn cancel_finished(&mut self, ok: bool) {
        self.cancelling = false;
        self.cancel_failed = !ok;
    }

    pub(crate) fn items(&self) -> Vec<JobsItem> {
        let mut items = vec![JobsItem::Title, JobsItem::Hint];
        if self.list_failed || self.cancel_failed {
            items.push(JobsItem::Failed);
        }
        match &self.jobs {
            None if !self.list_failed => items.push(JobsItem::Loading),
            Some(jobs) if jobs.is_empty() => items.push(JobsItem::Empty),
            _ => {}
        }
        items.extend(self.jobs.iter().flatten().cloned().map(JobsItem::Job));
        items.push(JobsItem::Refresh);
        items
    }
}

/// A `jobs.list` is due when none runs and 5,000 ms passed since the last one was sent.
pub(crate) fn poll_due(now_ms: i64, last_sent_ms: Option<i64>, in_flight: bool) -> bool {
    !in_flight && last_sent_ms.is_none_or(|last| now_ms - last >= JOBS_POLL_MS)
}

/// The poll rules of one open panel. A user action (Refresh, a finished cancel) skips the
/// interval but never the in-flight guard: it queues one send for when the request returns.
#[derive(Debug, Default)]
pub(crate) struct Poller {
    connected: bool,
    in_flight: bool,
    last_sent_ms: Option<i64>,
    queued: bool,
}

impl Poller {
    pub(crate) fn connected(&mut self) {
        self.connected = true;
        self.last_sent_ms = None;
    }

    pub(crate) fn disconnected(&mut self) {
        self.connected = false;
        self.queued = false;
    }

    pub(crate) fn due(&self, now_ms: i64) -> bool {
        self.connected && poll_due(now_ms, self.last_sent_ms, self.in_flight)
    }

    pub(crate) fn begin(&mut self, now_ms: i64) {
        self.in_flight = true;
        self.last_sent_ms = Some(now_ms);
        self.queued = false;
    }

    /// `true` when a queued user request must be sent now.
    pub(crate) fn finish(&mut self) -> bool {
        self.in_flight = false;
        std::mem::take(&mut self.queued) && self.connected
    }

    /// `true` when a user request may be sent now; else it waits for the request in flight.
    pub(crate) fn request_now(&mut self) -> bool {
        if !self.connected {
            return false;
        }
        if self.in_flight {
            self.queued = true;
            return false;
        }
        true
    }

    /// Milliseconds until the next poll is due; `None` while none can be sent.
    pub(crate) fn wait_ms(&self, now_ms: i64) -> Option<i64> {
        if !self.connected || self.in_flight {
            return None;
        }
        Some(
            self.last_sent_ms
                .map_or(0, |last| (last + JOBS_POLL_MS - now_ms).max(0)),
        )
    }
}

// ---------------------------------------------------------------- Direct route

/// The efforts, in the canonical order (`effortIds`).
#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub(crate) enum Effort {
    Low,
    Medium,
    High,
    Xhigh,
    Max,
}

impl Effort {
    pub(crate) const ALL: [Effort; 5] = [
        Effort::Low,
        Effort::Medium,
        Effort::High,
        Effort::Xhigh,
        Effort::Max,
    ];

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Effort::Low => "low",
            Effort::Medium => "medium",
            Effort::High => "high",
            Effort::Xhigh => "xhigh",
            Effort::Max => "max",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfilesInput {
    pub workspace_id: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StatusInput {
    pub workspace_id: String,
    pub request_id: String,
}

#[derive(Debug, Deserialize)]
pub(crate) struct ProfilesPayload {
    pub profiles: Vec<ProfileView>,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProfileView {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model: String,
    pub mode_id: Option<String>,
    pub notes: Option<String>,
    pub effort_ids: Vec<Effort>,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AllowedModel {
    pub provider: String,
    pub model: String,
    pub effort_ids: Vec<Effort>,
}

/// The `direct.run` input (`directInput`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DirectRequest {
    pub workspace_id: String,
    pub request_id: String,
    pub prompt: String,
    pub allowed_profile_ids: Vec<String>,
    pub allowed_models: Vec<AllowedModel>,
    pub share_with_jev: bool,
}

/// `direct.run` output. The fields the panel does not show are decoded to check the shape.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct RunResult {
    pub agent_id: String,
    pub selection: RunSelection,
    pub routing_ms: Option<f64>,
    #[serde(default)]
    pub usage: Value,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
#[allow(dead_code)]
pub(crate) struct RunSelection {
    pub id: String,
    pub name: String,
    pub provider: String,
    pub model: String,
    pub mode_id: Option<String>,
    pub thinking_option_id: String,
    pub feature_values: Option<Value>,
}

#[derive(Debug, Deserialize)]
pub(crate) struct StatusPayload {
    pub records: Vec<Value>,
}

/// What the created-agent card shows of a selection.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct SelectionView {
    pub name: Option<String>,
    pub model: Option<String>,
    pub thinking_option_id: Option<String>,
}

impl RunResult {
    fn selection_view(&self) -> SelectionView {
        SelectionView {
            name: Some(self.selection.name.clone()),
            model: Some(self.selection.model.clone()),
            thinking_option_id: Some(self.selection.thinking_option_id.clone()),
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum RecordStatus {
    Routing,
    Creating,
    Created,
    Failed,
    Interrupted,
}

impl RecordStatus {
    fn parse(status: &str) -> Option<Self> {
        Some(match status {
            "routing" => Self::Routing,
            "creating" => Self::Creating,
            "created" => Self::Created,
            "failed" => Self::Failed,
            "interrupted" => Self::Interrupted,
            _ => return None,
        })
    }

    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Routing => "routing",
            Self::Creating => "creating",
            Self::Created => "created",
            Self::Failed => "failed",
            Self::Interrupted => "interrupted",
        }
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct DirectRecord {
    pub request_id: String,
    pub status: RecordStatus,
    pub agent_id: Option<String>,
    pub selection: Option<SelectionView>,
    pub error: Option<String>,
}

fn text(object: &serde_json::Map<String, Value>, key: &str) -> Option<String> {
    object.get(key)?.as_str().map(str::to_string)
}

/// `asDirectRecord`: a record counts only when `requestId` is a string and `status` is one of
/// the five known values.
pub(crate) fn as_direct_record(value: &Value) -> Option<DirectRecord> {
    let record = value.as_object()?;
    let request_id = text(record, "requestId")?;
    let status = RecordStatus::parse(record.get("status")?.as_str()?)?;
    let selection = record
        .get("selection")
        .and_then(Value::as_object)
        .map(|selection| SelectionView {
            name: text(selection, "name"),
            model: text(selection, "model"),
            thinking_option_id: text(selection, "thinkingOptionId"),
        });
    Some(DirectRecord {
        request_id,
        status,
        agent_id: text(record, "agentId"),
        selection,
        error: text(record, "error"),
    })
}

/// `isLunaModel`: `/(^|[/-])luna([/-]|$)/i`.
pub(crate) fn is_luna_model(model: &str) -> bool {
    let model = model.to_lowercase();
    model.match_indices("luna").any(|(start, word)| {
        let before = model[..start].chars().next_back();
        let after = model[start + word.len()..].chars().next();
        matches!(before, None | Some('/' | '-')) && matches!(after, None | Some('/' | '-'))
    })
}

/// The efforts a model offers, in the canonical order. Luna offers max only.
pub(crate) fn offered_effort_ids(model: &str, offered: &[Effort]) -> Vec<Effort> {
    Effort::ALL
        .into_iter()
        .filter(|effort| offered.contains(effort))
        .filter(|effort| !is_luna_model(model) || *effort == Effort::Max)
        .collect()
}

/// `JSON.stringify([provider, model])`.
pub(crate) fn model_key(provider: &str, model: &str) -> String {
    serde_json::to_string(&[provider, model]).unwrap_or_default()
}

pub(crate) fn new_request_id() -> String {
    uuid::Uuid::new_v4().to_string()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ModelGroup {
    pub key: String,
    pub provider: String,
    pub model: String,
    pub effort_ids: Vec<Effort>,
    pub profile_count: usize,
}

/// One group per model among the selected profiles, in profile order.
pub(crate) fn build_model_groups(
    profiles: &[ProfileView],
    selected_profile_ids: &[String],
) -> Vec<ModelGroup> {
    let mut groups: Vec<ModelGroup> = Vec::new();
    for profile in profiles {
        if !selected_profile_ids.contains(&profile.id) {
            continue;
        }
        let key = model_key(&profile.provider, &profile.model);
        let offered = offered_effort_ids(&profile.model, &profile.effort_ids);
        match groups.iter_mut().find(|group| group.key == key) {
            Some(group) => {
                group.effort_ids = Effort::ALL
                    .into_iter()
                    .filter(|effort| group.effort_ids.contains(effort) || offered.contains(effort))
                    .collect();
                group.profile_count += 1;
            }
            None => groups.push(ModelGroup {
                key,
                provider: profile.provider.clone(),
                model: profile.model.clone(),
                effort_ids: offered,
                profile_count: 1,
            }),
        }
    }
    groups
}

pub(crate) type SelectedEfforts = BTreeMap<String, Vec<Effort>>;

pub(crate) fn build_allowed_models(
    groups: &[ModelGroup],
    selected: &SelectedEfforts,
) -> Vec<AllowedModel> {
    groups
        .iter()
        .map(|group| AllowedModel {
            provider: group.provider.clone(),
            model: group.model.clone(),
            effort_ids: selected
                .get(&group.key)
                .into_iter()
                .flatten()
                .copied()
                .filter(|effort| group.effort_ids.contains(effort))
                .collect(),
        })
        .collect()
}

/// Keeps the efforts of the models that a selected profile still uses.
pub(crate) fn keep_efforts_for_profiles(
    current: &SelectedEfforts,
    profiles: &[ProfileView],
    profile_ids: &[String],
) -> SelectedEfforts {
    let keys: Vec<String> = profiles
        .iter()
        .filter(|profile| profile_ids.contains(&profile.id))
        .map(|profile| model_key(&profile.provider, &profile.model))
        .collect();
    current
        .iter()
        .filter(|(key, _)| keys.contains(key))
        .map(|(key, efforts)| (key.clone(), efforts.clone()))
        .collect()
}

/// Removes `effort` when it is selected; else adds it and keeps the canonical order.
pub(crate) fn toggled_efforts(current: &[Effort], effort: Effort, selected: bool) -> Vec<Effort> {
    if selected {
        return current.iter().copied().filter(|e| *e != effort).collect();
    }
    Effort::ALL
        .into_iter()
        .filter(|candidate| current.contains(candidate) || *candidate == effort)
        .collect()
}

#[derive(Clone, Copy, Debug)]
pub(crate) struct OutcomeInput<'a> {
    pub request_id: Option<&'a str>,
    pub result: Option<&'a RunResult>,
    pub records: Option<&'a [Value]>,
    pub run_success: bool,
    pub run_error: bool,
    pub status_success: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Outcome {
    pub matching_record: Option<DirectRecord>,
    pub agent_id: Option<String>,
    pub selection: Option<SelectionView>,
    pub may_reset: bool,
    pub may_retry_same_request: bool,
}

pub(crate) fn derive_outcome(input: OutcomeInput<'_>) -> Outcome {
    let matching_record = input.request_id.and_then(|request_id| {
        input
            .records?
            .iter()
            .filter_map(as_direct_record)
            .find(|record| record.request_id == request_id)
    });
    let created = matching_record
        .as_ref()
        .filter(|record| record.status == RecordStatus::Created);
    let agent_id = input
        .result
        .map(|result| result.agent_id.clone())
        .or_else(|| created.and_then(|record| record.agent_id.clone()));
    let selection = input
        .result
        .map(RunResult::selection_view)
        .or_else(|| created.and_then(|record| record.selection.clone()));
    let may_reset = input.run_success
        || matching_record.as_ref().is_some_and(|record| {
            matches!(record.status, RecordStatus::Created | RecordStatus::Failed)
        });
    let may_retry_same_request = input.run_error
        && input.status_success
        && matching_record.is_none()
        && input.records.is_some_and(<[Value]>::is_empty);
    Outcome {
        matching_record,
        agent_id,
        selection,
        may_reset,
        may_retry_same_request,
    }
}

pub(crate) fn route_label(pending: bool, may_retry_same_request: bool) -> &'static str {
    if pending {
        return "Routing…";
    }
    if may_retry_same_request {
        "Retry same request"
    } else {
        "Route and run"
    }
}

pub(crate) fn route_aria_label(request_locked: bool) -> &'static str {
    if request_locked {
        "Retry the same direct routing request"
    } else {
        "Route and run"
    }
}

/// `canRoute`.
#[derive(Clone, Copy, Debug)]
pub(crate) struct RouteInput<'a> {
    pub prompt: &'a str,
    pub selected_profiles: usize,
    pub has_effort_for_every_model: bool,
    pub request_locked: bool,
    pub run_pending: bool,
    pub status_pending: bool,
    pub may_retry_same_request: bool,
}

pub(crate) fn can_route(input: RouteInput<'_>) -> bool {
    !input.run_pending
        && !input.status_pending
        && ((!input.request_locked
            && input.prompt.trim().chars().count() >= MIN_PROMPT
            && input.selected_profiles > 0
            && input.has_effort_for_every_model)
            || input.may_retry_same_request)
}

/// `/route` takes no arguments.
pub(crate) fn slash_route_allowed(args: &str) -> bool {
    args.trim().is_empty()
}

pub(crate) const ROUTE_ARGS_ERROR: &str = "Use /route without arguments.";

/// The prompt cut to its limit, or `None` when it already fits (`maxLength={12000}`).
pub(crate) fn clamp_prompt(prompt: &str) -> Option<String> {
    (prompt.chars().count() > MAX_PROMPT).then(|| prompt.chars().take(MAX_PROMPT).collect())
}

pub(crate) fn created_line(selection: &SelectionView) -> String {
    format!(
        "{}{} · effort {}",
        selection
            .name
            .as_ref()
            .map(|name| format!("{name} · "))
            .unwrap_or_default(),
        selection.model.as_deref().unwrap_or("Unknown model"),
        selection.thinking_option_id.as_deref().unwrap_or("unknown"),
    )
}

/// A profile that offers no effort cannot be picked.
pub(crate) fn profile_has_no_effort(profile: &ProfileView) -> bool {
    offered_effort_ids(&profile.model, &profile.effort_ids).is_empty()
}

// ---- items of the Direct route list

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ProfileRow {
    pub id: String,
    pub title: String,
    pub subtitle: String,
    pub no_effort: bool,
    pub notes: Option<String>,
    pub selected: bool,
    pub disabled: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct EffortCard {
    pub key: String,
    pub title: String,
    pub note: Option<String>,
    pub chips: Vec<(Effort, bool)>,
    pub locked: bool,
}

/// One row group of the Direct route list.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum DirectItem {
    Title,
    Prompt {
        locked: bool,
    },
    ProfilesIntro,
    ProfilesLoading,
    ProfilesError {
        fetching: bool,
    },
    ProfilesEmpty,
    Profile(ProfileRow),
    EffortIntro,
    EffortGroup(EffortCard),
    Route {
        enabled: bool,
        label: &'static str,
        aria: &'static str,
    },
    RequestId(String),
    RunError {
        checking: bool,
    },
    StatusError,
    StatusNone,
    Record {
        status: RecordStatus,
        error: Option<String>,
    },
    Created {
        agent_id: Option<String>,
        selection: SelectionView,
    },
    Reset,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum ProfilesState {
    Loading,
    Failed,
    Ready(Vec<ProfileView>),
}

/// The state of one request of a mutation (`run`, `status`).
#[derive(Clone, Debug, PartialEq)]
pub(crate) enum Call<T> {
    Idle,
    Pending,
    Success(T),
    Failed,
}

impl<T> Call<T> {
    fn is_pending(&self) -> bool {
        matches!(self, Call::Pending)
    }
}

/// `DirectPanelState`: the prompt, the selected profiles, the selected efforts per model key,
/// and the locked request, with the calls in flight. A host or workspace change makes a new
/// plugin activation or panel input, so the state starts fresh without a key of its own.
#[derive(Clone, Debug)]
pub(crate) struct DirectState {
    pub workspace_id: String,
    pub prompt: String,
    pub selected_profiles: Vec<String>,
    pub selected_efforts: SelectedEfforts,
    pub request: Option<DirectRequest>,
    pub profiles: ProfilesState,
    pub profiles_fetching: bool,
    pub run: Call<RunResult>,
    pub status: Call<Vec<Value>>,
}

impl DirectState {
    pub(crate) fn new(workspace_id: String) -> Self {
        Self {
            workspace_id,
            prompt: String::new(),
            selected_profiles: Vec::new(),
            selected_efforts: SelectedEfforts::new(),
            request: None,
            profiles: ProfilesState::Loading,
            profiles_fetching: true,
            run: Call::Idle,
            status: Call::Idle,
        }
    }

    fn profile_list(&self) -> &[ProfileView] {
        match &self.profiles {
            ProfilesState::Ready(profiles) => profiles,
            _ => &[],
        }
    }

    pub(crate) fn locked(&self) -> bool {
        self.request.is_some()
    }

    fn groups(&self) -> Vec<ModelGroup> {
        build_model_groups(self.profile_list(), &self.selected_profiles)
    }

    fn allowed_models(&self, groups: &[ModelGroup]) -> Vec<AllowedModel> {
        build_allowed_models(groups, &self.selected_efforts)
    }

    pub(crate) fn outcome(&self) -> Outcome {
        derive_outcome(OutcomeInput {
            request_id: self.request.as_ref().map(|r| r.request_id.as_str()),
            result: match &self.run {
                Call::Success(result) => Some(result),
                _ => None,
            },
            records: match &self.status {
                Call::Success(records) => Some(records),
                _ => None,
            },
            run_success: matches!(self.run, Call::Success(_)),
            run_error: matches!(self.run, Call::Failed),
            status_success: matches!(self.status, Call::Success(_)),
        })
    }

    pub(crate) fn can_route(&self) -> bool {
        let groups = self.groups();
        let allowed = self.allowed_models(&groups);
        self.can_route_with(&allowed, self.outcome().may_retry_same_request)
    }

    fn can_route_with(&self, allowed: &[AllowedModel], may_retry: bool) -> bool {
        can_route(RouteInput {
            prompt: &self.prompt,
            selected_profiles: self.selected_profiles.len(),
            has_effort_for_every_model: !allowed.is_empty()
                && allowed.iter().all(|model| !model.effort_ids.is_empty()),
            request_locked: self.locked(),
            run_pending: self.run.is_pending(),
            status_pending: self.status.is_pending(),
            may_retry_same_request: may_retry,
        })
    }

    pub(crate) fn set_prompt(&mut self, prompt: String) {
        self.prompt = prompt;
    }

    fn profile_toggle_disabled(&self, profile: &ProfileView) -> bool {
        let selected = self.selected_profiles.contains(&profile.id);
        self.locked()
            || (!selected && self.selected_profiles.len() >= MAX_PROFILES)
            || profile_has_no_effort(profile)
    }

    pub(crate) fn toggle_profile(&mut self, profile_id: &str) {
        let Some(profile) = self.profile_list().iter().find(|p| p.id == profile_id) else {
            return;
        };
        if self.profile_toggle_disabled(profile) {
            return;
        }
        let next: Vec<String> = if self.selected_profiles.iter().any(|id| id == profile_id) {
            self.selected_profiles
                .iter()
                .filter(|id| *id != profile_id)
                .cloned()
                .collect()
        } else {
            let mut ids = self.selected_profiles.clone();
            ids.push(profile_id.to_string());
            ids
        };
        self.selected_efforts =
            keep_efforts_for_profiles(&self.selected_efforts, self.profile_list(), &next);
        self.selected_profiles = next;
    }

    pub(crate) fn toggle_effort(&mut self, group_key: &str, effort: Effort) {
        if self.locked() {
            return;
        }
        let current = self
            .selected_efforts
            .get(group_key)
            .cloned()
            .unwrap_or_default();
        let selected = current.contains(&effort);
        self.selected_efforts.insert(
            group_key.to_string(),
            toggled_efforts(&current, effort, selected),
        );
    }

    pub(crate) fn profiles_loaded(&mut self, result: Result<Vec<ProfileView>, ()>) {
        self.profiles_fetching = false;
        self.profiles = match result {
            Ok(profiles) => ProfilesState::Ready(profiles),
            Err(()) => ProfilesState::Failed,
        };
    }

    /// `true` when profiles should load again (the Retry profiles button).
    pub(crate) fn retry_profiles(&mut self) -> bool {
        if self.profiles_fetching {
            return false;
        }
        self.profiles_fetching = true;
        true
    }

    /// Locks the request and starts the run. Returns what to send, or `None` when routing is
    /// not allowed. A retry sends the locked request again.
    pub(crate) fn route(
        &mut self,
        make_request_id: impl FnOnce() -> String,
    ) -> Option<DirectRequest> {
        if !self.can_route() {
            return None;
        }
        let request = match self.request.clone() {
            Some(request) => request,
            None => {
                let groups = self.groups();
                DirectRequest {
                    workspace_id: self.workspace_id.clone(),
                    request_id: make_request_id(),
                    prompt: self.prompt.clone(),
                    allowed_profile_ids: self.selected_profiles.clone(),
                    allowed_models: self.allowed_models(&groups),
                    share_with_jev: true,
                }
            }
        };
        self.request = Some(request.clone());
        self.status = Call::Idle;
        self.run = Call::Pending;
        Some(request)
    }

    pub(crate) fn run_finished(&mut self, result: Result<RunResult, ()>) {
        self.run = match result {
            Ok(result) => Call::Success(result),
            Err(()) => Call::Failed,
        };
    }

    /// The `direct.status` input, or `None` without a request or while a check runs.
    pub(crate) fn check_status(&mut self) -> Option<StatusInput> {
        let request = self.request.as_ref()?;
        if self.status.is_pending() {
            return None;
        }
        let input = StatusInput {
            workspace_id: self.workspace_id.clone(),
            request_id: request.request_id.clone(),
        };
        self.status = Call::Pending;
        Some(input)
    }

    pub(crate) fn status_finished(&mut self, result: Result<Vec<Value>, ()>) {
        self.status = match result {
            Ok(records) => Call::Success(records),
            Err(()) => Call::Failed,
        };
    }

    /// "Reset for a new task": clears everything but the loaded profiles.
    pub(crate) fn reset(&mut self) {
        self.prompt.clear();
        self.selected_profiles.clear();
        self.selected_efforts.clear();
        self.request = None;
        self.run = Call::Idle;
        self.status = Call::Idle;
    }

    pub(crate) fn items(&self) -> Vec<DirectItem> {
        let locked = self.locked();
        let outcome = self.outcome();
        let mut items = vec![DirectItem::Title, DirectItem::Prompt { locked }];
        items.push(DirectItem::ProfilesIntro);
        match &self.profiles {
            ProfilesState::Loading => items.push(DirectItem::ProfilesLoading),
            ProfilesState::Failed => items.push(DirectItem::ProfilesError {
                fetching: self.profiles_fetching,
            }),
            ProfilesState::Ready(profiles) if profiles.is_empty() => {
                items.push(DirectItem::ProfilesEmpty);
            }
            ProfilesState::Ready(profiles) => {
                for profile in profiles {
                    let selected = self.selected_profiles.contains(&profile.id);
                    items.push(DirectItem::Profile(ProfileRow {
                        id: profile.id.clone(),
                        title: if selected {
                            format!("Selected · {}", profile.name)
                        } else {
                            profile.name.clone()
                        },
                        subtitle: format!(
                            "{} · {} · mode {}",
                            profile.provider,
                            profile.model,
                            profile.mode_id.as_deref().unwrap_or("default")
                        ),
                        no_effort: profile_has_no_effort(profile),
                        notes: profile.notes.clone().filter(|notes| !notes.is_empty()),
                        selected,
                        disabled: self.profile_toggle_disabled(profile),
                    }));
                }
            }
        }
        let groups = self.groups();
        let allowed = self.allowed_models(&groups);
        if !groups.is_empty() {
            items.push(DirectItem::EffortIntro);
            for group in &groups {
                let selected = self.selected_efforts.get(&group.key);
                items.push(DirectItem::EffortGroup(EffortCard {
                    key: group.key.clone(),
                    title: format!("{} · {}", group.provider, group.model),
                    note: (group.profile_count > 1).then(|| {
                        format!("{} selected profiles use this model.", group.profile_count)
                    }),
                    chips: group
                        .effort_ids
                        .iter()
                        .map(|effort| (*effort, selected.is_some_and(|s| s.contains(effort))))
                        .collect(),
                    locked,
                }));
            }
        }
        let may_retry = outcome.may_retry_same_request;
        items.push(DirectItem::Route {
            enabled: self.can_route_with(&allowed, may_retry),
            label: route_label(self.run.is_pending(), may_retry),
            aria: route_aria_label(locked),
        });
        if let Some(request) = &self.request {
            items.push(DirectItem::RequestId(request.request_id.clone()));
        }
        if matches!(self.run, Call::Failed) {
            items.push(DirectItem::RunError {
                checking: self.status.is_pending(),
            });
        }
        if matches!(self.status, Call::Failed) {
            items.push(DirectItem::StatusError);
        }
        if matches!(self.status, Call::Success(_)) && outcome.matching_record.is_none() {
            items.push(DirectItem::StatusNone);
        }
        if let Some(record) = &outcome.matching_record {
            items.push(DirectItem::Record {
                status: record.status,
                error: record.error.clone().filter(|error| !error.is_empty()),
            });
        }
        if let Some(selection) = outcome.selection {
            items.push(DirectItem::Created {
                agent_id: outcome.agent_id,
                selection,
            });
        }
        if outcome.may_reset {
            items.push(DirectItem::Reset);
        }
        items
    }
}

#[cfg(test)]
mod tests {
    use super::{
        AttemptView, Call, CheckView, DirectItem, DirectRequest, DirectState, Effort, JobView,
        JobsItem, JobsPayload, JobsState, ModelGroup, OutcomeInput, Poller, ProfileView,
        ProfilesPayload, RecordStatus, RouteInput, RunResult, SelectedEfforts, SelectionView,
        StatusPayload, as_direct_record, attempt_child_line, attempt_output, attempt_title,
        build_allowed_models, build_model_groups, can_route, cancellable, check_text, clamp_prompt,
        created_line, decode_jobs, derive_outcome, is_luna_model, job_title,
        keep_efforts_for_profiles, keyed_checks, model_key, new_request_id, offered_effort_ids,
        poll_due, route_aria_label, route_label, slash_route_allowed, tail_chars, toggled_efforts,
    };
    use serde_json::{Value, json};

    fn profile(id: &str, provider: &str, model: &str, efforts: &[Effort]) -> ProfileView {
        ProfileView {
            id: id.into(),
            name: format!("Name {id}"),
            provider: provider.into(),
            model: model.into(),
            mode_id: None,
            notes: None,
            effort_ids: efforts.to_vec(),
        }
    }

    fn ids(values: &[&str]) -> Vec<String> {
        values.iter().map(|v| v.to_string()).collect()
    }

    fn run_result() -> RunResult {
        serde_json::from_value(json!({
            "agentId": "agent-9",
            "selection": {
                "id": "p1", "name": "Fast", "provider": "codex", "model": "gpt-5",
                "modeId": "auto", "thinkingOptionId": "high",
                "featureValues": {"fast": true}
            },
            "routingMs": 812.5,
            "usage": {"input": 10}
        }))
        .unwrap()
    }

    // ---- Delegated work

    #[test]
    fn cancellable_is_true_only_for_the_four_open_statuses() {
        for status in ["queued", "running", "needs_input", "interrupted"] {
            assert!(cancellable(status), "{status}");
        }
        for status in ["passed", "unverified", "failed", "cancelled", ""] {
            assert!(!cancellable(status), "{status}");
        }
    }

    #[test]
    fn tail_counts_chars_not_bytes() {
        assert_eq!(tail_chars("abcdef", 3), "def");
        assert_eq!(tail_chars("abc", 3), "abc");
        assert_eq!(tail_chars("ab", 3), "ab");
        assert_eq!(tail_chars("ééééé", 2), "éé");
        let long: String = "x".repeat(1300);
        assert_eq!(tail_chars(&long, 1200).chars().count(), 1200);
    }

    #[test]
    fn keyed_checks_give_unique_keys_to_identical_checks() {
        let check = CheckView {
            exit_code: Some(1),
            output: "boom".into(),
        };
        let other = CheckView {
            exit_code: None,
            output: "boom".into(),
        };
        let checks = [check.clone(), check.clone(), other, check];
        let keyed = keyed_checks(&checks);
        let keys: Vec<&str> = keyed.iter().map(|k| k.key.as_str()).collect();
        assert_eq!(keys, ["1:boom:0", "1:boom:1", "null:boom:0", "1:boom:2"]);
        assert_eq!(
            keyed.iter().map(|k| k.number).collect::<Vec<_>>(),
            [1, 2, 3, 4]
        );
    }

    #[test]
    fn check_and_attempt_texts_copy_electron() {
        let check = CheckView {
            exit_code: None,
            output: "z".repeat(1500),
        };
        let text = check_text(2, &check);
        assert!(text.starts_with("Check 2: exit unknown\n"));
        assert_eq!(
            text.chars().count(),
            "Check 2: exit unknown\n".chars().count() + 1200
        );
        let attempt: AttemptView = serde_json::from_value(json!({
            "childId": "c1", "phase": "work",
            "profile": {"name": "Fast", "model": "gpt-5"},
            "output": ""
        }))
        .unwrap();
        assert_eq!(
            attempt_title(&attempt),
            "work · Fast · gpt-5 · effort default"
        );
        assert_eq!(attempt_child_line(&attempt), "c1 · running");
        assert_eq!(attempt_output(&attempt), None);
        let attempt: AttemptView = serde_json::from_value(json!({
            "childId": "c1", "phase": "review", "status": "idle",
            "profile": {"name": "Fast", "model": "gpt-5", "thinkingOptionId": "max"},
            "output": "o".repeat(3000)
        }))
        .unwrap();
        assert_eq!(
            attempt_title(&attempt),
            "review · Fast · gpt-5 · effort max"
        );
        assert_eq!(attempt_child_line(&attempt), "c1 · idle");
        assert_eq!(attempt_output(&attempt).unwrap().chars().count(), 2500);
    }

    fn jobs_payload() -> JobsPayload {
        serde_json::from_value(json!({
            "jobs": [
                {
                    "key": "k1", "parentId": "agent-1", "cwd": "/repo",
                    "task": {"id": "t1", "goal": "Do it"},
                    "status": "running", "createdAt": 1, "message": "Working",
                    "attempts": [{
                        "childId": "c1", "phase": "work", "startedAt": 1,
                        "profile": {"name": "Fast", "model": "gpt-5", "thinkingOptionId": "high"},
                        "checks": [{"argv": ["npm", "test"], "exitCode": 0, "output": "ok",
                                    "durationMs": 5, "timedOut": false}]
                    }]
                },
                {"task": {"id": "bad"}, "status": 7},
                {
                    "task": {"id": "t2"}, "status": "passed", "message": "Done",
                    "notifyError": "no parent", "attempts": []
                }
            ],
            "history": [{"anything": true}]
        }))
        .unwrap()
    }

    #[test]
    fn a_malformed_job_is_skipped_and_the_others_show() {
        let jobs = decode_jobs(jobs_payload());
        let ids: Vec<&str> = jobs.iter().map(|j| j.task.id.as_str()).collect();
        assert_eq!(ids, ["t1", "t2"]);
        assert_eq!(job_title(&jobs[0]), "t1 · running");
        assert_eq!(
            jobs[0].attempts[0].checks.as_ref().unwrap()[0].exit_code,
            Some(0)
        );
        assert_eq!(jobs[1].notify_error.as_deref(), Some("no parent"));
    }

    fn job(id: &str) -> JobView {
        JobView {
            task: super::TaskRef { id: id.into() },
            status: "running".into(),
            message: "m".into(),
            notify_error: None,
            attempts: Vec::new(),
        }
    }

    #[test]
    fn the_jobs_list_follows_the_electron_order() {
        let mut state = JobsState::default();
        assert_eq!(
            state.items(),
            [
                JobsItem::Title,
                JobsItem::Hint,
                JobsItem::Loading,
                JobsItem::Refresh
            ]
        );
        state.listed(Err(()));
        assert_eq!(
            state.items(),
            [
                JobsItem::Title,
                JobsItem::Hint,
                JobsItem::Failed,
                JobsItem::Refresh
            ]
        );
        state.listed(Ok(vec![]));
        assert_eq!(
            state.items(),
            [
                JobsItem::Title,
                JobsItem::Hint,
                JobsItem::Empty,
                JobsItem::Refresh
            ]
        );
        state.listed(Ok(vec![job("a"), job("b")]));
        assert_eq!(state.items().len(), 5);
        // A later failure keeps the jobs that were shown.
        state.listed(Err(()));
        assert_eq!(
            state.items(),
            [
                JobsItem::Title,
                JobsItem::Hint,
                JobsItem::Failed,
                JobsItem::Job(job("a")),
                JobsItem::Job(job("b")),
                JobsItem::Refresh
            ]
        );
    }

    #[test]
    fn cancel_is_one_at_a_time_and_its_failure_shows_until_the_next_cancel() {
        let mut state = JobsState::default();
        assert!(state.begin_cancel());
        assert!(!state.begin_cancel());
        state.cancel_finished(false);
        assert!(state.cancel_failed && !state.cancelling);
        assert!(state.items().contains(&JobsItem::Failed));
        assert!(state.begin_cancel());
        assert!(!state.cancel_failed);
        state.cancel_finished(true);
        assert!(!state.items().contains(&JobsItem::Failed));
    }

    #[test]
    fn poll_due_waits_5000_ms_and_never_runs_twice_at_once() {
        assert!(poll_due(0, None, false));
        assert!(!poll_due(4_999, Some(0), false));
        assert!(poll_due(5_000, Some(0), false));
        assert!(poll_due(9_000, Some(0), false));
        // Never while a request is in flight, however old the last send is.
        assert!(!poll_due(60_000, Some(0), true));
        assert!(!poll_due(0, None, true));
    }

    #[test]
    fn the_poller_sends_once_per_interval_and_only_while_connected() {
        let mut poller = Poller::default();
        assert!(!poller.due(0), "not connected");
        assert_eq!(poller.wait_ms(0), None);
        poller.connected();
        assert!(poller.due(0));
        poller.begin(0);
        assert!(!poller.due(10_000), "in flight");
        assert_eq!(poller.wait_ms(10_000), None);
        assert!(!poller.finish());
        assert!(!poller.due(4_999));
        assert_eq!(poller.wait_ms(1_000), Some(4_000));
        assert!(poller.due(5_000));
        assert_eq!(poller.wait_ms(6_000), Some(0));
        // A reconnect polls at once.
        poller.begin(5_000);
        poller.finish();
        poller.disconnected();
        assert!(!poller.due(99_000));
        poller.connected();
        assert!(poller.due(5_001));
    }

    #[test]
    fn a_user_request_skips_the_interval_but_not_the_in_flight_guard() {
        let mut poller = Poller::default();
        assert!(!poller.request_now(), "not connected");
        poller.connected();
        poller.begin(0);
        poller.finish();
        assert!(
            poller.request_now(),
            "1 s after the last send, a click sends"
        );
        poller.begin(1_000);
        assert!(!poller.request_now(), "queued behind the request in flight");
        assert!(
            poller.finish(),
            "the queued one goes when the first returns"
        );
        assert!(!poller.finish());
        poller.begin(1_100);
        assert!(!poller.request_now());
        poller.disconnected();
        assert!(!poller.finish(), "a disconnect drops the queued request");
    }

    // ---- Direct route

    #[test]
    fn luna_offers_max_only_and_the_order_is_canonical() {
        assert_eq!(
            offered_effort_ids("claude-luna-1", &[Effort::Low, Effort::Max]),
            [Effort::Max]
        );
        assert_eq!(
            offered_effort_ids("gpt-5", &[Effort::Max, Effort::Low]),
            [Effort::Low, Effort::Max]
        );
        assert_eq!(
            offered_effort_ids("luna", &[Effort::Low]),
            Vec::<Effort>::new()
        );
    }

    #[test]
    fn is_luna_model_matches_whole_words() {
        for model in ["luna", "x/luna-2", "Luna", "a-luna", "LUNA/1"] {
            assert!(is_luna_model(model), "{model}");
        }
        for model in ["lunar", "alunax", "lunaa", "xluna", ""] {
            assert!(!is_luna_model(model), "{model}");
        }
    }

    #[test]
    fn two_profiles_on_one_model_make_one_group_with_the_union_of_efforts() {
        let profiles = [
            profile("a", "codex", "gpt-5", &[Effort::Low]),
            profile("b", "claude", "opus", &[Effort::High]),
            profile("c", "codex", "gpt-5", &[Effort::High, Effort::Max]),
        ];
        let groups = build_model_groups(&profiles, &ids(&["c", "a"]));
        assert_eq!(
            groups,
            [ModelGroup {
                key: model_key("codex", "gpt-5"),
                provider: "codex".into(),
                model: "gpt-5".into(),
                effort_ids: vec![Effort::Low, Effort::High, Effort::Max],
                profile_count: 2,
            }]
        );
        assert_eq!(model_key("codex", "gpt-5"), r#"["codex","gpt-5"]"#);
    }

    #[test]
    fn deselecting_the_last_profile_of_a_model_drops_its_efforts() {
        let profiles = [
            profile("a", "codex", "gpt-5", &[Effort::Low]),
            profile("c", "codex", "gpt-5", &[Effort::High]),
            profile("b", "claude", "opus", &[Effort::High]),
        ];
        let mut current = SelectedEfforts::new();
        current.insert(model_key("codex", "gpt-5"), vec![Effort::Low]);
        current.insert(model_key("claude", "opus"), vec![Effort::High]);
        let kept = keep_efforts_for_profiles(&current, &profiles, &ids(&["c", "b"]));
        assert_eq!(kept.len(), 2, "another profile still uses gpt-5");
        let kept = keep_efforts_for_profiles(&current, &profiles, &ids(&["b"]));
        assert_eq!(
            kept.keys().collect::<Vec<_>>(),
            [&model_key("claude", "opus")]
        );
    }

    #[test]
    fn toggled_efforts_keeps_the_canonical_order() {
        assert_eq!(
            toggled_efforts(&[Effort::Max, Effort::Low], Effort::High, false),
            [Effort::Low, Effort::High, Effort::Max]
        );
        assert_eq!(
            toggled_efforts(&[Effort::Low, Effort::High], Effort::Low, true),
            [Effort::High]
        );
        assert_eq!(
            toggled_efforts(&[], Effort::Medium, false),
            [Effort::Medium]
        );
    }

    #[test]
    fn allowed_models_keep_only_the_offered_selected_efforts() {
        let profiles = [profile("a", "codex", "gpt-5", &[Effort::Low, Effort::High])];
        let groups = build_model_groups(&profiles, &ids(&["a"]));
        let mut selected = SelectedEfforts::new();
        selected.insert(groups[0].key.clone(), vec![Effort::Low, Effort::Max]);
        let allowed = build_allowed_models(&groups, &selected);
        assert_eq!(allowed[0].effort_ids, [Effort::Low]);
        assert_eq!(
            build_allowed_models(&groups, &SelectedEfforts::new())[0].effort_ids,
            []
        );
    }

    fn route_input(prompt: &str) -> RouteInput<'_> {
        RouteInput {
            prompt,
            selected_profiles: 1,
            has_effort_for_every_model: true,
            request_locked: false,
            run_pending: false,
            status_pending: false,
            may_retry_same_request: false,
        }
    }

    #[test]
    fn can_route_cases() {
        assert!(can_route(route_input("12345678")));
        assert!(!can_route(route_input("  1234567  ")), "7 trimmed chars");
        assert!(can_route(route_input("  12345678  ")));
        let mut input = route_input("12345678");
        input.selected_profiles = 0;
        assert!(!can_route(input));
        let mut input = route_input("12345678");
        input.has_effort_for_every_model = false;
        assert!(!can_route(input));
        let mut input = route_input("12345678");
        input.run_pending = true;
        assert!(!can_route(input));
        let mut input = route_input("12345678");
        input.status_pending = true;
        assert!(!can_route(input));
        let mut input = route_input("12345678");
        input.request_locked = true;
        assert!(!can_route(input), "a locked request cannot start a new one");
        input.may_retry_same_request = true;
        assert!(can_route(input), "but it can retry itself");
        input.run_pending = true;
        assert!(!can_route(input));
    }

    fn outcome_input<'a>(
        request_id: Option<&'a str>,
        records: Option<&'a [Value]>,
    ) -> OutcomeInput<'a> {
        OutcomeInput {
            request_id,
            result: None,
            records,
            run_success: false,
            run_error: false,
            status_success: records.is_some(),
        }
    }

    #[test]
    fn derive_outcome_cases() {
        // A run error with a successful empty status may retry the same request.
        let empty: Vec<Value> = vec![];
        let mut input = outcome_input(Some("r1"), Some(&empty));
        input.run_error = true;
        let outcome = derive_outcome(input);
        assert!(outcome.may_retry_same_request && !outcome.may_reset);
        // A record for another request, or a malformed one, is not an empty status.
        let other = vec![json!({"requestId": "r2", "status": "created"})];
        let mut input = outcome_input(Some("r1"), Some(&other));
        input.run_error = true;
        assert!(!derive_outcome(input).may_retry_same_request);
        // A created record gives the agent and the selection.
        let created = vec![json!({
            "requestId": "r1", "status": "created", "agentId": "a7",
            "selection": {"name": "Fast", "model": "gpt-5", "thinkingOptionId": "low"}
        })];
        let mut input = outcome_input(Some("r1"), Some(&created));
        input.run_error = true;
        let outcome = derive_outcome(input);
        assert_eq!(outcome.agent_id.as_deref(), Some("a7"));
        assert_eq!(outcome.selection.unwrap().name.as_deref(), Some("Fast"));
        assert!(outcome.may_reset && !outcome.may_retry_same_request);
        // A failed record may reset.
        let failed = vec![json!({"requestId": "r1", "status": "failed", "error": "no"})];
        let outcome = derive_outcome(outcome_input(Some("r1"), Some(&failed)));
        assert!(outcome.may_reset);
        assert_eq!(
            outcome.matching_record.unwrap().error.as_deref(),
            Some("no")
        );
        // The run result wins over the record, and a success may reset.
        let result = run_result();
        let mut input = outcome_input(Some("r1"), Some(&created));
        input.result = Some(&result);
        input.run_success = true;
        let outcome = derive_outcome(input);
        assert_eq!(outcome.agent_id.as_deref(), Some("agent-9"));
        assert_eq!(
            outcome.selection.unwrap().thinking_option_id.as_deref(),
            Some("high")
        );
        assert!(outcome.may_reset);
        // No request: no record matches.
        assert_eq!(
            derive_outcome(outcome_input(None, Some(&created))).matching_record,
            None
        );
    }

    #[test]
    fn a_record_with_an_unknown_status_or_a_non_string_request_id_is_ignored() {
        assert_eq!(
            as_direct_record(&json!({"requestId": "r", "status": "unknown"})),
            None
        );
        assert_eq!(
            as_direct_record(&json!({"requestId": 5, "status": "created"})),
            None
        );
        assert_eq!(as_direct_record(&json!({"status": "created"})), None);
        assert_eq!(as_direct_record(&json!("created")), None);
        assert_eq!(as_direct_record(&json!(null)), None);
        for status in ["routing", "creating", "created", "failed", "interrupted"] {
            let record = as_direct_record(&json!({"requestId": "r", "status": status})).unwrap();
            assert_eq!(record.status.as_str(), status);
        }
        assert_eq!(
            as_direct_record(&json!({"requestId": "r", "status": "routing"}))
                .unwrap()
                .status,
            RecordStatus::Routing
        );
    }

    #[test]
    fn route_label_cases() {
        assert_eq!(route_label(true, false), "Routing…");
        assert_eq!(route_label(true, true), "Routing…");
        assert_eq!(route_label(false, true), "Retry same request");
        assert_eq!(route_label(false, false), "Route and run");
        assert_eq!(
            route_aria_label(true),
            "Retry the same direct routing request"
        );
        assert_eq!(route_aria_label(false), "Route and run");
    }

    #[test]
    fn route_takes_no_arguments() {
        assert!(slash_route_allowed(""));
        assert!(slash_route_allowed("  "));
        assert!(!slash_route_allowed("x"));
    }

    #[test]
    fn the_prompt_is_cut_to_12000_chars() {
        assert_eq!(clamp_prompt("short"), None);
        assert_eq!(clamp_prompt(&"a".repeat(12_000)), None);
        assert_eq!(
            clamp_prompt(&"é".repeat(12_001)).unwrap().chars().count(),
            12_000
        );
    }

    #[test]
    fn the_created_line_falls_back_like_electron() {
        assert_eq!(
            created_line(&SelectionView {
                name: Some("Fast".into()),
                model: Some("gpt-5".into()),
                thinking_option_id: Some("high".into()),
            }),
            "Fast · gpt-5 · effort high"
        );
        assert_eq!(
            created_line(&SelectionView::default()),
            "Unknown model · effort unknown"
        );
    }

    // ---- the state machine

    fn ready_state() -> DirectState {
        let mut state = DirectState::new("ws-1".into());
        state.profiles_loaded(Ok(vec![
            profile("a", "codex", "gpt-5", &[Effort::Low, Effort::High]),
            profile("b", "claude", "opus", &[Effort::High]),
            profile("n", "codex", "luna", &[Effort::Low]),
        ]));
        state
    }

    fn routed_state() -> DirectState {
        let mut state = ready_state();
        state.set_prompt("Build the thing".into());
        state.toggle_profile("a");
        let key = model_key("codex", "gpt-5");
        state.toggle_effort(&key, Effort::High);
        state.toggle_effort(&key, Effort::Low);
        state
    }

    #[test]
    fn the_run_input_has_camel_case_keys_and_a_v4_request_id() {
        let mut state = routed_state();
        assert!(state.can_route());
        let request = state.route(new_request_id).unwrap();
        let value = serde_json::to_value(&request).unwrap();
        assert_eq!(value["workspaceId"], "ws-1");
        assert_eq!(value["prompt"], "Build the thing");
        assert_eq!(value["allowedProfileIds"], json!(["a"]));
        assert_eq!(
            value["allowedModels"],
            json!([{"provider": "codex", "model": "gpt-5", "effortIds": ["low", "high"]}])
        );
        assert_eq!(value["shareWithJev"], json!(true));
        let id = uuid::Uuid::parse_str(value["requestId"].as_str().unwrap()).unwrap();
        assert_eq!(id.get_version_num(), 4);
        let mut keys: Vec<&str> = value
            .as_object()
            .unwrap()
            .keys()
            .map(String::as_str)
            .collect();
        keys.sort_unstable();
        assert_eq!(
            keys,
            [
                "allowedModels",
                "allowedProfileIds",
                "prompt",
                "requestId",
                "shareWithJev",
                "workspaceId"
            ]
        );
    }

    #[test]
    fn at_most_eight_profiles_and_a_profile_without_effort_cannot_be_picked() {
        let mut state = DirectState::new("ws".into());
        let profiles: Vec<ProfileView> = (0..9)
            .map(|i| profile(&format!("p{i}"), "codex", &format!("m{i}"), &[Effort::Low]))
            .chain([profile("none", "codex", "mx", &[])])
            .collect();
        state.profiles_loaded(Ok(profiles));
        for i in 0..9 {
            state.toggle_profile(&format!("p{i}"));
        }
        assert_eq!(state.selected_profiles.len(), 8);
        assert!(!state.selected_profiles.contains(&"p8".to_string()));
        state.toggle_profile("p0");
        state.toggle_profile("none");
        assert!(!state.selected_profiles.contains(&"none".to_string()));
        state.toggle_profile("p8");
        assert!(state.selected_profiles.contains(&"p8".to_string()));
    }

    #[test]
    fn deselecting_the_last_profile_of_a_model_clears_its_selected_efforts() {
        let mut state = routed_state();
        assert_eq!(state.selected_efforts.len(), 1);
        state.toggle_profile("a");
        assert!(state.selected_efforts.is_empty());
        assert!(state.selected_profiles.is_empty());
    }

    #[test]
    fn a_locked_request_freezes_the_selection_until_reset() {
        let mut state = routed_state();
        state.route(new_request_id).unwrap();
        assert!(state.locked() && !state.can_route(), "a run is pending");
        state.toggle_profile("b");
        state.toggle_effort(&model_key("codex", "gpt-5"), Effort::Low);
        assert_eq!(state.selected_profiles, ["a"]);
        assert_eq!(state.selected_efforts.values().next().unwrap().len(), 2);
        state.run_finished(Ok(run_result()));
        assert!(state.outcome().may_reset);
        state.reset();
        assert!(!state.locked());
        assert!(state.prompt.is_empty() && state.selected_profiles.is_empty());
        assert!(state.selected_efforts.is_empty());
        assert_eq!(state.run, Call::Idle);
        assert!(matches!(state.profiles, super::ProfilesState::Ready(_)));
    }

    #[test]
    fn a_failed_run_checks_the_status_then_retries_the_same_request() {
        let mut state = routed_state();
        let first = state.route(new_request_id).unwrap();
        state.run_finished(Err(()));
        assert!(
            state
                .items()
                .contains(&DirectItem::RunError { checking: false })
        );
        assert!(!state.can_route(), "retry waits for the status check");
        let input = state.check_status().unwrap();
        assert_eq!(input.request_id, first.request_id);
        assert_eq!(input.workspace_id, "ws-1");
        assert_eq!(state.check_status(), None, "one check at a time");
        assert!(
            state
                .items()
                .contains(&DirectItem::RunError { checking: true })
        );
        state.status_finished(Ok(vec![]));
        assert!(state.items().contains(&DirectItem::StatusNone));
        assert!(state.can_route());
        let items = state.items();
        assert!(items.iter().any(|item| matches!(
            item,
            DirectItem::Route {
                enabled: true,
                label: "Retry same request",
                ..
            }
        )));
        let again: DirectRequest = state.route(|| panic!("a retry keeps its id")).unwrap();
        assert_eq!(again, first);
        assert_eq!(state.status, Call::Idle, "a new run clears the old check");
        assert_eq!(state.run, Call::Pending);
    }

    #[test]
    fn a_status_error_and_an_interrupted_record_show_their_notices() {
        let mut state = routed_state();
        state.route(new_request_id).unwrap();
        state.run_finished(Err(()));
        state.check_status().unwrap();
        state.status_finished(Err(()));
        assert!(state.items().contains(&DirectItem::StatusError));
        let id = state.request.as_ref().unwrap().request_id.clone();
        state.check_status().unwrap();
        state.status_finished(Ok(vec![json!({"requestId": id, "status": "interrupted"})]));
        assert!(state.items().iter().any(|item| matches!(
            item,
            DirectItem::Record {
                status: RecordStatus::Interrupted,
                error: None
            }
        )));
        assert!(!state.outcome().may_reset);
    }

    #[test]
    fn the_page_order_matches_electron() {
        let mut state = routed_state();
        let items = state.items();
        let order: Vec<&str> = items
            .iter()
            .map(|item| match item {
                DirectItem::Title => "title",
                DirectItem::Prompt { .. } => "prompt",
                DirectItem::ProfilesIntro => "profiles-intro",
                DirectItem::Profile(_) => "profile",
                DirectItem::EffortIntro => "effort-intro",
                DirectItem::EffortGroup(_) => "effort-group",
                DirectItem::Route { .. } => "route",
                _ => "other",
            })
            .collect();
        assert_eq!(
            order,
            [
                "title",
                "prompt",
                "profiles-intro",
                "profile",
                "profile",
                "profile",
                "effort-intro",
                "effort-group",
                "route"
            ]
        );
        state.route(new_request_id).unwrap();
        state.run_finished(Ok(run_result()));
        let items = state.items();
        let tail: Vec<&DirectItem> = items.iter().rev().take(3).collect();
        assert!(matches!(tail[0], DirectItem::Reset));
        assert!(matches!(tail[1], DirectItem::Created { .. }));
        assert!(matches!(tail[2], DirectItem::RequestId(_)));
    }

    #[test]
    fn profile_rows_show_selection_disabled_state_and_the_luna_note() {
        let state = routed_state();
        let rows: Vec<_> = state
            .items()
            .into_iter()
            .filter_map(|item| match item {
                DirectItem::Profile(row) => Some(row),
                _ => None,
            })
            .collect();
        assert_eq!(rows[0].title, "Selected · Name a");
        assert_eq!(rows[0].subtitle, "codex · gpt-5 · mode default");
        assert!(rows[0].selected && !rows[0].disabled);
        assert_eq!(rows[1].title, "Name b");
        // Luna offers max only, so a profile with low only has no effort.
        assert!(rows[2].no_effort && rows[2].disabled);
    }

    #[test]
    fn profiles_load_once_and_retry_only_on_request() {
        let mut state = DirectState::new("ws".into());
        assert_eq!(state.items()[3], DirectItem::ProfilesLoading);
        state.profiles_loaded(Err(()));
        assert_eq!(
            state.items()[3],
            DirectItem::ProfilesError { fetching: false }
        );
        assert!(state.retry_profiles());
        assert!(!state.retry_profiles());
        assert_eq!(
            state.items()[3],
            DirectItem::ProfilesError { fetching: true }
        );
        state.profiles_loaded(Ok(vec![]));
        assert_eq!(state.items()[3], DirectItem::ProfilesEmpty);
    }

    // ---- real payloads

    #[test]
    fn decodes_a_real_direct_profiles_payload() {
        let payload: ProfilesPayload = serde_json::from_value(json!({
            "profiles": [
                {"id": "p1", "name": "Fast", "provider": "codex", "model": "gpt-5",
                 "modeId": "auto", "notes": "Cheap", "effortIds": ["low", "max"]},
                {"id": "p2", "name": "Deep", "provider": "claude", "model": "opus",
                 "effortIds": []}
            ]
        }))
        .unwrap();
        assert_eq!(payload.profiles[0].effort_ids, [Effort::Low, Effort::Max]);
        assert_eq!(payload.profiles[0].mode_id.as_deref(), Some("auto"));
        assert_eq!(payload.profiles[1].notes, None);
        assert!(
            serde_json::from_value::<ProfilesPayload>(
                json!({"profiles": [{"id": "x", "name": "n", "provider": "p", "model": "m",
                                  "effortIds": ["ultra"]}]})
            )
            .is_err()
        );
    }

    #[test]
    fn decodes_a_real_direct_run_payload() {
        let result = run_result();
        assert_eq!(result.agent_id, "agent-9");
        assert_eq!(result.selection.mode_id.as_deref(), Some("auto"));
        let bare: RunResult = serde_json::from_value(json!({
            "agentId": "a", "routingMs": 3,
            "selection": {"id": "p", "name": "n", "provider": "x", "model": "m",
                          "thinkingOptionId": "low"},
            "usage": null
        }))
        .unwrap();
        assert_eq!(bare.selection.mode_id, None);
        assert!(serde_json::from_value::<RunResult>(json!({"agentId": "a"})).is_err());
    }

    #[test]
    fn decodes_a_real_direct_status_payload() {
        let payload: StatusPayload = serde_json::from_value(json!({
            "records": [
                {"requestId": "r1", "status": "created", "agentId": "a1"},
                {"junk": true},
                "text"
            ]
        }))
        .unwrap();
        assert_eq!(payload.records.len(), 3);
        let valid: Vec<_> = payload
            .records
            .iter()
            .filter_map(as_direct_record)
            .collect();
        assert_eq!(valid.len(), 1);
        assert_eq!(valid[0].agent_id.as_deref(), Some("a1"));
    }

    #[test]
    fn the_jobs_list_payload_ignores_its_history() {
        let payload: JobsPayload =
            serde_json::from_value(json!({"jobs": [], "history": [1, 2]})).unwrap();
        assert!(payload.jobs.is_empty());
    }
}
