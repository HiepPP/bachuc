//! The Runs view of `client/page.tsx` (`HostBoard` 1355-1692, `RunColumn` 1110-1232, the lane
//! rail 1245-1340): one lane for the active host (ADR-0040), its rail, and two columns. Each
//! column is a GPUI `list` with one item per card and per subagent row.

use crate::card::{
    CARD_RADIUS, CONTROL_RADIUS, CardAction, CardUi, PANEL_EDGE, Paint, button, children_row,
    column_card, spinner, tabular,
};
use crate::model::text::{
    column_name, connection_label, host_name, is_laptop, snapshot_warning, unavailable_notice,
};
use crate::model::tree::{
    ColumnItem, RunTree, board_columns, column_items, item_key, splice_range, total, unread_count,
};
use crate::model::{ConnectionState, OrbSettings, SEPARATOR};
use crate::page::BoardPage;
use crate::store::BoardStore;
use bachuc_icons::{IconName, icon};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::Cell;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use std::time::Duration;

const COLUMN_GAP: f32 = 24.;
const RAIL_HEIGHT: f32 = 40.;
const OVERDRAW: f32 = 400.;
const COLUMNS: [(&str, &str); 2] = [
    ("Running", "No conversations are running."),
    ("Just finished", "No finished conversations observed yet."),
];

struct Column {
    list: ListState,
    trees: Vec<RunTree>,
    items: Vec<ColumnItem>,
    keys: Vec<String>,
    /// The column width the last frame measured.
    width: Rc<Cell<f32>>,
    built_width: f32,
}

impl Column {
    fn new() -> Self {
        Self {
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            trees: Vec::new(),
            items: Vec::new(),
            keys: Vec::new(),
            width: Rc::new(Cell::new(0.)),
            built_width: -1.,
        }
    }
}

/// What the Runs view keeps across frames.
pub(crate) struct RunsState {
    columns: [Column; 2],
    collapsed: HashSet<String>,
    collapsed_version: u64,
    pub(crate) cards: HashMap<String, CardUi>,
    pub(crate) page_error: Option<String>,
    /// The store revision, scale bits, and collapse version the items were built from.
    built: Option<(u64, u32, u64)>,
    hues: BTreeMap<String, u16>,
    orb: OrbSettings,
    ticking: bool,
    _ticker: Option<Task<()>>,
}

impl RunsState {
    pub(crate) fn new() -> Self {
        Self {
            columns: [Column::new(), Column::new()],
            collapsed: HashSet::new(),
            collapsed_version: 0,
            cards: HashMap::new(),
            page_error: None,
            built: None,
            hues: BTreeMap::new(),
            orb: OrbSettings::default(),
            ticking: false,
            _ticker: None,
        }
    }

    pub(crate) fn toggle(&mut self, agent_id: &str) {
        if !self.collapsed.remove(agent_id) {
            self.collapsed.insert(agent_id.to_string());
        }
        self.collapsed_version += 1;
    }

    pub(crate) fn card(&mut self, run_id: &str) -> &mut CardUi {
        self.cards.entry(run_id.to_string()).or_default()
    }

    /// Rebuilds the column items when the snapshot, the scale, the collapsed set, or a column
    /// width changed. Items that kept their key keep their place in the list.
    fn sync(&mut self, store: &BoardStore, scale: f32) {
        self.hues = store.hues().clone();
        self.orb = store.orb();
        let key = (store.revision(), scale.to_bits(), self.collapsed_version);
        let resized = self
            .columns
            .iter()
            .any(|column| column.width.get() != column.built_width);
        if self.built == Some(key) && !resized {
            return;
        }
        let rescaled = self.built.is_none_or(|built| built.1 != key.1);
        self.built = Some(key);
        let columns = store
            .snapshot()
            .map(|snapshot| board_columns(&snapshot.runs))
            .unwrap_or_default();
        for (column, trees) in self
            .columns
            .iter_mut()
            .zip([columns.running, columns.finished])
        {
            let width = column.width.get();
            let panel = width - 2. - 2. * PANEL_EDGE * scale;
            let items = column_items(&trees, &self.collapsed, panel, scale);
            let keys: Vec<String> = items.iter().map(|item| item_key(&trees, item)).collect();
            let (range, count) = splice_range(&column.keys, &keys);
            if !range.is_empty() || count > 0 {
                column.list.splice(range, count);
            }
            if rescaled || width != column.built_width {
                column.list.remeasure();
            }
            column.built_width = width;
            column.trees = trees;
            column.items = items;
            column.keys = keys;
        }
    }

