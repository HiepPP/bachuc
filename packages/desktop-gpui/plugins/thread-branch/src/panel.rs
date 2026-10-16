//! The Turn diff panel of `client/turn-diff-panel.tsx`. With no open file it lists the
//! agent's turns (`thread-branch.turn-history`, read again every `POLL`); with one it shows
//! that file's diff (`thread-branch.file-diff`) or, for an image, the images before and after
//! (`thread-branch.file-image`).
//!
//! Snapshot trees never change, so a diff or an image is read once and kept for the panel's
//! life.

use crate::card::{CardProps, OpenFile, render_card};
use crate::selection::Selection;
use crate::store::POLL;
use crate::turn::{
    CODE_SIZE, DiffRow, FileDiffReply, FileImageReply, FileRequest, MAX_ROWS, ParsedDiff,
    TurnEntry, TurnHistory, badge_label, decode_data_uri, gutter_width, image_type, max_line,
    parse_unified_diff, too_long, turn_time,
};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::PluginContext;
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_SM, RADIUS_BASE, RADIUS_MD, SPACE_0_5, SPACE_1, SPACE_2, SPACE_3,
    WEIGHT_MEDIUM, WEIGHT_SEMIBOLD,
};
use bachuc_theme::{Tokens, code_font_size, mono_font_family, tokens, ui_font_size};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::json;
use std::cell::Cell;
use std::collections::HashMap;
use std::ops::Range;
use std::rc::Rc;
use std::sync::Arc;

const DIFF_RPC: &str = "thread-branch.file-diff";
const IMAGE_RPC: &str = "thread-branch.file-image";
const HISTORY_RPC: &str = "thread-branch.turn-history";
/// `LINE_HEIGHT` of a diff row.
const LINE_HEIGHT: f32 = 20.;
/// The marker column of a diff row (`styles.marker`).
const MARKER_WIDTH: f32 = 16.;
/// The gutter's right padding (`styles.gutter`).
const GUTTER_PADDING: f32 = 8.;
/// `styles.historyContent` `gap`.
const HISTORY_GAP: f32 = 14.;
/// `styles.image` `height`, `styles.imageSide` `flexBasis`, and `styles.images` `gap`.
const IMAGE_HEIGHT: f32 = 360.;
const IMAGE_SIDE_WIDTH: f32 = 240.;
const IMAGE_GAP: f32 = 12.;
/// How many pixels of rows the wrapped list keeps drawn beyond the pane.
const LIST_OVERDRAW: f32 = 400.;
/// The Wrap switch icon and the badge text (`turn-diff-panel.tsx:100,260`).
const WRAP_ICON: f32 = 13.;
const BADGE_TEXT: f32 = 11.;
const BADGE_PADDING: f32 = 6.;
const SWITCH_PADDING_Y: f32 = 3.;
/// The row tints of the main diff view (`diff-colors.ts:4-5`): `rgba(46, 160, 67, 0.15)` and
/// `rgba(248, 81, 73, 0.1)`. No theme token has them.
const ADD_ROW_TINT: (u8, u8, u8, f32) = (46, 160, 67, 0.15);
const REMOVE_ROW_TINT: (u8, u8, u8, f32) = (248, 81, 73, 0.1);

fn row_tint((r, g, b, a): (u8, u8, u8, f32)) -> Hsla {
    Rgba {
        r: r as f32 / 255.,
        g: g as f32 / 255.,
        b: b as f32 / 255.,
        a,
    }
    .into()
}

enum Load<T> {
    Loading,
    Ready(T),
    Failed(String),
}

/// A parsed diff and what the list needs to size itself.
struct DiffView {
    parsed: ParsedDiff,
    truncated: bool,
    gutter: f32,
    /// The index of the longest shown row, which sets the width of the unwrapped list.
    widest: usize,
}

impl DiffView {
    fn new(reply: FileDiffReply) -> Self {
        let parsed = parse_unified_diff(&reply.diff);
        let gutter = gutter_width(max_line(&parsed.rows), CODE_SIZE);
        let widest = parsed
            .rows
            .iter()
            .take(MAX_ROWS)
            .enumerate()
            .max_by_key(|(_, row)| row.text().chars().count())
            .map_or(0, |(index, _)| index);
        Self {
            parsed,
            truncated: reply.truncated,
            gutter,
            widest,
        }
    }

