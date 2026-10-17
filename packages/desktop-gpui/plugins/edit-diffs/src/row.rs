//! The `edit-diff` row, as `EditDiffRow` of `client/edit-row.tsx`: a header button that opens
//! and closes the diff, then the diff body.

use crate::diff::{
    Hunk, RowKind, Square, apply_locations, build_hunks, change_bar, count_changes, number_rows,
};
use crate::edit_item::{EditRowData, Status, display_path, header_label, shown_status};
use crate::highlight::{Run, highlight_hunks};
use crate::state::{LocateRequest, LocateResponse, LocateStep, Locator, is_expanded};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{PluginContext, PluginSettings, TimelineRenderInput};
use bachuc_theme::metrics::{FONT_BASE, FONT_CODE, ICON_SM, RADIUS_LG, SPACE_2, SPACE_3};
use bachuc_theme::{
    SyntaxColors, code_font_size, mono_font_family, syntax_colors, tokens, ui_font_size,
};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::{Rc, Weak};
use std::time::Instant;

const BAR_SQUARE: f32 = 7.;
const BAR_GAP: f32 = 2.;
const BAR_RADIUS: f32 = 1.5;
const PATH_SIZE: f32 = 12.5;
const STAT_SIZE: f32 = 13.;
const MAX_DIFF_HEIGHT: f32 = 480.;
const BODY_GAP: f32 = 6.;
const LINE_HEIGHT: f32 = 18.;
/// The add and remove tints of `edit-row.tsx`, over the status tokens.
const ADD_TINT: f32 = 0.15;
const REMOVE_TINT: f32 = 0.1;
const UNAVAILABLE: &str = "Plugin timeline item unavailable.";

/// What a row keeps across the views the timeline builds for its call: the press override and
/// the locate answers. The timeline builds a new view when the item data changes, so this state
/// lives by call while any of its views lives.
#[derive(Default)]
pub(crate) struct RowMemory {
    toggled: Option<bool>,
    locator: Option<Locator>,
}

/// The renderer's state shared by every row.
pub(crate) struct Rows {
    pub ctx: PluginContext,
    pub settings: Entity<PluginSettings>,
    memories: RefCell<HashMap<String, Weak<RefCell<RowMemory>>>>,
}

impl Rows {
    pub(crate) fn new(ctx: PluginContext, settings: Entity<PluginSettings>) -> Self {
        Self {
            ctx,
            settings,
            memories: RefCell::default(),
        }
    }

    /// The memory of one call; an item without an id gets its own.
    fn memory(&self, key: Option<String>) -> Rc<RefCell<RowMemory>> {
        let Some(key) = key else {
            return Rc::default();
        };
        let mut memories = self.memories.borrow_mut();
        memories.retain(|_, memory| memory.strong_count() > 0);
        if let Some(memory) = memories.get(&key).and_then(Weak::upgrade) {
            return memory;
        }
        let memory: Rc<RefCell<RowMemory>> = Rc::default();
        memories.insert(key, Rc::downgrade(&memory));
        memory
    }
}

/// The renderer's view factory.
pub(crate) fn build(rows: &Rc<Rows>, input: TimelineRenderInput, cx: &mut App) -> AnyView {
    let key = input
        .item
        .id
        .as_ref()
        .map(|id| format!("{}/{id}", input.agent_id));
    let memory = rows.memory(key);
    let rows = rows.clone();
    cx.new(|cx| EditRowView::new(rows, input, memory, cx))
        .into()
}

/// One line of the diff body.
#[derive(Clone, Debug, PartialEq)]
enum Line {
    /// The vertical ellipsis between two hunks.
    Gap,
    Row {
        kind: RowKind,
        text: String,
        number: Option<i64>,
        hunk: usize,
        row: usize,
    },
}

/// The numbered lines of the hunks, with the gutter width in characters.
fn body_lines(hunks: &[Hunk]) -> (Vec<Line>, usize) {
    let mut lines = Vec::new();
    for (hunk_index, hunk) in hunks.iter().enumerate() {
        if hunk_index > 0 {
            lines.push(Line::Gap);
        }
        for (row, numbered) in number_rows(hunk).into_iter().enumerate() {
            lines.push(Line::Row {
                kind: numbered.kind,
                text: numbered.text,
                number: numbered.line,
                hunk: hunk_index,
                row,
            });
        }
    }
    let gutter = lines
        .iter()
        .filter_map(|line| match line {
            Line::Row {
                number: Some(n), ..
            } => Some(n.to_string().len()),
            _ => None,
        })
        .max()
        .unwrap_or(0)
        .max(1);
    (lines, gutter)
}

