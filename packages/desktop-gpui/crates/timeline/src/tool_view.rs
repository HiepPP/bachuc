//! Tool call badges and their detail views, as `ExpandableBadge` and `ToolCall` in
//! `packages/app/src/components/message.tsx` and `ToolCallDetailsContent` in
//! `tool-call-details.tsx`, and the edit rows of a step, as the edit-diffs plugin draws them.

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use bachuc_explorer::syntax::{self, Run, SyntaxColors};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_CODE, FONT_SM, ICON_SM, ICON_XS, RADIUS_BASE, RADIUS_LG, RADIUS_MD, SPACE_1,
    SPACE_2, SPACE_3, SPACE_4,
};
use bachuc_theme::{
    Scheme, Tokens, code_font_size, mono_font_family, scheme, tokens, ui_font_size,
};
use gpui_kit::base::input::HighlightStyleResolver;
use gpui_kit::component::highlighter::SyntaxHighlighter;
use gpui_kit::component::{Rope, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::Value;

use crate::diff::{self, DiffLine, LineKind};
use crate::grouping::entry_key;
use crate::tools::{self, Section};
use crate::{LineRuns, TimelineEvent, TimelineView, short_path};

/// The badge reaches past the stream padding, as `marginHorizontal: -13`.
pub(crate) const BADGE_BLEED: f32 = 13.;
const ICON_SLOT: f32 = 22.;
pub(crate) const CODE_LINE_HEIGHT: f32 = 18.;
/// `ToolCall`'s default `maxDetailHeight`.
pub(crate) const TOOL_DETAIL_MAX_HEIGHT: f32 = 400.;
/// Lines one code section lays out; the rest is cut so a huge output stays cheap to draw.
const MAX_CODE_LINES: usize = 2000;
/// The edit-diffs plugin: change bar squares, their size, and the diff height.
const CHANGE_BAR_SQUARES: usize = 7;
const BAR_SQUARE: f32 = 7.;
const MAX_DIFF_HEIGHT: f32 = 480.;
const EDIT_PATH_SIZE: f32 = 12.5;
const EDIT_STAT_SIZE: f32 = 13.;
/// The diff line tints of `diff-viewer.tsx` and the edit-diffs plugin, over the status tokens.
const ADD_TINT: f32 = 0.15;
const REMOVE_TINT: f32 = 0.1;
/// The label of a running tool call, as `labelLoading`.
const LOADING_OPACITY: f32 = 0.72;

/// `changeBar`: the squares split by the added share; a side that changed keeps one square.
pub fn change_bar(added: usize, removed: usize) -> Vec<bool> {
    let total = added + removed;
    if total == 0 {
        return Vec::new();
    }
    let mut add = ((added as f32 / total as f32) * CHANGE_BAR_SQUARES as f32).round() as usize;
    if added > 0 && removed > 0 {
        add = add.clamp(1, CHANGE_BAR_SQUARES - 1);
    }
    (0..CHANGE_BAR_SQUARES).map(|index| index < add).collect()
}

/// The edit row status word, as `headerLabel` of the edit-diffs plugin. A completed call has
/// none.
pub fn edit_status_label(item: &Value) -> Option<String> {
    let is_write = item["detail"]["type"] == "write";
    let verb = if is_write { "Write" } else { "Edit" };
    match item["status"].as_str().unwrap_or("") {
        "running" | "executing" => Some(if is_write { "Writing" } else { "Editing" }.into()),
        "failed" => Some(format!("{verb} failed")),
        "canceled" => Some(format!("{verb} canceled")),
        _ => None,
    }
}

/// A path under the agent folder without the folder, as `displayPath`.
pub fn display_path(path: &str, cwd: &str) -> String {
    let root = cwd.trim_end_matches('/');
    if root.is_empty() {
        return path.to_string();
    }
    [format!("{root}/"), format!("/private{root}/")]
        .iter()
        .find_map(|prefix| path.strip_prefix(prefix.as_str()))
        .map_or_else(|| path.to_string(), str::to_string)
}

/// The tree-sitter language of a fence info string such as `rust` or `ts {1,3}`, with the
/// aliases of `highlighted-code-block.tsx`.
pub(crate) fn fence_language(info: &str) -> Option<&'static str> {
    let first = info.split_whitespace().next()?.trim_start_matches('.');
    let first = first.to_lowercase();
    let extension = match first.as_str() {
        "typescript" => "ts",
        "javascript" => "js",
        "python" => "py",
        "rust" => "rs",
        "shell" => "sh",
        other => other,
    };
    syntax::language_for(&format!("fence.{extension}"))
}