    fn shown(&self) -> usize {
        self.parsed.rows.len().min(MAX_ROWS)
    }
}

struct ImageSides {
    before: Option<Arc<Image>>,
    after: Option<Arc<Image>>,
    too_large: bool,
}

/// An image from a decoded `data:` URI; `None` for a side with no file or an unknown type.
fn to_image(side: Option<(String, Vec<u8>)>) -> Option<Arc<Image>> {
    let (mime, bytes) = side?;
    Some(Arc::new(Image::from_bytes(
        ImageFormat::from_mime_type(&mime)?,
        bytes,
    )))
}

/// What a diff row needs from the theme, read once per draw: a list asks for each row alone,
/// and the mono family lookup lists the installed fonts.
struct Look {
    t: Tokens,
    mono: &'static str,
    size: Pixels,
}

pub(crate) struct TurnDiffPanel {
    ctx: PluginContext,
    selection: Selection,
    /// The Wrap switch; it starts on and keeps its last value for the session.
    wrap: Rc<Cell<bool>>,
    agent_id: String,
    turns: Option<Rc<Vec<TurnEntry>>>,
    turns_error: Option<String>,
    turns_busy: bool,
    diffs: HashMap<FileRequest, Load<Rc<DiffView>>>,
    images: HashMap<FileRequest, Load<Rc<ImageSides>>>,
    /// The wrapped rows have different heights, so they use the variable-height list.
    list: ListState,
    /// The file that `list` counts rows for.
    listed: Option<FileRequest>,
    scroll: UniformListScrollHandle,
    /// The mono family, looked up once per `render`.
    mono: &'static str,
    _history: Task<()>,
    _selection: Subscription,
}

impl TurnDiffPanel {
    pub(crate) fn new(
        ctx: PluginContext,
        selection: Selection,
        wrap: Rc<Cell<bool>>,
        agent_id: String,
        cx: &mut Context<Self>,
    ) -> Self {
        let history = cx.spawn(async move |this, cx| {
            loop {
                if this
                    .update(cx, |this, cx| this.refresh_history(cx))
                    .is_err()
                {
                    break;
                }
                cx.background_executor().timer(POLL).await;
            }
        });
        // A file that failed to load is read again the next time it opens.
        let subscription = selection.observe(cx, |this: &mut Self, cx| {
            this.diffs
                .retain(|_, load| !matches!(load, Load::Failed(_)));
            this.images
                .retain(|_, load| !matches!(load, Load::Failed(_)));
            cx.notify();
        });
        Self {
            ctx,
            selection,
            wrap,
            agent_id,
            turns: None,
            turns_error: None,
            turns_busy: false,
            diffs: HashMap::new(),
            images: HashMap::new(),
            list: ListState::new(0, ListAlignment::Top, px(LIST_OVERDRAW)),
            listed: None,
            scroll: UniformListScrollHandle::new(),
            mono: mono_font_family(cx),
            _history: history,
            _selection: subscription,
        }
    }