/// The decoded row and everything built from it.
struct Edit {
    data: EditRowData,
    cwd: Option<String>,
    hunks: Vec<Hunk>,
    added: usize,
    removed: usize,
    bar: Vec<Square>,
    lines: Rc<Vec<Line>>,
    gutter: usize,
    /// The index of the longest line, which sets the body width.
    widest: usize,
}

pub(crate) struct EditRowView {
    rows: Rc<Rows>,
    edit: Option<Edit>,
    memory: Rc<RefCell<RowMemory>>,
    /// The runs per hunk and row, by the palette they were made with. A view's hunks never
    /// change: new data gets a new view.
    runs: Option<(SyntaxColors, Rc<Vec<Vec<Vec<Run>>>>)>,
    highlighting: Option<(SyntaxColors, Task<()>)>,
    scroll: UniformListScrollHandle,
    locate_timer: Option<Task<()>>,
    _settings: Subscription,
}

impl EditRowView {
    fn new(
        rows: Rc<Rows>,
        input: TimelineRenderInput,
        memory: Rc<RefCell<RowMemory>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let settings = cx.observe(&rows.settings, |this, _, cx| {
            this.poll_locate(cx);
            cx.notify();
        });
        let cwd = rows.ctx.client(cx).and_then(|client| {
            let client = client.read(cx);
            client
                .agents()
                .iter()
                .find(|agent| agent.id == input.agent_id)
                .map(|agent| agent.cwd.clone())
        });
        let mut view = Self {
            rows,
            edit: None,
            memory,
            runs: None,
            highlighting: None,
            scroll: UniformListScrollHandle::new(),
            locate_timer: None,
            _settings: settings,
        };
        view.edit = EditRowData::decode(&input.item.data).map(|data| {
            let hunks = build_hunks(&data.source());
            let (added, removed) = count_changes(&hunks);
            let request = LocateRequest::new(&data.file_path, cwd.as_deref(), &hunks);
            let mut memory = view.memory.borrow_mut();
            match memory.locator.as_mut() {
                Some(locator) => locator.set_request(request, Instant::now()),
                None => memory.locator = Some(Locator::new(request, Instant::now())),
            }
            Edit {
                data,
                cwd,
                hunks,
                added,
                removed,
                bar: change_bar(added, removed),
                lines: Rc::default(),
                gutter: 1,
                widest: 0,
            }
        });
        view.rebuild_lines();
        view.poll_locate(cx);
        view
    }

    fn expanded(&self, cx: &App) -> bool {
        is_expanded(
            self.memory.borrow().toggled,
            self.rows.settings.read(cx).state(),
        )
    }

    /// Numbers the lines again, with the locate answer when one is kept.
    fn rebuild_lines(&mut self) {
        let memory = self.memory.borrow();
        let Some(edit) = self.edit.as_mut() else {
            return;
        };
        let located = memory
            .locator
            .as_ref()
            .and_then(|locator| locator.located(edit.data.status));
        let hunks = match located {
            Some(located) => apply_locations(&edit.hunks, located),
            None => edit.hunks.clone(),
        };
        let (lines, gutter) = body_lines(&hunks);
        edit.widest = lines
            .iter()
            .enumerate()
            .max_by_key(|(_, line)| match line {
                Line::Row { text, .. } => text.chars().count(),
                Line::Gap => 0,
            })
            .map_or(0, |(index, _)| index);
        edit.lines = Rc::new(lines);
        edit.gutter = gutter;
    }

    fn toggle(&mut self, cx: &mut Context<Self>) {
        let expanded = self.expanded(cx);
        self.memory.borrow_mut().toggled = Some(!expanded);
        self.poll_locate(cx);
        cx.notify();
    }

