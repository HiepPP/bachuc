//! The diff viewer pane: the uncommitted diff of one file as a unified diff, as the Electron
//! diff document draws it (`git/diff-document/paint.web.ts`): a file header, then numbered lines
//! over the addition and deletion tints.

use crate::code::{code_font, code_text, gutter_width};
use crate::diff::{DiffLine, LineKind, NoDiff, ParsedDiff, max_number, no_diff_reason, parse};
use crate::syntax::{Run, SyntaxColors, highlight, language_for};
use crate::ui::{Change, centered_text, change_icon, diff_stat};
use bachuc_client::{Client, DirectoryEvent};
use bachuc_theme::metrics::{FONT_BASE, SPACE_1, SPACE_2, SPACE_3};
use bachuc_theme::{scheme, tokens, ui_font_size};
use gpui_kit::component::h_flex;
use gpui_kit::component::theme::Theme;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::ops::Range;

/// The height of one diff line.
pub const LINE_HEIGHT: f32 = 22.;
/// `DIFF_FILE_HEADER_HEIGHT` and the left and right insets of its content.
const HEADER_HEIGHT: f32 = 30.;
const HEADER_LEFT: f32 = 12.;
const HEADER_RIGHT: f32 = 8.;
/// `CODE_LEFT_PADDING`, and the space between a number and the gutter edge.
const CODE_LEFT_PADDING: f32 = 8.;
const NUMBER_RIGHT: f32 = 7.;
/// The addition and deletion tints of `createDiffPalette`.
const ADDITION_TINT: f32 = 0.15;
const DELETION_TINT: f32 = 0.1;
/// A diff over this many bytes of text shows in the plain color.
const HIGHLIGHT_LIMIT: usize = 512 * 1024;

struct Doc {
    parsed: ParsedDiff,
    /// Colored ranges per entry of `parsed.lines`; empty until the highlighter finishes.
    runs: Vec<Vec<Run>>,
    longest: usize,
    gutter: f32,
    scheme: bachuc_theme::Scheme,
}

enum State {
    Loading,
    Failed(String),
    /// The daemon says the diff is too large to preview.
    TooLarge,
    Binary,
    /// The file has no uncommitted change.
    Empty,
    Ready(Doc),
}

pub struct DiffView {
    client: Entity<Client>,
    cwd: String,
    path: String,
    state: State,
    scroll: UniformListScrollHandle,
    /// Bumped by each load, so a slow answer for an older load is dropped.
    load: u64,
    _subscriptions: Vec<Subscription>,
}

