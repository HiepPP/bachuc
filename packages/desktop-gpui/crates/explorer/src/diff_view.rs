//! The diff viewer pane, as the Electron diff document draws it (`git/diff-document`): a header
//! per file, then numbered lines over the addition and deletion tints, all files in one scroll.
//!
//! A view shows one of three sources: one uncommitted file (`new`), every uncommitted file from
//! one `checkout_diff` request (`new_all`), or one file of one commit (`new_commit`). The rows
//! come from `DiffDoc`, and the `list` element builds only the rows in view.

use crate::code::{code_font, code_text, gutter_width};
use crate::diff::{LineKind, NoDiff, no_diff_reason, parse};
use crate::diff_doc::{DiffDoc, DiffRow, FileBody, Latest};
use crate::syntax::{Run, ThemeSyntax, highlight, language_for};
use crate::ui::{centered_text, change_icon, chevron, diff_stat};
use bachuc_client::{Client, ClientEvent, DirectoryEvent, FileDiff, FileDiffStatus};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_BASE, FONT_SM, ICON_SM, RADIUS_MD, SPACE_1, SPACE_2, SPACE_3};
use bachuc_theme::{syntax_colors, tokens, ui_font_size};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// The height of one diff line.
pub const LINE_HEIGHT: f32 = 22.;
/// `DIFF_FILE_HEADER_HEIGHT` and the left and right insets of its content.
const HEADER_HEIGHT: f32 = 30.;
const HEADER_LEFT: f32 = 12.;
const HEADER_RIGHT: f32 = 8.;
/// `WORKSPACE_SECONDARY_HEADER_HEIGHT` and `SMALL_ICON_BUTTON_SIZE` of the toolbar.
const TOOLBAR_HEIGHT: f32 = 36.;
const TOOLBAR_BUTTON: f32 = 20.;
/// `CODE_LEFT_PADDING`, and the space between a number and the gutter edge.
const CODE_LEFT_PADDING: f32 = 8.;
const NUMBER_RIGHT: f32 = 7.;
/// The addition and deletion tints of `createDiffPalette`.
const ADDITION_TINT: f32 = 0.15;
const DELETION_TINT: f32 = 0.1;
/// A file over this many bytes of diff text shows in the plain color.
const HIGHLIGHT_LIMIT: usize = 512 * 1024;
/// How far past the visible rows the list builds rows.
const OVERDRAW: f32 = 200.;

/// What the view shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Source {
    File { path: String },
    All { focus_path: Option<String> },
    Commit { sha: String, path: String },
}

enum State {
    Loading,
    Failed(String),
    /// No file has a change.
    Empty,
    Ready,
}

pub struct DiffView {
    client: Entity<Client>,
    cwd: String,
    source: Source,
    state: State,
    doc: DiffDoc,
    /// Colored ranges per file and line; empty until the highlighter finishes.
    runs: Vec<Vec<Vec<Run>>>,
    /// The syntax colors the runs were made for.
    colors: ThemeSyntax,
    gutter: f32,
    list: ListState,
    requests: Latest,
    /// The first answer scrolls to the focus file; later reloads keep the scroll.
    scrolled: bool,
    _subscriptions: Vec<Subscription>,
}

impl DiffView {
    /// The uncommitted diff of one file.
    pub fn new(
        client: Entity<Client>,
        cwd: String,
        path: String,
        window: &mut Window,
        cx: &mut Context<DiffView>,
    ) -> DiffView {
        Self::with_source(client, cwd, Source::File { path }, window, cx)
    }

    /// Every uncommitted file, scrolled to `focus_path` when the diff has it.
    pub fn new_all(
        client: Entity<Client>,
        cwd: String,
        focus_path: Option<String>,
        window: &mut Window,
        cx: &mut Context<DiffView>,
    ) -> DiffView {
        Self::with_source(client, cwd, Source::All { focus_path }, window, cx)
    }

    /// One file of one commit.
    pub fn new_commit(
        client: Entity<Client>,
        cwd: String,
        sha: String,
        path: String,
        window: &mut Window,
        cx: &mut Context<DiffView>,
    ) -> DiffView {
        Self::with_source(client, cwd, Source::Commit { sha, path }, window, cx)
    }

