//! The Recaps view of `client/recaps.tsx` (`RecapsView` 231-281) under the lane rail: the last
//! 7 days of recaps as one GPUI `list` with one item per day header, project header, and
//! entry. It polls `board.recaps` only while it shows.

use crate::card::{CARD_RADIUS, CONTROL_RADIUS, spinner};
use crate::model::colors::repo_mark;
use crate::model::recaps::{
    CopyState, RECAP_DAYS, RECAPS_POLL_MS, RecapDay, RecapItem, RecapsBody, copy_text,
    decode_recaps, recap_day_markdown, recap_items, recap_meta, recaps_body,
};
use crate::model::text::now_ms;
use crate::model::tree::splice_range;
use crate::model::view_poll::ViewPoll;
use crate::page::{BoardPage, BoardView};
use crate::store::BoardStore;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::json;
use std::collections::HashMap;
use std::time::Duration;

const OVERDRAW: f32 = 400.;

/// What the Recaps view keeps across frames.
pub(crate) struct RecapsState {
    poll: ViewPoll,
    days: Option<Vec<RecapDay>>,
    failed: bool,
    items: Vec<RecapItem>,
    keys: Vec<String>,
    list: ListState,
    /// The Copy button of each day.
    copy: HashMap<String, CopyState>,
    _request: Option<Task<()>>,
    _wake: Option<Task<()>>,
}

impl RecapsState {
    pub(crate) fn new() -> Self {
        Self {
            poll: ViewPoll::default(),
            days: None,
            failed: false,
            items: Vec::new(),
            keys: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            copy: HashMap::new(),
            _request: None,
            _wake: None,
        }
    }

    fn set_days(&mut self, days: Vec<RecapDay>) {
        let items = recap_items(&days);
        let keys: Vec<String> = items.iter().map(|item| item_key(&days, *item)).collect();
        let (range, count) = splice_range(&self.keys, &keys);
        if !range.is_empty() || count > 0 {
            self.list.splice(range, count);
        }
        self.items = items;
        self.keys = keys;
        self.days = Some(days);
    }
}

fn item_key(days: &[RecapDay], item: RecapItem) -> String {
    match item {
        RecapItem::Day(d) => format!("d:{}", days[d].day),
        RecapItem::Project(d, p) => format!("p:{}:{}", days[d].day, days[d].projects[p].key),
        RecapItem::Entry(d, p, e) => {
            let entry = &days[d].projects[p].entries[e];
            format!(
                "e:{}:{}:{}:{}",
                days[d].day, days[d].projects[p].key, entry.agent_id, entry.ended_at
            )
        }
    }
}