impl DiffView {
    pub fn new(
        client: Entity<Client>,
        cwd: String,
        path: String,
        _window: &mut Window,
        cx: &mut Context<DiffView>,
    ) -> DiffView {
        let subscriptions = vec![
            cx.subscribe(
                &client,
                |this: &mut DiffView, _, event: &DirectoryEvent, cx| {
                    if let DirectoryEvent::CheckoutChanged { cwd } = event
                        && *cwd == this.cwd
                    {
                        this.reload(cx);
                    }
                },
            ),
            cx.observe_global::<Theme>(|this, cx| this.theme_changed(cx)),
        ];
        let mut view = DiffView {
            client,
            cwd,
            path,
            state: State::Loading,
            scroll: UniformListScrollHandle::new(),
            load: 0,
            _subscriptions: subscriptions,
        };
        view.reload(cx);
        view
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    /// Fetches the diff again. The old diff stays on screen until the new one arrives.
    pub fn reload(&mut self, cx: &mut Context<DiffView>) {
        self.load += 1;
        let load = self.load;
        let cwd = self.cwd.clone();
        let path = self.path.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.file_diff(&cwd, &path, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| this.apply(load, result, cx));
        })
        .detach();
    }

    fn apply(&mut self, load: u64, result: anyhow::Result<String>, cx: &mut Context<DiffView>) {
        if load != self.load {
            return;
        }
        self.state = match result {
            Ok(text) if text.trim().is_empty() => State::Empty,
            Ok(text) => {
                let parsed = parse(&text);
                if parsed.lines.is_empty() {
                    State::Empty
                } else {
                    let (code_size, _) = code_font(cx);
                    let longest = (0..parsed.lines.len())
                        .max_by_key(|&index| parsed.lines[index].text.len())
                        .unwrap_or(0);
                    let gutter = gutter_width(max_number(&parsed.lines), code_size.into());
                    State::Ready(Doc {
                        parsed,
                        runs: Vec::new(),
                        longest,
                        gutter,
                        scheme: scheme(cx),
                    })
                }
            }
            Err(error) => {
                let message = format!("{error:#}");
                match no_diff_reason(&message) {
                    Some(NoDiff::TooLarge) => State::TooLarge,
                    Some(NoDiff::Binary) => State::Binary,
                    None => {
                        log::warn!("file_diff {}: {message}", self.path);
                        State::Failed(message)
                    }
                }
            }
        };
        self.start_highlight(cx);
        cx.notify();
    }

    fn theme_changed(&mut self, cx: &mut Context<DiffView>) {
        let current = scheme(cx);
        if let State::Ready(doc) = &self.state
            && doc.scheme != current
        {
            self.start_highlight(cx);
        }
        cx.notify();
    }

    /// Colors the lines off the UI thread: the context and added lines, then the removed lines,
    /// are parsed as one text in diff order, which the grammar reads well enough for colors.
    fn start_highlight(&mut self, cx: &mut Context<DiffView>) {
        let State::Ready(doc) = &mut self.state else {
            return;
        };
        let scheme = scheme(cx);
        doc.scheme = scheme;
        let Some(language) = language_for(&self.path) else {
            doc.runs.clear();
            return;
        };
        let body: Vec<(usize, &DiffLine)> = doc
            .parsed
            .lines
            .iter()
            .enumerate()
            .filter(|(_, line)| line.kind != LineKind::Header)
            .collect();
        let size: usize = body.iter().map(|(_, line)| line.text.len() + 1).sum();
        if size > HIGHLIGHT_LIMIT {
            doc.runs.clear();
            return;
        }
        let indexes: Vec<usize> = body.iter().map(|(index, _)| *index).collect();
        let text = body
            .iter()
            .map(|(_, line)| line.text.as_str())
            .collect::<Vec<_>>()
            .join("\n");
        let total = doc.parsed.lines.len();
        let load = self.load;
        let colors = SyntaxColors::for_scheme(scheme);
        cx.spawn(async move |this, cx| {
            let per_body_line = cx
                .background_executor()
                .spawn(async move { highlight(language, &text, &colors) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.load != load {
                    return;
                }
                if let State::Ready(doc) = &mut this.state
                    && doc.scheme == scheme
                    && doc.parsed.lines.len() == total
                {
                    let mut runs = vec![Vec::new(); total];
                    for (runs_of_line, index) in per_body_line.into_iter().zip(indexes) {
                        runs[index] = runs_of_line;
                    }
                    doc.runs = runs;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn render_header(&self, doc: Option<&ParsedDiff>, cx: &App) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let (name, directory) = match self.path.rsplit_once('/') {
            Some((directory, name)) => (name.to_string(), Some(directory.to_string())),
            None => (self.path.clone(), None),
        };
        let change = match doc {
            Some(doc) if doc.is_new => Change::Added,
            Some(doc) if doc.is_deleted => Change::Deleted,
            _ => Change::Modified,
        };
        let label = match doc {
            Some(doc) => format!("{}, +{}, -{}", self.path, doc.added, doc.removed),
            None => self.path.clone(),
        };
        h_flex()
            .id("diff-view-header")
            .role(Role::Heading)
            .aria_label(label)
            .flex_shrink_0()
            .h(px(HEADER_HEIGHT))
            .pl(px(HEADER_LEFT))
            .pr(px(HEADER_RIGHT))
            .gap(px(SPACE_2))
            .justify_between()
            .bg(colors.surface0)
            .border_b_1()
            .border_color(colors.border_accent)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(SPACE_1))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_color(colors.foreground)
                            .child(name),
                    )
                    .when_some(directory, |this, directory| {
                        this.child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_color(colors.foreground_muted)
                                .child(directory),
                        )
                    }),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap(px(SPACE_2))
                    .when_some(doc, |this, doc| {
                        this.child(diff_stat(doc.added, doc.removed, cx))
                    })
                    .child(change_icon(change, &colors)),
            )
    }

    fn status_row(
        &self,
        id: &'static str,
        label: &'static str,
        cx: &App,
    ) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        div()
            .id(id)
            .role(Role::Status)
            .aria_label(label)
            .px(px(HEADER_LEFT))
            .py(px(SPACE_3))
            .text_size(code_font(cx).0)
            .font_family(code_font(cx).1)
            .text_color(colors.foreground_muted)
            .child(label)
    }

    fn render_lines(
        &mut self,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let State::Ready(doc) = &self.state else {
            return Vec::new();
        };
        let colors = tokens(cx);
        let (size, family) = code_font(cx);
        let addition = colors.status_success.opacity(ADDITION_TINT);
        let deletion = colors.status_danger.opacity(DELETION_TINT);
        range
            .filter_map(|index| {
                let line = doc.parsed.lines.get(index)?;
                let runs = doc.runs.get(index).map_or(&[][..], Vec::as_slice);
                let (background, number_color, text_color) = match line.kind {
                    LineKind::Add => (addition, colors.status_success, colors.foreground),
                    LineKind::Remove => (deletion, colors.status_danger, colors.foreground),
                    LineKind::Header => (
                        colors.surface0,
                        colors.foreground_muted,
                        colors.foreground_muted,
                    ),
                    LineKind::Context => {
                        (colors.surface0, colors.foreground_muted, colors.foreground)
                    }
                };
                let kind = match line.kind {
                    LineKind::Add => "added",
                    LineKind::Remove => "removed",
                    LineKind::Header => "hunk",
                    LineKind::Context => "unchanged",
                };
                let label: String = line.text.chars().take(200).collect();
                Some(
                    h_flex()
                        .id(("diff-line", index))
                        .role(Role::Code)
                        .aria_label(format!("{kind} line {}: {label}", line.number.unwrap_or(0)))
                        .h(px(LINE_HEIGHT))
                        .font_family(family)
                        .text_size(size)
                        .line_height(px(LINE_HEIGHT))
                        .bg(background)
                        .text_color(text_color)
                        .child(
                            div()
                                .w(px(doc.gutter))
                                .h_full()
                                .flex_shrink_0()
                                .pr(px(NUMBER_RIGHT))
                                .text_right()
                                .text_color(number_color)
                                .border_r_1()
                                .border_color(colors.border)
                                .children(line.number.map(|number| number.to_string())),
                        )
                        .child(
                            div()
                                .pl(px(CODE_LEFT_PADDING))
                                .pr(px(SPACE_3))
                                .whitespace_nowrap()
                                .child(code_text(&line.text, runs)),
                        )
                        .into_any_element(),
                )
            })
            .collect()
    }
}