/// `oneLight` and `oneDark` of `packages/highlight/src/themes.ts`: the app colors code with the
/// "one" theme unless the user picks another (`use-settings/storage.ts`).
pub(crate) fn one_colors(scheme: Scheme) -> SyntaxColors {
    let c = |hex| Hsla::from(rgb(hex));
    // base, keyword, comment, string, number, function, type, tag, attribute, operator.
    let [
        base,
        keyword,
        comment,
        string,
        number,
        function,
        type_,
        tag,
        attribute,
        operator,
    ] = match scheme {
        Scheme::Light => [
            0x383a42, 0xa626a4, 0xa0a1a7, 0x50a14f, 0x986801, 0x4078f2, 0xc18401, 0xe45649,
            0x986801, 0x0184bc,
        ],
        Scheme::Dark => [
            0xabb2bf, 0xc678dd, 0x5c6370, 0x98c379, 0xd19a66, 0x61afef, 0xe5c07b, 0xe06c75,
            0xd19a66, 0x56b6c2,
        ],
    }
    .map(c);
    // `expandRolePalette`.
    SyntaxColors {
        keyword,
        comment,
        string,
        number,
        literal: number,
        function,
        class: type_,
        type_,
        tag,
        attribute,
        property: attribute,
        variable: base,
        operator,
        punctuation: base,
        regexp: string,
        escape: number,
        meta: comment,
        heading: function,
        link: string,
    }
}

/// The app's lezer grammars give a macro name no role, so `println!` keeps the text color.
struct OneColors(SyntaxColors);

impl HighlightStyleResolver for OneColors {
    fn style(&self, name: &str) -> Option<HighlightStyle> {
        if name == "function.macro" || name.starts_with("function.macro.") {
            return None;
        }
        self.0.style(name)
    }
}

/// Colored ranges of `text` parsed as `language`, in the "one" colors.
pub(crate) fn code_styles(language: &str, text: &str, scheme: Scheme) -> Vec<Run> {
    let mut highlighter = SyntaxHighlighter::new(language);
    highlighter.update(None, &Rope::from_str(text), None);
    highlighter.styles(&(0..text.len()), &OneColors(one_colors(scheme)))
}

/// A small square icon button with a role and a label.
pub(crate) fn icon_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    name: IconName,
    color: Hsla,
    t: &Tokens,
) -> Stateful<Div> {
    let fill = t.interaction_highlight;
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .flex()
        .flex_none()
        .items_center()
        .justify_center()
        .p(px(SPACE_1))
        .rounded(px(RADIUS_MD))
        .cursor_pointer()
        .hover(move |button| button.bg(fill))
        .child(icon(name).with_size(px(ICON_SM)).text_color(color))
}

