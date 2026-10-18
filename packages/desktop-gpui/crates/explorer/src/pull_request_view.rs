//! The Pull request pane, as the Electron PR panel draws it (`git/pull-request-panel/pane.tsx`):
//! a toolbar with View and Refresh, the header with the title, number, state, and repo, the
//! collapsible Checks section, and the collapsible Activity section with its count pills and the
//! reviews and comments of the timeline. Item bodies are Markdown, through gpui-kit's text view.
//!
//! The view never polls. It fetches the PR status on creation, on each connection, on Refresh,
//! and once per 500 ms burst of `GitStatusEvent::Changed` for its folder. The timeline loads only
//! when the status has a number, a repo owner, and a repo name.

use crate::git_actions::RefreshGate;
use bachuc_client::{
    Client, ClientEvent, GitAction, GitStatusEvent, PrCheck, PrStatus, PrStatusReply,
    PrTimelineItem,
};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_LG, ICON_SM, ICON_XS, RADIUS_BASE, RADIUS_LG, RADIUS_MD, RADIUS_SM,
    SPACE_0_5, SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_6,
};
use bachuc_theme::{
    ShadowSize, Tokens, mono_font_family, popup_offset, shadow, tokens, ui_font_size,
};
use gpui_kit::component::text::TextView;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// The wait that folds a burst of status events into one refetch.
const REFETCH_COALESCE: Duration = Duration::from_millis(500);
const SPIN: Duration = Duration::from_millis(1000);
/// `WORKSPACE_SECONDARY_HEADER_HEIGHT` and `SMALL_ICON_BUTTON_SIZE` of the toolbar.
const TOOLBAR_HEIGHT: f32 = 36.;
const TOOLBAR_BUTTON: f32 = 20.;
/// The `viewButton` height.
const VIEW_BUTTON_HEIGHT: f32 = 24.;
/// The title line height of `styles.title`.
const TITLE_LINE_HEIGHT: f32 = 22.;
/// `LIST_MAX_HEIGHT` of `checks-section.tsx`: about eight rows, then the list scrolls.
const CHECKS_MAX_HEIGHT: f32 = 268.;
/// `CONTROL_HEIGHTS.compact`, the check row height.
const CHECK_ROW_HEIGHT: f32 = 32.;
const ENTRY_HEADER_MIN_HEIGHT: f32 = 36.;
const AVATAR_SIZE: f32 = 20.;
const KEBAB_SLOT: f32 = 22.;
/// The kebab button: the icon with 2 px of padding on each side.
const KEBAB_TRIGGER_HEIGHT: f32 = ICON_SM + 4.;
const MENU_WIDTH: f32 = 200.;
const MENU_ITEM_HEIGHT: f32 = 28.;
const TEXT_LINE_FACTOR: f32 = 1.4;

/// What the view asks the app to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PullRequestEvent {
    /// View pull request, a check, or an activity item.
    OpenUrl { url: String },
}

/// A note for the app toast, apart from `PullRequestEvent` because the app matches that enum
/// exhaustively. A failed Refresh sends one with `ok` false, as Electron's `toast.error`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct PullRequestNoticeEvent {
    pub ok: bool,
    pub message: String,
}

// ---- rules ----

/// `PrState` of `data.ts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum PrState {
    Open,
    Draft,
    Merged,
    Closed,
}

/// `derivePrState`: merged wins, then anything not open is closed, then draft.
fn pr_state(status: &PrStatus) -> PrState {
    if status.is_merged || status.state == "merged" {
        PrState::Merged
    } else if status.state != "open" {
        PrState::Closed
    } else if status.is_draft {
        PrState::Draft
    } else {
        PrState::Open
    }
}

/// `getStateLabel`.
fn state_label(state: PrState) -> &'static str {
    match state {
        PrState::Open => "Open",
        PrState::Draft => "Draft",
        PrState::Merged => "Merged",
        PrState::Closed => "Closed",
    }
}

/// The three Activity pills. A pill with a zero count is not drawn.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
struct CountPills {
    approvals: usize,
    changes_requested: usize,
    comments: usize,
}

/// The pill counts of `PullRequestPane`: a comment or a `commented` review counts as a comment.
fn count_pills(items: &[PrTimelineItem]) -> CountPills {
    let mut pills = CountPills::default();
    for item in items {
        match item {
            PrTimelineItem::Review { state, .. } => match state.as_str() {
                "approved" => pills.approvals += 1,
                "changes_requested" => pills.changes_requested += 1,
                "commented" => pills.comments += 1,
                _ => {}
            },
            PrTimelineItem::Comment { .. } => pills.comments += 1,
        }
    }
    pills
}

/// `mapActivity`: a comment with an empty body, and a `commented` review with an empty body,
/// are dropped.
fn visible_activity(items: Vec<PrTimelineItem>) -> Vec<PrTimelineItem> {
    items
        .into_iter()
        .filter(|item| match item {
            PrTimelineItem::Comment { body, .. } => !body.trim().is_empty(),
            PrTimelineItem::Review { body, state, .. } => {
                state != "commented" || !body.trim().is_empty()
            }
        })
        .collect()
}

/// `getActivityVerb`, lower case as the card draws it.
fn activity_verb(item: &PrTimelineItem) -> &'static str {
    match item {
        PrTimelineItem::Comment { .. } => "commented",
        PrTimelineItem::Review { state, .. } => match state.as_str() {
            "approved" => "approved",
            "changes_requested" => "requested changes",
            _ => "reviewed",
        },
    }
}

/// `formatAge`.
fn format_age(created_at_ms: i64, now_ms: i64) -> String {
    let seconds = (now_ms - created_at_ms).max(0) / 1000;
    if seconds < 60 {
        return "just now".into();
    }
    let minutes = seconds / 60;
    if minutes < 60 {
        return format!("{minutes}m ago");
    }
    let hours = minutes / 60;
    if hours < 24 {
        return format!("{hours}h ago");
    }
    let days = hours / 24;
    if days < 30 {
        format!("{days}d ago")
    } else if days < 365 {
        format!("{}mo ago", days / 30)
    } else {
        format!("{}y ago", days / 365)
    }
}

/// The PR whose timeline the view loads.
#[derive(Clone, Debug, PartialEq, Eq)]
struct TimelineTarget {
    number: u64,
    repo_owner: String,
    repo_name: String,
}

/// `shouldFetchTimelineFrom`: GitHub features on, and a number, an owner, and a name.
fn timeline_target(reply: &PrStatusReply) -> Option<TimelineTarget> {
    let status = reply.status.as_ref()?;
    if !reply.github_features_enabled || status.repo_owner.is_empty() || status.repo_name.is_empty()
    {
        return None;
    }
    Some(TimelineTarget {
        number: status.number?,
        repo_owner: status.repo_owner.clone(),
        repo_name: status.repo_name.clone(),
    })
}

/// `CheckPresentation` of `check-presentation.ts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum CheckKind {
    ActionRequired,
    Warning,
    Failure,
    Pending,
    Manual,
    Success,
    Ignored,
}

/// `PRESENTATION_ORDER`: what the user can act on, then what is running, then the rest.
const CHECK_ORDER: [CheckKind; 7] = [
    CheckKind::ActionRequired,
    CheckKind::Warning,
    CheckKind::Failure,
    CheckKind::Pending,
    CheckKind::Manual,
    CheckKind::Success,
    CheckKind::Ignored,
];

/// `mapCheckStatus`: a status outside the five known ones is pending.
fn is_pending_status(status: &str) -> bool {
    !matches!(status, "success" | "failure" | "skipped" | "cancelled")
}

/// `classifyCheck`: a known trait wins over the status.
fn classify_check(check: &PrCheck) -> CheckKind {
    let has = |name: &str| check.traits.iter().any(|t| t == name);
    if has("action_required") {
        CheckKind::ActionRequired
    } else if has("warning") {
        CheckKind::Warning
    } else if has("manual") {
        CheckKind::Manual
    } else {
        match check.status.as_str() {
            "success" => CheckKind::Success,
            "failure" => CheckKind::Failure,
            "skipped" | "cancelled" => CheckKind::Ignored,
            _ => CheckKind::Pending,
        }
    }
}