impl Render for DiffView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let root = div()
            .id("diff-view")
            .role(Role::Pane)
            .aria_label(format!("Diff {}", self.path))
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.surface0)
            .text_color(colors.foreground);
        match &self.state {
            State::Loading => root.child(centered_text(
                "diff-view-loading",
                "Loading diff...",
                false,
                cx,
            )),
            State::Failed(message) => {
                root.child(centered_text("diff-view-error", message.clone(), true, cx))
            }
            // A file the daemon sends without hunks shows a status line under its header, as the
            // `status` row of the Electron diff document does.
            State::TooLarge => root
                .child(self.render_header(None, cx))
                .child(self.status_row("diff-view-too-large", "Diff too large to display", cx)),
            State::Binary => root
                .child(self.render_header(None, cx))
                .child(self.status_row("diff-view-binary", "Binary file", cx)),
            State::Empty => root.child(centered_text(
                "diff-view-empty",
                "No changes to display",
                false,
                cx,
            )),
            State::Ready(doc) => {
                let header = self.render_header(Some(&doc.parsed), cx);
                let count = doc.parsed.lines.len();
                let longest = doc.longest;
                root.child(header).child(
                    div()
                        .id("diff-view-lines")
                        .role(Role::Document)
                        .aria_label(format!("Changes in {}", self.path))
                        .flex_1()
                        .min_h_0()
                        .child(
                            uniform_list("diff-view-rows", count, cx.processor(Self::render_lines))
                                .track_scroll(&self.scroll)
                                .with_horizontal_sizing_behavior(
                                    ListHorizontalSizingBehavior::Unconstrained,
                                )
                                .with_width_from_item(Some(longest))
                                .size_full(),
                        ),
                )
            }
        }
    }
}
