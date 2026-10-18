//! The New workspace panel `board` (`client/board.tsx`). The screen has a project but no
//! workspace yet, so it shows the rail cards of the project's root: no run, agent, or overview.

use crate::model::{changed_range, folder_name};
use crate::rail::{CARD_GAP, Rail, RailHost, RailItem, item_frame, starts_card};
use crate::store::WatchtowerStore;
use bachuc_theme::metrics::{FONT_SM, RADIUS_MD, SPACE_2, SPACE_3, WEIGHT_SEMIBOLD};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// The title's `fontSize: 17`; the Refresh label's `fontSize: 13`.
const TITLE_SIZE: f32 = 17.;
const BUTTON_TEXT: f32 = 13.;
const OVERDRAW: f32 = 400.;

#[derive(Clone, Debug, PartialEq, Eq)]
enum PanelItem {
    Header { title: String, fetching: bool },
    Rail(RailItem),
}

pub(crate) struct BoardPanel {
    store: Entity<WatchtowerStore>,
    project_id: String,
    project_name: Option<String>,
    rail: Rail,
    items: Vec<PanelItem>,
    revision: Option<u64>,
    list: ListState,
    _observe: Subscription,
}

impl BoardPanel {
    pub(crate) fn new(
        store: Entity<WatchtowerStore>,
        project_id: String,
        cwd: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let observe = cx.observe(&store, |panel, _, cx| panel.sync(cx));
        cx.defer_in(window, |panel, _, cx| {
            let project_id = panel.project_id.clone();
            panel
                .store
                .update(cx, |store, cx| store.ensure_project(&project_id, cx));
            panel.sync(cx);
        });
        Self {
            store,
            project_id,
            project_name: folder_name(cwd),
            rail: Rail::new(None, false),
            items: Vec::new(),
            revision: None,
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            _observe: observe,
        }
    }

    fn sync(&mut self, cx: &mut Context<Self>) {
        let (board, fetching, revision) = {
            let store = self.store.read(cx);
            (
                store.project_board(&self.project_id),
                store.is_fetching_project(&self.project_id),
                store.revision(),
            )
        };
        let source_key = format!("project:{}", self.project_id);
        self.rail.set_board(board, &source_key, None);
        let title = self
            .rail
            .board
            .ready()
            .map_or_else(|| "Watchtower".to_string(), |board| board.title.clone());
        let mut items = vec![PanelItem::Header { title, fetching }];
        items.extend(self.rail.items().into_iter().map(PanelItem::Rail));
        // New data changes rows in place, so every row measures again.
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
            PanelItem::Header { title, fetching } => {
                (true, self.render_header(title, *fetching, cx))
            }
            PanelItem::Rail(item) => (starts_card(item), self.rail.render_item(item, cx)),
        };
        item_frame()
            .px(bachuc_theme::zoomed(CARD_GAP, cx))
            .when(starts, |this| this.pt(bachuc_theme::zoomed(CARD_GAP, cx)))
            .when(last, |this| this.pb(bachuc_theme::zoomed(CARD_GAP, cx)))
            .child(body)
            .into_any_element()
    }

    fn render_header(&self, title: &str, fetching: bool, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let project = self.project_name.as_deref().unwrap_or("Project");
        div()
            .flex()
            .flex_row()
            .items_center()
            .gap(bachuc_theme::zoomed(SPACE_3, cx))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .flex()
                    .flex_col()
                    .child(
                        div()
                            .id("wt-panel-title")
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
                            .truncate()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.foreground_muted)
                            .child(format!("{project} · Read-only")),
                    ),
            )
            .child(
                div()
                    .id("wt-panel-refresh")
                    .role(Role::Button)
                    .aria_label("Refresh Watchtower board")
                    .px(bachuc_theme::zoomed(SPACE_3, cx))
                    .py(bachuc_theme::zoomed(SPACE_2, cx))
                    .border_1()
                    .border_color(colors.border)
                    .rounded(bachuc_theme::zoomed(RADIUS_MD, cx))
                    .bg(colors.surface2)
                    .text_size(ui_font_size(BUTTON_TEXT, cx))
                    .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                    .text_color(colors.foreground)
                    .when(!fetching, |this| {
                        this.cursor_pointer()
                            .on_click(cx.listener(|panel, _, _, cx| {
                                let project_id = panel.project_id.clone();
                                panel
                                    .store
                                    .update(cx, |store, cx| store.refresh_project(&project_id, cx));
                            }))
                    })
                    .child(if fetching { "Loading…" } else { "Refresh" }),
            )
            .into_any_element()
    }
}

impl RailHost for BoardPanel {
    fn rail_mut(&mut self) -> &mut Rail {
        &mut self.rail
    }

    fn rail_changed(&mut self, cx: &mut Context<Self>) {
        self.sync(cx);
    }
}

impl Render for BoardPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        div()
            .id("wt-panel")
            .role(Role::Group)
            .aria_label("Watchtower board")
            .size_full()
            .bg(colors.surface0)
            .child(list(self.list.clone(), cx.processor(Self::render_row)).size_full())
    }
}
