//! The file viewer pane: a read-only file with line numbers and syntax colors, an image, or a
//! placeholder, as `file-pane/pane.tsx` and its CodeMirror source view draw them. No editing.

use crate::code::{code_font, code_text, gutter_width};
use crate::syntax::{Run, SyntaxColors, highlight, language_for};
use crate::ui::{centered_text, file_meta_state};
use bachuc_client::{Client, FileContent, FileKind};
use bachuc_theme::metrics::{FONT_SM, RADIUS_SM, SPACE_2, SPACE_3, TAB_BAR_HEIGHT};
use bachuc_theme::{scheme, tokens, ui_font_size};
use gpui_kit::component::h_flex;
use gpui_kit::component::theme::Theme;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::ops::Range;
use std::sync::Arc;

/// Text up to this size is colored; larger text shows in the plain color. The Electron native
/// budget (`SOURCE_PRESENTATION_BUDGETS.native.highlighted`).
pub const HIGHLIGHT_LIMIT: u64 = 1024 * 1024;
/// A text or binary file over this size shows the placeholder (`native.plain`).
pub const TEXT_LIMIT: u64 = 10 * 1024 * 1024;
/// An image over this size shows the placeholder.
pub const IMAGE_LIMIT: u64 = 50 * 1024 * 1024;
/// `.cm-scroller { line-height: 1.45 }`.
const LINE_HEIGHT_FACTOR: f32 = 1.45;
/// `.cm-content { padding: 16px 0 }`.
const CONTENT_PADDING: f32 = 16.;
/// The CodeMirror gutter element padding (`0 3px 0 5px`) and line padding (`0 2px 0 6px`).
const GUTTER_PAD_LEFT: f32 = 5.;
const GUTTER_PAD_RIGHT: f32 = 3.;
const LINE_PAD_LEFT: f32 = 6.;

struct TextDoc {
    text: String,
    /// Byte offset of each line start.
    starts: Vec<usize>,
    /// Colored ranges per line, empty until the highlighter finishes.
    runs: Vec<Vec<Run>>,
    /// The index of the longest line, which sizes the horizontal scroll.
    longest: usize,
    size: u64,
    language: Option<&'static str>,
    /// The scheme the runs were made for.
    scheme: bachuc_theme::Scheme,
}

impl TextDoc {
    fn new(text: String, size: u64, path: &str, scheme: bachuc_theme::Scheme) -> TextDoc {
        let mut starts = vec![0usize];
        starts.extend(text.match_indices('\n').map(|(at, _)| at + 1));
        let longest = (0..starts.len())
            .max_by_key(|&index| line_range(&starts, text.len(), index).len())
            .unwrap_or(0);
        let language = if size <= HIGHLIGHT_LIMIT {
            language_for(path)
        } else {
            None
        };
        TextDoc {
            text,
            starts,
            runs: Vec::new(),
            longest,
            size,
            language,
            scheme,
        }
    }

    fn line_count(&self) -> usize {
        self.starts.len()
    }

    fn line(&self, index: usize) -> &str {
        &self.text[line_range(&self.starts, self.text.len(), index)]
    }
}

/// The text of line `index` without its line break.
fn line_range(starts: &[usize], len: usize, index: usize) -> Range<usize> {
    let start = starts[index];
    let end = starts.get(index + 1).map_or(len, |next| next - 1);
    start..end
}

enum State {
    Loading,
    Failed(String),
    Text(TextDoc),
    Image(Arc<Image>),
    Binary { size: u64 },
    TooLarge { size: u64 },
}

pub struct FileView {
    client: Entity<Client>,
    cwd: String,
    path: String,
    line: Option<u32>,
    state: State,
    scroll: UniformListScrollHandle,
    /// Bumped by each load, so a slow answer for an older load is dropped.
    load: u64,
    _subscriptions: Vec<Subscription>,
}

/// `Image` formats the viewer can draw, by mime type.
fn image_format(mime: &str) -> Option<ImageFormat> {
    Some(match mime {
        "image/png" => ImageFormat::Png,
        "image/jpeg" | "image/jpg" => ImageFormat::Jpeg,
        "image/gif" => ImageFormat::Gif,
        "image/webp" => ImageFormat::Webp,
        "image/bmp" => ImageFormat::Bmp,
        "image/tiff" => ImageFormat::Tiff,
        "image/svg+xml" => ImageFormat::Svg,
        _ => return None,
    })
}

fn is_image_path(path: &str) -> bool {
    path.rsplit_once('.').is_some_and(|(_, extension)| {
        matches!(
            extension.to_lowercase().as_str(),
            "png" | "jpg" | "jpeg" | "gif" | "webp" | "bmp" | "tif" | "tiff" | "svg" | "ico"
        )
    })
}

/// `formatFileSize` of `file-pane/bar.tsx`.
pub fn format_file_size(size: u64) -> String {
    if size < 1024 {
        format!("{size} B")
    } else if size < 1024 * 1024 {
        format!("{:.1} KB", size as f64 / 1024.)
    } else {
        format!("{:.1} MB", size as f64 / (1024. * 1024.))
    }
}

