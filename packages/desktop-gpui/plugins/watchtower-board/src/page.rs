//! The `watchtower` surface (`client/page.tsx` and `client/overview.tsx`): the workspace choice
//! and its placeholders, then the overview. From 720 px the rail sits on the left with its own
//! list and the overview fills the main list; below that one list holds the whole page.

use crate::cells::GAP;
use crate::decision_modal::{DecisionModal, decision_overlay};
use crate::model::RAIL_WIDTH;
use crate::model::{
    AgentTarget, BodyItem, BuilderMatch, CellKind, DerivedCache, DerivedKey, Layout, Loaded,
    Overview, PageWorkspace, RejectPress, RejectStep, accept_decision_prompt, actions_disabled,
    answer_prompt, body_items, changed_range, choose_workspace, default_answer_prompt,
    derive_overview, layout_for, manual_checks_prompt, now_ms, parse_markdown, project_name,
    reject_decision_prompt, reject_step, send_disabled, with_live_status,
};
use crate::needs_you::{NeedsAction, NeedsYouState};
use crate::overview::render_body;
use crate::rail::{Rail, RailHost, RailItem, item_frame, starts_card};
use crate::store::WatchtowerStore;
use crate::task_list::TaskListRow;
use bachuc_client::{Client, ClientEvent, DirectoryEvent};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{MessageLevel, PluginContext};
use bachuc_theme::metrics::{FONT_BASE, FONT_SM, SPACE_4, WEIGHT_SEMIBOLD};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::Sizable as _;
use gpui_kit::component::input::{InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::{BTreeMap, BTreeSet};
use std::time::Duration;

/// The header: title `fontSize: 18`, subtitle `marginTop: 2`, `marginBottom: 14`, and the
/// 30 px Refresh button with radius 7 and a 15 px icon.
const TITLE_SIZE: f32 = 18.;
const SUBTITLE_GAP: f32 = 2.;
const HEADER_GAP: f32 = 14.;
const REFRESH_SIZE: f32 = 30.;
const REFRESH_RADIUS: f32 = 7.;
const REFRESH_ICON: f32 = 15.;
/// The main column's `paddingTop: 18` beside the rail; the rail's `padding: 16`.
const MAIN_PADDING_TOP: f32 = 18.;
const RAIL_PADDING: f32 = 16.;
const OVERDRAW: f32 = 400.;
/// The "active 5m ago" lines and the derived key move on with the minute.
const MINUTE: Duration = Duration::from_secs(60);

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PageItem {
    Header,
    Rail(RailItem),
    Body(BodyItem),
}

/// What runs after a send that the agent took.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum AfterSend {
    Nothing,
    CloseAnswer,
    ClearReject,
}

pub(crate) struct WatchtowerPage {
    pub(crate) store: Entity<WatchtowerStore>,
    pub(crate) ctx: PluginContext,
    /// Held strong: a subscription holds its emitter weakly.
    client: Option<Entity<Client>>,
    /// `params.workspaceId`.
    param_workspace: Option<String>,
    shown: PageWorkspace,
    project: Option<String>,
    rail: Rail,
    pub(crate) overview: Loaded<Overview>,
    pub(crate) builders: BuilderMatch,
    pub(crate) target: AgentTarget,
    pub(crate) derived: DerivedCache,
    fetching: bool,
    width: f32,
    pub(crate) layout: Layout,
    rail_items: Vec<RailItem>,
    main_items: Vec<PageItem>,
    rail_list: ListState,
    main_list: ListState,
    /// The cells whose Show more is open.
    pub(crate) expanded: BTreeSet<CellKind>,
    pub(crate) needs: NeedsYouState,
    decision: Option<DecisionModal>,
    decision_generation: u64,
    _subscriptions: Vec<Subscription>,
    _clock: Task<()>,
}

/// Splices the changed items, then measures every row again so a changed height shows.
fn apply_items<T: PartialEq>(list: &ListState, old: &mut Vec<T>, new: Vec<T>) {
    if let Some((range, count)) = changed_range(old, &new) {
        list.splice(range, count);
    }
    list.remeasure();
    *old = new;
}

