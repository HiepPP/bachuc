//! The `watchtower` surface (`client/page.tsx`): the workspace choice and its placeholders, then
//! the overview. Until TASK-194 the overview body is the rail cards in one column.

use crate::model::{
    Loaded, PageWorkspace, changed_range, choose_workspace, initial_task, project_name,
    with_live_status,
};
use crate::rail::{CARD_GAP, Rail, RailHost, RailItem, item_frame, starts_card};
use crate::store::WatchtowerStore;
use bachuc_client::{Client, ClientEvent, DirectoryEvent};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::PluginContext;
use bachuc_theme::metrics::{FONT_BASE, FONT_SM, SPACE_4, SPACE_6, WEIGHT_SEMIBOLD};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::Sizable as _;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::BTreeMap;

/// The header: title `fontSize: 18`, subtitle `marginTop: 2`, `marginBottom: 14`, and the
/// 30 px Refresh button with radius 7 and a 15 px icon.
const TITLE_SIZE: f32 = 18.;
const SUBTITLE_GAP: f32 = 2.;
const HEADER_GAP: f32 = 14.;
const REFRESH_SIZE: f32 = 30.;
const REFRESH_RADIUS: f32 = 7.;
const REFRESH_ICON: f32 = 15.;
/// The page padding: 24, or 16 when compact.
const PAGE_PADDING: f32 = SPACE_6;
const OVERDRAW: f32 = 400.;

#[derive(Clone, Debug, PartialEq, Eq)]
enum PageItem {
    Header {
        title: String,
        subtitle: String,
        fetching: bool,
    },
    Rail(RailItem),
}

