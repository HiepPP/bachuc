//! The Timeline view of `client/timeline.tsx`: the last 24 hours of the active host on one axis,
//! with the legend, the load strip, the hour axis, the host header, and the footer fixed, and
//! one GPUI `list` item per thread row and per subagent row. It polls `board.timeline` only
//! while it shows.

use crate::card::{spinner, tabular};
use crate::model::colors::{project_initial, repo_mark};
use crate::model::text::{host_name, now_ms};
use crate::model::timeline::{
    BarColors, LABEL_MIN_WIDTH, LEGEND, TIMELINE_POLL_MS, TimelineAnswer, TimelineBar,
    TimelineHostInput, TimelineKind, TimelineModel, TimelineRow, bar_colors, bar_label, bar_text,
    decode_timeline, host_meta, host_notes, load_height, peak_label, subagent_text, task_chip_text,
    timeline_footer, timeline_model,
};
use crate::model::tree::splice_range;
use crate::model::view_poll::{ViewPoll, minute_of, until_next_minute};
use crate::page::{BoardPage, BoardView};
use crate::store::{BoardStore, watchtower_open};
use bachuc_theme::Tokens;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::json;
use std::cell::Cell;
use std::collections::BTreeMap;
use std::rc::Rc;
use std::time::Duration;

const ROW_HEIGHT: f32 = 26.;
const SUB_ROW_HEIGHT: f32 = 18.;
const STRIP_HEIGHT: f32 = 46.;
const AXIS_HEIGHT: f32 = 22.;
const LABEL_GAP: f32 = 14.;
const LABEL_WIDTH: f32 = 230.;
const OVERDRAW: f32 = 400.;
/// The single lane's key in the model; GPUI shows only the active host (ADR-0040).
const HOST: &str = "host";
const MONO: &str = "Menlo";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Item {
    Thread(usize),
    Subagents(usize),
}

/// What the Timeline keeps across frames.
pub(crate) struct TimelineState {
    poll: ViewPoll,
    answer: Option<TimelineAnswer>,
    failed: bool,
    /// Grows with each answer that changed.
    revision: u64,
    model: Option<TimelineModel>,
    /// The revision, minute, chart width bits, and scale bits the model was built from.
    built: Option<(u64, i64, u32, u32)>,
    items: Vec<Item>,
    keys: Vec<String>,
    list: ListState,
    width: Rc<Cell<f32>>,
    _request: Option<Task<()>>,
    _wake: Option<Task<()>>,
}

impl TimelineState {
    pub(crate) fn new() -> Self {
        Self {
            poll: ViewPoll::default(),
            answer: None,
            failed: false,
            revision: 0,
            model: None,
            built: None,
            items: Vec::new(),
            keys: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            width: Rc::new(Cell::new(0.)),
            _request: None,
            _wake: None,
        }
    }

    /// Rebuilds the model only on new data, a new minute, a new chart width, or a new scale.
    fn sync(&mut self, now: i64, chart: f32, scale: f32) {
        let minute = minute_of(now);
        let key = (self.revision, minute, chart.to_bits(), scale.to_bits());
        if self.built == Some(key) {
            return;
        }
        let rescaled = self.built.is_none_or(|built| built.3 != key.3);
        self.built = Some(key);
        let threads = self
            .answer
            .as_ref()
            .map_or(&[][..], |answer| answer.threads.as_slice());
        let model = timeline_model(
            &[TimelineHostInput {
                host_id: HOST,
                threads,
            }],
            (minute * 60_000) as f64,
            f64::from(chart),
        );
        let rows = model.host_rows(HOST);
        let mut items = Vec::new();
        let mut keys = Vec::new();
        for (ix, row) in rows.iter().enumerate() {
            items.push(Item::Thread(ix));
            keys.push(format!("t:{}", row.agent_id));
            if !row.subagents.is_empty() {
                items.push(Item::Subagents(ix));
                keys.push(format!("s:{}", row.agent_id));
            }
        }
        let (range, count) = splice_range(&self.keys, &keys);
        if !range.is_empty() || count > 0 {
            self.list.splice(range, count);
        }
        if rescaled {
            self.list.remeasure();
        }
        self.items = items;
        self.keys = keys;
        self.model = Some(model);
    }
}