/// `PRESENTATION_NOUN`.
fn check_noun(kind: CheckKind) -> &'static str {
    match kind {
        CheckKind::ActionRequired => "needs action",
        CheckKind::Warning => "warning",
        CheckKind::Failure => "failing",
        CheckKind::Pending => "in progress",
        CheckKind::Manual => "manual",
        CheckKind::Success => "successful",
        CheckKind::Ignored => "skipped",
    }
}

fn count_noun(count: usize) -> &'static str {
    if count == 1 { "check" } else { "checks" }
}

/// `formatCheckTiming`: a running check says so, and no duration shows nothing.
fn check_timing(duration: Option<&str>, status: &str) -> Option<String> {
    let duration = duration.filter(|duration| !duration.is_empty())?;
    Some(if is_pending_status(status) {
        format!("running {duration}")
    } else {
        duration.to_string()
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChecksGroup {
    kind: CheckKind,
    /// For example "2 failing checks".
    label: String,
    /// Indexes into the status checks.
    checks: Vec<usize>,
}

#[derive(Clone, Debug, PartialEq, Eq)]
struct ChecksSummary {
    /// `None` when there are no checks. `Failure` also stands for a warning.
    outcome: Option<CheckKind>,
    headline: &'static str,
    /// For example "2 failing, 1 successful checks"; empty with no checks.
    detail: String,
    groups: Vec<ChecksGroup>,
}

/// `summarizeChecks`: the headline names the worst outcome, and the groups follow `CHECK_ORDER`.
fn summarize_checks(checks: &[PrCheck]) -> ChecksSummary {
    let kinds: Vec<CheckKind> = checks.iter().map(classify_check).collect();
    let mut groups = Vec::new();
    let mut parts = Vec::new();
    for kind in CHECK_ORDER {
        let matching: Vec<usize> = (0..kinds.len()).filter(|&i| kinds[i] == kind).collect();
        if matching.is_empty() {
            continue;
        }
        let count = matching.len();
        parts.push(format!("{count} {}", check_noun(kind)));
        groups.push(ChecksGroup {
            kind,
            label: format!("{count} {} {}", check_noun(kind), count_noun(count)),
            checks: matching,
        });
    }
    // `selectOutcome`: a failure outranks anything still running.
    let outcome = if kinds.is_empty() {
        None
    } else if kinds.contains(&CheckKind::ActionRequired) {
        Some(CheckKind::ActionRequired)
    } else if kinds.contains(&CheckKind::Failure) || kinds.contains(&CheckKind::Warning) {
        Some(CheckKind::Failure)
    } else if checks.iter().any(|check| is_pending_status(&check.status)) {
        Some(CheckKind::Pending)
    } else {
        Some(CheckKind::Success)
    };
    let headline = match outcome {
        None => "No checks",
        Some(CheckKind::ActionRequired) => "Some checks need your attention",
        Some(CheckKind::Failure) => "Some checks were not successful",
        Some(CheckKind::Pending) => "Some checks haven't completed yet",
        Some(_) => "All checks have passed",
    };
    let detail = if parts.is_empty() {
        String::new()
    } else {
        format!("{} {}", parts.join(", "), count_noun(kinds.len()))
    };
    ChecksSummary {
        outcome,
        headline,
        detail,
        groups,
    }
}

/// The ring geometry of `checks-ring.tsx`, in a 20-unit box scaled to the drawn size.
const RING_VIEW_BOX: f32 = 20.;
const RING_STROKE: f32 = 3.;
const RING_RADIUS: f32 = (RING_VIEW_BOX - RING_STROKE) / 2.;
const RING_CIRCUMFERENCE: f32 = 2. * std::f32::consts::PI * RING_RADIUS;
/// The blank arc between two segments.
const RING_SEGMENT_GAP: f32 = 2.;
/// The shortest segment, so one failure among twenty checks still shows.
const RING_MIN_SEGMENT: f32 = 1.5;
/// Line pieces per full circle of a drawn arc.
const RING_ARC_STEPS: f32 = 64.;

/// One painted arc of the ring, in view-box units along the circle from twelve o'clock.
#[derive(Clone, Copy, Debug, PartialEq)]
struct RingSegment {
    kind: CheckKind,
    dash: f32,
    offset: f32,
}

/// `buildSegments`: each group gets a share of the circle, clockwise from twelve. The gap comes
/// out of the painted arc, so the shares still fill the circle; a single group owns all of it.
fn ring_segments(groups: &[ChecksGroup]) -> Vec<RingSegment> {
    let total: usize = groups.iter().map(|group| group.checks.len()).sum();
    let mut offset = 0.;
    groups
        .iter()
        .map(|group| {
            let arc = group.checks.len() as f32 / total as f32 * RING_CIRCUMFERENCE;
            let dash = if groups.len() == 1 {
                RING_CIRCUMFERENCE
            } else {
                (arc - RING_SEGMENT_GAP).max(RING_MIN_SEGMENT)
            };
            let segment = RingSegment {
                kind: group.kind,
                dash,
                offset,
            };
            offset += arc;
            segment
        })
        .collect()
}

/// The ring palette of `checks-ring.tsx`.
fn ring_color(kind: CheckKind, colors: &Tokens) -> Hsla {
    match kind {
        CheckKind::ActionRequired | CheckKind::Warning | CheckKind::Pending => {
            colors.status_warning
        }
        CheckKind::Failure => colors.status_danger,
        CheckKind::Manual => colors.foreground_muted,
        CheckKind::Success => colors.status_success,
        CheckKind::Ignored => colors.foreground_extra_muted,
    }
}

/// `formatPathAndLines` of an inline review comment.
fn comment_location(path: Option<&str>, line: Option<u32>) -> Option<String> {
    let path = path.filter(|path| !path.is_empty())?;
    Some(match line {
        Some(line) => format!("{path}:{line}"),
        None => path.to_string(),
    })
}

/// `IDENTITY_COLORS` and `deriveIdentityColorName` over the lower-case login, as
/// `deriveAvatarColor` picks a color.
const IDENTITY_COLORS: [u32; 10] = [
    0x7a6aa8, 0x3d7ea6, 0x388068, 0xa4673a, 0xb05c80, 0x6a70b8, 0x368080, 0xb06260, 0x8f7838,
    0x5179b0,
];

fn avatar_color(login: &str) -> u32 {
    let hash = login
        .to_lowercase()
        .encode_utf16()
        .fold(0u32, |hash, unit| {
            hash.wrapping_mul(31).wrapping_add(u32::from(unit))
        });
    IDENTITY_COLORS[hash as usize % IDENTITY_COLORS.len()]
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_or(0, |elapsed| elapsed.as_millis() as i64)
}

// ---- view ----

enum Load {
    Loading,
    Failed(String),
    Ready(Box<PrStatusReply>),
}

enum Activity {
    /// No timeline to load: no number, no repo, or GitHub features off.
    None,
    Loading,
    /// The message is in the log; the section shows Electron's fixed text.
    Failed,
    Ready(Vec<PrTimelineItem>),
}

pub struct PullRequestView {
    client: Entity<Client>,
    cwd: String,
    load: Load,
    activity: Activity,
    /// The PR the activity belongs to; a new PR resets the activity and its collapse state.
    activity_target: Option<TimelineTarget>,
    /// The fetch in flight or last applied; an older answer is dropped.
    generation: u64,
    refetch_gate: RefreshGate,
    refreshing: bool,
    checks_open: bool,
    activity_open: bool,
    collapsed_groups: Vec<CheckKind>,
    /// Indexes of the activity cards the user collapsed.
    collapsed_items: Vec<usize>,
    /// The activity card whose menu is open.
    menu_open: Option<usize>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<PullRequestEvent> for PullRequestView {}
impl EventEmitter<PullRequestNoticeEvent> for PullRequestView {}

impl PullRequestView {
    pub fn new(
        client: Entity<Client>,
        cwd: String,
        cx: &mut Context<PullRequestView>,
    ) -> PullRequestView {
        let subscriptions = vec![
            cx.subscribe(
                &client,
                |this: &mut PullRequestView, _, event: &GitStatusEvent, cx| {
                    if let GitStatusEvent::Changed { cwd } = event
                        && *cwd == this.cwd
                    {
                        this.schedule_fetch(cx);
                    }
                },
            ),
            // A request sent while the client connects fails, so load again on each connection.
            cx.subscribe(
                &client,
                |this: &mut PullRequestView, _, event: &ClientEvent, cx| {
                    if let ClientEvent::Connected = event {
                        this.fetch(cx);
                    }
                },
            ),
        ];
        let mut view = PullRequestView {
            client,
            cwd,
            load: Load::Loading,
            activity: Activity::None,
            activity_target: None,
            generation: 0,
            refetch_gate: RefreshGate::default(),
            refreshing: false,
            checks_open: true,
            activity_open: true,
            collapsed_groups: Vec::new(),
            collapsed_items: Vec::new(),
            menu_open: None,
            _subscriptions: subscriptions,
        };
        view.fetch(cx);
        view
    }

    fn schedule_fetch(&mut self, cx: &mut Context<PullRequestView>) {
        if !self.refetch_gate.request() {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(REFETCH_COALESCE).await;
            this.update(cx, |this, cx| {
                this.refetch_gate.fire();
                this.fetch(cx);
            })
            .ok();
        })
        .detach();
    }

    /// Fetches the status, then the timeline when the status names one. The data on screen
    /// stays until the new answer arrives.
    fn fetch(&mut self, cx: &mut Context<PullRequestView>) {
        self.generation += 1;
        if !self.client.read(cx).is_connected() {
            // `ClientEvent::Connected` loads it.
            return;
        }
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.pr_status(&cwd, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| this.apply_status(generation, result, cx))
                .ok();
        })
        .detach();
    }

    fn apply_status(
        &mut self,
        generation: u64,
        result: anyhow::Result<PrStatusReply>,
        cx: &mut Context<PullRequestView>,
    ) {
        if generation != self.generation {
            return;
        }
        let reply = match result {
            Ok(reply) => reply,
            Err(error) => {
                let message = format!("{error:#}");
                log::warn!("pr_status {}: {message}", self.cwd);
                // A failed refetch keeps the PR on screen.
                if !matches!(self.load, Load::Ready(_)) {
                    self.load = Load::Failed(message);
                }
                cx.notify();
                return;
            }
        };
        let target = timeline_target(&reply);
        self.load = Load::Ready(Box::new(reply));
        if target != self.activity_target {
            self.activity_target = target.clone();
            self.collapsed_items.clear();
            self.menu_open = None;
            self.activity = if target.is_some() {
                Activity::Loading
            } else {
                Activity::None
            };
        }
        if let Some(target) = target {
            self.fetch_timeline(generation, target, cx);
        }
        cx.notify();
    }

    fn fetch_timeline(
        &mut self,
        generation: u64,
        target: TimelineTarget,
        cx: &mut Context<PullRequestView>,
    ) {
        let cwd = self.cwd.clone();
        let task = self.client.update(cx, |client, cx| {
            client.pr_timeline(
                &cwd,
                target.number,
                &target.repo_owner,
                &target.repo_name,
                cx,
            )
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if generation != this.generation || this.activity_target.as_ref() != Some(&target) {
                    return;
                }
                match result {
                    Ok(items) => this.activity = Activity::Ready(visible_activity(items)),
                    Err(error) => {
                        log::warn!("pr_timeline {}: {error:#}", this.cwd);
                        if !matches!(this.activity, Activity::Ready(_)) {
                            this.activity = Activity::Failed;
                        }
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn refresh_supported(&self, cx: &App) -> bool {
        self.client
            .read(cx)
            .server_info()
            .and_then(|info| info.features.as_ref())
            .is_some_and(|features| features.checkout_refresh == Some(true))
    }

    /// Refresh: `checkout.refresh.request`, then a fetch.
    fn refresh(&mut self, cx: &mut Context<PullRequestView>) {
        if self.refreshing {
            return;
        }
        self.refreshing = true;
        cx.notify();
        let cwd = self.cwd.clone();
        let task = self.client.update(cx, |client, cx| {
            client.git_action(&cwd, GitAction::Refresh, cx)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.refreshing = false;
                if let Err(error) = result {
                    let message = format!("{error:#}");
                    cx.emit(PullRequestNoticeEvent {
                        ok: false,
                        message: if message.is_empty() {
                            "Failed to refresh git state.".to_string()
                        } else {
                            message
                        },
                    });
                }
                this.fetch(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn retry(&mut self, cx: &mut Context<PullRequestView>) {
        self.load = Load::Loading;
        self.fetch(cx);
        cx.notify();
    }

    fn open_url(&mut self, url: String, cx: &mut Context<PullRequestView>) {
        self.menu_open = None;
        if !url.is_empty() {
            cx.emit(PullRequestEvent::OpenUrl { url });
        }
        cx.notify();
    }

    fn toggle_group(&mut self, kind: CheckKind, cx: &mut Context<PullRequestView>) {
        if let Some(index) = self.collapsed_groups.iter().position(|&k| k == kind) {
            self.collapsed_groups.remove(index);
        } else {
            self.collapsed_groups.push(kind);
        }
        cx.notify();
    }

    fn toggle_item(&mut self, index: usize, cx: &mut Context<PullRequestView>) {
        if let Some(at) = self.collapsed_items.iter().position(|&i| i == index) {
            self.collapsed_items.remove(at);
        } else {
            self.collapsed_items.push(index);
        }
        cx.notify();
    }

    fn toggle_menu(&mut self, index: usize, cx: &mut Context<PullRequestView>) {
        self.menu_open = if self.menu_open == Some(index) {
            None
        } else {
            Some(index)
        };
        cx.notify();
    }

    fn copy_body(&mut self, body: String, cx: &mut Context<PullRequestView>) {
        self.menu_open = None;
        cx.write_to_clipboard(ClipboardItem::new_string(body));
        cx.notify();
    }

    /// A pressable element with a button role, a click handler, and the AX press handler.
    fn pressable(
        &self,
        id: impl Into<ElementId>,
        label: impl Into<SharedString>,
        cx: &mut Context<PullRequestView>,
        press: impl Fn(&mut PullRequestView, &mut Context<PullRequestView>) + 'static,
    ) -> Stateful<Div> {
        let press = Rc::new(press);
        let click = press.clone();
        let view = cx.weak_entity();
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| {
                cx.stop_propagation();
                click(this, cx);
            }))
            .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                view.update(cx, |this, cx| press(this, cx)).ok();
            })
    }

    fn text(&self, size: f32, color: Hsla, cx: &App) -> Div {
        let size = ui_font_size(size, cx);
        div()
            .text_size(size)
            .line_height(size * TEXT_LINE_FACTOR)
            .text_color(color)
    }

    fn spinner(&self, id: &'static str, size: f32, color: Hsla) -> AnyElement {
        icon(IconName::LoaderCircle)
            .with_size(px(size))
            .text_color(color)
            .with_animation(id, Animation::new(SPIN).repeat(), |icon, delta| {
                icon.transform(Transformation::rotate(percentage(delta)))
            })
            .into_any_element()
    }

    fn render_toolbar(&self, url: &str, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let open = url.to_string();
        let view_button = self
            .pressable("pr-view-open", "View pull request", cx, move |this, cx| {
                this.open_url(open.clone(), cx)
            })
            .h(px(VIEW_BUTTON_HEIGHT))
            .px(px(SPACE_1))
            .flex()
            .items_center()
            .gap(px(SPACE_1))
            .rounded(px(RADIUS_BASE))
            .hover(move |style| style.bg(colors.surface_sidebar_hover))
            .child(
                icon(IconName::ExternalLink)
                    .with_size(px(ICON_SM))
                    .text_color(colors.foreground),
            )
            .child(self.text(FONT_SM, colors.foreground, cx).child("View"));
        let refresh = self.refresh_supported(cx).then(|| {
            let label = if self.refreshing {
                "Refreshing"
            } else {
                "Refresh git and GitHub state"
            };
            let glyph = if self.refreshing {
                self.spinner("pr-view-refresh-spin", ICON_SM, colors.foreground_muted)
            } else {
                icon(IconName::RotateCw)
                    .with_size(px(ICON_SM))
                    .text_color(colors.foreground_muted)
                    .into_any_element()
            };
            self.pressable("pr-view-refresh", label, cx, |this, cx| this.refresh(cx))
                .size(px(TOOLBAR_BUTTON))
                .flex()
                .items_center()
                .justify_center()
                .rounded(px(RADIUS_MD))
                .hover(move |style| style.bg(colors.interaction_highlight))
                .child(glyph)
        });
        h_flex()
            .id("pr-view-toolbar")
            .role(Role::Toolbar)
            .aria_label("Pull request")
            .flex_shrink_0()
            .h(px(TOOLBAR_HEIGHT))
            .px(px(SPACE_3))
            .justify_between()
            .child(view_button)
            .children(refresh)
            .into_any_element()
    }

    fn render_header(&self, status: &PrStatus, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let state = pr_state(status);
        let (state_icon, state_color) = match state {
            PrState::Open => (IconName::GitPullRequest, colors.status_success),
            PrState::Draft => (IconName::GitPullRequestDraft, colors.foreground_muted),
            PrState::Merged => (IconName::GitMerge, colors.status_merged),
            PrState::Closed => (IconName::GitPullRequestClosed, colors.status_danger),
        };
        let number = status.number.map(|n| format!(" #{n}")).unwrap_or_default();
        let title = format!("{}{number}", status.title);
        let number_range = status.title.len()..title.len();
        let title_size = ui_font_size(FONT_BASE, cx);
        let repo = (!status.repo_owner.is_empty() && !status.repo_name.is_empty())
            .then(|| format!("{}/{}", status.repo_owner, status.repo_name));
        let url = status.url.clone();
        self.pressable("pr-view-header", title.clone(), cx, move |this, cx| {
            this.open_url(url.clone(), cx)
        })
        .group("pr-view-header")
        .flex()
        .flex_col()
        .gap(px(SPACE_2))
        .p(px(SPACE_4))
        .child(
            div()
                .text_size(title_size)
                .line_height(px(TITLE_LINE_HEIGHT))
                .text_color(colors.foreground)
                .child(StyledText::new(title).with_highlights([(
                    number_range,
                    HighlightStyle {
                        color: Some(colors.foreground_muted),
                        ..HighlightStyle::default()
                    },
                )])),
        )
        .child(
            h_flex()
                .min_h(px(16.))
                .gap(px(SPACE_1))
                .child(
                    icon(state_icon)
                        .with_size(px(ICON_SM))
                        .text_color(state_color),
                )
                .child(
                    self.text(FONT_SM, state_color, cx)
                        .flex_shrink_0()
                        .child(state_label(state)),
                )
                .when_some(repo, |this, repo| {
                    this.child(
                        self.text(FONT_SM, colors.foreground_muted, cx)
                            .ml(px(SPACE_1))
                            .min_w_0()
                            .flex_shrink(1.)
                            .truncate()
                            .child(repo),
                    )
                })
                .child(
                    div()
                        .ml(px(SPACE_1))
                        .flex_shrink_0()
                        .opacity(0.)
                        .group_hover("pr-view-header", |style| style.opacity(1.))
                        .child(
                            icon(IconName::ExternalLink)
                                .with_size(px(ICON_XS))
                                .text_color(colors.foreground_muted),
                        ),
                ),
        )
        .into_any_element()
    }

    fn render_checks(&self, status: &PrStatus, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let summary = summarize_checks(&status.checks);
        let label = if summary.detail.is_empty() {
            summary.headline.to_string()
        } else {
            format!("{}. {}", summary.headline, summary.detail)
        };
        let ring = checks_ring(&summary.groups, ICON_LG, &colors);
        let header = self
            .pressable("pr-view-checks-header", label, cx, |this, cx| {
                this.checks_open = !this.checks_open;
                cx.notify();
            })
            .flex()
            .items_center()
            .gap(px(SPACE_2))
            .p(px(SPACE_3))
            .hover(move |style| style.bg(colors.surface_sidebar_hover))
            .child(ring)
            .child(
                v_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(SPACE_0_5))
                    .child(
                        self.text(FONT_BASE, colors.foreground, cx)
                            .font_weight(FontWeight::MEDIUM)
                            .truncate()
                            .child(summary.headline),
                    )
                    .when(!summary.detail.is_empty(), |this| {
                        this.child(
                            self.text(FONT_SM, colors.foreground_muted, cx)
                                .truncate()
                                .child(summary.detail.clone()),
                        )
                    }),
            )
            .child(disclosure(self.checks_open, ICON_SM, &colors));
        let list = (self.checks_open && !summary.groups.is_empty()).then(|| {
            let groups: Vec<AnyElement> = summary
                .groups
                .iter()
                .map(|group| self.render_check_group(group, status, cx))
                .collect();
            div()
                .id("pr-view-checks-list")
                .role(Role::List)
                .aria_label("Checks")
                .max_h(px(CHECKS_MAX_HEIGHT))
                .overflow_y_scroll()
                .pb(px(SPACE_2))
                .children(groups)
        });
        v_flex().child(header).children(list).into_any_element()
    }

    fn render_check_group(
        &self,
        group: &ChecksGroup,
        status: &PrStatus,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let kind = group.kind;
        let collapsed = self.collapsed_groups.contains(&kind);
        let header = self
            .pressable(
                SharedString::from(format!("pr-view-check-group-{kind:?}")),
                group.label.clone(),
                cx,
                move |this, cx| this.toggle_group(kind, cx),
            )
            .flex()
            .items_center()
            .gap(px(SPACE_1))
            .px(px(SPACE_3))
            .pt(px(SPACE_2))
            .pb(px(SPACE_1))
            .child(
                self.text(FONT_SM, colors.foreground_muted, cx)
                    .font_weight(FontWeight::MEDIUM)
                    .child(group.label.clone()),
            )
            .child(disclosure(!collapsed, ICON_XS, &colors));
        let rows: Vec<AnyElement> = if collapsed {
            Vec::new()
        } else {
            group
                .checks
                .iter()
                .filter_map(|&index| {
                    let check = status.checks.get(index)?;
                    Some(self.render_check_row(index, check, kind, &status.url, cx))
                })
                .collect()
        };
        v_flex().child(header).children(rows).into_any_element()
    }

    fn render_check_row(
        &self,
        index: usize,
        check: &PrCheck,
        kind: CheckKind,
        pr_url: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let url = check
            .url
            .clone()
            .filter(|url| !url.is_empty())
            .unwrap_or_else(|| pr_url.to_string());
        self.pressable(
            ("pr-view-check", index),
            check.name.clone(),
            cx,
            move |this, cx| this.open_url(url.clone(), cx),
        )
        .h(px(CHECK_ROW_HEIGHT))
        .px(px(SPACE_3))
        .flex()
        .items_center()
        .gap(px(SPACE_2))
        .hover(move |style| style.bg(colors.surface_sidebar_hover))
        .child(check_icon(kind, ICON_SM, &colors))
        .child(
            self.text(FONT_BASE, colors.foreground, cx)
                .min_w_0()
                .flex_shrink(1.)
                .truncate()
                .child(check.name.clone()),
        )
        .when_some(check.workflow.clone(), |this, workflow| {
            this.child(
                self.text(FONT_SM, colors.foreground_muted, cx)
                    .min_w_0()
                    .flex_shrink(1.)
                    .truncate()
                    .child(workflow),
            )
        })
        .when_some(
            check_timing(check.duration.as_deref(), &check.status),
            |this, timing| {
                this.child(
                    self.text(FONT_SM, colors.foreground_muted, cx)
                        .ml_auto()
                        .flex_shrink_0()
                        .child(timing),
                )
            },
        )
        .into_any_element()
    }

    fn render_activity(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let items: &[PrTimelineItem] = match &self.activity {
            Activity::Ready(items) => items,
            _ => &[],
        };
        let pills = count_pills(items);
        let pill = |count: usize, glyph: IconName, size: f32, color: Hsla, cx: &App| {
            (count > 0).then(|| {
                h_flex()
                    .gap(px(3.))
                    .child(icon(glyph).with_size(px(size)).text_color(color))
                    .child(self.text(FONT_SM, color, cx).child(count.to_string()))
            })
        };
        let header = self
            .pressable("pr-view-activity-header", "Activity", cx, |this, cx| {
                this.activity_open = !this.activity_open;
                cx.notify();
            })
            .flex()
            .items_center()
            .gap(px(SPACE_2))
            .px(px(SPACE_3))
            .py(px(SPACE_2))
            .child(disclosure(self.activity_open, ICON_SM, &colors))
            .child(
                self.text(FONT_SM, colors.foreground_muted, cx)
                    .font_weight(FontWeight::MEDIUM)
                    .child("Activity"),
            )
            .child(
                h_flex()
                    .ml_auto()
                    .gap(px(SPACE_2))
                    .children(pill(
                        pills.approvals,
                        IconName::CircleCheck,
                        ICON_XS,
                        colors.status_success,
                        cx,
                    ))
                    .children(pill(
                        pills.changes_requested,
                        IconName::CircleX,
                        ICON_XS,
                        colors.status_danger,
                        cx,
                    ))
                    .children(pill(
                        pills.comments,
                        IconName::MessageSquare,
                        11.,
                        colors.foreground_muted,
                        cx,
                    )),
            );
        let body = self.activity_open.then(|| {
            let empty = |text: String| {
                self.text(FONT_SM, colors.foreground_muted, cx)
                    .px(px(SPACE_3))
                    .py(px(SPACE_2))
                    .child(text)
                    .into_any_element()
            };
            let children: Vec<AnyElement> = match &self.activity {
                Activity::Loading => vec![activity_skeleton(&colors)],
                Activity::Failed => {
                    vec![empty("Unable to load pull request activity".into())]
                }
                Activity::None => vec![empty("No activity yet".into())],
                Activity::Ready(items) if items.is_empty() => {
                    vec![empty("No activity yet".into())]
                }
                Activity::Ready(items) => {
                    let now = now_ms();
                    items
                        .iter()
                        .enumerate()
                        .map(|(index, item)| self.render_item(index, item, now, cx))
                        .collect()
                }
            };
            v_flex().pb(px(SPACE_3)).children(children)
        });
        v_flex()
            .id("pr-view-activity")
            .role(Role::Group)
            .aria_label("Activity")
            .child(header)
            .children(body)
            .into_any_element()
    }

    fn render_item(
        &self,
        index: usize,
        item: &PrTimelineItem,
        now: i64,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let (author, body, created_at_ms, url, location) = match item {
            PrTimelineItem::Review {
                author,
                body,
                created_at_ms,
                url,
                ..
            } => (author, body, *created_at_ms, url, None),
            PrTimelineItem::Comment {
                author,
                body,
                created_at_ms,
                url,
                path,
                line,
            } => (
                author,
                body,
                *created_at_ms,
                url,
                comment_location(path.as_deref(), *line),
            ),
        };
        let has_body = !body.trim().is_empty();
        let collapsed = self.collapsed_items.contains(&index);
        let group = SharedString::from(format!("pr-view-item-{index}"));
        let open = url.clone().unwrap_or_default();
        let verb = activity_verb(item);
        let verb_color = match verb {
            "approved" => Some((IconName::CircleCheck, colors.status_success)),
            "requested changes" => Some((IconName::CircleX, colors.status_danger)),
            _ => None,
        };
        let initial = author
            .chars()
            .next()
            .map(|c| c.to_uppercase().to_string())
            .unwrap_or_default();
        let header = self
            .pressable(
                ("pr-view-item-header", index),
                format!("{author} {verb}"),
                cx,
                move |this, cx| {
                    if has_body {
                        this.toggle_item(index, cx);
                    } else {
                        this.open_url(open.clone(), cx);
                    }
                },
            )
            .flex()
            .items_center()
            .gap(px(SPACE_2))
            .px(px(SPACE_3))
            .py(px(SPACE_2))
            .min_h(px(ENTRY_HEADER_MIN_HEIGHT))
            .hover(move |style| style.bg(colors.surface_sidebar_hover))
            .child(
                div()
                    .size(px(AVATAR_SIZE))
                    .flex_shrink_0()
                    .rounded_full()
                    .bg(rgb(avatar_color(author)))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(px(10.))
                    .line_height(px(12.))
                    .text_color(white())
                    .child(initial),
            )
            .child(
                self.text(FONT_BASE, colors.foreground, cx)
                    .min_w_0()
                    .flex_shrink(1.)
                    .truncate()
                    .child(author.clone()),
            )
            .child(match verb_color {
                Some((glyph, color)) => h_flex()
                    .flex_shrink_0()
                    .gap(px(SPACE_1))
                    .child(icon(glyph).with_size(px(ICON_XS)).text_color(color))
                    .child(self.text(FONT_SM, color, cx).child(verb))
                    .into_any_element(),
                None => self
                    .text(FONT_SM, colors.foreground_muted, cx)
                    .flex_shrink_0()
                    .child(verb)
                    .into_any_element(),
            })
            .child(
                h_flex()
                    .ml_auto()
                    .flex_shrink_0()
                    .gap(px(SPACE_2))
                    .child(
                        self.text(FONT_SM, colors.foreground_muted, cx)
                            .child(format_age(created_at_ms, now)),
                    )
                    .child(self.render_kebab(index, &group, body, url.as_deref(), cx)),
            );
        let card = div()
            .id(("pr-view-item", index))
            .group(group)
            .role(Role::ListItem)
            .aria_label(format!("{author} {verb}"))
            .mx(px(SPACE_3));
        if !has_body {
            return card.mb(px(SPACE_2)).child(header).into_any_element();
        }
        let text_size = ui_font_size(FONT_BASE, cx);
        let view = cx.weak_entity();
        card.mb(px(SPACE_3))
            .border_1()
            .border_color(colors.border)
            .rounded(px(RADIUS_LG))
            .bg(colors.surface_sidebar)
            .overflow_hidden()
            .child(header)
            .when(!collapsed, |this| {
                this.child(
                    v_flex()
                        .px(px(SPACE_3))
                        .pt(px(SPACE_2))
                        .pb(px(SPACE_3))
                        .gap(px(SPACE_1))
                        .when_some(location, |this, location| {
                            this.child(
                                self.text(FONT_SM, colors.foreground_muted, cx)
                                    .font_family(mono_font_family(cx))
                                    .truncate()
                                    .child(location),
                            )
                        })
                        .child(
                            div()
                                .text_size(text_size)
                                .line_height(text_size * TEXT_LINE_FACTOR)
                                .text_color(colors.foreground)
                                .child(
                                    TextView::markdown(
                                        ElementId::Name(
                                            format!("pr-view-item-body-{index}").into(),
                                        ),
                                        body.clone(),
                                    )
                                    .text_size(text_size)
                                    .on_link_click(
                                        move |href, _, _, cx| {
                                            let url = href.to_string();
                                            view.update(cx, |this, cx| this.open_url(url, cx)).ok();
                                        },
                                    ),
                                ),
                        ),
                )
            })
            .into_any_element()
    }

    /// The item menu: Copy (when the item has a body) and Open on GitHub.
    fn render_kebab(
        &self,
        index: usize,
        group: &SharedString,
        body: &str,
        url: Option<&str>,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let open = self.menu_open == Some(index);
        let trigger = self
            .pressable(
                ("pr-view-item-menu", index),
                "Comment actions",
                cx,
                move |this, cx| this.toggle_menu(index, cx),
            )
            .p(px(2.))
            .rounded(px(RADIUS_BASE))
            .hover(move |style| style.bg(colors.surface2))
            .child(
                icon(IconName::Ellipsis)
                    .with_size(px(ICON_SM))
                    .text_color(colors.foreground_muted),
            );
        let menu = open.then(|| {
            let mut items = Vec::new();
            if !body.trim().is_empty() {
                let body = body.to_string();
                items.push(self.menu_item(
                    ("pr-view-item-copy", index),
                    "Copy",
                    IconName::Copy,
                    cx,
                    move |this, cx| this.copy_body(body.clone(), cx),
                ));
            }
            if let Some(url) = url.filter(|url| !url.is_empty()) {
                let url = url.to_string();
                items.push(self.menu_item(
                    ("pr-view-item-open", index),
                    "Open on GitHub",
                    IconName::ExternalLink,
                    cx,
                    move |this, cx| this.open_url(url.clone(), cx),
                ));
            }
            div()
                .absolute()
                .top(px(popup_offset(KEBAB_TRIGGER_HEIGHT)))
                .right_0()
                .child(
                    deferred(
                        anchored()
                            .anchor(Anchor::TopRight)
                            .snap_to_window_with_margin(px(8.))
                            .child(
                                v_flex()
                                    .id(("pr-view-item-menu-list", index))
                                    .role(Role::Menu)
                                    .aria_label("Comment actions")
                                    .w(px(MENU_WIDTH))
                                    .py(px(SPACE_1))
                                    .bg(colors.popover)
                                    .border_1()
                                    .border_color(colors.border_accent)
                                    .rounded(px(RADIUS_LG))
                                    .shadow(vec![shadow(ShadowSize::Md, cx)])
                                    .occlude()
                                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                        this.menu_open = None;
                                        cx.notify();
                                    }))
                                    .children(items),
                            ),
                    )
                    .with_priority(gpui_kit::base::POPUP_PRIORITY),
                )
        });
        div()
            .relative()
            .w(px(KEBAB_SLOT))
            .flex()
            .items_center()
            .justify_center()
            .when(!open, |this| {
                this.opacity(0.)
                    .group_hover(group.clone(), |style| style.opacity(1.))
            })
            .child(trigger)
            .children(menu)
            .into_any_element()
    }

    fn menu_item(
        &self,
        id: impl Into<ElementId>,
        label: &'static str,
        glyph: IconName,
        cx: &mut Context<Self>,
        press: impl Fn(&mut PullRequestView, &mut Context<PullRequestView>) + 'static,
    ) -> AnyElement {
        let colors = tokens(cx);
        self.pressable(id, label, cx, press)
            .role(Role::MenuItem)
            .min_h(px(MENU_ITEM_HEIGHT))
            .mx(px(SPACE_1))
            .px(px(SPACE_2))
            .flex()
            .items_center()
            .gap(px(SPACE_2))
            .rounded(px(RADIUS_MD))
            .hover(move |style| style.bg(colors.surface2))
            .child(
                icon(glyph)
                    .with_size(px(ICON_SM))
                    .text_color(colors.foreground_muted),
            )
            .child(self.text(FONT_BASE, colors.foreground, cx).child(label))
            .into_any_element()
    }

    fn render_error(&self, message: &str, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let message = if message.is_empty() {
            "Unable to load pull request status"
        } else {
            message
        };
        v_flex()
            .id("pr-view-error")
            .role(Role::Alert)
            .aria_label(message.to_string())
            .flex_1()
            .items_center()
            .justify_center()
            .gap(px(SPACE_3))
            .px(px(SPACE_4))
            .child(
                self.text(FONT_SM, colors.foreground_muted, cx)
                    .text_center()
                    .child(message.to_string()),
            )
            .child(
                self.pressable("pr-view-retry", "Retry", cx, |this, cx| this.retry(cx))
                    .h(px(VIEW_BUTTON_HEIGHT))
                    .px(px(SPACE_2))
                    .flex()
                    .items_center()
                    .gap(px(SPACE_1))
                    .rounded(px(RADIUS_BASE))
                    .hover(move |style| style.bg(colors.surface_sidebar_hover))
                    .child(
                        icon(IconName::RotateCw)
                            .with_size(px(ICON_SM))
                            .text_color(colors.foreground),
                    )
                    .child(self.text(FONT_SM, colors.foreground, cx).child("Retry")),
            )
            .into_any_element()
    }

    fn render_empty(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        v_flex()
            .id("pr-view-empty")
            .role(Role::Status)
            .aria_label("No pull request yet")
            .flex_1()
            .items_center()
            .justify_center()
            .gap(px(SPACE_2))
            .p(px(SPACE_6))
            .child(
                self.text(FONT_BASE, colors.foreground, cx)
                    .child("No pull request yet"),
            )
            .child(
                self.text(FONT_BASE, colors.foreground_muted, cx)
                    .text_center()
                    .child("Create a pull request for this checkout to see its details here."),
            )
            .into_any_element()
    }
}

fn disclosure(open: bool, size: f32, colors: &Tokens) -> AnyElement {
    icon(if open {
        IconName::ChevronDown
    } else {
        IconName::ChevronRight
    })
    .with_size(px(size))
    .text_color(colors.foreground_muted)
    .into_any_element()
}

/// `CHECK_PRESENTATION_VIEW_POLICY`. The manual kinds draw the Settings gear, because the icon
/// set has no `ManualStatusIcon`.
fn check_icon(kind: CheckKind, size: f32, colors: &Tokens) -> AnyElement {
    let (glyph, color) = match kind {
        CheckKind::Success => (IconName::CircleCheck, colors.status_success),
        CheckKind::Failure => (IconName::CircleX, colors.status_danger),
        CheckKind::Warning => (IconName::CircleX, colors.status_warning),
        CheckKind::ActionRequired => (IconName::Settings, colors.status_warning),
        CheckKind::Manual => (IconName::Settings, colors.foreground_muted),
        CheckKind::Pending => (IconName::CircleDot, colors.status_warning),
        CheckKind::Ignored => (IconName::CircleSlash, colors.foreground_muted),
    };
    div()
        .flex_shrink_0()
        .child(icon(glyph).with_size(px(size)).text_color(color))
        .into_any_element()
}

/// `ChecksRing`: the track, then one arc per check group in `CHECK_ORDER`, clockwise from
/// twelve. With no checks only the track shows.
fn checks_ring(groups: &[ChecksGroup], size: f32, colors: &Tokens) -> AnyElement {
    let track = colors.surface3;
    let arcs: Vec<(Hsla, f32, f32)> = ring_segments(groups)
        .into_iter()
        .map(|segment| {
            let turn = 2. * std::f32::consts::PI / RING_CIRCUMFERENCE;
            (
                ring_color(segment.kind, colors),
                -std::f32::consts::FRAC_PI_2 + segment.offset * turn,
                segment.dash * turn,
            )
        })
        .collect();
    let scale = size / RING_VIEW_BOX;
    let ring = canvas(
        |_, _, _| (),
        move |bounds, (), window, _| {
            let center = bounds.center();
            window.paint_quad(quad(
                Bounds::centered_at(center, gpui_kit::size(px(size), px(size))),
                Corners::all(px(size / 2.)),
                transparent_black(),
                Edges::all(px(RING_STROKE * scale)),
                track,
                BorderStyle::Solid,
            ));
            let radius = RING_RADIUS * scale;
            for &(color, start, sweep) in &arcs {
                let steps = (RING_ARC_STEPS * sweep / (2. * std::f32::consts::PI))
                    .ceil()
                    .max(1.) as usize;
                let mut path = PathBuilder::stroke(px(RING_STROKE * scale));
                for step in 0..=steps {
                    let angle = start + sweep * step as f32 / steps as f32;
                    let at = point(
                        center.x + px(radius * angle.cos()),
                        center.y + px(radius * angle.sin()),
                    );
                    if step == 0 {
                        path.move_to(at);
                    } else {
                        path.line_to(at);
                    }
                }
                if let Ok(path) = path.build() {
                    window.paint_path(path, color);
                }
            }
        },
    )
    .size(px(size));
    div().flex_shrink_0().child(ring).into_any_element()
}

fn skeleton_bar(width: impl Into<Length> + Clone, height: f32, colors: &Tokens) -> Div {
    div()
        .w(width)
        .h(px(height))
        .rounded(px(RADIUS_SM))
        .bg(colors.surface2)
}

/// `PullRequestPaneSkeleton`, without the pulse.
fn pane_skeleton(colors: &Tokens) -> AnyElement {
    let check_row = |_| {
        h_flex()
            .min_h(px(CHECK_ROW_HEIGHT))
            .px(px(SPACE_3))
            .gap(px(SPACE_2))
            .child(div().size(px(ICON_SM)).rounded_full().bg(colors.surface2))
            .child(skeleton_bar(relative(0.6), 12., colors))
    };
    v_flex()
        .id("pr-view-skeleton")
        .role(Role::Status)
        .aria_label("Loading pull request")
        .child(
            v_flex()
                .gap(px(SPACE_2))
                .p(px(SPACE_4))
                .child(skeleton_bar(relative(0.75), 16., colors))
                .child(skeleton_bar(relative(0.4), 12., colors)),
        )
        .child(
            h_flex()
                .h(px(TOOLBAR_HEIGHT))
                .px(px(SPACE_3))
                .gap(px(SPACE_2))
                .child(skeleton_bar(px(96.), 24., colors).rounded(px(RADIUS_LG)))
                .child(skeleton_bar(px(96.), 24., colors).rounded(px(RADIUS_LG))),
        )
        .child(
            v_flex()
                .py(px(SPACE_2))
                .gap(px(SPACE_1))
                .children((0..3).map(check_row)),
        )
        .child(v_flex().py(px(SPACE_2)).child(activity_skeleton(colors)))
        .into_any_element()
}

/// `PrActivitySkeleton`: two placeholder cards.
fn activity_skeleton(colors: &Tokens) -> AnyElement {
    let card = |_| {
        v_flex()
            .mx(px(SPACE_3))
            .mb(px(SPACE_3))
            .p(px(SPACE_3))
            .gap(px(SPACE_2))
            .border_1()
            .border_color(colors.border)
            .rounded(px(RADIUS_LG))
            .child(
                h_flex()
                    .gap(px(SPACE_2))
                    .child(
                        div()
                            .size(px(AVATAR_SIZE))
                            .rounded_full()
                            .bg(colors.surface2),
                    )
                    .child(skeleton_bar(relative(0.3), 12., colors)),
            )
            .child(skeleton_bar(relative(0.9), 12., colors))
            .child(skeleton_bar(relative(0.6), 12., colors))
    };
    v_flex().children((0..2).map(card)).into_any_element()
}

impl Render for PullRequestView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let root = div()
            .id("pull-request-view")
            .role(Role::Pane)
            .aria_label("Pull request")
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.surface_sidebar)
            .text_color(colors.foreground);
        let status = match &self.load {
            Load::Loading => return root.child(pane_skeleton(&colors)),
            Load::Failed(message) => {
                let message = message.clone();
                return root.child(self.render_error(&message, cx));
            }
            Load::Ready(reply) => reply.status.clone(),
        };
        let Some(status) = status else {
            return root.child(self.render_empty(cx));
        };
        root.child(
            div()
                .id("pr-view-scroll")
                .flex_1()
                .min_h_0()
                .overflow_y_scroll()
                .child(self.render_toolbar(&status.url, cx))
                .child(self.render_header(&status, cx))
                .child(self.render_checks(&status, cx))
                .child(div().h(px(1.)).bg(colors.border))
                .child(self.render_activity(cx)),
        )
    }
}