    /// Reads the agent's turns, unless a file shows or a read runs.
    fn refresh_history(&mut self, cx: &mut Context<Self>) {
        if self.turns_busy
            || self.agent_id.is_empty()
            || self.selection.for_agent(&self.agent_id).is_some()
        {
            return;
        }
        self.turns_busy = true;
        let answer = self.ctx.rpc_as::<_, TurnHistory>(
            HISTORY_RPC,
            &json!({ "agentId": self.agent_id }),
            cx,
        );
        cx.spawn(async move |this, cx| {
            let result = answer.await;
            this.update(cx, |this, cx| {
                this.turns_busy = false;
                match result {
                    Ok(history) => {
                        this.turns = Some(Rc::new(history.turns));
                        this.turns_error = None;
                    }
                    Err(error) => this.turns_error = Some(format!("{error:#}")),
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn ensure_diff(&mut self, file: &FileRequest, cx: &mut Context<Self>) {
        if self.diffs.contains_key(file) {
            return;
        }
        self.diffs.insert(file.clone(), Load::Loading);
        let answer = self.ctx.rpc_as::<_, FileDiffReply>(DIFF_RPC, file, cx);
        let key = file.clone();
        cx.spawn(async move |this, cx| {
            let result = answer.await;
            this.update(cx, |this, cx| {
                let load = match result {
                    Ok(reply) => Load::Ready(Rc::new(DiffView::new(reply))),
                    Err(error) => Load::Failed(format!("{error:#}")),
                };
                this.diffs.insert(key, load);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn ensure_image(&mut self, file: &FileRequest, cx: &mut Context<Self>) {
        if self.images.contains_key(file) {
            return;
        }
        self.images.insert(file.clone(), Load::Loading);
        let answer = self.ctx.rpc_as::<_, FileImageReply>(IMAGE_RPC, file, cx);
        let key = file.clone();
        cx.spawn(async move |this, cx| {
            let result = answer.await;
            // Decoding up to 3 MB of base64 stays off the main thread.
            let decoded = match result {
                Ok(reply) => Ok(cx
                    .background_spawn(async move {
                        (
                            reply.before.and_then(|uri| decode_data_uri(&uri)),
                            reply.after.and_then(|uri| decode_data_uri(&uri)),
                            reply.too_large,
                        )
                    })
                    .await),
                Err(error) => Err(format!("{error:#}")),
            };
            this.update(cx, |this, cx| {
                let load = match decoded {
                    Ok((before, after, too_large)) => Load::Ready(Rc::new(ImageSides {
                        before: to_image(before),
                        after: to_image(after),
                        too_large,
                    })),
                    Err(error) => Load::Failed(error),
                };
                this.images.insert(key, load);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// The rows of the open file, for the list callbacks.
    fn open_diff(&self) -> Option<Rc<DiffView>> {
        let file = self.selection.for_agent(&self.agent_id)?.file;
        match self.diffs.get(&file)? {
            Load::Ready(diff) => Some(diff.clone()),
            _ => None,
        }
    }

    fn look(&self, cx: &App) -> Look {
        Look {
            t: tokens(cx),
            mono: self.mono,
            size: code_font_size(CODE_SIZE, cx),
        }
    }

    fn render_wrapped(&mut self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let Some(diff) = self.open_diff() else {
            return div().into_any_element();
        };
        let look = self.look(cx);
        match diff.parsed.rows.get(index) {
            Some(row) => diff_row(row, diff.gutter, true, &look),
            None => div().into_any_element(),
        }
    }

    fn render_rows(&mut self, range: Range<usize>, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(diff) = self.open_diff() else {
            return Vec::new();
        };
        let look = self.look(cx);
        let end = range.end.min(diff.shown());
        diff.parsed.rows[range.start.min(end)..end]
            .iter()
            .map(|row| diff_row(row, diff.gutter, false, &look))
            .collect()
    }

    fn history_view(&mut self, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let size = ui_font_size(FONT_SM, cx);
        let open: OpenFile = {
            let selection = self.selection.clone();
            Rc::new(move |next, _workspace_id, _window, cx| selection.set(Some(next), cx))
        };
        let turns = self.turns.clone();
        let empty = turns.as_ref().is_some_and(|turns| turns.is_empty());
        let loading = turns.is_none() && self.turns_error.is_none();
        div()
            .id("thread-branch-turn-history")
            .role(Role::List)
            .aria_label("Changes by turn")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(
                v_flex()
                    .w_full()
                    .p(px(SPACE_3))
                    .gap(px(HISTORY_GAP))
                    .child(
                        div()
                            .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .text_color(t.foreground)
                            .child("Changes by turn"),
                    )
                    .when(loading, |view| {
                        view.child(div().text_color(t.foreground_muted).child("Loading…"))
                    })
                    .children(
                        self.turns_error
                            .clone()
                            .map(|error| div().text_color(t.status_danger).child(error)),
                    )
                    .when(empty, |view| {
                        view.child(
                            div().text_color(t.foreground_muted).child(
                                "No turn changes recorded for this agent in the last 30 days.",
                            ),
                        )
                    })
                    .children(turns.iter().flat_map(|turns| turns.iter()).map(|turn| {
                        let scope = format!("history/{}", turn.key);
                        v_flex()
                            .id(ElementId::Name(scope.clone().into()))
                            .role(Role::ListItem)
                            .aria_label(turn_time(&turn.ended_at))
                            .w_full()
                            .gap(px(SPACE_1 + SPACE_0_5))
                            .child(
                                div()
                                    .text_size(size)
                                    .text_color(t.foreground_muted)
                                    .child(turn_time(&turn.ended_at)),
                            )
                            .child(render_card(
                                CardProps {
                                    agent_id: &self.agent_id,
                                    data: &turn.diff,
                                    selected: None,
                                    scope: &scope,
                                    open: &open,
                                },
                                cx,
                            ))
                    })),
            )
            .into_any_element()
    }

    fn file_view(&mut self, file: FileRequest, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let is_image = image_type(&file.path).is_some();
        self.ensure_diff(&file, cx);
        if is_image {
            self.ensure_image(&file, cx);
        }
        let diff = match self.diffs.get(&file) {
            Some(Load::Ready(diff)) => Some(diff.clone()),
            _ => None,
        };
        let wrap = self.wrap.get();
        if let Some(diff) = &diff
            && self.listed.as_ref() != Some(&file)
        {
            self.list.reset(diff.shown());
            self.listed = Some(file.clone());
        }
        let rows = diff.as_ref().map_or(0, |diff| diff.shown());
        let message_size = ui_font_size(FONT_BASE, cx);
        let message = move |text: String, color: Hsla| {
            div()
                .p(px(SPACE_3))
                .text_size(message_size)
                .text_color(color)
                .child(text)
        };

        let mut body = v_flex().size_full().min_h_0();
        match self.diffs.get(&file) {
            Some(Load::Loading) | None => {
                body = body.child(message("Loading…".into(), t.foreground_muted));
            }
            Some(Load::Failed(error)) => {
                body = body.child(message(error.clone(), t.status_danger));
            }
            Some(Load::Ready(_)) => {}
        }
        if is_image {
            body = body.child(
                div()
                    .id("thread-branch-image-scroll")
                    .flex_1()
                    .min_h_0()
                    .overflow_y_scroll()
                    .child(self.image_preview(&file, cx)),
            );
        } else if diff.as_ref().is_some_and(|diff| diff.parsed.binary) {
            body = body.child(message(
                "Binary file. No line diff to show.".into(),
                t.foreground_muted,
            ));
        }
        if let Some(diff) = &diff
            && !diff.parsed.binary
            && diff.parsed.rows.is_empty()
        {
            body = body.child(message(
                "No changes to show for this file.".into(),
                t.foreground_muted,
            ));
        }
        if let Some(diff) = diff.as_ref().filter(|_| rows > 0) {
            let lines = if wrap {
                div()
                    .id("thread-branch-diff-lines")
                    .role(Role::List)
                    .aria_label("Diff lines")
                    .flex_1()
                    .min_h_0()
                    .child(
                        list(
                            self.list.clone(),
                            cx.processor(|this, index, _window, cx| this.render_wrapped(index, cx)),
                        )
                        .size_full()
                        .py(px(SPACE_1)),
                    )
                    .into_any_element()
            } else {
                div()
                    .id("thread-branch-diff-lines")
                    .role(Role::List)
                    .aria_label("Diff lines")
                    .flex_1()
                    .min_h_0()
                    .child(
                        uniform_list(
                            "thread-branch-diff-rows",
                            rows,
                            cx.processor(|this, range, _window, cx| this.render_rows(range, cx)),
                        )
                        .track_scroll(&self.scroll)
                        .with_horizontal_sizing_behavior(
                            ListHorizontalSizingBehavior::Unconstrained,
                        )
                        .with_width_from_item(Some(diff.widest))
                        .size_full()
                        .py(px(SPACE_1)),
                    )
                    .into_any_element()
            };
            body = body.child(lines);
            if too_long(diff.parsed.rows.len(), diff.truncated) {
                body = body.child(message(
                    "The diff is too long; only the start is shown.".into(),
                    t.foreground_muted,
                ));
            }
        }

        let header = self.file_header(&file, diff.as_deref(), rows > 0, wrap, cx);
        v_flex()
            .flex_1()
            .min_h_0()
            .child(header)
            .child(body)
            .into_any_element()
    }

    fn file_header(
        &self,
        file: &FileRequest,
        diff: Option<&DiffView>,
        has_rows: bool,
        wrap: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let mono = self.mono;
        let code = code_font_size(CODE_SIZE, cx);
        let small = ui_font_size(FONT_SM, cx);
        let badge_size = ui_font_size(BADGE_TEXT, cx);
        let parsed = diff.map(|diff| &diff.parsed);
        let badge = parsed.and_then(badge_label);
        let counts = parsed.filter(|parsed| !parsed.binary);

        let back = h_flex()
            .id("thread-branch-back")
            .role(Role::Button)
            .aria_label("Show all turns")
            .flex_none()
            .items_center()
            .gap(px(SPACE_0_5))
            .pr(px(SPACE_1))
            .cursor_pointer()
            .on_click(cx.listener(|this, _, _window, cx| {
                let selection = this.selection.clone();
                selection.set(None, cx);
            }))
            .child(
                icon(IconName::ChevronLeft)
                    .with_size(px(ICON_SM))
                    .text_color(t.accent),
            )
            .child(
                div()
                    .text_size(small)
                    .font_weight(FontWeight(WEIGHT_MEDIUM))
                    .text_color(t.accent)
                    .child("All turns"),
            );

        let switch = has_rows.then(|| {
            h_flex()
                .id("thread-branch-wrap")
                .role(Role::Switch)
                .aria_label("Wrap long lines")
                .aria_toggled(if wrap { Toggled::True } else { Toggled::False })
                .flex_none()
                .items_center()
                .gap(px(SPACE_1))
                .px(px(SPACE_2))
                .py(px(SWITCH_PADDING_Y))
                .rounded(px(RADIUS_MD))
                .border_1()
                .border_color(t.border)
                .cursor_pointer()
                .when(wrap, |switch| switch.bg(t.surface2))
                .on_click(cx.listener(|this, _, _window, cx| {
                    this.wrap.set(!this.wrap.get());
                    cx.notify();
                }))
                .child(
                    icon(IconName::WrapText)
                        .with_size(px(WRAP_ICON))
                        .text_color(if wrap { t.accent } else { t.foreground_muted }),
                )
                .child(
                    div()
                        .text_size(small)
                        .text_color(if wrap {
                            t.foreground
                        } else {
                            t.foreground_muted
                        })
                        .child("Wrap"),
                )
        });

        h_flex()
            .w_full()
            .flex_none()
            .items_center()
            .gap(px(SPACE_2))
            .px(px(SPACE_3))
            .py(px(SPACE_2))
            .border_b_1()
            .border_color(t.border)
            .bg(t.surface1)
            .child(back)
            .child(
                div()
                    .flex_shrink(1.)
                    .min_w_0()
                    .truncate()
                    .font_family(mono)
                    .text_size(code)
                    .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                    .text_color(t.foreground)
                    .child(file.path.clone()),
            )
            .children(badge.map(|badge| {
                div()
                    .flex_none()
                    .text_size(badge_size)
                    .text_color(t.foreground_muted)
                    .border_1()
                    .border_color(t.border)
                    .rounded(px(RADIUS_BASE))
                    .px(px(BADGE_PADDING))
                    .child(badge)
            }))
            .children(counts.map(|parsed| {
                h_flex()
                    .flex_none()
                    .gap(px(SPACE_2))
                    .font_family(mono)
                    .text_size(code)
                    .child(
                        div()
                            .text_color(t.diff_addition)
                            .child(format!("+{}", parsed.added)),
                    )
                    .child(
                        div()
                            .text_color(t.diff_deletion)
                            .child(format!("-{}", parsed.removed)),
                    )
            }))
            .child(div().flex_1())
            .children(switch)
            .into_any_element()
    }

    fn image_preview(&self, file: &FileRequest, cx: &App) -> AnyElement {
        let t = tokens(cx);
        let size = ui_font_size(FONT_BASE, cx);
        let note = |text: &str, color: Hsla| {
            div()
                .p(px(SPACE_3))
                .text_size(size)
                .text_color(color)
                .child(text.to_string())
                .into_any_element()
        };
        let sides = match self.images.get(file) {
            None | Some(Load::Loading) => return note("Loading image…", t.foreground_muted),
            Some(Load::Failed(error)) => return note(error, t.status_danger),
            Some(Load::Ready(sides)) => sides.clone(),
        };
        let shown: Vec<(&'static str, Arc<Image>)> =
            [("Before", &sides.before), ("After", &sides.after)]
                .into_iter()
                .filter_map(|(label, image)| Some((label, image.clone()?)))
                .collect();
        let labeled = shown.len() > 1;
        let label_size = ui_font_size(BADGE_TEXT, cx);
        h_flex()
            .w_full()
            .flex_wrap()
            .gap(px(IMAGE_GAP))
            .p(px(SPACE_3))
            .children(shown.into_iter().map(|(label, image)| {
                v_flex()
                    .flex_1()
                    .min_w(px(IMAGE_SIDE_WIDTH))
                    .gap(px(SPACE_1 + SPACE_0_5))
                    .when(labeled, |side| {
                        side.child(
                            div()
                                .text_size(label_size)
                                .text_color(t.foreground_muted)
                                .child(label),
                        )
                    })
                    .child(
                        div()
                            .id(label)
                            .role(Role::Image)
                            .aria_label(format!("{label}: {}", file.path))
                            .p(px(SPACE_2))
                            .border_1()
                            .border_color(t.border)
                            .rounded(px(RADIUS_MD))
                            .bg(t.surface1)
                            .child(
                                img(image)
                                    .w_full()
                                    .h(px(IMAGE_HEIGHT))
                                    .object_fit(ObjectFit::Contain),
                            ),
                    )
            }))
            .children(sides.too_large.then(|| {
                div()
                    .w_full()
                    .text_size(size)
                    .text_color(t.foreground_muted)
                    .child("An image over 3 MB is not previewed.")
            }))
            .into_any_element()
    }
}

/// One diff row: the two line numbers, the marker, and the text, or a hunk or note line.
fn diff_row(row: &DiffRow, gutter: f32, wrap: bool, look: &Look) -> AnyElement {
    let t = look.t;
    let (mono, code_size) = (look.mono, look.size);
    let line = || {
        h_flex()
            .w_full()
            .items_start()
            .font_family(mono)
            .text_size(code_size)
            .line_height(px(LINE_HEIGHT))
            .min_h(px(LINE_HEIGHT))
    };
    let code = |text: &str, color: Hsla| {
        let text = if text.is_empty() { " " } else { text };
        let cell = div().text_color(color);
        if wrap {
            cell.flex_1()
                .min_w_0()
                .pr(px(SPACE_3))
                .whitespace_normal()
                .child(text.to_string())
        } else {
            cell.flex_none()
                .pr(px(SPACE_3 + SPACE_1))
                .whitespace_nowrap()
                .child(text.to_string())
        }
    };
    let number = |line: Option<u32>| {
        div()
            .flex_none()
            .w(px(gutter))
            .pr(px(GUTTER_PADDING))
            .text_right()
            .text_color(t.foreground_muted)
            .child(line.map(|line| line.to_string()).unwrap_or_default())
    };
    let marker = |text: &'static str, color: Hsla| {
        div()
            .flex_none()
            .w(px(MARKER_WIDTH))
            .text_center()
            .text_color(color)
            .child(text)
    };
    match row {
        DiffRow::Hunk(text) => line()
            .bg(t.surface1)
            .child(div().flex_none().w(px(gutter * 2. + MARKER_WIDTH)))
            .child(code(text, t.foreground_muted))
            .into_any_element(),
        DiffRow::Note(text) => line()
            .child(div().flex_none().w(px(gutter * 2. + MARKER_WIDTH)))
            .child(code(text, t.foreground_muted))
            .into_any_element(),
        DiffRow::Add { text, new_line } => line()
            .bg(row_tint(ADD_ROW_TINT))
            .child(number(None))
            .child(number(Some(*new_line)))
            .child(marker("+", t.diff_addition))
            .child(code(text, t.foreground))
            .into_any_element(),
        DiffRow::Remove { text, old_line } => line()
            .bg(row_tint(REMOVE_ROW_TINT))
            .child(number(Some(*old_line)))
            .child(number(None))
            .child(marker("-", t.diff_deletion))
            .child(code(text, t.foreground))
            .into_any_element(),
        DiffRow::Context {
            text,
            old_line,
            new_line,
        } => line()
            .child(number(Some(*old_line)))
            .child(number(Some(*new_line)))
            .child(marker(" ", t.foreground))
            .child(code(text, t.foreground))
            .into_any_element(),
    }
}

impl Render for TurnDiffPanel {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        self.mono = mono_font_family(cx);
        let body = match self.selection.for_agent(&self.agent_id) {
            Some(selection) => self.file_view(selection.file, cx),
            None => self.history_view(cx),
        };
        v_flex()
            .id("thread-branch-turn-diff-panel")
            .role(Role::Group)
            .aria_label("Turn diff")
            .size_full()
            .bg(t.surface0)
            .child(body)
    }
}