    fn with_source(
        client: Entity<Client>,
        cwd: String,
        source: Source,
        _window: &mut Window,
        cx: &mut Context<DiffView>,
    ) -> DiffView {
        let subscriptions = vec![
            cx.subscribe(
                &client,
                |this: &mut DiffView, _, event: &DirectoryEvent, cx| {
                    // A commit does not change, so only the uncommitted views reload.
                    if let DirectoryEvent::CheckoutChanged { cwd } = event
                        && *cwd == this.cwd
                        && !matches!(this.source, Source::Commit { .. })
                    {
                        this.reload(cx);
                    }
                },
            ),
            cx.subscribe(
                &client,
                |this: &mut DiffView, _, event: &ClientEvent, cx| {
                    if let ClientEvent::Connected = event
                        && matches!(this.state, State::Loading | State::Failed(_))
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
            source,
            state: State::Loading,
            doc: DiffDoc::default(),
            runs: Vec::new(),
            colors: ThemeSyntax(syntax_colors(cx)),
            gutter: 0.,
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            requests: Latest::default(),
            scrolled: false,
            _subscriptions: subscriptions,
        };
        view.reload(cx);
        view
    }

    /// The file of a one-file view, or the focus file of an all-files view.
    pub fn path(&self) -> &str {
        match &self.source {
            Source::File { path } | Source::Commit { path, .. } => path,
            Source::All { focus_path } => focus_path.as_deref().unwrap_or(""),
        }
    }

    /// Fetches the diff again. The old diff stays on screen until the new one arrives, and an
    /// answer to an older request is dropped.
    pub fn reload(&mut self, cx: &mut Context<DiffView>) {
        enum Pending {
            File(String, Task<anyhow::Result<String>>),
            All(Task<anyhow::Result<Vec<FileDiff>>>),
            Commit(Task<anyhow::Result<Option<FileDiff>>>),
        }
        let request = self.requests.begin();
        let cwd = self.cwd.clone();
        let pending = match self.source.clone() {
            Source::File { path } => {
                let task = self
                    .client
                    .update(cx, |client, cx| client.file_diff(&cwd, &path, cx));
                Pending::File(path, task)
            }
            Source::All { .. } => Pending::All(
                self.client
                    .update(cx, |client, cx| client.checkout_diff(&cwd, cx)),
            ),
            Source::Commit { sha, path } => {
                Pending::Commit(self.client.update(cx, |client, cx| {
                    client.commit_file_diff(&cwd, &sha, &path, cx)
                }))
            }
        };
        cx.spawn(async move |this, cx| {
            let result = match pending {
                Pending::File(path, task) => one_file(path, task.await),
                Pending::All(task) => task.await,
                Pending::Commit(task) => task.await.map(|diff| diff.into_iter().collect()),
            };
            let _ = this.update(cx, |this, cx| this.apply(request, result, cx));
        })
        .detach();
    }

    fn apply(
        &mut self,
        request: u64,
        result: anyhow::Result<Vec<FileDiff>>,
        cx: &mut Context<DiffView>,
    ) {
        if !self.requests.is_current(request) {
            return;
        }
        match result {
            Err(error) => {
                let message = format!("{error:#}");
                log::warn!("diff {}: {message}", self.path());
                self.state = State::Failed(message);
            }
            Ok(diffs) => {
                self.doc = DiffDoc::new(diffs);
                self.runs.clear();
                let (code_size, _) = code_font(cx);
                self.gutter = gutter_width(self.doc.max_number(), code_size.into());
                self.state = if self.doc.is_empty() {
                    State::Empty
                } else {
                    State::Ready
                };
                self.reset_list();
                self.start_highlight(cx);
            }
        }
        cx.notify();
    }

    /// Gives the list the new row count. The first answer scrolls to the focus file; a reload
    /// keeps the top row where it was.
    fn reset_list(&mut self) {
        let keep = self.list.logical_scroll_top();
        self.list
            .reset_with_uniform_height(self.doc.rows().len(), px(LINE_HEIGHT));
        if !self.scrolled {
            self.scrolled = true;
            let focus = match &self.source {
                Source::All { focus_path } => focus_path.as_deref(),
                _ => None,
            };
            if let Some(row) = focus.and_then(|path| self.doc.header_row(path)) {
                self.list.scroll_to(ListOffset {
                    item_ix: row,
                    offset_in_item: px(0.),
                });
            }
        } else {
            self.list.scroll_to(keep);
        }
    }

    fn theme_changed(&mut self, cx: &mut Context<DiffView>) {
        if matches!(self.state, State::Ready) && self.colors != ThemeSyntax(syntax_colors(cx)) {
            self.start_highlight(cx);
        }
        cx.notify();
    }

    /// Colors each file off the UI thread: its context and added lines, then its removed lines,
    /// are parsed as one text in diff order, which the grammar reads well enough for colors.
    fn start_highlight(&mut self, cx: &mut Context<DiffView>) {
        let colors = ThemeSyntax(syntax_colors(cx));
        self.colors = colors;
        let mut jobs = Vec::new();
        for (file, entry) in self.doc.files().iter().enumerate() {
            let FileBody::Lines(parsed) = &entry.body else {
                continue;
            };
            let Some(language) = language_for(&entry.path) else {
                continue;
            };
            let body: Vec<usize> = (0..parsed.lines.len())
                .filter(|&index| parsed.lines[index].kind != LineKind::Header)
                .collect();
            let size: usize = body
                .iter()
                .map(|&index| parsed.lines[index].text.len() + 1)
                .sum();
            if size > HIGHLIGHT_LIMIT {
                continue;
            }
            let text = body
                .iter()
                .map(|&index| parsed.lines[index].text.as_str())
                .collect::<Vec<_>>()
                .join("\n");
            jobs.push((file, parsed.lines.len(), language, text, body));
        }
        if jobs.is_empty() {
            return;
        }
        let request = self.requests.current();
        let file_count = self.doc.files().len();
        cx.spawn(async move |this, cx| {
            let runs = cx
                .background_executor()
                .spawn(async move {
                    let mut runs: Vec<Vec<Vec<Run>>> = vec![Vec::new(); file_count];
                    for (file, total, language, text, body) in jobs {
                        let mut lines = vec![Vec::new(); total];
                        for (line_runs, index) in
                            highlight(language, &text, &colors).into_iter().zip(body)
                        {
                            lines[index] = line_runs;
                        }
                        runs[file] = lines;
                    }
                    runs
                })
                .await;
            let _ = this.update(cx, |this, cx| {
                // A newer answer or a theme change made these runs stale.
                if this.requests.is_current(request)
                    && this.colors == colors
                    && this.doc.files().len() == file_count
                {
                    this.runs = runs;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn toggle_file(&mut self, file: usize, cx: &mut Context<DiffView>) {
        let keep = self.list.logical_scroll_top();
        self.doc.toggle_file(file);
        self.list
            .reset_with_uniform_height(self.doc.rows().len(), px(LINE_HEIGHT));
        // Keep the clicked header in place.
        if let Some(row) = self
            .doc
            .rows()
            .iter()
            .position(|row| *row == DiffRow::Header { file })
        {
            self.list.scroll_to(if keep.item_ix > row {
                ListOffset {
                    item_ix: row,
                    offset_in_item: px(0.),
                }
            } else {
                keep
            });
        }
        cx.notify();
    }

    fn toggle_all(&mut self, cx: &mut Context<DiffView>) {
        let collapse = !self.doc.all_collapsed();
        self.doc.set_all_collapsed(collapse);
        self.list
            .reset_with_uniform_height(self.doc.rows().len(), px(LINE_HEIGHT));
        cx.notify();
    }

    fn toolbar_button(
        &self,
        id: &'static str,
        label: &'static str,
        glyph: IconName,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let colors = tokens(cx);
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .flex_shrink_0()
            .size(px(TOOLBAR_BUTTON))
            .rounded(px(RADIUS_MD))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(colors.interaction_highlight))
            .child(
                icon(glyph)
                    .with_size(px(ICON_SM))
                    .text_color(colors.foreground_extra_muted),
            )
    }

    /// The toolbar of the all-files view: the diff mode with the total count, then collapse all
    /// and refresh. The other Electron diff options (mode menu, split layout, whitespace, wrap)
    /// need client modes or renderers the GPUI client does not have yet.
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let (added, removed) = self.doc.totals();
        let all_collapsed = self.doc.all_collapsed();
        let (collapse_label, collapse_icon) = if all_collapsed {
            ("Expand all files", IconName::ListChevronsUpDown)
        } else {
            ("Collapse all files", IconName::ListChevronsDownUp)
        };
        h_flex()
            .id("diff-view-toolbar")
            .role(Role::Toolbar)
            .aria_label("Diff")
            .flex_shrink_0()
            .h(px(TOOLBAR_HEIGHT))
            .pl(px(SPACE_2))
            .pr(px(HEADER_RIGHT))
            .gap(px(SPACE_2))
            .bg(colors.surface0)
            .border_b_1()
            .border_color(colors.border)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(SPACE_1))
                    .child(
                        div()
                            .id("diff-view-mode")
                            .role(Role::Label)
                            .aria_label("Uncommitted")
                            .p(px(SPACE_1))
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.foreground_muted)
                            .child("Uncommitted"),
                    )
                    .when(matches!(self.state, State::Ready), |this| {
                        this.child(diff_stat(added, removed, cx))
                    }),
            )
            .when(matches!(self.state, State::Ready), |this| {
                this.child(
                    self.toolbar_button(
                        "diff-view-collapse-all",
                        collapse_label,
                        collapse_icon,
                        cx,
                    )
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_all(cx))),
                )
            })
            .child(
                self.toolbar_button("diff-view-refresh", "Refresh", IconName::RotateCw, cx)
                    .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
            )
    }

    fn render_row(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        match self.doc.rows().get(index).copied() {
            Some(DiffRow::Header { file }) => self.render_header(index, file, cx),
            Some(DiffRow::Line { file, line }) => self.render_line(index, file, line, cx),
            Some(DiffRow::Placeholder { reason, .. }) => self.render_placeholder(index, reason, cx),
            None => div().into_any_element(),
        }
    }

    fn render_header(&self, index: usize, file: usize, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let Some(entry) = self.doc.files().get(file) else {
            return div().into_any_element();
        };
        let path = entry.display_path();
        let (name, directory) = match entry.path.rsplit_once('/') {
            Some((directory, name)) => (name.to_string(), Some(directory.to_string())),
            None => (entry.path.clone(), None),
        };
        let renamed_from = entry
            .old_path
            .as_ref()
            .filter(|old| **old != entry.path)
            .cloned();
        let expanded = !self.doc.is_collapsed(file);
        let label = format!(
            "{path}, {}, +{}, -{}",
            entry.change.label(),
            entry.additions,
            entry.deletions
        );
        h_flex()
            .id(("diff-file-header", index))
            .role(Role::Button)
            .aria_label(label)
            .aria_expanded(expanded)
            .w_full()
            .h(px(HEADER_HEIGHT))
            .pl(px(HEADER_LEFT))
            .pr(px(HEADER_RIGHT))
            .gap(px(SPACE_2))
            .justify_between()
            .bg(colors.surface0)
            .border_b_1()
            .border_color(colors.border_accent)
            .when(index > 0, |this| this.border_t_1())
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_file(file, cx)))
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(SPACE_1))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .child(chevron(expanded, &colors))
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
                    })
                    .when_some(renamed_from, |this, old| {
                        this.child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_color(colors.foreground_extra_muted)
                                .child(format!("from {old}")),
                        )
                    }),
            )
            .child(
                h_flex()
                    .flex_shrink_0()
                    .gap(px(SPACE_2))
                    .child(diff_stat(entry.additions, entry.deletions, cx))
                    .child(change_icon(entry.change, &colors)),
            )
            .into_any_element()
    }

    fn render_placeholder(
        &self,
        index: usize,
        reason: NoDiff,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let (size, family) = code_font(cx);
        let label = match reason {
            NoDiff::TooLarge => "Diff too large to display",
            NoDiff::Binary => "Binary file",
        };
        div()
            .id(("diff-placeholder", index))
            .role(Role::Status)
            .aria_label(label)
            .w_full()
            .px(px(HEADER_LEFT))
            .py(px(SPACE_3))
            .text_size(size)
            .font_family(family)
            .text_color(colors.foreground_muted)
            .child(label)
            .into_any_element()
    }

    fn render_line(
        &self,
        index: usize,
        file: usize,
        line: usize,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let Some(FileBody::Lines(parsed)) = self.doc.files().get(file).map(|entry| &entry.body)
        else {
            return div().into_any_element();
        };
        let Some(line_data) = parsed.lines.get(line) else {
            return div().into_any_element();
        };
        let colors = tokens(cx);
        let (size, family) = code_font(cx);
        let runs = self
            .runs
            .get(file)
            .and_then(|lines| lines.get(line))
            .map_or(&[][..], Vec::as_slice);
        let addition = colors.status_success.opacity(ADDITION_TINT);
        let deletion = colors.status_danger.opacity(DELETION_TINT);
        let (background, number_color, text_color) = match line_data.kind {
            LineKind::Add => (addition, colors.status_success, colors.foreground),
            LineKind::Remove => (deletion, colors.status_danger, colors.foreground),
            LineKind::Header => (
                colors.surface0,
                colors.foreground_muted,
                colors.foreground_muted,
            ),
            LineKind::Context => (colors.surface0, colors.foreground_muted, colors.foreground),
        };
        let kind = match line_data.kind {
            LineKind::Add => "added",
            LineKind::Remove => "removed",
            LineKind::Header => "hunk",
            LineKind::Context => "unchanged",
        };
        let label: String = line_data.text.chars().take(200).collect();
        h_flex()
            .id(("diff-line", index))
            .role(Role::Code)
            .aria_label(format!(
                "{kind} line {}: {label}",
                line_data.number.unwrap_or(0)
            ))
            .w_full()
            .h(px(LINE_HEIGHT))
            .overflow_hidden()
            .font_family(family)
            .text_size(size)
            .line_height(px(LINE_HEIGHT))
            .bg(background)
            .text_color(text_color)
            .child(
                div()
                    .w(px(self.gutter))
                    .h_full()
                    .flex_shrink_0()
                    .pr(px(NUMBER_RIGHT))
                    .text_right()
                    .text_color(number_color)
                    .border_r_1()
                    .border_color(colors.border)
                    .children(line_data.number.map(|number| number.to_string())),
            )
            .child(
                div()
                    .pl(px(CODE_LEFT_PADDING))
                    .pr(px(SPACE_3))
                    .whitespace_nowrap()
                    .child(code_text(&line_data.text, runs)),
            )
            .into_any_element()
    }
}

