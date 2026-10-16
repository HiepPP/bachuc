//! The turn diff card of `client/turn-diff-card.tsx`: a header, the shared-turn warning and
//! commit rows, file rows with a diffstat bar, and the `and N more` footer. The timeline
//! draws it as a `thread-branch-turn-diff` item and the Turn diff panel draws it per turn.

use crate::selection::{FileSelection, Selection, selected_row};
use crate::turn::{
    Block, FileKind, FileRequest, TurnDiff, TurnFile, TurnSource, bar, file_kind,
    file_row_pressable, plural, split_path,
};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::TimelineRenderInput;
use bachuc_theme::metrics::{
    BORDER_WIDTH_2, FONT_BASE, FONT_SM, ICON_MD, ICON_SM, RADIUS_BASE, SPACE_1, SPACE_1_5, SPACE_2,
    SPACE_3, WEIGHT_MEDIUM, WEIGHT_SEMIBOLD,
};
use bachuc_theme::{Tokens, mono_font_family, tokens, ui_font_size};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

/// `borderRadius` of the card (`turn-diff-card.tsx:41`); no token has 10.
const CARD_RADIUS: f32 = 10.;
/// The header's vertical padding (`turn-diff-card.tsx:54`).
const HEADER_PADDING_Y: f32 = 10.;
/// The size of path, subject, and name text (`turn-diff-card.tsx:104`).
const ROW_TEXT: f32 = 13.;
/// The shared-turn icon (`turn-diff-card.tsx:287`).
const WARNING_ICON: f32 = 13.;
/// `styles.block`: a diffstat block is 7 px with a 1.5 px radius, 2 px apart.
const BLOCK_SIZE: f32 = 7.;
const BLOCK_RADIUS: f32 = 1.5;
const BLOCK_GAP: f32 = 2.;
/// `styles.counts` `minWidth`: keeps the bars of all rows in line.
const COUNTS_MIN_WIDTH: f32 = 72.;
/// `styles.counts` `gap`.
const COUNTS_GAP: f32 = 6.;
/// `styles.badge` font size.
const BADGE_TEXT: f32 = 11.;
/// A selected row keeps its text in line: 2 px accent border plus 10 px, as 12 px of padding.
const SELECTED_PADDING: f32 = 10.;
const UNAVAILABLE: &str = "Plugin timeline item unavailable.";

/// Opens a file's diff: the pressed selection and the id of the turn's workspace.
pub(crate) type OpenFile = Rc<dyn Fn(FileSelection, &str, &mut Window, &mut App)>;

pub(crate) struct CardProps<'a> {
    pub agent_id: &'a str,
    pub data: &'a TurnDiff,
    pub selected: Option<&'a FileSelection>,
    /// Keeps element ids apart when one window shows several cards.
    pub scope: &'a str,
    pub open: &'a OpenFile,
}

struct RowStyle {
    t: Tokens,
    mono: &'static str,
    text: Pixels,
}

fn selection_for(agent_id: &str, source: &TurnSource, path: &str) -> FileSelection {
    FileSelection {
        agent_id: agent_id.to_string(),
        file: FileRequest {
            root: source.root.clone(),
            from: source.from.clone(),
            to: source.to.clone(),
            path: path.to_string(),
        },
    }
}

fn marker(text: String, color: Hsla, mono: &'static str, size: Pixels) -> impl IntoElement {
    div()
        .flex_none()
        .font_family(mono)
        .text_size(size)
        .font_weight(FontWeight(WEIGHT_SEMIBOLD))
        .text_color(color)
        .child(text)
}