    fn has_running(&self) -> bool {
        !self.columns[0].trees.is_empty()
    }
}

impl BoardPage {
    /// The elapsed clock: one redraw a second, only while a running card is in view.
    fn tick(&mut self, cx: &mut Context<Self>) {
        if self.runs.ticking || !self.runs.has_running() {
            return;
        }
        self.runs.ticking = true;
        self.runs._ticker = Some(cx.spawn(async move |this, cx| {
            loop {
                cx.background_executor().timer(Duration::from_secs(1)).await;
                let keep = this
                    .update(cx, |page, cx| {
                        let keep = page.shows_runs() && page.runs.has_running();
                        if keep {
                            cx.notify();
                        } else {
                            page.runs.ticking = false;
                        }
                        keep
                    })
                    .unwrap_or(false);
                if !keep {
                    break;
                }
            }
        }));
    }

    pub(crate) fn render_runs(
        &mut self,
        store: &Entity<BoardStore>,
        scale: f32,
        now: i64,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let s = |value: f32| px(value * scale);
        let t = bachuc_theme::tokens(cx);
        let (connection, host, fetching, has_data) = {
            let store = store.read(cx);
            self.runs.sync(store, scale);
            (
                store.connection(now),
                store.host_label(cx),
                store.fetching(),
                store.snapshot().is_some(),
            )
        };
        self.tick(cx);

        let running = &self.runs.columns[0].trees;
        let finished = &self.runs.columns[1].trees;
        let figures = has_data.then(|| (total(running), total(finished)));
        let rail = self.rail(&host, connection, figures, None, scale, cx);

        let weak = store.downgrade();
        let retry = move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
            weak.update(cx, |store, cx| store.refresh(cx)).ok();
        };
        let notice = |message: &'static str, id: &'static str| {
            h_flex()
                .id(id)
                .role(Role::Alert)
                .aria_label(message)
                .items_center()
                .gap(s(10.))
                .py(s(4.))
                .pl(s(10.))
                .pr(s(4.))
                .rounded(s(CARD_RADIUS))
                .bg(t.surface2)
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .text_size(s(12.5))
                        .line_height(s(17.))
                        .text_color(t.foreground_muted)
                        .child(message),
                )
                .child(
                    button(
                        SharedString::from(format!("{id}-retry")),
                        "Retry loading Board",
                        t.accent,
                        false,
                    )
                    .flex_none()
                    .px(s(10.))
                    .py(s(4.))
                    .rounded(s(CONTROL_RADIUS))
                    .border_1()
                    .border_color(t.border)
                    .bg(t.surface0)
                    .active(|style| style.opacity(0.7))
                    .when(!fetching, |button| button.on_click(retry.clone()))
                    .child(
                        div()
                            .text_size(s(12.5))
                            .line_height(s(17.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.foreground)
                            .child(if fetching { "Retrying…" } else { "Retry" }),
                    ),
                )
        };

        let palette = Self::palette_notice(store, scale, cx);

        let body = match connection {
            ConnectionState::Connecting => h_flex()
                .id("board-loading")
                .role(Role::ProgressIndicator)
                .aria_label("Loading runs")
                .min_h(s(72.))
                .items_center()
                .justify_center()
                .gap(s(10.))
                .child(spinner(t.accent, bachuc_theme::zoomed(16., cx)))
                .child(
                    div()
                        .text_size(s(13.))
                        .line_height(s(18.))
                        .text_color(t.foreground_muted)
                        .child("Loading runs…"),
                )
                .into_any_element(),
            ConnectionState::Error | ConnectionState::Offline if !has_data => {
                notice(unavailable_notice(connection), "board-unavailable").into_any_element()
            }
            _ => v_flex()
                .flex_1()
                .min_h_0()
                .gap(s(10.))
                .children(self.runs.page_error.clone().map(|error| {
                    div()
                        .id("board-page-error")
                        .role(Role::Alert)
                        .aria_label(error.clone())
                        .text_size(s(12.5))
                        .line_height(s(17.))
                        .text_color(t.status_danger)
                        .child(error)
                }))
                .children(
                    snapshot_warning(connection).map(|warning| notice(warning, "board-warning")),
                )
                .child(
                    h_flex()
                        .flex_1()
                        .min_h_0()
                        .items_stretch()
                        .gap(s(COLUMN_GAP))
                        .child(self.column(0, scale, cx))
                        .child(self.column(1, scale, cx)),
                )
                .into_any_element(),
        };

