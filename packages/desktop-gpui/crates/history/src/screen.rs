//! The History view: the header, the search field, and the list of past agents in date groups,
//! sized as `sessions-screen.tsx` and the desktop `SessionRow` of `agent-list.tsx` draw them.

use std::ops::Range;
use std::rc::Rc;

use bachuc_client::{Client, HistoryEntry, HistoryPage};
use bachuc_icons::{IconName, icon, provider_fallback_icon, provider_logo};
use bachuc_theme::metrics::{
    BORDER_WIDTH_1, FONT_BASE, FONT_SM, HEADER_HEIGHT, ICON_SM, ICON_XS, RADIUS_MD, SPACE_1,
    SPACE_1_5, SPACE_2, SPACE_3, SPACE_4, SPACE_6, WEIGHT_MEDIUM,
};
use bachuc_theme::{Tokens, tokens, ui_font_size};
use chrono::{Local, Utc};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::{
    ARCHIVED_BADGE, Body, CLEAR_SEARCH, FALLBACK_TITLE, Footer, HistoryRow, LOAD_ERROR_TEXT,
    LOAD_MORE, LOADING_MORE, SEARCH_DEBOUNCE, SEARCH_PLACEHOLDER, TITLE, TOO_MANY_MATCHES_TEXT,
    TRY_AGAIN, activity_time, body, day_number, effective_search, footer, group_rows,
    highlight_ranges, merge_page, pending_badge, time_ago,
};

/// How far the list draws beyond the visible rows.
const OVERDRAW: f32 = 400.;
/// Line height of every text: about 1.3, as the Electron text renders.
const LINE: f32 = 1.3;
/// `workspaceTitleText` max width, and the `columnMeta` and `columnMetaFixed` widths.
const WORKSPACE_MAX_WIDTH: f32 = 320.;
const COLUMN_WIDTH: f32 = 132.;
const TIME_WIDTH: f32 = 72.;
/// `SEARCH_FIELD_MAX_WIDTH` and the 20 px input line.
const SEARCH_MAX_WIDTH: f32 = 420.;
const SEARCH_LINE: f32 = 20.;
/// `StatusBadge`: padding 3 px by 8 px, gap 6 px.
const BADGE_PAD_Y: f32 = 3.;
const BADGE_GAP: f32 = 6.;

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum HistoryEvent {
    OpenAgent {
        agent_id: String,
        workspace_id: Option<String>,
    },
}

/// The text of one row, worked out once per list change rather than per draw.
struct RowText {
    label: SharedString,
    label_marks: Vec<Range<usize>>,
    title: SharedString,
    title_marks: Vec<Range<usize>>,
    project: SharedString,
    project_marks: Vec<Range<usize>>,
    branch: SharedString,
    branch_marks: Vec<Range<usize>>,
    time_ago: SharedString,
}

pub struct HistoryScreen {
    client: Entity<Client>,
    input: Entity<InputState>,
    /// The search the shown list belongs to.
    search: Option<String>,
    entries: Vec<HistoryEntry>,
    texts: Vec<RowText>,
    rows: Vec<HistoryRow>,
    next_cursor: Option<String>,
    has_more: bool,
    search_truncated: bool,
    initial_load: bool,
    loading_more: bool,
    failed: bool,
    /// Bumped by each reload, so a reply for an older search is dropped.
    generation: u64,
    list: ListState,
    debounce: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<HistoryEvent> for HistoryScreen {}

impl HistoryScreen {
    pub fn new(
        client: Entity<Client>,
        window: &mut Window,
        cx: &mut Context<HistoryScreen>,
    ) -> HistoryScreen {
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(SEARCH_PLACEHOLDER));
        let input_focus = input.focus_handle(cx);
        let subscriptions = vec![
            // The field border follows the focus.
            cx.on_focus_in(&input_focus, window, |_, _, cx| cx.notify()),
            cx.on_focus_out(&input_focus, window, |_, _, _, cx| cx.notify()),
            cx.subscribe_in(&input, window, |this, input, event: &InputEvent, _, cx| {
                if let InputEvent::Change = event {
                    let value = input.read(cx).value().to_string();
                    this.schedule_search(value, cx);
                }
            }),
        ];
        let mut screen = HistoryScreen {
            client,
            input,
            search: None,
            entries: Vec::new(),
            texts: Vec::new(),
            rows: Vec::new(),
            next_cursor: None,
            has_more: false,
            search_truncated: false,
            initial_load: true,
            loading_more: false,
            failed: false,
            generation: 0,
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            debounce: None,
            _subscriptions: subscriptions,
        };
        screen.reload(cx);
        screen
    }