/// The measures of one frame.
struct Chart<'a> {
    t: Tokens,
    scale: f32,
    model: &'a TimelineModel,
    hues: &'a BTreeMap<String, u16>,
    store: WeakEntity<BoardStore>,
    watchtower: bool,
}

impl Chart<'_> {
    fn s(&self, value: f32) -> Pixels {
        px(value * self.scale)
    }

    fn muted(&self) -> Div {
        div()
            .text_size(self.s(11.))
            .line_height(self.s(15.))
            .font_features(tabular())
            .text_color(self.t.foreground_muted)
    }

    fn mono(&self) -> Div {
        self.muted().text_size(self.s(10.)).font_family(MONO)
    }

    /// A label column and a lane, as `Row`.
    fn row(&self, label: impl IntoElement, lane: impl IntoElement) -> Div {
        h_flex()
            .w_full()
            .items_center()
            .gap(self.s(LABEL_GAP))
            .child(
                div()
                    .flex_none()
                    .w(self.s(LABEL_WIDTH))
                    .min_w_0()
                    .child(label),
            )
            .child(div().flex_1().min_w_0().child(lane))
    }

    /// The fold breaks, drawn behind every lane so the folded time reads as one column.
    fn breaks(&self) -> impl Iterator<Item = Div> + '_ {
        self.model.breaks.iter().map(|gap| {
            div()
                .absolute()
                .top_0()
                .bottom_0()
                .left(px(gap.x as f32))
                .w(px(gap.width as f32))
                .bg(self.t.surface1)
                .border_l_1()
                .border_r_1()
                .border_dashed()
                .border_color(self.t.border)
        })
    }

    /// The 2 px now line over the bars.
    fn now_line(&self) -> Div {
        div()
            .absolute()
            .top_0()
            .bottom_0()
            .left(px(self.model.now_x as f32) - self.s(1.))
            .w(self.s(2.))
            .bg(self.t.status_danger)
    }

    fn hue(&self, row: &TimelineRow) -> Option<u16> {
        row.project_id
            .as_ref()
            .and_then(|id| self.hues.get(id).copied())
    }

    fn task_chip(&self, bar: &TimelineBar) -> Option<AnyElement> {
        let task = bar.task_id.as_deref()?;
        let id = task_chip_text(task).to_string();
        let t = self.t;
        let chip = div()
            .flex_none()
            .px(self.s(4.))
            .py(self.s(1.))
            .rounded(self.s(4.))
            .bg(t.foreground)
            .text_color(t.surface0)
            .text_size(self.s(9.5))
            .line_height(self.s(13.))
            .font_weight(FontWeight::BOLD)
            .child(id.clone());
        let Some(workspace_id) = bar.workspace_id.clone().filter(|_| self.watchtower) else {
            return Some(chip.into_any_element());
        };
        let store = self.store.clone();
        let task = task.to_string();
        Some(
            div()
                .id(SharedString::from(format!(
                    "board-timeline-task-{}",
                    bar.agent_id
                )))
                .role(Role::Link)
                .aria_label(format!("Open {id} in Watchtower"))
                .flex_none()
                .cursor_pointer()
                .active(|style| style.opacity(0.7))
                .on_click(move |_, _, cx| {
                    cx.stop_propagation();
                    store
                        .update(cx, |store, cx| {
                            store.open_watchtower(&workspace_id, Some(&task), cx)
                        })
                        .ok();
                })
                .child(chip)
                .into_any_element(),
        )
    }

    fn bar(
        &self,
        bar: &TimelineBar,
        hue: Option<u16>,
        top: f32,
        height: f32,
        sub: bool,
    ) -> AnyElement {
        let BarColors {
            fill,
            fill_opacity,
            border,
            text,
        } = bar_colors(bar.kind, hue, &self.t);
        let wide = bar.width >= LABEL_MIN_WIDTH;
        let store = self.store.clone();
        let agent_id = bar.agent_id.clone();
        div()
            .id(SharedString::from(format!(
                "board-timeline-bar-{}",
                bar.agent_id
            )))
            .role(Role::Button)
            .aria_label(bar_label(bar))
            .cursor_pointer()
            .absolute()
            .left(px(bar.x as f32))
            .top(self.s(top))
            .w(px(bar.width as f32))
            .h(self.s(height))
            .flex()
            .flex_row()
            .items_center()
            .gap(self.s(3.))
            .when(wide, |bar| bar.px(self.s(4.)))
            .overflow_hidden()
            .rounded(self.s(if sub { 3. } else { 4. }))
            .border_1()
            .when(bar.kind == TimelineKind::Cancelled, |bar| {
                bar.border_dashed()
            })
            .border_color(border)
            .bg(fill.opacity(fill_opacity))
            .active(|style| style.opacity(0.7))
            .on_click(move |_, _, cx| {
                store
                    .update(cx, |store, cx| store.open_agent(&agent_id, cx))
                    .ok();
            })
            .children(
                (bar.width >= LABEL_MIN_WIDTH / 2.)
                    .then(|| self.task_chip(bar))
                    .flatten(),
            )
            .when(!sub && wide, |el| {
                el.child(
                    div()
                        .min_w_0()
                        .flex_shrink(1.)
                        .truncate()
                        .text_size(self.s(10.))
                        .line_height(self.s(14.))
                        .font_features(tabular())
                        .font_weight(match bar.kind {
                            TimelineKind::Running | TimelineKind::Needs => FontWeight::SEMIBOLD,
                            _ => FontWeight::NORMAL,
                        })
                        .text_color(text)
                        .child(bar_text(bar)),
                )
            })
            .into_any_element()
    }

    fn thread_row(&self, row: &TimelineRow) -> AnyElement {
        let hue = self.hue(row);
        let mark = self.s(13.);
        let label = h_flex()
            .min_w_0()
            .items_center()
            .gap(self.s(6.))
            .child(
                div()
                    .flex_none()
                    .size(mark)
                    .rounded(self.s(4.))
                    .bg(repo_mark(hue, &self.t))
                    .flex()
                    .items_center()
                    .justify_center()
                    .text_size(self.s(8.5))
                    .font_weight(FontWeight::BOLD)
                    .text_color(self.t.surface1)
                    .child(project_initial(&row.project)),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_shrink(1.)
                    .truncate()
                    .text_size(self.s(11.5))
                    .line_height(self.s(16.))
                    .text_color(self.t.foreground)
                    .child(row.bar.title.clone()),
            );
        let lane = div()
            .relative()
            .h(self.s(ROW_HEIGHT))
            .border_t_1()
            .border_color(self.t.surface2)
            .children(self.breaks())
            .child(self.bar(&row.bar, hue, 5., 16., false))
            .child(self.now_line());
        div()
            .id(SharedString::from(format!(
                "board-timeline-row-{}",
                row.agent_id
            )))
            .role(Role::Row)
            .aria_label(row.bar.title.clone())
            .pt(self.s(2.))
            .child(self.row(label, lane))
            .into_any_element()
    }

    fn subagent_row(&self, row: &TimelineRow) -> AnyElement {
        let hue = self.hue(row);
        let text = subagent_text(row.subagents.len());
        let label = div()
            .pl(self.s(19.))
            .truncate()
            .text_size(self.s(10.5))
            .line_height(self.s(14.))
            .text_color(self.t.foreground_muted)
            .child(text.clone());
        let lane = div()
            .relative()
            .h(self.s(SUB_ROW_HEIGHT))
            .children(self.breaks())
            .children(
                row.subagents
                    .iter()
                    .map(|bar| self.bar(bar, hue, 3., 12., true)),
            )
            .child(self.now_line());
        div()
            .id(SharedString::from(format!(
                "board-timeline-subagents-{}",
                row.agent_id
            )))
            .role(Role::Row)
            .aria_label(format!("{}, {text}", row.bar.title))
            .pt(self.s(2.))
            .child(self.row(label, lane))
            .into_any_element()
    }

    fn legend(&self) -> Div {
        h_flex()
            .flex_wrap()
            .items_center()
            .gap(self.s(14.))
            .child(self.muted().text_size(self.s(12.)).child("Last 24 hours"))
            .child(div().flex_1())
            .children(LEGEND.into_iter().map(|(kind, label)| {
                let tone = bar_colors(kind, None, &self.t);
                h_flex()
                    .items_center()
                    .gap(self.s(5.))
                    .child(
                        div()
                            .w(self.s(14.))
                            .h(self.s(8.))
                            .rounded(self.s(2.))
                            .border_1()
                            .when(kind == TimelineKind::Cancelled, |swatch| {
                                swatch.border_dashed()
                            })
                            .border_color(tone.border)
                            .bg(tone.fill.opacity(tone.fill_opacity)),
                    )
                    .child(self.muted().child(label))
            }))
    }

    fn strip(&self) -> Div {
        let model = self.model;
        let drawable = (STRIP_HEIGHT - 16.) * self.scale;
        let lane = div()
            .id("board-timeline-load")
            .role(Role::Figure)
            .aria_label(peak_label(model.peak.as_ref()))
            .relative()
            .h(self.s(STRIP_HEIGHT))
            .border_b_1()
            .border_color(self.t.border)
            .children(self.breaks())
            .children(model.load.iter().map(|bucket| {
                div()
                    .absolute()
                    .bottom_0()
                    .left(px(bucket.x as f32 + 1.))
                    .w(px((bucket.width as f32 - 2.).max(1.)))
                    .h(px(load_height(bucket.count, model.peak.as_ref(), drawable)))
                    .rounded_t(self.s(2.))
                    .map(|bar| {
                        if bucket.running {
                            bar.bg(self.t.foreground)
                        } else {
                            bar.bg(self.t.status_success.opacity(0.35))
                        }
                    })
            }))
            .child(self.now_line())
            .children(model.peak.as_ref().map(|peak| {
                self.mono()
                    .absolute()
                    .left(self.s(2.))
                    .top_0()
                    .child(format!("peak {} · {}", peak.count, peak.label))
            }));
        self.row(self.muted().child("Agents at once"), lane)
    }

    fn axis(&self) -> Div {
        let lane = div()
            .relative()
            .h(self.s(AXIS_HEIGHT))
            .children(self.model.ticks.iter().map(|tick| {
                self.mono()
                    .absolute()
                    .top(self.s(4.))
                    .left(px(tick.x as f32) - self.s(18.))
                    .w(self.s(36.))
                    .text_center()
                    .child(tick.label.clone())
            }))
            .children(self.model.breaks.iter().enumerate().map(|(ix, gap)| {
                self.mono()
                    .id(("board-timeline-break", ix))
                    .role(Role::Note)
                    .aria_label(format!("Idle {}, not drawn to scale", gap.label))
                    .absolute()
                    .top(self.s(4.))
                    .left(px(gap.x as f32))
                    .w(px(gap.width as f32))
                    .text_center()
                    .text_color(self.t.foreground)
                    .child(gap.label.clone())
            }));
        self.row(div(), lane)
    }
}

