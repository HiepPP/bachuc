//! The diff viewer pane, as the Electron diff document draws it (`git/diff-document`): a header
//! per file, then numbered lines over the addition and deletion tints, all files in one scroll.
//!
//! A view shows one of three sources: one uncommitted file (`new`), every uncommitted file from
//! one `checkout_diff` request (`new_all`), or one file of one commit (`new_commit`). The rows
//! come from `DiffDoc`, and the `list` element builds only the rows in view.

use crate::changes::Change;
use crate::code::{code_font, code_text, gutter_width};
use crate::diff::{LineKind, NoDiff, no_diff_reason, parse};
use crate::diff_doc::{DiffDoc, DiffRow, FileBody, Latest};
use crate::diff_prefs::{DiffPrefs, FocusTracker, clamp_offset, code_widths, uniform_row_height};
use crate::markdown_diff::{BlockKind, RichBlock, block_diff, is_markdown_path, old_text};
use crate::syntax::{Run, ThemeSyntax, highlight, language_for};
use crate::ui::{
    centered_text, change_icon, chevron, diff_mode_menu, diff_stat, read_prefs, update_prefs,
};
use bachuc_client::{
    Client, ClientEvent, DiffCompare, DirectoryEvent, FileContent, FileDiff, FileDiffStatus,
    FileKind,
};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, ICON_SM, MAX_CONTENT_WIDTH, RADIUS_MD, SPACE_1, SPACE_2, SPACE_3,
};
use bachuc_theme::{syntax_colors, tokens, ui_font_size, zoom, zoomed};
use gpui_kit::component::text::TextView;
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
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
/// The width of the bar on the left of an added or removed block in the rendered markdown.
const BLOCK_BAR: f32 = 3.;
const BLOCK_LINE_FACTOR: f32 = 1.4;
const RENDERED_UNAVAILABLE: &str =
    "Rendered diff unavailable: the file changed since the diff loaded";

/// What the view shows.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Source {
    File { path: String },
    All { focus_path: Option<String> },
    Commit { sha: String, path: String },
}