impl WatchtowerPage {
    pub(crate) fn new(
        store: Entity<WatchtowerStore>,
        ctx: PluginContext,
        params: &BTreeMap<String, String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let client = ctx.client(cx);
        let mut subscriptions = vec![cx.observe(&store, |page, _, cx| page.sync(cx))];
        if let Some(client) = &client {
            subscriptions.push(cx.subscribe(client, |page, _, event: &ClientEvent, cx| {
                if matches!(event, ClientEvent::Connected | ClientEvent::AgentsChanged) {
                    page.sync(cx);
                }
            }));
            subscriptions.push(cx.subscribe(client, |page, _, event: &DirectoryEvent, cx| {
                if matches!(event, DirectoryEvent::ProjectsChanged) {
                    page.sync(cx);
                }
            }));
        }
        // The store keeps the opener's task; the task list selects it once the board loads.
        let initial = crate::model::initial_task(params);
        store.update(cx, |store, _| store.set_initial_task(initial));
        cx.defer_in(window, |page, _, cx| page.sync(cx));
        let clock = cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(MINUTE).await;
                if this.update(cx, |page, cx| page.sync(cx)).is_err() {
                    break;
                }
            }
        });
        Self {
            store,
            ctx,
            client,
            param_workspace: params.get("workspaceId").cloned(),
            shown: PageWorkspace::Message("Loading workspaces…"),
            project: None,
            rail: Rail::new(None, true),
            overview: Loaded::Loading,
            builders: BuilderMatch::default(),
            target: AgentTarget::NoAgent,
            derived: DerivedCache::default(),
            fetching: false,
            width: 0.,
            layout: layout_for(0.),
            rail_items: Vec::new(),
            main_items: Vec::new(),
            rail_list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            main_list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            expanded: BTreeSet::new(),
            needs: NeedsYouState::default(),
            decision: None,
            decision_generation: 0,
            _subscriptions: subscriptions,
            _clock: clock,
        }
    }

    fn choose(&self, cx: &App) -> (PageWorkspace, Option<String>) {
        let preselected = self.store.read(cx).preselected().map(str::to_string);
        let active = self.ctx.active_workspace_id(cx);
        let Some(client) = &self.client else {
            return (PageWorkspace::Message("Loading workspaces…"), None);
        };
        let client = client.read(cx);
        let ids: Vec<&str> = client
            .workspaces()
            .iter()
            .map(|workspace| workspace.id.as_str())
            .collect();
        let shown = choose_workspace(
            self.param_workspace.as_deref(),
            preselected.as_deref(),
            active.as_deref(),
            &ids,
            client.is_connected(),
        );
        let project = match &shown {
            PageWorkspace::Ready(id) => project_name(client.projects(), id),
            PageWorkspace::Message(_) => None,
        };
        (shown, project)
    }

    /// Takes a new snapshot from the store and the client. The derived data rebuilds only when
    /// the store revision, the live TASK set, or the minute changes.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let (shown, project) = self.choose(cx);
        if shown != self.shown {
            // Another workspace mounts a new overview: its selection, log, and actions start
            // again.
            self.rail = Rail::new(None, true);
            self.derived.clear();
            self.expanded.clear();
            self.needs = NeedsYouState::default();
            self.decision = None;
            if let PageWorkspace::Ready(workspace_id) = &shown {
                self.rail.workspace_id = Some(workspace_id.clone());
                self.store
                    .update(cx, |store, cx| store.ensure(workspace_id, cx));
            }
            self.shown = shown;
        }
        self.project = project;
        let PageWorkspace::Ready(workspace_id) = self.shown.clone() else {
            self.rail_items.clear();
            self.main_items.clear();
            self.rail_list.reset(0);
            self.main_list.reset(0);
            cx.notify();
            return;
        };
        let now = now_ms();
        let (board, overview, live, fetching, revision, initial, builders, target) = {
            let store = self.store.read(cx);
            let overview = store.overview(&workspace_id);
            let builders = overview.ready().map_or_else(BuilderMatch::default, |data| {
                store.builders(&workspace_id, &data.tasks, now, cx)
            });
            (
                store.board(&workspace_id),
                overview,
                store.live(&workspace_id),
                store.is_fetching(&workspace_id),
                store.revision(),
                store.initial_task().map(str::to_string),
                builders,
                store.agent_target(&workspace_id, cx),
            )
        };
        // The rail reads the same live status as the plan tree.
        let board = match board {
            Loaded::Ready(mut board) => {
                board.tasks = with_live_status(&board.tasks, &live);
                Loaded::Ready(board)
            }
            other => other,
        };
        self.rail
            .set_board(board, &workspace_id, initial.as_deref());
        self.rail.log = Some(
            overview
                .ready()
                .and_then(|overview| overview.run.as_ref())
                .map(|run| run.log.iter().rev().cloned().collect())
                .unwrap_or_default(),
        );
        let key = DerivedKey {
            revision,
            live: live.clone(),
            minute: now / MINUTE.as_millis() as i64,
        };
        self.derived.update(key, || {
            overview
                .ready()
                .map(|overview| derive_overview(overview, &live))
        });
        self.overview = overview;
        self.builders = builders;
        self.target = target;
        self.fetching = fetching;
        self.rebuild(cx);
    }

    /// Rebuilds the list items from the snapshot and the view state.
    fn rebuild(&mut self, cx: &mut Context<Self>) {
        let body = body_items(
            &self.overview,
            self.derived.get(),
            self.layout.grid,
            self.needs.show_all,
        );
        // The overview shows its warnings in the body, so the rail leaves out the board's.
        let rail: Vec<RailItem> = self
            .rail
            .items()
            .into_iter()
            .filter(|item| !matches!(item, RailItem::Warning(_)))
            .collect();
        let mut main = vec![PageItem::Header];
        let rail = if self.layout.stacked {
            // Header, progress, waiting, the body, tasks, and the autorun log.
            let split = rail
                .iter()
                .position(|item| *item == RailItem::TasksTop)
                .unwrap_or(rail.len());
            let (top, bottom) = rail.split_at(split);
            main.extend(top.iter().cloned().map(PageItem::Rail));
            main.extend(body.into_iter().map(PageItem::Body));
            main.extend(bottom.iter().cloned().map(PageItem::Rail));
            Vec::new()
        } else {
            main.extend(body.into_iter().map(PageItem::Body));
            rail
        };
        apply_items(&self.rail_list, &mut self.rail_items, rail);
        apply_items(&self.main_list, &mut self.main_items, main);
        cx.notify();
    }

    fn set_width(&mut self, width: f32, cx: &mut Context<Self>) {
        if (width - self.width).abs() < 0.5 {
            return;
        }
        self.width = width;
        let layout = layout_for(width);
        if layout != self.layout {
            let flipped = layout.stacked != self.layout.stacked;
            self.layout = layout;
            if flipped {
                // The rail moves between the two lists; both start at the top.
                self.rail_items.clear();
                self.main_items.clear();
                self.rail_list.reset(0);
                self.main_list.reset(0);
            }
            self.rebuild(cx);
        }
    }

    pub(crate) fn refresh(&mut self, cx: &mut Context<Self>) {
        if let PageWorkspace::Ready(workspace_id) = self.shown.clone() {
            self.store
                .update(cx, |store, cx| store.refresh(&workspace_id, cx));
        }
    }

    pub(crate) fn toggle_cell(&mut self, kind: CellKind, cx: &mut Context<Self>) {
        if !self.expanded.remove(&kind) {
            self.expanded.insert(kind);
        }
        self.rebuild(cx);
    }

    /// A plan tree row selects its TASK in the task list. The stacked page scrolls to it.
    pub(crate) fn select_task(&mut self, task_id: String, cx: &mut Context<Self>) {
        let tasks = self
            .rail
            .board
            .ready()
            .map(|board| board.tasks.clone())
            .unwrap_or_default();
        if let Some(list) = self.rail.task_list.as_mut() {
            list.select_from_outside(Some(task_id.clone()), &tasks);
        }
        self.rebuild(cx);
        if self.layout.stacked && let Some(index) = self.main_items.iter().position(|item| {
            matches!(
                item,
                PageItem::Rail(RailItem::TaskRow(TaskListRow::Task { id, .. })) if *id == task_id
            )
        }) {
            self.main_list.scroll_to_reveal_item(index);
        }
    }

    pub(crate) fn needs_action(
        &mut self,
        action: NeedsAction,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let rows = self
            .derived
            .get()
            .map(|derived| derived.rows.clone())
            .unwrap_or_default();
        let row = |index: usize| rows.get(index).cloned();
        let blocked = actions_disabled(&self.target, self.needs.sending);
        match action {
            NeedsAction::Toggle(key) => {
                self.needs.expanded = if self.needs.expanded.as_deref() == Some(key.as_str()) {
                    None
                } else {
                    Some(key)
                };
            }
            NeedsAction::Answer(index) => {
                if let Some(row) = row(index).filter(|_| !blocked) {
                    self.open_answer(index, row.id, window, cx);
                }
            }
            NeedsAction::SendAnswer(index) => self.submit_answer(index, cx),
            NeedsAction::CancelAnswer => self.needs.close_answer(),
            NeedsAction::UseDefault(index) => {
                if let Some(row) = row(index).filter(|_| !blocked)
                    && let Some(default) = &row.default_answer
                {
                    let prompt = default_answer_prompt(&row.id, &row.title, default);
                    self.run(prompt, AfterSend::Nothing, cx);
                }
            }
            NeedsAction::View { id, title } => self.open_decision(id, title, window, cx),
            NeedsAction::Accept(index) => {
                if let Some(row) = row(index).filter(|_| !blocked) {
                    self.needs.rejecting = None;
                    self.run(
                        accept_decision_prompt(&row.id, &row.title),
                        AfterSend::Nothing,
                        cx,
                    );
                }
            }
            NeedsAction::Reject(index) => {
                if let Some(row) = row(index).filter(|_| !blocked) {
                    let confirming = self.needs.rejecting.as_deref() == Some(row.id.as_str());
                    match reject_step(confirming, RejectPress::Reject) {
                        RejectStep::Confirm => self.needs.rejecting = Some(row.id),
                        RejectStep::Send => self.run(
                            reject_decision_prompt(&row.id, &row.title),
                            AfterSend::ClearReject,
                            cx,
                        ),
                        RejectStep::Back => self.needs.rejecting = None,
                    }
                }
            }
            NeedsAction::CancelReject => {
                if reject_step(true, RejectPress::Cancel) == RejectStep::Back {
                    self.needs.rejecting = None;
                }
            }
            NeedsAction::MarkDone(index) => {
                if let Some(row) = row(index).filter(|_| !blocked) {
                    self.run(manual_checks_prompt(&row.checks), AfterSend::Nothing, cx);
                }
            }
            NeedsAction::ShowMore => self.needs.show_all = !self.needs.show_all,
            NeedsAction::OpenAgent(agent_id) => self.ctx.open_agent(&agent_id, cx),
        }
        self.rebuild(cx);
    }

    fn open_answer(
        &mut self,
        index: usize,
        row_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.needs.rejecting = None;
        self.needs.close_answer();
        let input = cx.new(|cx| InputState::new(window, cx).placeholder("Your answer"));
        let subscription = cx.subscribe_in(
            &input,
            window,
            move |page, input, event: &InputEvent, _, cx| match event {
                InputEvent::Change => {
                    page.needs.draft = input.read(cx).value().to_string();
                    cx.notify();
                }
                InputEvent::PressEnter { .. } => page.submit_answer(index, cx),
                _ => {}
            },
        );
        input.update(cx, |input, cx| input.focus(window, cx));
        self.needs.answering = Some(row_id);
        self.needs.input = Some(input);
        self.needs._input_subscription = Some(subscription);
    }

    fn submit_answer(&mut self, index: usize, cx: &mut Context<Self>) {
        let Some(row) = self
            .derived
            .get()
            .and_then(|derived| derived.rows.get(index))
            .cloned()
        else {
            return;
        };
        if send_disabled(&self.target, self.needs.sending, &self.needs.draft) {
            return;
        }
        let prompt = answer_prompt(&row.id, &row.title, &self.needs.draft);
        self.run(prompt, AfterSend::CloseAnswer, cx);
    }

    /// Sends a prompt through the store. On success it names the agent and opens it; on an
    /// error it shows the error, as Electron's toasts do.
    fn run(&mut self, prompt: String, after: AfterSend, cx: &mut Context<Self>) {
        let PageWorkspace::Ready(workspace_id) = self.shown.clone() else {
            return;
        };
        self.needs.sending = true;
        let sent = self
            .store
            .update(cx, |store, cx| store.send(&workspace_id, prompt, cx));
        cx.spawn(async move |this, cx| {
            let result = sent.await;
            this.update(cx, |page, cx| {
                page.needs.sending = false;
                match result {
                    Ok((agent_id, name)) => {
                        page.ctx.show_message(
                            SharedString::from(format!("Sent to {name}")),
                            MessageLevel::Info,
                            cx,
                        );
                        page.ctx.open_agent(&agent_id, cx);
                        if after == AfterSend::CloseAnswer {
                            page.needs.close_answer();
                        }
                    }
                    Err(error) => page.ctx.show_message(
                        SharedString::from(error.to_string()),
                        MessageLevel::Error,
                        cx,
                    ),
                }
                if after == AfterSend::ClearReject {
                    page.needs.rejecting = None;
                }
                page.rebuild(cx);
            })
            .ok();
        })
        .detach();
        self.rebuild(cx);
    }

    pub(crate) fn open_decision(
        &mut self,
        id: String,
        title: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let focus = cx.focus_handle();
        focus.focus(window, cx);
        self.decision = Some(DecisionModal {
            id,
            title,
            state: Loaded::Loading,
            focus,
            generation: 0,
        });
        self.load_decision(cx);
    }

    /// Loads the open ADR; an answer for an older load or a closed modal is dropped.
    pub(crate) fn load_decision(&mut self, cx: &mut Context<Self>) {
        let PageWorkspace::Ready(workspace_id) = self.shown.clone() else {
            return;
        };
        self.decision_generation += 1;
        let generation = self.decision_generation;
        let Some(id) = self.decision.as_mut().map(|modal| {
            modal.generation = generation;
            modal.state = Loaded::Loading;
            modal.id.clone()
        }) else {
            return;
        };
        let read = self
            .store
            .update(cx, |store, cx| store.decision(&workspace_id, &id, cx));
        cx.spawn(async move |this, cx| {
            let result = read.await;
            this.update(cx, |page, cx| {
                if let Some(modal) = page
                    .decision
                    .as_mut()
                    .filter(|modal| modal.generation == generation)
                {
                    modal.state = match result {
                        Ok(detail) => {
                            Loaded::Ready((detail.file, parse_markdown(&detail.markdown)))
                        }
                        Err(error) => Loaded::Error(error.to_string()),
                    };
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    pub(crate) fn close_decision(&mut self, _: &mut Window, cx: &mut Context<Self>) {
        if self.decision.take().is_some() {
            cx.notify();
        }
    }

    fn header_text(&self) -> (String, String) {
        let overview = self.overview.ready();
        let title = match overview {
            Some(overview) if !overview.tasks.is_empty() => overview.plan.title.clone(),
            _ => "Watchtower".to_string(),
        };
        let subtitle = [
            self.project.clone(),
            overview.and_then(|overview| overview.plan.slug.clone()),
            overview
                .and_then(|overview| overview.plan.updated.as_ref())
                .map(|updated| format!("updated {updated}")),
        ]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
        (title, subtitle)
    }

    fn render_main_row(
        &mut self,
        index: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.main_items.get(index).cloned() else {
            return div().into_any_element();
        };
        let stacked = self.layout.stacked;
        let pad = self.layout.padding();
        let last = index + 1 == self.main_items.len();
        // Beside the rail the header's own margin is the only gap above the body.
        let gap = index > 0
            && (stacked || index > 1)
            && match &item {
                PageItem::Header => false,
                PageItem::Rail(item) => starts_card(item),
                PageItem::Body(item) => item.starts_block(),
            };
        let body = match &item {
            PageItem::Header => self.render_header(cx),
            PageItem::Rail(item) => self.rail.render_item(item, cx),
            PageItem::Body(item) => render_body(self, item, cx),
        };
        item_frame()
            .px(bachuc_theme::zoomed(pad, cx))
            .when(index == 0, |this| {
                this.pt(bachuc_theme::zoomed(
                    if stacked { pad } else { MAIN_PADDING_TOP },
                    cx,
                ))
            })
            .when(gap, |this| this.pt(bachuc_theme::zoomed(GAP, cx)))
            .when(last, |this| this.pb(bachuc_theme::zoomed(pad, cx)))
            .child(body)
            .into_any_element()
    }

    fn render_rail_row(
        &mut self,
        index: usize,
        _: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(item) = self.rail_items.get(index).cloned() else {
            return div().into_any_element();
        };
        let last = index + 1 == self.rail_items.len();
        item_frame()
            .px(bachuc_theme::zoomed(RAIL_PADDING, cx))
            .when(index == 0, |this| {
                this.pt(bachuc_theme::zoomed(RAIL_PADDING, cx))
            })
            .when(index > 0 && starts_card(&item), |this| {
                this.pt(bachuc_theme::zoomed(GAP, cx))
            })
            .when(last, |this| this.pb(bachuc_theme::zoomed(RAIL_PADDING, cx)))
            .child(self.rail.render_item(&item, cx))
            .into_any_element()
    }

    fn render_header(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let (title, subtitle) = self.header_text();
        let fetching = self.fetching;
        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(bachuc_theme::zoomed(SPACE_4, cx))
            .mb(bachuc_theme::zoomed(HEADER_GAP, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id("wt-page-title")
                            .role(Role::Heading)
                            .aria_label(SharedString::from(title.clone()))
                            .line_clamp(2)
                            .text_size(ui_font_size(TITLE_SIZE, cx))
                            .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                            .text_color(colors.foreground)
                            .child(title),
                    )
                    .child(
                        div()
                            .mt(bachuc_theme::zoomed(SUBTITLE_GAP, cx))
                            .truncate()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.foreground_muted)
                            .child(subtitle),
                    ),
            )
            .child(
                div()
                    .id("wt-page-refresh")
                    .role(Role::Button)
                    .aria_label("Refresh Watchtower overview")
                    .size(bachuc_theme::zoomed(REFRESH_SIZE, cx))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(bachuc_theme::zoomed(REFRESH_RADIUS, cx))
                    .when(fetching, |this| this.opacity(0.5))
                    .when(!fetching, |this| {
                        this.cursor_pointer()
                            .on_click(cx.listener(|page, _, _, cx| page.refresh(cx)))
                    })
                    .child(
                        icon(IconName::RefreshCw)
                            .with_size(bachuc_theme::zoomed(REFRESH_ICON, cx))
                            .text_color(colors.foreground_muted),
                    ),
            )
            .into_any_element()
    }
}

impl RailHost for WatchtowerPage {
    fn rail_mut(&mut self) -> &mut Rail {
        &mut self.rail
    }

    fn rail_changed(&mut self, cx: &mut Context<Self>) {
        self.rebuild(cx);
    }
}

impl Render for WatchtowerPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        // The outer width does not depend on the padding, so the layout cannot flip back and
        // forth.
        let weak = cx.entity().downgrade();
        let width = self.width;
        let measure = canvas(
            move |bounds, _, cx| {
                // The breakpoints are in unzoomed px: a zoomed page has proportionally fewer.
                let next = f32::from(bounds.size.width) / bachuc_theme::zoom(cx);
                if (next - width).abs() >= 0.5 {
                    cx.defer(move |cx| {
                        weak.update(cx, |page, cx| page.set_width(next, cx)).ok();
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .top_0()
        .left_0()
        .size_full();
        let screen = div()
            .id("wt-page")
            .role(Role::Main)
            .aria_label("Watchtower")
            .relative()
            .size_full()
            .bg(colors.surface0)
            .child(measure);
        match &self.shown {
            PageWorkspace::Message(message) => screen.child(
                div()
                    .id("wt-page-message")
                    .role(Role::Status)
                    .aria_label(*message)
                    .p(bachuc_theme::zoomed(self.layout.padding(), cx))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(colors.foreground_muted)
                    .child(*message),
            ),
            PageWorkspace::Ready(_) => {
                let main =
                    list(self.main_list.clone(), cx.processor(Self::render_main_row)).size_full();
                let body = if self.layout.stacked {
                    main.into_any_element()
                } else {
                    div()
                        .size_full()
                        .flex()
                        .flex_row()
                        .child(
                            div()
                                .id("wt-rail")
                                .role(Role::Complementary)
                                .aria_label("Board")
                                .w(bachuc_theme::zoomed(RAIL_WIDTH, cx))
                                .flex_shrink_0()
                                .h_full()
                                .border_r_1()
                                .border_color(colors.border)
                                .bg(colors.surface1)
                                .child(
                                    list(
                                        self.rail_list.clone(),
                                        cx.processor(Self::render_rail_row),
                                    )
                                    .size_full(),
                                ),
                        )
                        .child(div().flex_1().min_w_0().h_full().child(main))
                        .into_any_element()
                };
                let overlay = self
                    .decision
                    .as_ref()
                    .map(|modal| decision_overlay(modal, cx));
                screen.child(body).children(overlay)
            }
        }
    }
}