/// The answer of `Client::file_diff` as one `FileDiff`: no file when the text is empty, a
/// placeholder file for a too-large or binary file.
fn one_file(path: String, result: anyhow::Result<String>) -> anyhow::Result<Vec<FileDiff>> {
    let placeholder = |status| FileDiff {
        path: path.clone(),
        old_path: None,
        is_new: false,
        is_deleted: false,
        status,
        additions: 0,
        deletions: 0,
        text: String::new(),
    };
    match result {
        Ok(text) if text.trim().is_empty() => Ok(Vec::new()),
        Ok(text) => {
            let parsed = parse(&text);
            Ok(vec![FileDiff {
                path: path.clone(),
                old_path: None,
                is_new: parsed.is_new,
                is_deleted: parsed.is_deleted,
                status: FileDiffStatus::Ok,
                additions: parsed.added,
                deletions: parsed.removed,
                text,
            }])
        }
        Err(error) => match no_diff_reason(&format!("{error:#}")) {
            Some(NoDiff::TooLarge) => Ok(vec![placeholder(FileDiffStatus::TooLarge)]),
            Some(NoDiff::Binary) => Ok(vec![placeholder(FileDiffStatus::Binary)]),
            None => Err(error),
        },
    }
}

impl Render for DiffView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let root = div()
            .id("diff-view")
            .role(Role::Pane)
            .aria_label(match &self.source {
                Source::All { .. } => "Diff".to_string(),
                _ => format!("Diff {}", self.path()),
            })
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.surface0)
            .text_color(colors.foreground);
        let root = root.when(matches!(self.source, Source::All { .. }), |this| {
            this.child(self.render_toolbar(cx))
        });
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
            State::Empty => root.child(centered_text(
                "diff-view-empty",
                "No changes to display",
                false,
                cx,
            )),
            State::Ready => root.child(
                div()
                    .id("diff-view-lines")
                    .role(Role::Document)
                    .aria_label("Changes")
                    .flex_1()
                    .min_h_0()
                    .child(
                        list(
                            self.list.clone(),
                            cx.processor(|this, index: usize, window, cx| {
                                this.render_row(index, window, cx)
                            }),
                        )
                        .size_full(),
                    ),
            ),
        }
    }
}