#[cfg(test)]
mod tests {
    // Not `use super::*`: the glob of `gpui_kit::*` brings GPUI's `test` macro.
    use super::{
        CheckKind, CountPills, PrState, RING_CIRCUMFERENCE, RING_MIN_SEGMENT, RING_SEGMENT_GAP,
        RefreshGate, TimelineTarget, check_timing, comment_location, count_pills, format_age,
        pr_state, ring_color, ring_segments, state_label, summarize_checks, timeline_target,
        visible_activity,
    };
    use bachuc_client::{PrCheck, PrMergeCapability, PrStatus, PrStatusReply, PrTimelineItem};
    use bachuc_theme::Tokens;

    fn status() -> PrStatus {
        PrStatus {
            number: Some(12),
            url: "https://github.com/acme/app/pull/12".into(),
            title: "Add the PR view".into(),
            state: "open".into(),
            is_merged: false,
            is_draft: false,
            mergeable: None,
            checks: Vec::new(),
            review_decision: None,
            repo_owner: "acme".into(),
            repo_name: "app".into(),
            merge: PrMergeCapability::default(),
        }
    }

    fn reply(status: PrStatus) -> PrStatusReply {
        PrStatusReply {
            status: Some(status),
            github_features_enabled: true,
        }
    }

    fn review(state: &str, body: &str) -> PrTimelineItem {
        PrTimelineItem::Review {
            author: "ana".into(),
            body: body.into(),
            created_at_ms: 0,
            url: None,
            state: state.into(),
        }
    }