pub(crate) struct WatchtowerPage {
    store: Entity<WatchtowerStore>,
    ctx: PluginContext,
    /// Held strong: a subscription holds its emitter weakly.
    client: Option<Entity<Client>>,
    /// `params.workspaceId`.
    param_workspace: Option<String>,
    /// `params.task`.
    initial_task: Option<String>,
    shown: PageWorkspace,
    rail: Rail,
    items: Vec<PageItem>,
    revision: Option<u64>,
    list: ListState,
    _subscriptions: Vec<Subscription>,
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
        let initial = initial_task(params);
        // The store keeps the opener's task for the overview of TASK-194.
        store.update(cx, |store, _| store.set_initial_task(initial.clone()));
        cx.defer_in(window, |page, _, cx| page.sync(cx));
        Self {
            store,
            ctx,
            client,
            param_workspace: params.get("workspaceId").cloned(),
            initial_task: initial,
            shown: PageWorkspace::Message("Loading workspaces…"),
            rail: Rail::new(None, true),
            items: Vec::new(),
            revision: None,
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            _subscriptions: subscriptions,
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

    fn sync(&mut self, cx: &mut Context<Self>) {
        let (shown, project) = self.choose(cx);
        if shown != self.shown {
            // Another workspace mounts a new overview: its selection and log start again, and
            // every row measures again.
            self.rail = Rail::new(None, true);
            self.revision = None;
            if let PageWorkspace::Ready(workspace_id) = &shown {
                self.rail.workspace_id = Some(workspace_id.clone());
                self.store
                    .update(cx, |store, cx| store.ensure(workspace_id, cx));
            }
            self.shown = shown;
        }
        let PageWorkspace::Ready(workspace_id) = self.shown.clone() else {
            self.items.clear();
            self.list.reset(0);
            cx.notify();
            return;
        };
        let (board, overview, live, fetching, revision) = {
            let store = self.store.read(cx);
            (
                store.board(&workspace_id),
                store.overview(&workspace_id),
                store.live(&workspace_id),
                store.is_fetching(&workspace_id),
                store.revision(),
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
            .set_board(board, &workspace_id, self.initial_task.as_deref());
        let overview = overview.ready().cloned();
        self.rail.log = Some(
            overview
                .as_ref()
                .and_then(|overview| overview.run.as_ref())
                .map(|run| run.log.iter().rev().cloned().collect())
                .unwrap_or_default(),
        );
        let has_plan = overview
            .as_ref()
            .is_some_and(|overview| !overview.tasks.is_empty());
        let title = match &overview {
            Some(overview) if has_plan => overview.plan.title.clone(),
            _ => "Watchtower".to_string(),
        };
        let subtitle = [
            project,
            overview
                .as_ref()
                .and_then(|overview| overview.plan.slug.clone()),
            overview
                .as_ref()
                .and_then(|overview| overview.plan.updated.as_ref())
                .map(|updated| format!("updated {updated}")),
        ]
        .into_iter()
        .flatten()
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join(", ");
        let mut items = vec![PageItem::Header {
            title,
            subtitle,
            fetching,
        }];
        items.extend(self.rail.items().into_iter().map(PageItem::Rail));
        if self.revision != Some(revision) {
            self.list.splice(0..self.items.len(), items.len());
        } else if let Some((range, count)) = changed_range(&self.items, &items) {
            self.list.splice(range, count);
        }
        self.revision = Some(revision);
        self.items = items;
        cx.notify();
    }

    fn render_row(&mut self, index: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(item) = self.items.get(index).cloned() else {
            return div().into_any_element();
        };
        let last = index + 1 == self.items.len();
        let (starts, body) = match &item {
            PageItem::Header {
                title,
                subtitle,
                fetching,
            } => (false, self.render_header(title, subtitle, *fetching, cx)),
            PageItem::Rail(item) => (starts_card(item), self.rail.render_item(item, cx)),
        };
        item_frame()
            .px(px(PAGE_PADDING))
            .when(index == 0, |this| this.pt(px(PAGE_PADDING)))
            .when(starts && index > 1, |this| this.pt(px(CARD_GAP)))
            .when(last, |this| this.pb(px(PAGE_PADDING)))
            .child(body)
            .into_any_element()
    }

    fn render_header(
        &self,
        title: &str,
        subtitle: &str,
        fetching: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        div()
            .flex()
            .flex_row()
            .items_start()
            .gap(px(SPACE_4))
            .mb(px(HEADER_GAP))
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
                            .aria_label(SharedString::from(title.to_string()))
                            .line_clamp(2)
                            .text_size(ui_font_size(TITLE_SIZE, cx))
                            .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                            .text_color(colors.foreground)
                            .child(title.to_string()),
                    )
                    .child(
                        div()
                            .mt(px(SUBTITLE_GAP))
                            .truncate()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.foreground_muted)
                            .child(subtitle.to_string()),
                    ),
            )
            .child(
                div()
                    .id("wt-page-refresh")
                    .role(Role::Button)
                    .aria_label("Refresh Watchtower overview")
                    .size(px(REFRESH_SIZE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(REFRESH_RADIUS))
                    .when(fetching, |this| this.opacity(0.5))
                    .when(!fetching, |this| {
                        this.cursor_pointer()
                            .on_click(cx.listener(|page, _, _, cx| {
                                if let PageWorkspace::Ready(workspace_id) = page.shown.clone() {
                                    page.store
                                        .update(cx, |store, cx| store.refresh(&workspace_id, cx));
                                }
                            }))
                    })
                    .child(
                        icon(IconName::RefreshCw)
                            .with_size(px(REFRESH_ICON))
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
        self.sync(cx);
    }
}

impl Render for WatchtowerPage {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let screen = div()
            .id("wt-page")
            .role(Role::Main)
            .aria_label("Watchtower")
            .size_full()
            .bg(colors.surface0);
        match &self.shown {
            PageWorkspace::Message(message) => screen.child(
                div()
                    .id("wt-page-message")
                    .role(Role::Status)
                    .aria_label(*message)
                    .p(px(PAGE_PADDING))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(colors.foreground_muted)
                    .child(*message),
            ),
            PageWorkspace::Ready(_) => {
                screen.child(list(self.list.clone(), cx.processor(Self::render_row)).size_full())
            }
        }
    }
}