    /// Sends the locate request once it settled, or waits for it to settle.
    fn poll_locate(&mut self, cx: &mut Context<Self>) {
        let expanded = self.expanded(cx);
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let status = edit.data.status;
        let step = match self.memory.borrow_mut().locator.as_mut() {
            Some(locator) => locator.poll(status, expanded, &edit.hunks, Instant::now()),
            None => LocateStep::Idle,
        };
        match step {
            LocateStep::Idle => self.locate_timer = None,
            LocateStep::Wait(delay) => {
                self.locate_timer = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(delay).await;
                    this.update(cx, |this, cx| this.poll_locate(cx)).ok();
                }));
            }
            LocateStep::Send(key) => {
                self.locate_timer = None;
                let answer =
                    self.rows
                        .ctx
                        .rpc_as::<_, LocateResponse>("edit-diffs.locate", &key.1, cx);
                // The memory outlives this view, so the answer is kept even when the timeline
                // builds a new view meanwhile.
                let memory = self.memory.clone();
                cx.spawn(async move |this, cx| {
                    let kept = match answer.await {
                        Ok(response) => memory
                            .borrow_mut()
                            .locator
                            .as_mut()
                            .is_some_and(|locator| locator.answer(key, response.located())),
                        Err(error) => {
                            log::warn!("edit-diffs.locate failed: {error:#}");
                            false
                        }
                    };
                    if kept {
                        this.update(cx, |this, cx| {
                            this.rebuild_lines();
                            cx.notify();
                        })
                        .ok();
                    }
                })
                .detach();
            }
        }
    }

    /// Colors the hunks off the main thread, once per hunk text and syntax palette.
    fn ensure_highlight(&mut self, cx: &mut Context<Self>) {
        let Some(edit) = self.edit.as_ref() else {
            return;
        };
        let colors = syntax_colors(cx);
        if self.runs.as_ref().is_some_and(|(at, _)| *at == colors)
            || self
                .highlighting
                .as_ref()
                .is_some_and(|(at, _)| *at == colors)
        {
            return;
        }
        let hunks = edit.hunks.clone();
        let path = edit.data.file_path.clone();
        let task = cx.spawn(async move |this, cx| {
            let runs = cx
                .background_spawn(async move { highlight_hunks(&hunks, &path, colors) })
                .await;
            this.update(cx, |this, cx| {
                this.highlighting = None;
                if let Some(runs) = runs {
                    this.runs = Some((colors, Rc::new(runs)));
                    cx.notify();
                }
            })
            .ok();
        });
        self.highlighting = Some((colors, task));
    }

    fn render_lines(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(edit) = self.edit.as_ref() else {
            return Vec::new();
        };
        let t = tokens(cx);
        let mono = mono_font_family(cx);
        let code = code_font_size(FONT_CODE, cx);
        let width = edit.gutter;
        // Runs of an older palette still fit the text until the new ones are made.
        let runs = self.runs.as_ref().map(|(_, runs)| runs.clone());
        let line_box = || {
            h_flex()
                .w_full()
                .h(px(LINE_HEIGHT))
                .font_family(mono)
                .text_size(code)
                .line_height(px(LINE_HEIGHT))
                .whitespace_nowrap()
        };
        edit.lines[range]
            .iter()
            .map(|line| match line {
                Line::Gap => line_box()
                    .pl(px(SPACE_2))
                    .text_color(t.foreground_muted)
                    .child(format!("{:>width$}", "⋮"))
                    .into_any_element(),
                Line::Row {
                    kind,
                    text,
                    number,
                    hunk,
                    row,
                } => {
                    let (prefix, prefix_color, bg) = match kind {
                        RowKind::Add => {
                            ("+ ", t.status_success, t.status_success.opacity(ADD_TINT))
                        }
                        RowKind::Remove => {
                            ("- ", t.status_danger, t.status_danger.opacity(REMOVE_TINT))
                        }
                        RowKind::Context => ("  ", t.foreground_muted, transparent_black()),
                    };
                    let color = if *kind == RowKind::Context {
                        t.foreground_muted
                    } else {
                        t.foreground
                    };
                    let line_runs = runs
                        .as_ref()
                        .and_then(|runs| runs.get(*hunk)?.get(*row).cloned())
                        .unwrap_or_default();
                    line_box()
                        .pr(px(SPACE_3))
                        .bg(bg)
                        .child(
                            div()
                                .flex_none()
                                .px(px(SPACE_2))
                                .text_color(t.foreground_muted)
                                .child(format!(
                                    "{:>width$}",
                                    number.map_or(String::new(), |n| n.to_string())
                                )),
                        )
                        .child(div().flex_none().text_color(prefix_color).child(prefix))
                        .child(
                            div()
                                .flex_none()
                                .text_color(color)
                                .child(styled_line(text, line_runs)),
                        )
                        .into_any_element()
                }
            })
            .collect()
    }
}

/// One code line with its colored runs.
fn styled_line(line: &str, runs: Vec<Run>) -> StyledText {
    let text = if line.is_empty() { " " } else { line };
    let runs: Vec<Run> = runs
        .into_iter()
        .filter(|(range, _)| range.end <= text.len())
        .collect();
    StyledText::new(text.to_string()).with_highlights(runs)
}