    fn comment(body: &str) -> PrTimelineItem {
        PrTimelineItem::Comment {
            author: "bo".into(),
            body: body.into(),
            created_at_ms: 0,
            url: None,
            path: None,
            line: None,
        }
    }

    fn check(status: &str) -> PrCheck {
        PrCheck {
            name: format!("ci-{status}"),
            status: status.into(),
            url: None,
            workflow: None,
            duration: None,
            traits: Vec::new(),
        }
    }

    #[test]
    fn pull_request_state_label() {
        let open = status();
        assert_eq!(state_label(pr_state(&open)), "Open");
        let draft = PrStatus {
            is_draft: true,
            ..status()
        };
        assert_eq!(state_label(pr_state(&draft)), "Draft");
        let merged = PrStatus {
            is_merged: true,
            state: "closed".into(),
            ..status()
        };
        assert_eq!(state_label(pr_state(&merged)), "Merged");
        let merged_by_state = PrStatus {
            state: "merged".into(),
            is_draft: true,
            ..status()
        };
        assert_eq!(pr_state(&merged_by_state), PrState::Merged);
        let closed = PrStatus {
            state: "closed".into(),
            is_draft: true,
            ..status()
        };
        assert_eq!(state_label(pr_state(&closed)), "Closed");
    }

    #[test]
    fn pull_request_count_pills() {
        let items = vec![
            review("approved", ""),
            review("approved", "LGTM"),
            review("changes_requested", "Fix it"),
            review("commented", "Note"),
            comment("A comment"),
        ];
        assert_eq!(
            count_pills(&items),
            CountPills {
                approvals: 2,
                changes_requested: 1,
                comments: 2,
            }
        );
        assert_eq!(count_pills(&[]), CountPills::default());
    }