    /// Applies the input after `SEARCH_DEBOUNCE`; a newer change drops the pending one.
    fn schedule_search(&mut self, value: String, cx: &mut Context<Self>) {
        self.debounce = Some(cx.spawn(async move |this, cx| {
            cx.background_executor().timer(SEARCH_DEBOUNCE).await;
            this.update(cx, |this, cx| this.apply_search(&value, cx))
                .ok();
        }));
        cx.notify();
    }

    fn apply_search(&mut self, value: &str, cx: &mut Context<Self>) {
        let search = effective_search(value, self.client.read(cx).history_search_supported());
        if search != self.search {
            self.search = search;
            self.reload(cx);
        }
    }

    /// Drops the list and fetches the first page for the current search.
    fn reload(&mut self, cx: &mut Context<Self>) {
        self.generation += 1;
        self.entries.clear();
        self.next_cursor = None;
        self.has_more = false;
        self.search_truncated = false;
        self.initial_load = true;
        self.loading_more = false;
        self.failed = false;
        self.rebuild();
        self.fetch(None, cx);
        cx.notify();
    }

    fn load_more(&mut self, cx: &mut Context<Self>) {
        if self.loading_more || !self.has_more {
            return;
        }
        let Some(cursor) = self.next_cursor.clone() else {
            return;
        };
        self.loading_more = true;
        self.rebuild();
        self.fetch(Some(cursor), cx);
        cx.notify();
    }