        v_flex()
            .flex_1()
            .min_h_0()
            .child(rail)
            .child(
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .gap(s(10.))
                    .mt(s(4.))
                    .children(palette)
                    .child(body),
            )
            .into_any_element()
    }

    /// The host rail: device, name, connection, a hairline, then the lane's counts or, without
    /// counts, its plain `meta`.
    pub(crate) fn rail(
        &self,
        host: &str,
        connection: ConnectionState,
        figures: Option<(usize, usize)>,
        meta_text: Option<&'static str>,
        scale: f32,
        cx: &App,
    ) -> AnyElement {
        let s = |value: f32| px(value * scale);
        let t = bachuc_theme::tokens(cx);
        let tone = match connection {
            ConnectionState::Error => t.status_danger,
            ConnectionState::Live => t.status_success,
            _ => t.status_warning,
        };
        let label = connection_label(connection);
        let meta = |text: String| {
            div()
                .text_size(s(12.))
                .line_height(s(16.))
                .font_features(tabular())
                .text_color(t.foreground_muted)
                .child(text)
        };
        h_flex()
            .flex_none()
            .items_center()
            .gap(s(10.))
            .h(s(RAIL_HEIGHT))
            .bg(t.surface0)
            .child(
                icon(if is_laptop(host) {
                    IconName::Laptop
                } else {
                    IconName::HardDrive
                })
                .with_size(s(18.))
                .text_color(t.foreground),
            )
            .child(
                div()
                    .id("board-host")
                    .role(Role::Heading)
                    .aria_label(host_name(host).to_string())
                    .min_w_0()
                    .truncate()
                    .text_size(s(15.))
                    .line_height(s(20.))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t.foreground)
                    .child(host_name(host).to_string()),
            )
            .child(
                h_flex()
                    .id("board-connection")
                    .role(Role::Status)
                    .aria_label(format!("Connection {label}"))
                    .flex_none()
                    .items_center()
                    .gap(s(6.))
                    .child(div().size(s(6.)).rounded(s(3.)).bg(tone))
                    .child(meta(label.to_string()).text_color(
                        if connection == ConnectionState::Live {
                            t.foreground_muted
                        } else {
                            tone
                        },
                    )),
            )
            .child(div().flex_1().min_w(s(12.)).h(px(1.)).bg(t.border))
            .children(figures.map(|(running, finished)| {
                div()
                    .id("board-figures")
                    .role(Role::Status)
                    .aria_label(format!("{running} running, {finished} finished"))
                    .flex_none()
                    .child(meta(format!(
                        "{running} running{SEPARATOR}{finished} finished"
                    )))
            }))
            .when_some(meta_text.filter(|_| figures.is_none()), |rail, text| {
                rail.child(meta(text.to_string()).flex_shrink(1.).truncate())
            })
            .into_any_element()
    }

    /// "Could not save project colors. Tap to retry."
    pub(crate) fn palette_notice(
        store: &Entity<BoardStore>,
        scale: f32,
        cx: &App,
    ) -> Option<AnyElement> {
        let s = |value: f32| px(value * scale);
        let t = bachuc_theme::tokens(cx);
        let weak = store.downgrade();
        store.read(cx).palette_error(cx).then(|| {
            div()
                .id("board-palette-error")
                .role(Role::Button)
                .aria_label("Could not save project colors. Tap to retry.")
                .cursor_pointer()
                .text_size(s(12.5))
                .line_height(s(17.))
                .text_color(t.status_warning)
                .on_click(move |_, _, cx| {
                    weak.update(cx, |store, cx| store.retry_palette(cx)).ok();
                })
                .child("Could not save project colors. Tap to retry.")
                .into_any_element()
        })
    }

    /// One status column: its title with counts, then the lazy card list.
    fn column(&self, at: usize, scale: f32, cx: &mut Context<Self>) -> AnyElement {
        let s = |value: f32| px(value * scale);
        let t = bachuc_theme::tokens(cx);
        let column = &self.runs.columns[at];
        let (title, _) = COLUMNS[at];
        let count = total(&column.trees);
        let unread = unread_count(&column.trees);
        let label = |size: f32| {
            div()
                .text_size(s(size))
                .line_height(s(16.))
                .text_color(t.foreground_muted)
        };
        let width = column.width.clone();
        let page = cx.weak_entity();
        // Records the column width; a change redraws, so the subagent grid follows it.
        let measure = canvas(
            move |bounds, _, cx| {
                let measured = f32::from(bounds.size.width);
                if width.get() != measured {
                    width.set(measured);
                    cx.defer(move |cx| {
                        page.update(cx, |_, cx| cx.notify()).ok();
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        v_flex()
            .id(SharedString::from(format!("board-column-{at}")))
            .role(Role::Group)
            .aria_label(column_name(title, count, unread))
            .relative()
            .flex_1()
            .min_w_0()
            .min_h_0()
            .gap(s(10.))
            .child(measure)
            .child(
                h_flex()
                    .items_baseline()
                    .gap(s(6.))
                    .px(s(2.))
                    .mb(s(2.))
                    .child(
                        label(12.)
                            .id(SharedString::from(format!("board-column-title-{at}")))
                            .role(Role::Heading)
                            .aria_label(title)
                            .font_weight(FontWeight::SEMIBOLD)
                            .child(title),
                    )
                    .child(label(12.).font_features(tabular()).child(count.to_string()))
                    .when(unread > 0, |header| {
                        header.child(
                            h_flex()
                                .items_center()
                                .gap(s(5.))
                                .child(div().size(s(6.)).rounded(s(3.)).bg(t.status_success))
                                .child(
                                    label(12.)
                                        .font_weight(FontWeight::SEMIBOLD)
                                        .font_features(tabular())
                                        .text_color(t.status_success)
                                        .child(format!("{unread} new")),
                                ),
                        )
                    }),
            )
            .child(
                list(
                    column.list.clone(),
                    cx.processor(move |page: &mut BoardPage, ix, window, cx| {
                        page.render_item(at, ix, window, cx)
                    }),
                )
                .flex_1()
                .min_h_0()
                .w_full(),
            )
            .into_any_element()
    }

    fn render_item(
        &mut self,
        at: usize,
        ix: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(store) = self.store.upgrade() else {
            return div().into_any_element();
        };
        let scale =
            crate::model::settings::board_scale(store.read(cx).size()) * bachuc_theme::zoom(cx);
        let online = store.read(cx).is_connected();
        let watchtower = crate::store::watchtower_open(cx);
        let page = cx.weak_entity();
        let on: crate::card::OnAction = Rc::new(move |action: CardAction, window, cx| {
            page.update(cx, |page, cx| page.card_action(action, window, cx))
                .ok();
        });
        let runs = &self.runs;
        let column = &runs.columns[at];
        let Some(item) = column.items.get(ix) else {
            return div().into_any_element();
        };
        let paint = Paint {
            t: bachuc_theme::tokens(cx),
            scale,
            now: crate::model::text::now_ms(),
            hues: &runs.hues,
            orb: runs.orb,
            online,
            watchtower,
            collapsed: &runs.collapsed,
            cards: &runs.cards,
            on,
        };
        let s = |value: f32| px(value * scale);
        match item {
            ColumnItem::Empty => div()
                .id(SharedString::from(format!("board-empty-{at}")))
                .role(Role::Note)
                .aria_label(COLUMNS[at].1)
                .px(s(2.))
                .text_size(s(12.))
                .line_height(s(16.))
                .text_color(paint.t.foreground_muted)
                .child(COLUMNS[at].1)
                .into_any_element(),
            ColumnItem::Card { tree } => div()
                .w_full()
                .when(ix > 0, |item| item.pt(s(10.)))
                .child(column_card(&paint, &column.trees[*tree], window, cx))
                .into_any_element(),
            ColumnItem::Children {
                tree,
                cells,
                first,
                last,
            } => {
                let panel = column.width.get() - 2. - 2. * PANEL_EDGE * scale;
                children_row(
                    &paint,
                    &column.trees[*tree],
                    cells,
                    *first,
                    *last,
                    panel,
                    window,
                    cx,
                )
            }
        }
    }
}