/// `path` relative to `cwd`, which is what the daemon takes.
fn relative_path(cwd: &str, path: &str) -> String {
    path.strip_prefix(cwd)
        .map(|rest| rest.trim_start_matches('/'))
        .filter(|rest| !rest.is_empty())
        .unwrap_or(path)
        .to_string()
}

impl FileView {
    pub fn new(
        client: Entity<Client>,
        cwd: String,
        path: String,
        line: Option<u32>,
        _window: &mut Window,
        cx: &mut Context<FileView>,
    ) -> FileView {
        let path = relative_path(&cwd, &path);
        let mut view = FileView {
            client,
            cwd,
            path,
            line,
            state: State::Loading,
            scroll: UniformListScrollHandle::new(),
            load: 0,
            _subscriptions: vec![cx.observe_global::<Theme>(|this, cx| this.theme_changed(cx))],
        };
        view.reload(cx);
        view
    }

    /// Shows the same file again from the daemon, for example after the file changed.
    pub fn reload(&mut self, cx: &mut Context<FileView>) {
        self.load += 1;
        let load = self.load;
        let cwd = self.cwd.clone();
        let path = self.path.clone();
        // The daemon reads a file of any size, so a large one is found out first, from the
        // listing of its folder, and never downloaded.
        let parent = path
            .rsplit_once('/')
            .map_or(".", |(parent, _)| parent)
            .to_string();
        let listing = self
            .client
            .update(cx, |client, cx| client.list_directory(&cwd, &parent, cx));
        let client = self.client.clone();
        cx.spawn(async move |this, cx| {
            let known_size = listing
                .await
                .ok()
                .and_then(|entries| entries.into_iter().find(|entry| entry.path == path))
                .map(|entry| entry.size);
            let limit = if is_image_path(&path) {
                IMAGE_LIMIT
            } else {
                TEXT_LIMIT
            };
            if let Some(size) = known_size.filter(|size| *size > limit) {
                let _ = this.update(cx, |this, cx| {
                    this.apply(load, Ok(State::TooLarge { size }), cx)
                });
                return;
            }
            let task = client.update(cx, |client, cx| client.read_file(&cwd, &path, cx));
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                let state = result.map(|content| this.state_for(content, cx));
                this.apply(load, state, cx)
            });
        })
        .detach();
        cx.notify();
    }

    /// Scrolls the line (1-based) to the middle of the view.
    pub fn scroll_to_line(&mut self, line: u32, cx: &mut Context<FileView>) {
        self.line = Some(line);
        if let State::Text(doc) = &self.state {
            let index = (line.max(1) as usize - 1).min(doc.line_count() - 1);
            self.scroll
                .scroll_to_item_strict(index, ScrollStrategy::Center);
        }
        cx.notify();
    }

    pub fn path(&self) -> &str {
        &self.path
    }

    fn state_for(&mut self, content: FileContent, cx: &mut Context<FileView>) -> State {
        match content.kind {
            FileKind::Text => {
                let size = content
                    .size
                    .max(content.text.as_ref().map_or(0, |t| t.len() as u64));
                if size > TEXT_LIMIT {
                    return State::TooLarge { size };
                }
                let doc = TextDoc::new(
                    content.text.unwrap_or_default(),
                    size,
                    &self.path,
                    scheme(cx),
                );
                State::Text(doc)
            }
            FileKind::Image => {
                let format = content.mime_type.as_deref().and_then(image_format);
                match (format, content.bytes) {
                    (Some(format), Some(bytes)) if content.size <= IMAGE_LIMIT => {
                        State::Image(Arc::new(Image::from_bytes(format, bytes)))
                    }
                    (_, _) if content.size > IMAGE_LIMIT => State::TooLarge { size: content.size },
                    _ => State::Binary { size: content.size },
                }
            }
            FileKind::Binary => State::Binary { size: content.size },
        }
    }

    fn apply(&mut self, load: u64, state: anyhow::Result<State>, cx: &mut Context<FileView>) {
        if load != self.load {
            return;
        }
        self.state = match state {
            Ok(state) => state,
            Err(error) => {
                log::warn!("read_file {}: {error:#}", self.path);
                State::Failed(format!("{error:#}"))
            }
        };
        if matches!(self.state, State::Text(_)) {
            self.start_highlight(cx);
            if let Some(line) = self.line {
                self.scroll_to_line(line, cx);
            }
        }
        cx.notify();
    }

    fn theme_changed(&mut self, cx: &mut Context<FileView>) {
        let current = scheme(cx);
        if let State::Text(doc) = &self.state
            && doc.scheme != current
        {
            self.start_highlight(cx);
        }
        cx.notify();
    }

    /// Colors the text off the UI thread. The lines draw plain until the runs arrive.
    fn start_highlight(&mut self, cx: &mut Context<FileView>) {
        let State::Text(doc) = &mut self.state else {
            return;
        };
        let scheme = scheme(cx);
        doc.scheme = scheme;
        let Some(language) = doc.language else {
            doc.runs.clear();
            return;
        };
        let text = doc.text.clone();
        let load = self.load;
        let colors = SyntaxColors::for_scheme(scheme);
        cx.spawn(async move |this, cx| {
            let runs = cx
                .background_executor()
                .spawn(async move { highlight(language, &text, &colors) })
                .await;
            let _ = this.update(cx, |this, cx| {
                if this.load != load {
                    return;
                }
                if let State::Text(doc) = &mut this.state
                    && doc.scheme == scheme
                {
                    doc.runs = runs;
                    cx.notify();
                }
            });
        })
        .detach();
    }

    fn render_toolbar(&self, doc: &TextDoc, cx: &App) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let size = format_file_size(doc.size);
        let lines = format!("{} lines", doc.line_count());
        h_flex()
            .id("file-view-bar")
            .role(Role::Status)
            .aria_label(format!("{size}, {lines}"))
            .flex_shrink_0()
            .h(px(TAB_BAR_HEIGHT))
            .px(px(SPACE_3))
            .gap(px(SPACE_2))
            .text_size(ui_font_size(FONT_SM, cx))
            .text_color(colors.foreground_extra_muted)
            .child(size)
            .child(lines)
    }

    fn render_lines(
        &mut self,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let State::Text(doc) = &self.state else {
            return Vec::new();
        };
        let colors = tokens(cx);
        let (size, family) = code_font(cx);
        let size_px: f32 = size.into();
        let gutter = gutter_text_width(doc.line_count(), size_px);
        let line_height = px(size_px * LINE_HEIGHT_FACTOR);
        let target = self.line.map(|line| line as usize);
        range
            .filter(|&index| index < doc.line_count())
            .map(|index| {
                let text = doc.line(index);
                let runs = doc.runs.get(index).map_or(&[][..], Vec::as_slice);
                let label: String = text.chars().take(200).collect();
                h_flex()
                    .id(("file-line", index))
                    .role(Role::Code)
                    .aria_label(format!("Line {}: {label}", index + 1))
                    .h(line_height)
                    .font_family(family)
                    .text_size(size)
                    .line_height(line_height)
                    .text_color(colors.foreground)
                    .when(target == Some(index + 1), |this| {
                        this.bg(colors.interaction_highlight)
                    })
                    .child(
                        div()
                            .w(px(gutter))
                            .h_full()
                            .flex_shrink_0()
                            .pl(px(GUTTER_PAD_LEFT))
                            .pr(px(GUTTER_PAD_RIGHT))
                            .text_right()
                            .text_color(colors.foreground_muted)
                            .border_r_1()
                            .border_color(colors.border)
                            .child((index + 1).to_string()),
                    )
                    .child(
                        div()
                            .pl(px(LINE_PAD_LEFT))
                            .pr(px(SPACE_2))
                            .whitespace_nowrap()
                            .child(code_text(text, runs)),
                    )
                    .into_any_element()
            })
            .collect()
    }
}