impl BoardPage {
    /// Sends `board.timeline` when due, and wakes for the next poll or the next minute, only
    /// while the Timeline shows.
    fn drive_timeline(&mut self, store: &Entity<BoardStore>, now: i64, cx: &mut Context<Self>) {
        let online = store.read(cx).is_connected();
        let state = &mut self.timeline;
        if state.poll.due(now, TIMELINE_POLL_MS, true, online) {
            state.poll.begin(now);
            let ctx = store.read(cx).context();
            let answer = ctx.rpc("board.timeline", json!({}), cx);
            state._request = Some(cx.spawn(async move |page, cx| {
                let result = answer.await;
                page.update(cx, |page, cx| page.receive_timeline(result, cx))
                    .ok();
            }));
        }
        let state = &mut self.timeline;
        if state._wake.is_some() {
            return;
        }
        let poll = if online && !state.poll.in_flight() {
            state.poll.wait(now, TIMELINE_POLL_MS)
        } else {
            i64::MAX
        };
        let delay = poll.min(until_next_minute(now)).max(0) as u64;
        state._wake = Some(cx.spawn(async move |page, cx| {
            cx.background_executor()
                .timer(Duration::from_millis(delay))
                .await;
            page.update(cx, |page, cx| {
                page.timeline._wake = None;
                if page.view() == BoardView::Timeline {
                    cx.notify();
                }
            })
            .ok();
        }));
    }