impl BoardPage {
    /// Sends `board.recaps` when due, and wakes for the next poll, only while Recaps shows.
    fn drive_recaps(&mut self, store: &Entity<BoardStore>, cx: &mut Context<Self>) {
        let now = now_ms();
        let online = store.read(cx).is_connected();
        let state = &mut self.recaps;
        if state.poll.due(now, RECAPS_POLL_MS, true, online) {
            state.poll.begin(now);
            let ctx = store.read(cx).context();
            let answer = ctx.rpc("board.recaps", json!({ "days": RECAP_DAYS }), cx);
            state._request = Some(cx.spawn(async move |page, cx| {
                let result = answer.await;
                page.update(cx, |page, cx| page.receive_recaps(result, cx))
                    .ok();
            }));
        }
        let state = &mut self.recaps;
        if state._wake.is_some() || !online || state.poll.in_flight() {
            return;
        }
        let delay = state.poll.wait(now, RECAPS_POLL_MS) as u64;
        state._wake = Some(cx.spawn(async move |page, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(delay))
                .await;
            page.update(cx, |page, cx| {
                page.recaps._wake = None;
                if page.view() == BoardView::Recaps {
                    cx.notify();
                }
            })
            .ok();
        }));
    }

    fn receive_recaps(
        &mut self,
        result: anyhow::Result<serde_json::Value>,
        cx: &mut Context<Self>,
    ) {
        let state = &mut self.recaps;
        state.poll.finish();
        match result.and_then(|value| decode_recaps(&value)) {
            Ok(days) => {
                state.failed = false;
                if state.days.as_ref() != Some(&days) {
                    state.set_days(days);
                }
            }
            Err(error) => {
                log::warn!("board: board.recaps failed: {error:#}");
                state.failed = true;
            }
        }
        state._wake = None;
        cx.notify();
    }

    fn copy_day(&mut self, d: usize, cx: &mut Context<Self>) {
        let Some(day) = self.recaps.days.as_ref().and_then(|days| days.get(d)) else {
            return;
        };
        let markdown = recap_day_markdown(day);
        cx.write_to_clipboard(ClipboardItem::new_string(markdown.clone()));
        // The clipboard write reports no error; reading it back tells whether it took.
        let copied = cx
            .read_from_clipboard()
            .and_then(|item| item.text())
            .is_some_and(|text| text == markdown);
        let state = if copied {
            CopyState::Copied
        } else {
            CopyState::Failed
        };
        self.recaps.copy.insert(day.day.clone(), state);
        cx.notify();
    }

    pub(crate) fn render_recaps(
        &mut self,
        store: &Entity<BoardStore>,
        scale: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let s = |value: f32| px(value * scale);
        let t = bachuc_theme::tokens(cx);
        self.drive_recaps(store, cx);
        let (connection, host) = {
            let store = store.read(cx);
            (store.connection(now_ms()), store.host_label(cx))
        };
        let rail = self.rail(&host, connection, None, Some("Last 7 days"), scale, cx);
        let state = &self.recaps;
        let body = match recaps_body(state.days.as_deref(), state.failed) {
            RecapsBody::Loading => div()
                .id("board-recaps-loading")
                .role(Role::ProgressIndicator)
                .aria_label("Loading recaps")
                .child(spinner(t.accent, px(20.)))
                .into_any_element(),
            RecapsBody::Message(text) => div()
                .id("board-recaps-message")
                .role(Role::Note)
                .aria_label(text)
                .px(s(2.))
                .text_size(s(12.))
                .line_height(s(16.))
                .text_color(t.foreground_muted)
                .child(text)
                .into_any_element(),
            RecapsBody::Days => list(
                state.list.clone(),
                cx.processor(|page: &mut BoardPage, ix, _window, cx| {
                    page.render_recap_item(ix, cx)
                }),
            )
            .flex_1()
            .min_h_0()
            .w_full()
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
                    .children(Self::palette_notice(store, scale, cx))
                    .child(body),
            )
            .into_any_element()
    }

    fn render_recap_item(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(store) = self.store.upgrade() else {
            return div().into_any_element();
        };
        let state = &self.recaps;
        let (Some(days), Some(item)) = (state.days.as_ref(), state.items.get(ix)) else {
            return div().into_any_element();
        };
        let board = store.read(cx);
        let scale = crate::model::settings::board_scale(board.size());
        let s = |value: f32| px(value * scale);
        let t = bachuc_theme::tokens(cx);
        match *item {
            RecapItem::Day(d) => {
                let day = &days[d].day;
                let copy = state.copy.get(day).copied().unwrap_or_default();
                h_flex()
                    .when(d > 0, |item| item.mt(s(20.)))
                    .items_center()
                    .gap(s(8.))
                    .pb(s(6.))
                    .border_b_1()
                    .border_color(t.border)
                    .child(
                        div()
                            .id(SharedString::from(format!("board-recap-day-{day}")))
                            .role(Role::Heading)
                            .aria_label(day.clone())
                            .flex_1()
                            .text_size(s(13.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.foreground)
                            .child(day.clone()),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!("board-recap-copy-{day}")))
                            .role(Role::Button)
                            .aria_label(format!("Copy recaps for {day} as markdown"))
                            .cursor_pointer()
                            .px(s(10.))
                            .py(s(4.))
                            .rounded(s(CONTROL_RADIUS))
                            .active(|style| style.bg(t.surface2))
                            .on_click(cx.listener(move |page, _, _, cx| page.copy_day(d, cx)))
                            .text_size(s(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.foreground_muted)
                            .child(copy_text(copy)),
                    )
                    .into_any_element()
            }
            RecapItem::Project(d, p) => {
                let project = &days[d].projects[p];
                let hue = project
                    .project_id
                    .as_ref()
                    .and_then(|id| board.hues().get(id).copied());
                h_flex()
                    .mt(s(10.))
                    .items_center()
                    .gap(s(8.))
                    .child(
                        div()
                            .flex_none()
                            .size(s(10.))
                            .rounded(s(3.))
                            .bg(repo_mark(hue, &t)),
                    )
                    .child(
                        div()
                            .id(SharedString::from(format!(
                                "board-recap-project-{}-{}",
                                days[d].day, project.key
                            )))
                            .role(Role::Heading)
                            .aria_label(project.name.clone())
                            .min_w_0()
                            .truncate()
                            .text_size(s(12.))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(t.foreground_muted)
                            .child(project.name.clone()),
                    )
                    .into_any_element()
            }
            RecapItem::Entry(d, p, e) => {
                let entry = &days[d].projects[p].entries[e];
                let meta = recap_meta(entry);
                let weak = self.store.clone();
                let agent_id = entry.agent_id.clone();
                div()
                    .pt(s(6.))
                    .child(
                        v_flex()
                            .id(SharedString::from(format!(
                                "board-recap-{}",
                                item_key(days, *item)
                            )))
                            .role(Role::Button)
                            .aria_label(format!("Open conversation {}", entry.title))
                            .cursor_pointer()
                            .gap(s(3.))
                            .py(s(8.))
                            .px(s(10.))
                            .rounded(s(CARD_RADIUS))
                            .border_1()
                            .border_color(t.border)
                            .bg(t.surface1)
                            .active(|style| style.bg(t.surface2))
                            .on_click(move |_, _, cx| {
                                weak.update(cx, |store, cx| store.open_agent(&agent_id, cx))
                                    .ok();
                            })
                            .child(
                                div()
                                    .truncate()
                                    .text_size(s(13.5))
                                    .line_height(s(19.))
                                    .font_weight(FontWeight::SEMIBOLD)
                                    .text_color(t.foreground)
                                    .child(entry.title.clone()),
                            )
                            .children(entry.did.clone().filter(|did| !did.is_empty()).map(|did| {
                                div()
                                    .text_size(s(12.5))
                                    .line_height(s(18.))
                                    .text_color(t.foreground_muted)
                                    .child(did)
                            }))
                            .when(!meta.is_empty(), |row| {
                                row.child(
                                    div()
                                        .truncate()
                                        .text_size(s(12.))
                                        .line_height(s(16.))
                                        .text_color(t.foreground_muted)
                                        .child(meta),
                                )
                            }),
                    )
                    .into_any_element()
            }
        }
    }
}