/// The gutter: digits for the last line number, at least two, and the element padding.
fn gutter_text_width(lines: usize, size: f32) -> f32 {
    // `gutter_width` adds 12 px for the diff gutter; the editor gutter has 8 px of padding.
    gutter_width(lines as u32, size) - 12. + GUTTER_PAD_LEFT + GUTTER_PAD_RIGHT
}

impl Render for FileView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let root = div()
            .id("file-view")
            .role(Role::Pane)
            .aria_label(format!("File {}", self.path))
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.surface0)
            .text_color(colors.foreground);
        match &self.state {
            State::Loading => root.child(centered_text(
                "file-view-loading",
                "Loading file...",
                false,
                cx,
            )),
            State::Failed(message) => {
                root.child(centered_text("file-view-error", message.clone(), true, cx))
            }
            State::TooLarge { size } => root.child(file_meta_state(
                "file-view-too-large",
                "This file is too large to display",
                Some(format_file_size(*size)),
                cx,
            )),
            State::Binary { size } => root.child(file_meta_state(
                "file-view-binary",
                "Binary preview unavailable",
                Some(format_file_size(*size)),
                cx,
            )),
            State::Image(image) => root.child(
                div()
                    .id("file-view-image")
                    .role(Role::Image)
                    .aria_label(self.path.clone())
                    .flex_1()
                    .min_h_0()
                    .p(px(SPACE_3))
                    .child(
                        img(image.clone())
                            .size_full()
                            .object_fit(ObjectFit::Contain)
                            .rounded(px(RADIUS_SM)),
                    ),
            ),
            State::Text(doc) => {
                let bar = self.render_toolbar(doc, cx);
                let count = doc.line_count();
                let longest = doc.longest;
                root.child(bar).child(
                    div()
                        .id("file-view-text")
                        .role(Role::Document)
                        .aria_label(format!("Contents of {}", self.path))
                        .flex_1()
                        .min_h_0()
                        .child(
                            uniform_list(
                                "file-view-lines",
                                count,
                                cx.processor(Self::render_lines),
                            )
                            .track_scroll(&self.scroll)
                            .with_horizontal_sizing_behavior(
                                ListHorizontalSizingBehavior::Unconstrained,
                            )
                            .with_width_from_item(Some(longest))
                            .size_full()
                            .py(px(CONTENT_PADDING)),
                        ),
                )
            }
        }
    }
}