    #[test]
    fn pull_request_activity_drops_empty_comments() {
        let items = visible_activity(vec![
            comment("  "),
            review("commented", ""),
            review("approved", ""),
            comment("kept"),
        ]);
        assert_eq!(items, vec![review("approved", ""), comment("kept")]);
    }

    #[test]
    fn pull_request_timeline_needs_number_and_repo() {
        assert_eq!(
            timeline_target(&reply(status())),
            Some(TimelineTarget {
                number: 12,
                repo_owner: "acme".into(),
                repo_name: "app".into(),
            })
        );
        let no_number = PrStatus {
            number: None,
            ..status()
        };
        assert_eq!(timeline_target(&reply(no_number)), None);
        let no_owner = PrStatus {
            repo_owner: String::new(),
            ..status()
        };
        assert_eq!(timeline_target(&reply(no_owner)), None);
        let no_name = PrStatus {
            repo_name: String::new(),
            ..status()
        };
        assert_eq!(timeline_target(&reply(no_name)), None);
        let off = PrStatusReply {
            github_features_enabled: false,
            ..reply(status())
        };
        assert_eq!(timeline_target(&off), None);
        let none = PrStatusReply {
            status: None,
            github_features_enabled: true,
        };
        assert_eq!(timeline_target(&none), None);
    }