    fn fetch(&mut self, cursor: Option<String>, cx: &mut Context<Self>) {
        let generation = self.generation;
        let search = self.search.clone();
        let task = self.client.update(cx, |client, cx| {
            client.fetch_agent_history(search.as_deref(), cursor.as_deref(), cx)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if this.generation == generation {
                    this.apply_page(result.map_err(|error| error.to_string()), cx);
                }
            })
            .ok();
        })
        .detach();
    }

    fn apply_page(&mut self, result: Result<HistoryPage, String>, cx: &mut Context<Self>) {
        self.initial_load = false;
        self.loading_more = false;
        match result {
            Ok(page) => {
                self.failed = false;
                self.next_cursor = page.next_cursor;
                self.has_more = page.has_more;
                self.search_truncated |= page.search_truncated;
                merge_page(&mut self.entries, page.entries);
            }
            Err(_) => self.failed = true,
        }
        self.rebuild();
        cx.notify();
    }

    fn searching(&self) -> bool {
        self.search.is_some()
    }

    fn footer(&self) -> Footer {
        footer(
            self.searching(),
            self.search_truncated,
            self.has_more,
            self.loading_more,
        )
    }

    /// Regroups the entries and resets the list: the rows, then one footer item.
    fn rebuild(&mut self) {
        let now = Utc::now();
        let today = day_number(&now.fixed_offset(), &Local);
        let times: Vec<_> = self.entries.iter().map(activity_time).collect();
        let days: Vec<Option<i64>> = times
            .iter()
            .map(|at| at.as_ref().map(|at| day_number(at, &Local)))
            .collect();
        self.rows = group_rows(&days, today);
        let query = self.search.as_deref().unwrap_or("");
        self.texts = self
            .entries
            .iter()
            .zip(&times)
            .map(|(entry, at)| {
                let label = entry
                    .workspace_name
                    .clone()
                    .filter(|name| !name.is_empty())
                    .unwrap_or_else(|| entry.project_name.clone());
                let title = if entry.title.is_empty() {
                    FALLBACK_TITLE.to_string()
                } else {
                    entry.title.clone()
                };
                let branch = entry.branch.clone().unwrap_or_default();
                RowText {
                    label_marks: highlight_ranges(query, &label),
                    label: label.into(),
                    title_marks: highlight_ranges(query, &entry.title),
                    title: title.into(),
                    project_marks: highlight_ranges(query, &entry.project_name),
                    project: entry.project_name.clone().into(),
                    branch_marks: highlight_ranges(query, &branch),
                    branch: branch.into(),
                    time_ago: at
                        .as_ref()
                        .map(|at| time_ago(at, &now, &Local))
                        .unwrap_or_default()
                        .into(),
                }
            })
            .collect();
        self.list.reset(self.rows.len() + 1);
    }

    fn open(&mut self, index: usize, cx: &mut Context<Self>) {
        if let Some(entry) = self.entries.get(index) {
            cx.emit(HistoryEvent::OpenAgent {
                agent_id: entry.agent_id.clone(),
                workspace_id: entry.workspace_id.clone(),
            });
        }
    }

    fn clear_search(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.input
            .update(cx, |input, cx| input.set_value("", window, cx));
        self.debounce = None;
        self.apply_search("", cx);
        cx.notify();
    }

    fn render_row(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        match self.rows.get(ix).copied() {
            Some(HistoryRow::Header(section)) => {
                let size = ui_font_size(FONT_BASE, cx);
                div()
                    .mt(px(SPACE_2))
                    .mb(px(SPACE_2))
                    .px(px(SPACE_3))
                    .text_size(size)
                    .line_height(size * LINE)
                    .font_weight(FontWeight(WEIGHT_MEDIUM))
                    .text_color(t.foreground_muted)
                    .child(section.label())
                    .into_any_element()
            }
            Some(HistoryRow::Entry(index)) => self.render_entry(index, &t, cx),
            None => self.render_footer(&t, cx),
        }
    }

    fn render_entry(&self, index: usize, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        let (Some(entry), Some(text)) = (self.entries.get(index), self.texts.get(index)) else {
            return div().into_any_element();
        };
        let size = ui_font_size(FONT_BASE, cx);
        let mark = HighlightStyle {
            color: Some(t.foreground),
            background_color: Some(t.surface3),
            ..HighlightStyle::default()
        };
        let marked = |value: &SharedString, marks: &[Range<usize>]| {
            StyledText::new(value.clone()).with_highlights(marks.iter().map(|r| (r.clone(), mark)))
        };
        let column = |width: f32| {
            div()
                .flex_none()
                .w(px(width))
                .truncate()
                .text_color(t.foreground_muted)
        };
        let title = h_flex()
            .gap(px(SPACE_2))
            .flex_shrink(1.)
            .min_w_0()
            .child(provider_icon(&entry.provider, t.foreground_muted))
            .child(
                div()
                    .min_w_0()
                    .flex_shrink(1.)
                    .truncate()
                    .text_color(t.foreground_muted)
                    .child(marked(&text.title, &text.title_marks)),
            );
        let content = h_flex()
            .flex_1()
            .min_w_0()
            .overflow_hidden()
            .gap(px(SPACE_2))
            .child(
                div()
                    .flex_none()
                    .max_w(px(WORKSPACE_MAX_WIDTH))
                    .truncate()
                    .text_color(t.foreground)
                    .child(marked(&text.label, &text.label_marks)),
            )
            .child(
                icon(IconName::ChevronRight)
                    .with_size(px(ICON_XS))
                    .text_color(t.foreground_muted),
            )
            .child(title)
            .when(entry.archived, |row| {
                row.child(badge(
                    ARCHIVED_BADGE.into(),
                    Some(IconName::Archive),
                    false,
                    t,
                    cx,
                ))
            })
            .when(entry.pending_permissions > 0, |row| {
                row.child(badge(
                    pending_badge(entry.pending_permissions).into(),
                    None,
                    true,
                    t,
                    cx,
                ))
            });
        let columns = h_flex()
            .flex_none()
            .gap(px(SPACE_3))
            .child(column(COLUMN_WIDTH).child(marked(&text.project, &text.project_marks)))
            .child(column(COLUMN_WIDTH).child(marked(&text.branch, &text.branch_marks)))
            .child(
                column(TIME_WIDTH)
                    .text_align(TextAlign::Right)
                    .child(text.time_ago.clone()),
            );
        // The 4 px `listContent` gap sits under each row, outside its hover area.
        let row = h_flex()
            .id(("history-row", index))
            .role(Role::Button)
            .aria_label(format!("{}: {}", text.label, text.title))
            .items_center()
            .py(px(SPACE_2))
            .px(px(SPACE_3))
            .text_size(size)
            .line_height(size * LINE)
            .cursor_pointer()
            .hover(|row| row.bg(t.surface1))
            .active(|row| row.bg(t.surface2))
            .child(content)
            .child(columns)
            .on_press(cx, move |this, _, cx| this.open(index, cx));
        div().pb(px(SPACE_1)).child(row).into_any_element()
    }

    fn render_footer(&self, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        let content = match self.footer() {
            Footer::None => return div().h(px(SPACE_6)).into_any_element(),
            Footer::TooManyMatches => {
                let size = ui_font_size(FONT_BASE, cx);
                div()
                    .text_size(size)
                    .line_height(size * LINE)
                    .text_color(t.foreground_muted)
                    .child(TOO_MANY_MATCHES_TEXT)
                    .into_any_element()
            }
            Footer::LoadMore { loading } => text_button(
                "history-load-more",
                if loading { LOADING_MORE } else { LOAD_MORE },
                !loading,
                t,
                cx,
                |this, _, cx| this.load_more(cx),
            )
            .into_any_element(),
        };
        h_flex()
            .justify_center()
            .py(px(SPACE_4))
            .mb(px(SPACE_6))
            .child(content)
            .into_any_element()
    }

    fn render_search(&self, window: &Window, t: &Tokens, cx: &mut Context<Self>) -> AnyElement {
        let focused = self.input.focus_handle(cx).is_focused(window);
        let size = ui_font_size(FONT_BASE, cx);
        let has_text = !self.input.read(cx).value().is_empty();
        h_flex()
            .px(px(SPACE_6))
            .pt(px(SPACE_4))
            .child(
                h_flex()
                    .id("history-search")
                    .role(Role::Search)
                    .aria_label(SEARCH_PLACEHOLDER)
                    .flex_1()
                    .min_w_0()
                    .max_w(px(SEARCH_MAX_WIDTH))
                    .gap(px(SPACE_2))
                    .items_center()
                    .py(px(SPACE_1_5))
                    .px(px(SPACE_3))
                    .rounded(px(RADIUS_MD))
                    .border(px(BORDER_WIDTH_1))
                    .border_color(if focused { t.border_accent } else { t.border })
                    .bg(if focused { t.surface2 } else { t.surface1 })
                    .child(
                        icon(IconName::Search)
                            .with_size(px(ICON_SM))
                            .text_color(t.foreground_muted),
                    )
                    .child(
                        div().flex_1().min_w_0().h(px(SEARCH_LINE)).child(
                            Input::new(&self.input)
                                .appearance(false)
                                .bordered(false)
                                .focus_bordered(false)
                                .px_0()
                                .py_0()
                                .text_size(size)
                                .aria_label(SEARCH_PLACEHOLDER),
                        ),
                    )
                    .when(has_text, |field| {
                        field.child(
                            div()
                                .id("history-search-clear")
                                .role(Role::Button)
                                .aria_label(CLEAR_SEARCH)
                                .flex_none()
                                .cursor_pointer()
                                .child(
                                    icon(IconName::X)
                                        .with_size(px(ICON_SM))
                                        .text_color(t.foreground_muted),
                                )
                                .on_press(cx, |this, window, cx| this.clear_search(window, cx)),
                        )
                    }),
            )
            .into_any_element()
    }

    fn render_state(&self, text: &'static str, t: &Tokens, cx: &mut Context<Self>) -> Div {
        let size = ui_font_size(FONT_BASE, cx);
        v_flex()
            .flex_1()
            .min_h_0()
            .items_center()
            .justify_center()
            .gap(px(SPACE_6))
            .p(px(SPACE_6))
            .child(
                div()
                    .id("history-state")
                    .role(Role::Status)
                    .aria_label(text)
                    .text_size(size)
                    .line_height(size * LINE)
                    .text_color(t.foreground_muted)
                    .child(text),
            )
    }
}