impl Render for EditRowView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let expanded = self.expanded(cx);
        if expanded {
            self.ensure_highlight(cx);
        }
        let Some(edit) = self.edit.as_ref() else {
            return div()
                .id("edit-diff-unavailable")
                .role(Role::Note)
                .aria_label(UNAVAILABLE)
                .py(px(SPACE_2))
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(t.foreground_muted)
                .child(UNAVAILABLE)
                .into_any_element();
        };
        let data = &edit.data;
        let failed = data.status == Status::Failed;
        let muted = t.foreground_muted;
        let stat = ui_font_size(STAT_SIZE, cx);
        let mono = mono_font_family(cx);
        let (added, removed) = (edit.added, edit.removed);

        let header =
            h_flex()
                .id("edit-diff-header")
                .role(Role::Button)
                .aria_label(format!(
                    "{} {}",
                    header_label(data.action, data.status),
                    data.file_path
                ))
                .aria_expanded(expanded)
                .flex_shrink(1.)
                .min_w_0()
                .max_w_full()
                .gap(px(SPACE_2))
                .items_center()
                .cursor_pointer()
                .on_click(cx.listener(|this, _, _, cx| this.toggle(cx)))
                .child(
                    div()
                        .w(px(ICON_SM))
                        .flex_none()
                        .flex()
                        .justify_center()
                        .child(
                            icon(if expanded {
                                IconName::ChevronDown
                            } else {
                                IconName::ChevronRight
                            })
                            .with_size(px(ICON_SM))
                            .text_color(muted),
                        ),
                )
                .child(
                    div()
                        .flex_shrink(1.)
                        .min_w_0()
                        .truncate()
                        .font_family(mono)
                        .text_size(code_font_size(PATH_SIZE, cx))
                        .text_color(if failed {
                            t.status_danger
                        } else {
                            t.foreground
                        })
                        .child(display_path(&data.file_path, edit.cwd.as_deref())),
                )
                .when(!edit.bar.is_empty(), |header| {
                    header.child(h_flex().flex_none().gap(px(BAR_GAP)).children(
                        edit.bar.iter().map(|square| {
                            div()
                                .size(px(BAR_SQUARE))
                                .rounded(px(BAR_RADIUS))
                                .bg(match square {
                                    Square::Add => t.status_success,
                                    Square::Remove => t.status_danger,
                                })
                        }),
                    ))
                })
                .when(added > 0, |header| {
                    header.child(
                        div()
                            .flex_none()
                            .text_size(stat)
                            .text_color(t.status_success)
                            .child(format!("+{added}")),
                    )
                })
                .when(removed > 0, |header| {
                    header.child(
                        div()
                            .flex_none()
                            .text_size(stat)
                            .text_color(t.status_danger)
                            .child(format!("−{removed}")),
                    )
                })
                .children(shown_status(data.action, data.status).map(|status| {
                    div()
                        .flex_none()
                        .text_size(stat)
                        .text_color(if failed { t.status_danger } else { muted })
                        .child(status)
                }));

        let body = expanded.then(|| {
            let content = if edit.lines.is_empty() {
                div()
                    .p(px(SPACE_2))
                    .text_size(stat)
                    .text_color(muted)
                    .child("No changes to show.")
                    .into_any_element()
            } else {
                uniform_list(
                    "edit-diff-lines",
                    edit.lines.len(),
                    cx.processor(|this, range, _window, cx| this.render_lines(range, cx)),
                )
                .track_scroll(&self.scroll)
                .with_horizontal_sizing_behavior(ListHorizontalSizingBehavior::Unconstrained)
                .with_width_from_item(Some(edit.widest))
                .w_full()
                .max_h(px(MAX_DIFF_HEIGHT))
                .py(px(BODY_GAP))
                .into_any_element()
            };
            v_flex()
                .id("edit-diff-body")
                .role(Role::Group)
                .aria_label(format!(
                    "Diff of {}, {added} added, {removed} removed",
                    data.file_path
                ))
                .w_full()
                .mt(px(BODY_GAP))
                .border_1()
                .border_color(t.border)
                .rounded(px(RADIUS_LG))
                .overflow_hidden()
                .bg(t.surface1)
                .children(data.error.as_ref().map(|error| {
                    div()
                        .p(px(SPACE_2))
                        .text_size(stat)
                        .text_color(t.status_danger)
                        .child(error.clone())
                }))
                .child(content)
        });

        v_flex()
            .id("edit-diff")
            .w_full()
            .child(h_flex().w_full().child(header))
            .children(body)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob import brings a `test` attribute of its own.
    use super::{Line, body_lines};
    use crate::diff::parse_unified_diff;

    #[test]
    fn body_lines_number_rows_and_put_an_ellipsis_between_hunks() {
        let hunks = parse_unified_diff("@@ -9,1 +9,2 @@\n-a\n+b\n+c\n@@ -120 +121 @@\n-d\n+e\n");
        let (lines, gutter) = body_lines(&hunks);
        assert_eq!(lines.len(), 6);
        assert_eq!(lines[3], Line::Gap);
        let numbers: Vec<Option<i64>> = lines
            .iter()
            .filter_map(|line| match line {
                Line::Row { number, .. } => Some(*number),
                Line::Gap => None,
            })
            .collect();
        assert_eq!(
            numbers,
            vec![Some(9), Some(9), Some(10), Some(120), Some(121)]
        );
        assert_eq!(gutter, 3);
        // Unlocated rows keep a one-character gutter.
        assert_eq!(body_lines(&parse_unified_diff("@@\n-a\n+b\n")).1, 1);
    }
}