impl TimelineView {
    /// Colored runs of each line of `text`, cached by text and scheme. None when the path has no
    /// grammar.
    fn syntax_runs(&self, text: &str, path: Option<&str>, cx: &App) -> Option<LineRuns> {
        let language = syntax::language_for(path?)?;
        let scheme = scheme(cx);
        let mut hasher = DefaultHasher::new();
        text.hash(&mut hasher);
        language.hash(&mut hasher);
        (scheme as u8).hash(&mut hasher);
        let key = hasher.finish();
        let mut cache = self.syntax_cache.borrow_mut();
        if let Some(runs) = cache.get(&key) {
            return Some(runs.clone());
        }
        // Only the open details ask, so the cache holds a handful; drop it when it grows.
        if cache.len() > 64 {
            cache.clear();
        }
        let runs = Rc::new(syntax::split_lines(
            text,
            &code_styles(language, text, scheme),
        ));
        cache.insert(key, runs.clone());
        Some(runs)
    }

    /// Runs per diff line: the old and new sides parse as their own documents, as
    /// `highlightDiffLines` does, and each line takes the runs of its side.
    fn diff_runs(&self, lines: &[DiffLine], path: Option<&str>, cx: &App) -> Vec<Vec<Run>> {
        let side = |skip: LineKind| -> (String, Vec<usize>) {
            let mut text = String::new();
            let mut rows = Vec::new();
            for (index, line) in lines.iter().enumerate() {
                if line.kind == skip || line.kind == LineKind::Hunk {
                    continue;
                }
                if !rows.is_empty() {
                    text.push('\n');
                }
                text.push_str(&line.text);
                rows.push(index);
            }
            (text, rows)
        };
        let mut out = vec![Vec::new(); lines.len()];
        for skip in [LineKind::Add, LineKind::Remove] {
            let (text, rows) = side(skip);
            let Some(runs) = self.syntax_runs(&text, path, cx) else {
                return out;
            };
            for (row, line_runs) in rows.iter().zip(runs.iter()) {
                // Context lines are on both sides; the new side wins.
                if skip == LineKind::Remove || lines[*row].kind == LineKind::Remove {
                    out[*row] = line_runs.clone();
                }
            }
        }
        out
    }

    /// A standalone tool call: the plan card for a plan, else the expandable badge.
    pub(crate) fn render_tool_call(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let entry = &self.entries[index];
        let item = &entry["item"];
        if item["detail"]["type"] == "plan" {
            return self.render_plan_tool(entry_key(entry), item, cx);
        }
        self.render_badge(entry_key(entry), item, cx)
    }

    /// A reasoning entry outside a turn, drawn as the app draws it: a "Thinking" badge.
    pub(crate) fn render_thought(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let entry = &self.entries[index];
        let item = serde_json::json!({
            "name": "thinking",
            "status": "completed",
            "detail": {"type": "unknown", "input": entry["item"]["text"], "output": null},
        });
        self.render_badge(entry_key(entry), &item, cx)
    }