/// What a reload of a one-file view found for the rendered markdown diff.
enum Rendered {
    /// Rendered is off, or the diff has nothing to render.
    Off,
    /// The file could not be read as text, or the patch does not match it.
    Unavailable,
    Blocks(Vec<RichBlock>),
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
    /// The zoom the list heights and the gutter were made for; `render` redoes them on a change.
    zoom: f32,
    list: ListState,
    requests: Latest,
    /// The focus file to scroll to: the one the view opened with, then each later Changes click.
    focus: FocusTracker,
    /// The request of the diff on screen or in flight, for an all-files view.
    compare: Option<DiffCompare>,
    /// Long lines wrap instead of scrolling sideways.
    wrap: bool,
    /// A markdown file shows as rendered blocks instead of lines. Starts off and lives for the
    /// view only.
    markdown_mode: bool,
    /// The blocks of the whole file; kept on screen while a reload is in flight.
    rendered: Option<Vec<RichBlock>>,
    /// The last Rendered request could not be answered, so the view fell back to lines.
    rendered_unavailable: bool,
    /// The sideways offset of each file's code, in pixels.
    offsets: Vec<f32>,
    /// The advance width of one code character, measured at the last render.
    advance: f32,
    /// The list's width at the last frame, read in `render`. A row callback runs while the list
    /// holds its state borrowed, so it must not read the `ListState` itself.
    pane_width: f32,
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
            cx.observe_global::<DiffPrefs>(|this, cx| this.prefs_changed(cx)),
        ];
        let initial_focus = match &source {
            Source::All { focus_path } => focus_path.clone(),
            _ => None,
        };
        let (revision, wrap) = read_prefs(cx, |prefs| (prefs.focus_revision(), prefs.wrap_lines));
        let mut view = DiffView {
            client,
            cwd,
            source,
            state: State::Loading,
            doc: DiffDoc::default(),
            runs: Vec::new(),
            colors: ThemeSyntax(syntax_colors(cx)),
            gutter: 0.,
            zoom: zoom(cx),
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            requests: Latest::default(),
            focus: FocusTracker::new(revision, initial_focus),
            compare: None,
            wrap,
            markdown_mode: false,
            rendered: None,
            rendered_unavailable: false,
            offsets: Vec::new(),
            advance: 0.,
            pane_width: 0.,
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
            File(
                String,
                Task<anyhow::Result<String>>,
                Option<Task<anyhow::Result<FileContent>>>,
            ),
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
                // The working file is read only while Rendered is on.
                let read = self.markdown_mode.then(|| {
                    self.client
                        .update(cx, |client, cx| client.read_file(&cwd, &path, cx))
                });
                Pending::File(path, task, read)
            }
            Source::All { .. } => {
                let compare = read_prefs(cx, |prefs| prefs.compare(&cwd));
                self.compare = Some(compare.clone());
                Pending::All(self.client.update(cx, |client, cx| {
                    client.checkout_diff_with(&cwd, compare, cx)
                }))
            }
            Source::Commit { sha, path } => {
                Pending::Commit(self.client.update(cx, |client, cx| {
                    client.commit_file_diff(&cwd, &sha, &path, cx)
                }))
            }
        };
        cx.spawn(async move |this, cx| {
            let mut rendered = Rendered::Off;
            let result = match pending {
                Pending::File(path, task, read) => {
                    let result = one_file(path, task.await);
                    if let Some(read) = read {
                        let content = read.await;
                        rendered = match result.as_ref().ok().and_then(|diffs| renderable(diffs)) {
                            Some(diff_text) => {
                                let blocks = cx
                                    .background_executor()
                                    .spawn(async move { rich_blocks(&diff_text, content) })
                                    .await;
                                blocks.map_or(Rendered::Unavailable, Rendered::Blocks)
                            }
                            None => Rendered::Off,
                        };
                    }
                    result
                }
                Pending::All(task) => task.await,
                Pending::Commit(task) => task.await.map(|diff| diff.into_iter().collect()),
            };
            let _ = this.update(cx, |this, cx| this.apply(request, result, rendered, cx));
        })
        .detach();
    }

    fn apply(
        &mut self,
        request: u64,
        result: anyhow::Result<Vec<FileDiff>>,
        rendered: Rendered,
        cx: &mut Context<DiffView>,
    ) {
        if !self.requests.is_current(request) {
            return;
        }
        self.rendered_unavailable = false;
        self.rendered = match rendered {
            Rendered::Off => None,
            Rendered::Unavailable => {
                self.markdown_mode = false;
                self.rendered_unavailable = true;
                None
            }
            Rendered::Blocks(blocks) => Some(blocks),
        };
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
                self.gutter = self.gutter_for(code_size.into());
                self.state = if self.doc.is_empty() {
                    State::Empty
                } else {
                    State::Ready
                };
                self.offsets.resize(self.doc.files().len(), 0.);
                self.reset_list();
                self.apply_focus();
                self.start_highlight(cx);
            }
        }
        cx.notify();
    }

    /// The height of one diff line at the current zoom.
    fn line_height(&self) -> f32 {
        LINE_HEIGHT * self.zoom
    }

    /// The gutter width: digits at the zoomed code size, and the 12 px padding at the zoom.
    fn gutter_for(&self, code_size: f32) -> f32 {
        gutter_width(self.doc.max_number(), code_size) - 12. + 12. * self.zoom
    }

    /// Gives the list the new row count and keeps the top row where it was.
    fn reset_list(&mut self) {
        let keep = self.list.logical_scroll_top();
        self.reset_rows();
        self.list.scroll_to(keep);
    }

    /// Resets the list to the row model: one height while lines scroll sideways, measured rows
    /// while they wrap.
    fn reset_rows(&mut self) {
        let count = self.doc.rows().len();
        match uniform_row_height(self.wrap, self.line_height()) {
            Some(height) => self.list.reset_with_uniform_height(count, px(height)),
            None => self.list.reset(count),
        }
    }

    /// Scrolls to the pending focus file once a diff is on screen, opening it when collapsed. A
    /// path the diff does not have is dropped without a scroll.
    fn apply_focus(&mut self) {
        let Some(path) = self.focus.take(matches!(self.state, State::Ready)) else {
            return;
        };
        let rows = self.doc.rows().len();
        let Some(row) = self.doc.reveal(&path) else {
            return;
        };
        if self.doc.rows().len() != rows {
            self.reset_rows();
        }
        self.list.scroll_to(ListOffset {
            item_ix: row,
            offset_in_item: px(0.),
        });
    }

    /// Follows the shared choices: the wrap toggle, and for an all-files view the mode, the
    /// whitespace toggle, and the focus requests for its folder.
    fn prefs_changed(&mut self, cx: &mut Context<DiffView>) {
        let (wrap, compare, request) = read_prefs(cx, |prefs| {
            (
                prefs.wrap_lines,
                prefs.compare(&self.cwd),
                prefs.focus().cloned(),
            )
        });
        if wrap != self.wrap {
            self.wrap = wrap;
            self.reset_list();
        }
        if matches!(self.source, Source::All { .. }) {
            if self.compare.as_ref() != Some(&compare) {
                self.reload(cx);
            }
            if self.focus.observe(request.as_ref(), &self.cwd) {
                self.apply_focus();
            }
        }
        cx.notify();
    }

    /// The width of a file's code with its paddings, and the width the pane leaves for code.
    fn code_widths(&self, file: usize) -> (f32, f32) {
        code_widths(
            self.doc.longest_columns(file),
            self.advance,
            (CODE_LEFT_PADDING + SPACE_3) * self.zoom,
            self.pane_width,
            self.gutter,
        )
    }

    /// A horizontal trackpad or shift-wheel delta over a file's lines moves only that file.
    fn scroll_sideways(
        &mut self,
        file: usize,
        event: &ScrollWheelEvent,
        cx: &mut Context<DiffView>,
    ) {
        let delta = event.delta.pixel_delta(px(self.line_height()));
        let shift_wheel = event.modifiers.shift && delta.x == px(0.);
        let delta: f32 = if shift_wheel { delta.y } else { delta.x }.into();
        if delta == 0. {
            return;
        }
        if shift_wheel {
            // The list would scroll the same delta vertically.
            cx.stop_propagation();
        }
        let (content, code) = self.code_widths(file);
        let Some(offset) = self.offsets.get_mut(file) else {
            return;
        };
        // A positive delta moves the content right, toward the line start.
        let next = clamp_offset(*offset, -delta, content, code);
        if next != *offset {
            *offset = next;
            cx.notify();
        }
    }

    fn toggle_wrap(&mut self, cx: &mut Context<DiffView>) {
        update_prefs(cx, |prefs| {
            prefs.wrap_lines = !prefs.wrap_lines;
            true
        });
    }

    /// Whether the view can show Rendered: a changed markdown file of a one-file view with line
    /// rows on screen. A deleted file has no working file to read.
    fn markdown_available(&self) -> bool {
        let Source::File { path } = &self.source else {
            return false;
        };
        is_markdown_path(path)
            && matches!(self.state, State::Ready)
            && self.doc.files().first().is_some_and(|file| {
                file.change != Change::Deleted && matches!(file.body, FileBody::Lines(_))
            })
    }

    /// Switches between lines and rendered blocks. Turning Rendered on reloads, which reads the
    /// file; this is also the retry after the "unavailable" notice.
    fn toggle_markdown(&mut self, cx: &mut Context<DiffView>) {
        self.markdown_mode = !self.markdown_mode;
        self.rendered_unavailable = false;
        if self.markdown_mode {
            self.reload(cx);
        } else {
            self.rendered = None;
        }
        cx.notify();
    }

    fn toggle_whitespace(&mut self, cx: &mut Context<DiffView>) {
        update_prefs(cx, |prefs| {
            prefs.hide_whitespace = !prefs.hide_whitespace;
            true
        });
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
        self.reset_rows();
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
        self.reset_rows();
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
            .size(zoomed(TOOLBAR_BUTTON, cx))
            .rounded(zoomed(RADIUS_MD, cx))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(colors.interaction_highlight))
            .child(
                icon(glyph)
                    .with_size(zoomed(ICON_SM, cx))
                    .text_color(colors.foreground_extra_muted),
            )
    }

    /// The toolbar of the all-files view: the diff mode menu with the total count, then collapse
    /// all, the whitespace and wrap toggles, and refresh. The split layout needs a paired-row
    /// renderer the view does not have.
    fn render_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let (added, removed) = self.doc.totals();
        let all_collapsed = self.doc.all_collapsed();
        let (collapse_label, collapse_icon) = if all_collapsed {
            ("Expand all files", IconName::ListChevronsUpDown)
        } else {
            ("Collapse all files", IconName::ListChevronsDownUp)
        };
        let hide_whitespace = read_prefs(cx, |prefs| prefs.hide_whitespace);
        let whitespace_label = if hide_whitespace {
            "Show whitespace"
        } else {
            "Hide whitespace"
        };
        let wrap_label = if self.wrap {
            "Scroll long lines"
        } else {
            "Wrap long lines"
        };
        h_flex()
            .id("diff-view-toolbar")
            .role(Role::Toolbar)
            .aria_label("Diff")
            .flex_shrink_0()
            .h(zoomed(TOOLBAR_HEIGHT, cx))
            .pl(zoomed(SPACE_2, cx))
            .pr(zoomed(HEADER_RIGHT, cx))
            .gap(zoomed(SPACE_2, cx))
            .bg(colors.surface0)
            .border_b_1()
            .border_color(colors.border)
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(zoomed(SPACE_1, cx))
                    .child(diff_mode_menu("diff-view-mode", &self.cwd, cx))
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
                self.toolbar_button(
                    "changes-toggle-whitespace",
                    whitespace_label,
                    IconName::Pilcrow,
                    cx,
                )
                .aria_selected(hide_whitespace)
                .when(hide_whitespace, |this| {
                    this.bg(colors.interaction_highlight)
                })
                .on_click(cx.listener(|this, _, _, cx| this.toggle_whitespace(cx))),
            )
            .child(
                self.toolbar_button(
                    "changes-toggle-wrap-lines",
                    wrap_label,
                    IconName::WrapText,
                    cx,
                )
                .aria_selected(self.wrap)
                .when(self.wrap, |this| this.bg(colors.interaction_highlight))
                .on_click(cx.listener(|this, _, _, cx| this.toggle_wrap(cx))),
            )
            .child(
                self.toolbar_button("diff-view-refresh", "Refresh", IconName::RotateCw, cx)
                    .on_click(cx.listener(|this, _, _, cx| this.reload(cx))),
            )
    }

    /// The toolbar of a one-file or commit view: the wrap toggle at the right end. It has no mode
    /// menu, collapse, whitespace, or refresh control, because the view has one file and always
    /// sends the plain uncommitted compare.
    fn render_file_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let wrap_label = if self.wrap {
            "Scroll long lines"
        } else {
            "Wrap long lines"
        };
        h_flex()
            .id("diff-file-toolbar")
            .role(Role::Toolbar)
            .aria_label("Diff")
            .flex_shrink_0()
            .h(zoomed(TOOLBAR_HEIGHT, cx))
            .pl(zoomed(SPACE_2, cx))
            .pr(zoomed(HEADER_RIGHT, cx))
            .gap(zoomed(SPACE_2, cx))
            .bg(colors.surface0)
            .border_b_1()
            .border_color(colors.border)
            .child(div().flex_1().min_w_0())
            .when(self.markdown_available(), |this| {
                let (label, glyph) = if self.markdown_mode {
                    ("Show lines", IconName::Code)
                } else {
                    ("Show rendered", IconName::BookOpen)
                };
                this.child(
                    self.toolbar_button("diff-file-markdown", label, glyph, cx)
                        .aria_selected(self.markdown_mode)
                        .when(self.markdown_mode, |this| {
                            this.bg(colors.interaction_highlight)
                        })
                        .on_click(cx.listener(|this, _, _, cx| this.toggle_markdown(cx))),
                )
            })
            .child(
                self.toolbar_button("diff-file-wrap", wrap_label, IconName::WrapText, cx)
                    .aria_selected(self.wrap)
                    .when(self.wrap, |this| this.bg(colors.interaction_highlight))
                    .on_click(cx.listener(|this, _, _, cx| this.toggle_wrap(cx))),
            )
    }

    /// The whole new file as markdown blocks in their own scroll column, apart from the line
    /// list. Added and removed blocks get the line tints and a bar on the left.
    fn render_rendered(&self, blocks: &[RichBlock], cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let text_size = ui_font_size(FONT_BASE, cx);
        div()
            .id("diff-view-rendered")
            .role(Role::Document)
            .aria_label("Rendered changes")
            .flex_1()
            .min_h_0()
            .overflow_y_scroll()
            .child(
                v_flex()
                    .w_full()
                    .max_w(zoomed(MAX_CONTENT_WIDTH, cx))
                    .mx_auto()
                    .py(zoomed(SPACE_3, cx))
                    .children(blocks.iter().enumerate().map(|(index, block)| {
                        let (background, bar) = match block.kind {
                            BlockKind::Same => (None, None),
                            BlockKind::Added => (
                                Some(colors.status_success.opacity(ADDITION_TINT)),
                                Some(colors.status_success),
                            ),
                            BlockKind::Removed => (
                                Some(colors.status_danger.opacity(DELETION_TINT)),
                                Some(colors.status_danger),
                            ),
                        };
                        let label = match block.kind {
                            BlockKind::Same => "unchanged block",
                            BlockKind::Added => "added block",
                            BlockKind::Removed => "removed block",
                        };
                        div()
                            .id(("diff-rendered-block", index))
                            .role(Role::Group)
                            .aria_label(label)
                            .w_full()
                            .border_l(px(BLOCK_BAR))
                            .border_color(bar.unwrap_or_else(gpui::transparent_black))
                            .px(zoomed(SPACE_3, cx))
                            .py(zoomed(SPACE_1, cx))
                            .when_some(background, |this, background| this.bg(background))
                            .text_size(text_size)
                            .line_height(text_size * BLOCK_LINE_FACTOR)
                            .text_color(colors.foreground)
                            .child(TextView::markdown(
                                ElementId::Name(format!("diff-rendered-text-{index}").into()),
                                block.markdown.clone(),
                            ))
                    })),
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
            .h(zoomed(HEADER_HEIGHT, cx))
            .pl(zoomed(HEADER_LEFT, cx))
            .pr(zoomed(HEADER_RIGHT, cx))
            .gap(zoomed(SPACE_2, cx))
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
                    .gap(zoomed(SPACE_1, cx))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .child(chevron(expanded, &colors, cx))
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
                    .gap(zoomed(SPACE_2, cx))
                    .child(diff_stat(entry.additions, entry.deletions, cx))
                    .child(change_icon(entry.change, &colors, cx)),
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
            .px(zoomed(HEADER_LEFT, cx))
            .py(zoomed(SPACE_3, cx))
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
        let wrap = self.wrap;
        // Clamped again here, because the pane may have grown since the last scroll.
        let offset = if wrap {
            0.
        } else {
            let (content, code) = self.code_widths(file);
            clamp_offset(
                self.offsets.get(file).copied().unwrap_or(0.),
                0.,
                content,
                code,
            )
        };
        let code = div()
            .pl(zoomed(CODE_LEFT_PADDING, cx))
            .pr(zoomed(SPACE_3, cx))
            .child(code_text(&line_data.text, runs));
        let code = if wrap {
            code.flex_1().min_w_0().whitespace_normal()
        } else {
            code.flex_shrink_0()
                .relative()
                .left(px(-offset))
                .whitespace_nowrap()
        };
        h_flex()
            .id(("diff-line", index))
            .role(Role::Code)
            .aria_label(format!(
                "{kind} line {}: {label}",
                line_data.number.unwrap_or(0)
            ))
            .w_full()
            .map(|row| {
                if wrap {
                    row.min_h(zoomed(LINE_HEIGHT, cx)).items_stretch()
                } else {
                    row.h(zoomed(LINE_HEIGHT, cx))
                }
            })
            .overflow_hidden()
            .font_family(family)
            .text_size(size)
            .line_height(zoomed(LINE_HEIGHT, cx))
            .bg(background)
            .text_color(text_color)
            .when(!wrap, |row| {
                row.on_scroll_wheel(
                    cx.listener(move |this, event, _, cx| this.scroll_sideways(file, event, cx)),
                )
            })
            .child(
                div()
                    .w(px(self.gutter))
                    .when(!wrap, |gutter| gutter.h_full())
                    .flex_shrink_0()
                    .pr(zoomed(NUMBER_RIGHT, cx))
                    .text_right()
                    .text_color(number_color)
                    .border_r_1()
                    .border_color(colors.border)
                    .children(line_data.number.map(|number| number.to_string())),
            )
            .child(
                // The gutter stays; only the code moves under it.
                div()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .flex()
                    .child(code),
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

/// The diff text of a one-file view that Rendered can use: a changed file with a text diff.
fn renderable(diffs: &[FileDiff]) -> Option<String> {
    diffs
        .first()
        .filter(|diff| diff.status == FileDiffStatus::Ok && !diff.is_deleted)
        .map(|diff| diff.text.clone())
}

/// The blocks of the working file against its old side; `None` when the file is not text or the
/// patch does not match it.
fn rich_blocks(diff_text: &str, content: anyhow::Result<FileContent>) -> Option<Vec<RichBlock>> {
    let content = content.ok()?;
    if content.kind != FileKind::Text {
        return None;
    }
    let new = content.text?;
    let old = old_text(&new, diff_text)?;
    Some(block_diff(&old, &new))
}

impl Render for DiffView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let (code_size, family) = code_font(cx);
        if self.zoom != zoom(cx) {
            // Row heights and the gutter were made for the old zoom.
            self.zoom = zoom(cx);
            self.gutter = self.gutter_for(code_size.into());
            self.reset_list();
        }
        let font_id = window.text_system().resolve_font(&font(family));
        self.advance = window
            .text_system()
            .em_advance(font_id, code_size)
            .map_or(0., f32::from);
        self.pane_width = self.list.viewport_bounds().size.width.into();
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
        let root = if matches!(self.source, Source::All { .. }) {
            root.child(self.render_toolbar(cx))
        } else {
            root.child(self.render_file_toolbar(cx))
        };
        let show_blocks = self.markdown_mode && self.markdown_available();
        let root = root.when(
            self.rendered_unavailable && self.markdown_available(),
            |this| {
                this.child(
                    div()
                        .id("diff-view-rendered-notice")
                        .role(Role::Status)
                        .aria_label(RENDERED_UNAVAILABLE)
                        .flex_shrink_0()
                        .px(zoomed(SPACE_2, cx))
                        .py(zoomed(SPACE_1, cx))
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .text_color(colors.foreground_muted)
                        .border_b_1()
                        .border_color(colors.border)
                        .child(RENDERED_UNAVAILABLE),
                )
            },
        );
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
            State::Ready if show_blocks && self.rendered.is_some() => {
                root.child(self.render_rendered(self.rendered.as_deref().unwrap_or_default(), cx))
            }
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