pub(crate) fn render_card(props: CardProps<'_>, cx: &App) -> AnyElement {
    let CardProps {
        agent_id,
        data,
        selected,
        scope,
        open,
    } = props;
    let t = tokens(cx);
    let style = RowStyle {
        t,
        mono: mono_font_family(cx),
        text: ui_font_size(FONT_SM, cx),
    };
    let hidden = (data.file_count as usize).saturating_sub(data.files.len());
    let commits = data.commits.len();
    let source = data.source.as_ref();

    let header = h_flex()
        .w_full()
        .items_center()
        .gap(px(SPACE_2))
        .px(px(SPACE_3))
        .py(px(HEADER_PADDING_Y))
        .child(
            icon(IconName::FileDiff)
                .with_size(px(ICON_MD))
                .text_color(t.foreground_muted),
        )
        .child(
            div()
                .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(t.foreground)
                .child(format!(
                    "{} changed",
                    plural(data.file_count as usize, "file", "files")
                )),
        )
        .when(data.file_count > 0, |header| {
            header
                .child(marker(
                    format!("+{}", data.added),
                    t.diff_addition,
                    style.mono,
                    style.text,
                ))
                .child(marker(
                    format!("-{}", data.deleted),
                    t.diff_deletion,
                    style.mono,
                    style.text,
                ))
        })
        .child(div().flex_1())
        .when(commits > 0, |header| {
            header.child(
                div()
                    .text_size(style.text)
                    .text_color(t.foreground_muted)
                    .child(plural(commits, "commit", "commits")),
            )
        });

    let notices = (data.shared || commits > 0).then(|| {
        v_flex()
            .w_full()
            .border_t_1()
            .border_color(t.border)
            .bg(t.surface0)
            .when(data.shared, |section| {
                section.child(
                    h_flex()
                        .items_center()
                        .gap(px(SPACE_1_5))
                        .px(px(SPACE_3))
                        .py(px(SPACE_1_5))
                        .child(
                            icon(IconName::TriangleAlert)
                                .with_size(px(WARNING_ICON))
                                .text_color(t.status_warning),
                        )
                        .child(
                            div()
                                .text_size(style.text)
                                .text_color(t.status_warning)
                                .child("May include changes from another agent"),
                        ),
                )
            })
            .children(data.commits.iter().map(|commit| {
                h_flex()
                    .items_center()
                    .gap(px(SPACE_2))
                    .px(px(SPACE_3))
                    .py(px(SPACE_1))
                    .child(
                        icon(IconName::GitCommitHorizontal)
                            .with_size(px(ICON_SM))
                            .text_color(t.foreground_muted),
                    )
                    .child(
                        div()
                            .flex_none()
                            .font_family(style.mono)
                            .text_size(style.text)
                            .text_color(t.accent)
                            .child(commit.sha.clone()),
                    )
                    .child(
                        div()
                            .flex_shrink(1.)
                            .min_w_0()
                            .truncate()
                            .text_size(ui_font_size(ROW_TEXT, cx))
                            .text_color(t.foreground)
                            .child(commit.subject.clone()),
                    )
            }))
    });

    let files =
        (!data.files.is_empty()).then(|| {
            v_flex()
                .w_full()
                .border_t_1()
                .border_color(t.border)
                .bg(t.surface0)
                .children(data.files.iter().map(|file| {
                    file_row(file, agent_id, source, selected, scope, open, &style, cx)
                }))
        });

    let footer = (hidden > 0).then(|| {
        h_flex()
            .w_full()
            .items_center()
            .gap(px(SPACE_1_5))
            .px(px(SPACE_3))
            .py(px(SPACE_2))
            .border_t_1()
            .border_color(t.border)
            .text_size(style.text)
            .text_color(t.foreground_muted)
            .child(format!(
                "and {hidden} more {} not listed",
                if hidden == 1 { "file" } else { "files" }
            ))
    });

    v_flex()
        .id(ElementId::Name(format!("{scope}/card").into()))
        .role(Role::Group)
        .aria_label(crate::turn::turn_diff_header(data))
        .w_full()
        .border_1()
        .border_color(t.border)
        .rounded(px(CARD_RADIUS))
        .bg(t.surface1)
        .overflow_hidden()
        .child(header)
        .children(notices)
        .children(files)
        .children(footer)
        .into_any_element()
}