    /// `ExpandableBadge`: icon, name, and summary; a click opens the details below it.
    pub(crate) fn render_badge(
        &self,
        key: u64,
        item: &Value,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let shown = tools::presentation(item, &self.cwd);
        let sections = tools::sections(&item["detail"], shown.error.as_deref());
        let can_open = !sections.is_empty();
        let expanded = can_open && self.expanded.contains(&key);
        let status = item["status"].as_str().unwrap_or("");
        let is_loading = matches!(status, "running" | "executing");
        let is_error = status == "failed";
        let group: SharedString = format!("badge-{key}").into();
        let size = ui_font_size(FONT_BASE, cx);

        let tool_icon = if is_error {
            icon(IconName::TriangleAlert)
                .with_size(px(ICON_XS))
                .text_color(t.destructive.opacity(0.8))
        } else {
            icon(shown.icon)
                .with_size(px(ICON_XS))
                .text_color(if expanded {
                    t.foreground
                } else {
                    t.foreground_muted
                })
        };
        let chevron = icon(if expanded {
            IconName::ChevronDown
        } else {
            IconName::ChevronRight
        })
        .with_size(px(ICON_SM))
        .text_color(t.foreground);
        // The tool icon turns into the chevron on hover or while open. Hover swaps visibility
        // of two stacked layers, never `display`: GPUI hit-tests hover in prepaint against the
        // last frame's hitbox ids and again in paint against this frame's, and a div whose
        // display turns on between the two paints children it never prepainted, which panics
        // with "must call prepaint before paint".
        let layer = || {
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
        };
        let icon_slot = div()
            .relative()
            .size(px(ICON_SLOT))
            .flex_none()
            .mr(px(SPACE_1))
            .flex()
            .items_center()
            .justify_center()
            .map(|slot| {
                if !can_open {
                    slot.child(tool_icon)
                } else if expanded {
                    slot.child(chevron)
                } else {
                    slot.child(
                        layer()
                            .group_hover(group.clone(), |s| s.invisible())
                            .child(tool_icon),
                    )
                    .child(
                        layer()
                            .invisible()
                            .group_hover(group.clone(), |s| s.visible())
                            .child(chevron),
                    )
                }
            });
        let active = |base: Hsla| if expanded { t.foreground } else { base };
        let label = div()
            .flex_none()
            .text_color(active(t.foreground_muted))
            .when(is_loading, |label| {
                label.text_color(t.foreground.opacity(LOADING_OPACITY))
            })
            .group_hover(group.clone(), |s| s.text_color(t.foreground))
            .child(shown.name.clone());
        let summary = shown.summary.clone().map(|summary| {
            div()
                .flex_shrink(1.)
                .min_w_0()
                .ml(px(SPACE_2))
                .truncate()
                .text_color(active(t.foreground_muted))
                .group_hover(group.clone(), |s| s.text_color(t.foreground))
                .child(summary)
        });
        let open_file = shown.file_path.clone().map(|path| {
            let cwd = self.cwd.clone();
            icon_button(
                ("open-file", key),
                "Open file",
                IconName::FileSymlink,
                t.foreground_muted,
                &t,
            )
            .ml(px(SPACE_1))
            .invisible()
            .group_hover(group.clone(), |s| s.visible())
            .on_click(cx.listener(move |_, _, _, cx| {
                cx.stop_propagation();
                cx.emit(TimelineEvent::OpenFile {
                    cwd: cwd.clone(),
                    path: path.clone(),
                    line: None,
                });
            }))
        });
        let mut aria = shown.name.clone();
        if let Some(summary) = &shown.summary {
            aria.push_str(&format!(": {summary}"));
        }
        if is_error {
            aria.push_str(", failed");
        } else if is_loading {
            aria.push_str(", running");
        }

        let header = h_flex()
            .id(("badge", key))
            .role(Role::Button)
            .aria_label(aria.clone())
            .when(can_open, |header| header.aria_expanded(expanded))
            .items_center()
            .px(px(SPACE_2))
            .py(px(SPACE_1))
            .rounded(px(RADIUS_LG))
            .border_1()
            .border_color(transparent_black())
            .when(expanded, |header| {
                header
                    .bg(t.surface1)
                    .border_color(t.border)
                    .rounded_b(px(0.))
            })
            .when(can_open, |header| {
                header
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle_expanded(key, cx)))
            })
            .text_size(size)
            .child(icon_slot)
            .child(label)
            .children(summary)
            .children(open_file);
        let details = expanded.then(|| {
            div()
                .id(("badge-detail", key))
                .role(Role::Group)
                .aria_label(format!("{} details", shown.name))
                .rounded_b(px(RADIUS_LG))
                .border_1()
                .border_t_0()
                .border_color(t.border)
                .overflow_hidden()
                .child(self.render_sections(
                    key,
                    &sections,
                    &item["detail"],
                    TOOL_DETAIL_MAX_HEIGHT,
                    cx,
                ))
        });
        v_flex()
            .id(("tool", key))
            .group(group)
            .role(Role::Group)
            .aria_label(format!("Tool call {aria}"))
            .mx(px(-BADGE_BLEED))
            .child(header)
            .children(details)
            .into_any_element()
    }

    /// The sections of a tool detail, as `buildDetailSections` lays them out.
    pub(crate) fn render_sections(
        &self,
        key: u64,
        sections: &[Section],
        detail: &Value,
        max_height: f32,
        cx: &App,
    ) -> AnyElement {
        let t = tokens(cx);
        let full_bleed = matches!(detail["type"].as_str(), Some("edit" | "shell" | "write"));
        let path = detail["filePath"].as_str();
        v_flex()
            .gap(px(if full_bleed { SPACE_2 } else { SPACE_4 }))
            .children(sections.iter().enumerate().map(|(position, section)| {
                let id = ElementId::Name(format!("section-{key}-{position}").into());
                match section {
                    Section::Shell { command, output } => {
                        let mut text = format!("$ {command}");
                        if !output.is_empty() {
                            text.push_str("\n\n");
                            text.push_str(output);
                        }
                        let prompt = HighlightStyle {
                            color: Some(t.foreground_muted),
                            ..Default::default()
                        };
                        self.code_area(id, max_height, true, &t, cx)
                            .child(
                                StyledText::new(cut_lines(&text)).with_highlights([(0..2, prompt)]),
                            )
                            .into_any_element()
                    }
                    Section::Code {
                        text,
                        path: code_path,
                        start_line,
                        full_bleed,
                    } => self
                        .code_lines(
                            id,
                            text,
                            code_path.as_deref(),
                            *start_line,
                            max_height,
                            *full_bleed,
                            &t,
                            cx,
                        )
                        .into_any_element(),
                    Section::Diff(lines) => self
                        .render_detail_diff(id, lines, path, max_height, &t, cx)
                        .into_any_element(),
                    Section::SubAgent {
                        session,
                        actions,
                        log,
                    } => {
                        let mono = mono_font_family(cx);
                        self.code_area(id, max_height, true, &t, cx)
                            .children(session.clone().map(|session| {
                                div()
                                    .mb(px(SPACE_2))
                                    .text_color(t.foreground_muted)
                                    .child(format!("session {session}"))
                            }))
                            .when(!actions.is_empty(), |area| {
                                area.child(v_flex().gap(px(SPACE_1)).mb(px(SPACE_2)).children(
                                    actions.iter().map(|(tool, summary)| {
                                        h_flex()
                                            .gap(px(SPACE_2))
                                            .font_family(mono)
                                            .child(
                                                div()
                                                    .text_color(t.foreground_muted)
                                                    .child(tool.clone()),
                                            )
                                            .children(summary.clone())
                                    }),
                                ))
                            })
                            .when(!log.is_empty(), |area| area.child(cut_lines(log)))
                            .into_any_element()
                    }
                    Section::Plain(text) => div()
                        .id(id)
                        .max_h(px(max_height))
                        .overflow_y_scroll()
                        .p(px(SPACE_3))
                        .rounded(px(RADIUS_BASE))
                        .border_1()
                        .border_color(t.border)
                        .bg(t.surface2)
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .line_height(px(22.))
                        .text_color(t.foreground)
                        .child(text.clone())
                        .into_any_element(),
                    Section::Header(title) => div()
                        .px(px(SPACE_3))
                        .py(px(SPACE_2))
                        .border_b_1()
                        .border_color(t.border)
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .text_color(t.foreground_muted)
                        .child(title.clone())
                        .into_any_element(),
                    Section::Error(error) => v_flex()
                        .gap(px(SPACE_2))
                        .child(
                            div()
                                .text_size(ui_font_size(FONT_SM, cx))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(t.destructive)
                                .child("ERROR"),
                        )
                        .child(
                            div()
                                .id(id)
                                .overflow_x_scroll()
                                .p(px(SPACE_3))
                                .rounded(px(RADIUS_BASE))
                                .border_1()
                                .border_color(t.destructive)
                                .bg(t.surface2)
                                .font_family(mono_font_family(cx))
                                .text_size(code_font_size(FONT_CODE, cx))
                                .line_height(px(CODE_LINE_HEIGHT))
                                .whitespace_nowrap()
                                .text_color(t.destructive)
                                .child(error.clone()),
                        )
                        .into_any_element(),
                }
            }))
            .into_any_element()
    }

    /// A monospace scroll area: full bleed on `surface1`, or a bordered box on `surface2`.
    fn code_area(
        &self,
        id: ElementId,
        max_height: f32,
        full_bleed: bool,
        t: &Tokens,
        cx: &App,
    ) -> Stateful<Div> {
        div()
            .id(id)
            .max_h(px(max_height))
            .overflow_scroll()
            .p(px(SPACE_3))
            .font_family(mono_font_family(cx))
            .text_size(code_font_size(FONT_CODE, cx))
            .line_height(px(CODE_LINE_HEIGHT))
            .whitespace_nowrap()
            .text_color(t.foreground)
            .map(|area| {
                if full_bleed {
                    area.bg(t.surface1)
                } else {
                    area.rounded(px(RADIUS_BASE))
                        .border_1()
                        .border_color(t.border)
                        .bg(t.surface2)
                }
            })
    }

    /// `ScrollableTextSection`: code with syntax colors when the path has a grammar, and a line
    /// gutter when the section starts at a line.
    #[allow(clippy::too_many_arguments)]
    fn code_lines(
        &self,
        id: ElementId,
        text: &str,
        path: Option<&str>,
        start_line: Option<u64>,
        max_height: f32,
        full_bleed: bool,
        t: &Tokens,
        cx: &App,
    ) -> Stateful<Div> {
        let area = self.code_area(id, max_height, full_bleed, t, cx);
        let runs = self.syntax_runs(text, path, cx);
        if runs.is_none() && start_line.is_none() {
            return area.child(cut_lines(text));
        }
        let lines: Vec<&str> = text.lines().take(MAX_CODE_LINES).collect();
        let last = start_line.map(|start| start + lines.len().max(1) as u64 - 1);
        let gutter = last.map_or(0, |last| last.to_string().len());
        let muted = t.foreground_muted;
        area.child(
            v_flex().children(lines.into_iter().enumerate().map(|(index, line)| {
                let line_runs = runs.as_ref().and_then(|runs| runs.get(index)).cloned();
                h_flex()
                    .children(start_line.map(|start| {
                        div()
                            .flex_none()
                            .pr(px(SPACE_3))
                            .text_color(muted)
                            .child(format!("{:>gutter$}", start + index as u64))
                    }))
                    .child(styled_line(line, line_runs.unwrap_or_default()))
            })),
        )
    }

    /// `DiffViewer`: one row per line with its sign, syntax colors, and the line tint.
    fn render_detail_diff(
        &self,
        id: ElementId,
        lines: &[DiffLine],
        path: Option<&str>,
        max_height: f32,
        t: &Tokens,
        cx: &App,
    ) -> Stateful<Div> {
        let runs = self.diff_runs(lines, path, cx);
        let area = div()
            .id(id)
            .max_h(px(max_height))
            .overflow_scroll()
            .bg(t.surface1)
            .p(px(SPACE_3))
            .font_family(mono_font_family(cx))
            .text_size(code_font_size(FONT_CODE, cx))
            .whitespace_nowrap();
        if lines.is_empty() {
            return area
                .text_color(t.foreground_muted)
                .child("No changes to show.");
        }
        area.child(
            v_flex().children(lines.iter().zip(runs).map(|(line, line_runs)| {
                let (sign, bg, color) = match line.kind {
                    LineKind::Add => ("+", t.status_success.opacity(ADD_TINT), t.foreground),
                    LineKind::Remove => ("-", t.status_danger.opacity(REMOVE_TINT), t.foreground),
                    LineKind::Context => (" ", t.surface1, t.foreground_muted),
                    LineKind::Hunk => ("", t.surface1, t.foreground_muted),
                };
                h_flex()
                    .py(px(SPACE_1))
                    .bg(bg)
                    .text_color(color)
                    .child(sign)
                    .child(styled_line(&line.text, line_runs))
            })),
        )
    }

    /// An edit or write call in a step, as the edit-diffs plugin's `EditDiffRow`: a header
    /// button that hides or shows the diff, an open-file button, and the diff.
    pub(crate) fn render_edit(&self, index: usize, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let entry = &self.entries[index];
        let key = entry_key(entry);
        let item = &entry["item"];
        let path = crate::grouping::edited_file_path(entry)
            .unwrap_or("")
            .to_string();
        let shown_path = display_path(&path, &self.cwd);
        let diff = self.diffs.get(&key).cloned().unwrap_or_default();
        let (added, removed) = diff.stats;
        let expanded = !self.hidden_diffs.contains(&key);
        let failed = item["status"] == "failed";
        let mono = mono_font_family(cx);
        let stat = ui_font_size(EDIT_STAT_SIZE, cx);
        let status = edit_status_label(item);

        let toggle = h_flex()
            .id(("edit-toggle", key))
            .role(Role::Button)
            .aria_label(format!(
                "{} diff of {}",
                if expanded { "Hide" } else { "Show" },
                short_path(&path)
            ))
            .aria_expanded(expanded)
            .flex_shrink(1.)
            .min_w_0()
            .gap(px(SPACE_2))
            .items_center()
            .cursor_pointer()
            .on_click(cx.listener(move |this, _, _, cx| this.toggle_diff(key, cx)))
            .child(
                div().w(px(ICON_SM)).flex_none().child(
                    icon(if expanded {
                        IconName::ChevronDown
                    } else {
                        IconName::ChevronRight
                    })
                    .with_size(px(ICON_SM))
                    .text_color(t.foreground_muted),
                ),
            )
            .child(
                div()
                    .flex_shrink(1.)
                    .min_w_0()
                    .truncate()
                    .font_family(mono)
                    .text_size(code_font_size(EDIT_PATH_SIZE, cx))
                    .text_color(if failed {
                        t.status_danger
                    } else {
                        t.foreground
                    })
                    .child(shown_path),
            )
            .child(
                h_flex()
                    .gap(px(2.))
                    .children(change_bar(added, removed).into_iter().map(|is_add| {
                        div().size(px(BAR_SQUARE)).rounded(px(1.5)).bg(if is_add {
                            t.status_success
                        } else {
                            t.status_danger
                        })
                    })),
            )
            .when(added > 0, |row| {
                row.child(
                    div()
                        .text_size(stat)
                        .text_color(t.status_success)
                        .child(format!("+{added}")),
                )
            })
            .when(removed > 0, |row| {
                row.child(
                    div()
                        .text_size(stat)
                        .text_color(t.status_danger)
                        .child(format!("−{removed}")),
                )
            })
            .children(status.map(|status| {
                div()
                    .text_size(stat)
                    .text_color(if failed {
                        t.status_danger
                    } else {
                        t.foreground_muted
                    })
                    .child(status)
            }));
        let cwd = self.cwd.clone();
        let open_path = path.clone();
        let header = h_flex()
            .gap(px(SPACE_1))
            .items_center()
            .child(toggle)
            .child(
                icon_button(
                    ("edit-open", key),
                    "Open file",
                    IconName::FileSymlink,
                    t.foreground_muted,
                    &t,
                )
                .on_click(cx.listener(move |_, _, _, cx| {
                    cx.emit(TimelineEvent::OpenFile {
                        cwd: cwd.clone(),
                        path: open_path.clone(),
                        line: None,
                    })
                })),
            );

        let body = expanded.then(|| {
            let error = tools::presentation(item, &self.cwd).error;
            let numbers = diff::numbers(&diff.lines);
            let width = numbers
                .iter()
                .flatten()
                .max()
                .map_or(1, |n| n.to_string().len());
            // The edit draws on every frame it is on screen, so its runs are made once.
            let scheme = scheme(cx);
            let cached = diff
                .runs
                .borrow()
                .as_ref()
                .filter(|(at, _)| *at == scheme)
                .map(|(_, runs)| runs.clone());
            let runs = cached.unwrap_or_else(|| {
                let runs = Rc::new(self.diff_runs(&diff.lines, Some(&path), cx));
                *diff.runs.borrow_mut() = Some((scheme, runs.clone()));
                runs
            });
            let runs = runs.iter().cloned();
            let code = code_font_size(FONT_CODE, cx);
            let mut seen_hunk = false;
            let rows: Vec<AnyElement> = diff
                .lines
                .iter()
                .zip(numbers)
                .zip(runs)
                .filter_map(|((line, number), line_runs)| {
                    if line.kind == LineKind::Hunk {
                        // The first hunk needs no separator.
                        let first = !seen_hunk;
                        seen_hunk = true;
                        return (!first).then(|| {
                            div()
                                .pl(px(SPACE_2))
                                .text_color(t.foreground_muted)
                                .child(format!("{:>width$}", "⋮"))
                                .into_any_element()
                        });
                    }
                    let (prefix, prefix_color, bg) = match line.kind {
                        LineKind::Add => {
                            ("+ ", t.status_success, t.status_success.opacity(ADD_TINT))
                        }
                        LineKind::Remove => {
                            ("- ", t.status_danger, t.status_danger.opacity(REMOVE_TINT))
                        }
                        _ => ("  ", t.foreground_muted, transparent_black()),
                    };
                    let color = if line.kind == LineKind::Context {
                        t.foreground_muted
                    } else {
                        t.foreground
                    };
                    Some(
                        h_flex()
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
                            .child(div().text_color(prefix_color).child(prefix))
                            .child(
                                div()
                                    .text_color(color)
                                    .child(styled_line(&line.text, line_runs)),
                            )
                            .into_any_element(),
                    )
                })
                .collect();
            div()
                .id(("diff", key))
                .role(Role::Group)
                .aria_label(format!(
                    "Diff of {}, {added} added, {removed} removed",
                    short_path(&path)
                ))
                .mt(px(SPACE_1 + 2.))
                .rounded(px(RADIUS_LG))
                .border_1()
                .border_color(t.border)
                .overflow_hidden()
                .bg(t.surface1)
                .children(error.map(|error| {
                    div()
                        .p(px(SPACE_2))
                        .text_size(stat)
                        .text_color(t.status_danger)
                        .child(error)
                }))
                .child(if rows.is_empty() {
                    div()
                        .p(px(SPACE_2))
                        .text_size(stat)
                        .text_color(t.foreground_muted)
                        .child("No changes to show.")
                        .into_any_element()
                } else {
                    div()
                        .id(("diff-body", key))
                        .max_h(px(MAX_DIFF_HEIGHT))
                        .overflow_scroll()
                        .py(px(SPACE_1 + 2.))
                        .font_family(mono)
                        .text_size(code)
                        .line_height(px(CODE_LINE_HEIGHT))
                        .whitespace_nowrap()
                        .children(rows)
                        .into_any_element()
                })
        });
        v_flex()
            .id(("edit", key))
            .role(Role::Group)
            .aria_label(format!("Edit of {}", short_path(&path)))
            .child(header)
            .children(body)
            .into_any_element()
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

/// The first [`MAX_CODE_LINES`] lines of `text`.
fn cut_lines(text: &str) -> String {
    match text.match_indices('\n').nth(MAX_CODE_LINES - 1) {
        Some((at, _)) => text[..at].to_string(),
        None => text.to_string(),
    }
}