    #[test]
    fn pull_request_format_age() {
        let now = 400 * 24 * 3_600_000;
        assert_eq!(format_age(now - 30_000, now), "just now");
        assert_eq!(format_age(now + 5_000, now), "just now");
        assert_eq!(format_age(now - 5 * 60_000, now), "5m ago");
        assert_eq!(format_age(now - 3 * 3_600_000, now), "3h ago");
        assert_eq!(format_age(now - 2 * 86_400_000, now), "2d ago");
        assert_eq!(format_age(now - 65 * 86_400_000, now), "2mo ago");
        assert_eq!(format_age(now - 400 * 86_400_000, now), "1y ago");
    }

    #[test]
    fn pull_request_checks_summary() {
        let none = summarize_checks(&[]);
        assert_eq!(none.outcome, None);
        assert_eq!(none.headline, "No checks");
        assert_eq!(none.detail, "");
        assert!(none.groups.is_empty());

        let checks = vec![
            check("success"),
            check("failure"),
            check("queued"),
            check("failure"),
            check("skipped"),
        ];
        let summary = summarize_checks(&checks);
        assert_eq!(summary.outcome, Some(CheckKind::Failure));
        assert_eq!(summary.headline, "Some checks were not successful");
        assert_eq!(
            summary.detail,
            "2 failing, 1 in progress, 1 successful, 1 skipped checks"
        );
        let kinds: Vec<CheckKind> = summary.groups.iter().map(|group| group.kind).collect();
        assert_eq!(
            kinds,
            vec![
                CheckKind::Failure,
                CheckKind::Pending,
                CheckKind::Success,
                CheckKind::Ignored
            ]
        );
        assert_eq!(summary.groups[0].label, "2 failing checks");
        assert_eq!(summary.groups[0].checks, vec![1, 3]);
        assert_eq!(summary.groups[1].label, "1 in progress check");

        let passed = summarize_checks(&[check("success"), check("skipped")]);
        assert_eq!(passed.headline, "All checks have passed");
        let running = summarize_checks(&[check("success"), check("pending")]);
        assert_eq!(running.headline, "Some checks haven't completed yet");
    }