#[allow(clippy::too_many_arguments)]
fn file_row(
    file: &TurnFile,
    agent_id: &str,
    source: Option<&TurnSource>,
    selected: Option<&FileSelection>,
    scope: &str,
    open: &OpenFile,
    style: &RowStyle,
    cx: &App,
) -> AnyElement {
    let t = style.t;
    let pressable = file_row_pressable(source, file);
    let is_selected = selected_row(selected, agent_id, source, &file.path);
    let (dir, name) = split_path(&file.path);
    let kind = match file_kind(file) {
        FileKind::Image => IconName::FileImage,
        FileKind::Code => IconName::FileCode,
        FileKind::Binary => IconName::File,
    };
    let group = SharedString::from(format!("{scope}/{}", file.path));
    let text = ui_font_size(ROW_TEXT, cx);

    let path = h_flex()
        .flex_1()
        .min_w_0()
        .child(
            div()
                .flex_shrink(1.)
                .min_w_0()
                .overflow_hidden()
                .whitespace_nowrap()
                .text_ellipsis_start()
                .text_size(text)
                .text_color(t.foreground_muted)
                .child(dir.to_string()),
        )
        .child(
            div()
                .flex_none()
                .text_size(text)
                .font_weight(FontWeight(WEIGHT_MEDIUM))
                .text_color(t.foreground)
                .child(name.to_string()),
        );

    let counts = h_flex()
        .min_w(px(COUNTS_MIN_WIDTH))
        .justify_end()
        .gap(px(COUNTS_GAP))
        .map(|counts| match file.added {
            None => counts.child(
                div()
                    .flex_none()
                    .font_family(style.mono)
                    .text_size(px(BADGE_TEXT))
                    .text_color(t.foreground_muted)
                    .border_1()
                    .border_color(t.border)
                    .rounded(px(RADIUS_BASE))
                    .px(px(SPACE_1))
                    .child(if file.large {
                        "large, not saved"
                    } else {
                        "binary"
                    }),
            ),
            Some(added) => counts
                .when(added > 0, |counts| {
                    counts.child(marker(
                        format!("+{added}"),
                        t.diff_addition,
                        style.mono,
                        style.text,
                    ))
                })
                .when(file.deleted.unwrap_or(0) > 0, |counts| {
                    counts.child(marker(
                        format!("-{}", file.deleted.unwrap_or(0)),
                        t.diff_deletion,
                        style.mono,
                        style.text,
                    ))
                }),
        });

    let blocks = h_flex()
        .flex_none()
        .gap(px(BLOCK_GAP))
        .children(bar(file).into_iter().map(|block| {
            div()
                .size(px(BLOCK_SIZE))
                .rounded(px(BLOCK_RADIUS))
                .bg(match block {
                    Block::Add => t.diff_addition,
                    Block::Remove => t.diff_deletion,
                    Block::None => t.border,
                })
        }));

    // Hover changes the background and this color only; the layout never changes.
    let chevron = div()
        .flex_none()
        .text_color(if is_selected {
            t.accent
        } else {
            t.foreground_muted
        })
        .group_hover(group.clone(), move |s| s.text_color(t.accent))
        .child(icon(IconName::ChevronRight).with_size(px(ICON_SM)));

    let row = h_flex()
        .id(ElementId::Name(group.clone()))
        .group(group)
        .w_full()
        .items_center()
        .gap(px(SPACE_2))
        .pr(px(SPACE_3))
        .py(px(SPACE_1_5))
        .map(|row| {
            if is_selected {
                row.pl(px(SELECTED_PADDING))
                    .border_l(px(BORDER_WIDTH_2))
                    .border_color(t.accent)
                    .bg(t.surface2)
            } else {
                row.pl(px(SPACE_3))
            }
        })
        .child(
            icon(kind)
                .with_size(px(ICON_SM))
                .text_color(t.foreground_muted),
        )
        .child(path)
        .child(counts)
        .child(blocks);

    match (pressable, source) {
        (true, Some(source)) => {
            let selection = selection_for(agent_id, source, &file.path);
            let workspace_id = source.workspace_id.clone();
            let open = open.clone();
            row.child(chevron)
                .role(Role::Button)
                .aria_label(format!("Open the turn diff of {}", file.path))
                .cursor_pointer()
                .hover(move |row| row.bg(t.surface2))
                .on_click(move |_, window, cx| {
                    open(selection.clone(), &workspace_id, window, cx);
                })
                .into_any_element()
        }
        _ => row
            .role(Role::ListItem)
            .aria_label(file.path.clone())
            .into_any_element(),
    }
}

/// The card in the timeline: the item data decoded once, redrawn when the open file changes.
pub(crate) struct TurnCardView {
    agent_id: String,
    scope: String,
    data: Option<TurnDiff>,
    selection: Selection,
    open: OpenFile,
    _selection: Subscription,
}

impl TurnCardView {
    pub(crate) fn new(
        input: TimelineRenderInput,
        selection: Selection,
        open: OpenFile,
        cx: &mut Context<Self>,
    ) -> Self {
        let data: Option<TurnDiff> = serde_json::from_value(input.item.data.clone())
            .map_err(|error| log::warn!("thread-branch-turn-diff item: {error}"))
            .ok();
        let key = input
            .item
            .id
            .clone()
            .or_else(|| {
                data.as_ref()
                    .and_then(|data| data.source.as_ref())
                    .map(|source| source.to.clone())
            })
            .unwrap_or_default();
        Self {
            scope: format!("timeline/{}/{key}", input.agent_id),
            agent_id: input.agent_id,
            data,
            _selection: selection.observe(cx, |_, cx| cx.notify()),
            selection,
            open,
        }
    }
}

impl Render for TurnCardView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(data) = &self.data else {
            let t = tokens(cx);
            return div()
                .id("thread-branch-turn-diff-unavailable")
                .role(Role::Note)
                .aria_label(UNAVAILABLE)
                .py(px(SPACE_2))
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(t.foreground_muted)
                .child(UNAVAILABLE)
                .into_any_element();
        };
        let selected = self.selection.get();
        render_card(
            CardProps {
                agent_id: &self.agent_id,
                data,
                selected: selected.as_ref(),
                scope: &self.scope,
                open: &self.open,
            },
            cx,
        )
    }
}