    fn receive_timeline(
        &mut self,
        result: anyhow::Result<serde_json::Value>,
        cx: &mut Context<Self>,
    ) {
        let state = &mut self.timeline;
        state.poll.finish();
        match result.and_then(|value| decode_timeline(&value)) {
            Ok(answer) => {
                state.failed = false;
                if state.answer.as_ref() != Some(&answer) {
                    state.answer = Some(answer);
                    state.revision += 1;
                }
            }
            Err(error) => {
                log::warn!("board: board.timeline failed: {error:#}");
                state.failed = true;
            }
        }
        // The next wake waits for the new poll time.
        state._wake = None;
        cx.notify();
    }

    pub(crate) fn render_timeline(
        &mut self,
        store: &Entity<BoardStore>,
        scale: f32,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let s = |value: f32| px(value * scale);
        let t = bachuc_theme::tokens(cx);
        let now = now_ms();
        self.drive_timeline(store, now, cx);
        let (online, host) = {
            let store = store.read(cx);
            (store.is_connected(), store.host_label(cx))
        };
        let width = self.timeline.width.get();
        let chart = (width - (LABEL_WIDTH + LABEL_GAP) * scale).max(1.);
        self.timeline.sync(now, chart, scale);

        let measured = self.timeline.width.clone();
        let page = cx.weak_entity();
        // Records the view width; a change redraws, so the axis follows it.
        let measure = canvas(
            move |bounds, _, cx| {
                let value = f32::from(bounds.size.width);
                if measured.get() != value {
                    measured.set(value);
                    cx.defer(move |cx| {
                        page.update(cx, |_, cx| cx.notify()).ok();
                    });
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        let state = &self.timeline;
        let Some(model) = state.model.as_ref() else {
            return div().into_any_element();
        };
        let hues = store.read(cx).hues().clone();
        let chart = Chart {
            t,
            scale,
            model,
            hues: &hues,
            store: store.downgrade(),
            watchtower: watchtower_open(cx),
        };
        let rows = model.host_rows(HOST).len();
        let loaded = state.answer.is_some();
        let meta = host_meta(online, loaded, state.failed, rows);
        let notes = host_notes(online, state.answer.as_ref(), state.failed, rows);
        let pending = online && !loaded && !state.failed;
        let header = h_flex()
            .items_center()
            .gap(s(8.))
            .pt(s(10.))
            .child(
                div()
                    .id("board-timeline-host")
                    .role(Role::Heading)
                    .aria_label(host_name(&host).to_string())
                    .min_w_0()
                    .flex_shrink(1.)
                    .truncate()
                    .text_size(s(12.))
                    .font_weight(FontWeight::BOLD)
                    .text_color(t.foreground)
                    .child(host_name(&host).to_string()),
            )
            .child(
                chart
                    .muted()
                    .line_height(s(16.))
                    .text_color(if online {
                        t.foreground_muted
                    } else {
                        t.status_warning
                    })
                    .child(meta),
            )
            .when(pending, |header| {
                header.child(spinner(t.accent, bachuc_theme::zoomed(16., cx)))
            })
            .child(div().flex_1().h(px(1.)).bg(t.border));

        let body = v_flex()
            .flex_1()
            .min_h_0()
            .gap(s(2.))
            .child(header)
            .children(
                notes
                    .into_iter()
                    .map(|note| chart.muted().line_height(s(16.)).child(note)),
            )
            .child(
                list(
                    state.list.clone(),
                    cx.processor(|page: &mut BoardPage, ix, _window, cx| {
                        page.render_timeline_item(ix, cx)
                    }),
                )
                .flex_1()
                .min_h_0()
                .w_full(),
            );

        v_flex()
            .id("board-timeline")
            .relative()
            .flex_1()
            .min_h_0()
            .gap(s(6.))
            .child(measure)
            .child(chart.legend())
            .when(width > 0., |view| {
                view.child(chart.strip())
                    .child(chart.axis())
                    .child(body)
                    .child(
                        chart
                            .muted()
                            .flex_none()
                            .pt(s(8.))
                            .mt(s(6.))
                            .border_t_1()
                            .border_color(t.surface2)
                            .child(timeline_footer(model)),
                    )
            })
            .into_any_element()
    }

    fn render_timeline_item(&mut self, ix: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(store) = self.store.upgrade() else {
            return div().into_any_element();
        };
        let state = &self.timeline;
        let (Some(model), Some(item)) = (state.model.as_ref(), state.items.get(ix)) else {
            return div().into_any_element();
        };
        let board = store.read(cx);
        let chart = Chart {
            t: bachuc_theme::tokens(cx),
            scale: crate::model::settings::board_scale(board.size()) * bachuc_theme::zoom(cx),
            model,
            hues: board.hues(),
            store: store.downgrade(),
            watchtower: watchtower_open(cx),
        };
        let rows = model.host_rows(HOST);
        match *item {
            Item::Thread(row) => chart.thread_row(&rows[row]),
            Item::Subagents(row) => chart.subagent_row(&rows[row]),
        }
    }
}