    fn with_traits(status: &str, traits: &[&str]) -> PrCheck {
        PrCheck {
            traits: traits.iter().map(|t| t.to_string()).collect(),
            ..check(status)
        }
    }

    #[test]
    fn pull_request_checks_traits_win_over_the_status() {
        let checks = vec![
            with_traits("success", &["manual"]),
            with_traits("failure", &["warning"]),
            with_traits("pending", &["action_required"]),
            with_traits("success", &["unknown"]),
        ];
        let summary = summarize_checks(&checks);
        let kinds: Vec<CheckKind> = summary.groups.iter().map(|group| group.kind).collect();
        assert_eq!(
            kinds,
            vec![
                CheckKind::ActionRequired,
                CheckKind::Warning,
                CheckKind::Manual,
                CheckKind::Success
            ]
        );
        assert_eq!(summary.outcome, Some(CheckKind::ActionRequired));
        assert_eq!(summary.headline, "Some checks need your attention");
        assert_eq!(
            summary.detail,
            "1 needs action, 1 warning, 1 manual, 1 successful checks"
        );
        let warned = summarize_checks(&[with_traits("success", &["warning"]), check("pending")]);
        assert_eq!(warned.headline, "Some checks were not successful");
        // A manual check that is still pending keeps the run in progress, as `selectOutcome`.
        let manual = summarize_checks(&[with_traits("pending", &["manual"])]);
        assert_eq!(manual.headline, "Some checks haven't completed yet");
    }