impl Render for HistoryScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let title_size = ui_font_size(FONT_BASE, cx);
        let searchable = self.client.read(cx).history_search_supported();
        let state = body(
            self.initial_load,
            self.failed,
            self.entries.len(),
            self.searching(),
        );
        let content = match state {
            Body::Loading => self.render_state(LOADING_MORE, &t, cx).into_any_element(),
            Body::Error => self
                .render_state(LOAD_ERROR_TEXT, &t, cx)
                .child(text_button(
                    "history-try-again",
                    TRY_AGAIN,
                    true,
                    &t,
                    cx,
                    |this, _, cx| this.reload(cx),
                ))
                .into_any_element(),
            Body::Empty { text, clear_search } => self
                .render_state(text, &t, cx)
                .when(clear_search, |state| {
                    state.child(text_button(
                        "history-clear-search",
                        CLEAR_SEARCH,
                        true,
                        &t,
                        cx,
                        |this, window, cx| this.clear_search(window, cx),
                    ))
                })
                .into_any_element(),
            Body::List => div()
                .flex_1()
                .min_h_0()
                .px(px(SPACE_6))
                .pt(px(SPACE_4))
                .child(list(self.list.clone(), cx.processor(Self::render_row)).size_full())
                .into_any_element(),
        };
        v_flex()
            .id("history")
            .size_full()
            .bg(t.surface0)
            .child(
                h_flex()
                    .flex_none()
                    .h(px(HEADER_HEIGHT))
                    .px(px(SPACE_4))
                    .items_center()
                    .border_b(px(BORDER_WIDTH_1))
                    .border_color(t.border)
                    .child(
                        div()
                            .text_size(title_size)
                            .line_height(title_size * LINE)
                            .font_weight(FontWeight::LIGHT)
                            .text_color(t.foreground)
                            .child(TITLE),
                    ),
            )
            .when(searchable, |screen| {
                screen.child(self.render_search(window, &t, cx))
            })
            .child(content)
    }
}