    #[test]
    fn pull_request_ring_segments_follow_the_groups() {
        let checks = vec![
            check("success"),
            check("success"),
            check("failure"),
            check("success"),
            check("pending"),
        ];
        let segments = ring_segments(&summarize_checks(&checks).groups);
        let kinds: Vec<CheckKind> = segments.iter().map(|segment| segment.kind).collect();
        assert_eq!(
            kinds,
            vec![CheckKind::Failure, CheckKind::Pending, CheckKind::Success]
        );
        let share = RING_CIRCUMFERENCE / 5.;
        let close = |a: f32, b: f32| (a - b).abs() < 1e-3;
        assert!(close(segments[0].offset, 0.));
        assert!(close(segments[0].dash, share - RING_SEGMENT_GAP));
        assert!(close(segments[1].offset, share));
        assert!(close(segments[2].offset, 2. * share));
        assert!(close(segments[2].dash, 3. * share - RING_SEGMENT_GAP));

        let colors = Tokens::light();
        let painted: Vec<_> = segments
            .iter()
            .map(|segment| ring_color(segment.kind, &colors))
            .collect();
        assert_eq!(
            painted,
            vec![
                colors.status_danger,
                colors.status_warning,
                colors.status_success
            ]
        );
        assert_eq!(
            ring_color(CheckKind::Ignored, &colors),
            colors.foreground_extra_muted
        );
        assert_eq!(
            ring_color(CheckKind::Manual, &colors),
            colors.foreground_muted
        );

        // One group owns the whole circle with no gap; a tiny share keeps the minimum arc.
        let single = ring_segments(&summarize_checks(&[check("success")]).groups);
        assert_eq!(single.len(), 1);
        assert!(close(single[0].dash, RING_CIRCUMFERENCE));
        let mut many: Vec<PrCheck> = (0..40).map(|_| check("success")).collect();
        many.push(check("failure"));
        let tiny = ring_segments(&summarize_checks(&many).groups);
        assert!(close(tiny[0].dash, RING_MIN_SEGMENT));
        assert!(ring_segments(&[]).is_empty());
    }

    #[test]
    fn pull_request_check_duration() {
        assert_eq!(
            check_timing(Some("2m 5s"), "success").as_deref(),
            Some("2m 5s")
        );
        assert_eq!(check_timing(Some("40s"), "failure").as_deref(), Some("40s"));
        assert_eq!(
            check_timing(Some("1m"), "pending").as_deref(),
            Some("running 1m")
        );
        assert_eq!(
            check_timing(Some("1m"), "queued").as_deref(),
            Some("running 1m")
        );
        assert_eq!(check_timing(None, "success"), None);
        assert_eq!(check_timing(Some(""), "pending"), None);
    }

    #[test]
    fn pull_request_comment_location() {
        assert_eq!(
            comment_location(Some("src/a.rs"), Some(4)).as_deref(),
            Some("src/a.rs:4")
        );
        assert_eq!(
            comment_location(Some("src/a.rs"), None).as_deref(),
            Some("src/a.rs")
        );
        assert_eq!(comment_location(Some(""), Some(4)), None);
        assert_eq!(comment_location(None, Some(4)), None);
    }

    /// How many refetches the view runs for status events at `events` (ms), with the 500 ms
    /// timer the view starts on the first event of a burst.
    fn refetches(events: &[u64]) -> usize {
        let mut gate = RefreshGate::default();
        let mut timer: Option<u64> = None;
        let mut fired = 0;
        for &at in events {
            if timer.is_some_and(|due| due <= at) {
                gate.fire();
                fired += 1;
                timer = None;
            }
            if gate.request() {
                timer = Some(at + 500);
            }
        }
        fired + usize::from(timer.is_some())
    }

    #[test]
    fn pull_request_coalesces_status_events() {
        let burst: Vec<u64> = (0..10).map(|i| i * 10).collect();
        assert_eq!(burst.len(), 10);
        assert!(burst.last().is_some_and(|&last| last < 100));
        assert_eq!(refetches(&burst), 1);
        assert_eq!(refetches(&[0, 100, 600, 700]), 2);
        assert_eq!(refetches(&[]), 0);
    }
}