/// A click and an AX press run the same handler; GPUI's default AX press does nothing here.
trait OnPress: StatefulInteractiveElement + Sized {
    fn on_press(
        self,
        cx: &Context<HistoryScreen>,
        handler: impl Fn(&mut HistoryScreen, &mut Window, &mut Context<HistoryScreen>) + 'static,
    ) -> Self {
        let handler = Rc::new(handler);
        let click = handler.clone();
        let view = cx.weak_entity();
        self.on_click(cx.listener(move |this, _, window, cx| click(this, window, cx)))
            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                view.update(cx, |this, cx| handler(this, window, cx)).ok();
            })
    }
}

impl<E: StatefulInteractiveElement> OnPress for E {}

/// The ghost `Button`: muted text that takes a surface on hover.
fn text_button(
    id: &'static str,
    label: &'static str,
    enabled: bool,
    t: &Tokens,
    cx: &mut Context<HistoryScreen>,
    handler: impl Fn(&mut HistoryScreen, &mut Window, &mut Context<HistoryScreen>) + 'static,
) -> Stateful<Div> {
    let size = ui_font_size(FONT_BASE, cx);
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .px(px(SPACE_3))
        .py(px(SPACE_1_5))
        .rounded(px(RADIUS_MD))
        .text_size(size)
        .line_height(size * LINE)
        .text_color(t.foreground)
        .child(label)
        .when(!enabled, |button| button.opacity(0.5))
        .when(enabled, |button| {
            button
                .cursor_pointer()
                .hover(|b| b.bg(t.surface2))
                .on_press(cx, handler)
        })
}

/// `StatusBadge`: muted, or warning for the pending count.
fn badge(
    label: SharedString,
    leading: Option<IconName>,
    warning: bool,
    t: &Tokens,
    cx: &App,
) -> Div {
    let size = ui_font_size(FONT_SM, cx);
    let color = if warning {
        t.status_warning
    } else {
        t.foreground_muted
    };
    h_flex()
        .flex_none()
        .gap(px(BADGE_GAP))
        .items_center()
        .px(px(SPACE_2))
        .py(px(BADGE_PAD_Y))
        .rounded_full()
        .border(px(BORDER_WIDTH_1))
        .border_color(if warning {
            transparent_black()
        } else {
            t.border
        })
        .bg(if warning { t.tint_warning } else { t.surface3 })
        .text_size(size)
        .line_height(size * LINE)
        .text_color(color)
        .when_some(leading, |badge, name| {
            badge.child(icon(name).with_size(px(FONT_SM)).text_color(color))
        })
        .child(label)
}

fn provider_icon(provider: &str, color: Hsla) -> AnyElement {
    match provider_logo(provider, color) {
        Some(image) => img(image).flex_none().size(px(ICON_SM)).into_any_element(),
        None => icon(provider_fallback_icon(provider))
            .with_size(px(ICON_SM))
            .text_color(color)
            .into_any_element(),
    }
}
