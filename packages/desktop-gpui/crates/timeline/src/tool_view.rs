//! Tool call badges and their detail views, as `ExpandableBadge` and `ToolCall` in
//! `packages/app/src/components/message.tsx` and `ToolCallDetailsContent` in
//! `tool-call-details.tsx`. Edit rows are the edit-diffs plugin's (`bachuc-plugin-edit-diffs`).

use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::rc::Rc;

use bachuc_explorer::syntax::{self, Run, ThemeSyntax};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_CODE, FONT_SM, ICON_SM, ICON_XS, RADIUS_BASE, RADIUS_LG, RADIUS_MD, SPACE_1,
    SPACE_2, SPACE_3, SPACE_4,
};
use bachuc_theme::{
    SyntaxColors, Tokens, code_font_size, mono_font_family, scheme, syntax_colors, tokens,
    ui_font_size, zoomed,
};
use gpui_kit::component::highlighter::SyntaxHighlighter;
use gpui_kit::component::{Rope, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::Value;

use crate::diff::{DiffLine, LineKind};
use crate::grouping::entry_key;
use crate::motion;
use crate::tools::{self, HookLine, Section};
use crate::{LineRuns, MotionKey, TimelineEvent, TimelineView};

/// The badge reaches past the stream padding, as `marginHorizontal: -13`.
pub(crate) const BADGE_BLEED: f32 = 13.;
const ICON_SLOT: f32 = 22.;
pub(crate) const CODE_LINE_HEIGHT: f32 = 18.;
/// `ToolCall`'s default `maxDetailHeight`.
pub(crate) const TOOL_DETAIL_MAX_HEIGHT: f32 = 400.;
/// Lines one code section lays out; the rest is cut so a huge output stays cheap to draw.
const MAX_CODE_LINES: usize = 2000;
/// The diff line tints of `diff-viewer.tsx`, over the status tokens.
const ADD_TINT: f32 = 0.15;
const REMOVE_TINT: f32 = 0.1;
/// The label of a running tool call, as `labelLoading`.
const LOADING_OPACITY: f32 = 0.72;

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

/// Colored ranges of `text` parsed as `language`, in the colors of the syntax theme picked in
/// Settings (`bachuc_theme::syntax_colors`). The resolver leaves a macro name uncolored, as the
/// app's lezer grammars do.
pub(crate) fn code_styles(language: &str, text: &str, colors: SyntaxColors) -> Vec<Run> {
    let mut highlighter = SyntaxHighlighter::new(language);
    highlighter.update(None, &Rope::from_str(text), None);
    highlighter.styles(&(0..text.len()), &ThemeSyntax(colors))
}

/// A small square icon button with a role and a label.
pub(crate) fn icon_button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    name: IconName,
    color: Hsla,
    t: &Tokens,
    cx: &App,
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
        .p(zoomed(SPACE_1, cx))
        .rounded(zoomed(RADIUS_MD, cx))
        .cursor_pointer()
        .hover(move |button| button.bg(fill))
        .child(icon(name).with_size(zoomed(ICON_SM, cx)).text_color(color))
}

/// The running badge's label and summary with the moving shimmer band, as
/// `buildShimmerTextStyle` draws it. GPUI cannot clip a background to text, so the glyphs under
/// the band take its white alpha and the rest keep the loading color.
pub(crate) fn shimmer_draw(
    label: String,
    summary: Option<String>,
) -> impl Fn(u128, &mut Window, &mut App) -> AnyElement {
    // `shape_line` shapes one line and panics on a newline.
    let label = label.replace('\n', " ");
    let summary = summary.map(|summary| summary.replace('\n', " "));
    move |ms, _window, cx| {
        let base = tokens(cx).foreground.opacity(LOADING_OPACITY);
        let size = ui_font_size(FONT_BASE, cx);
        let font = Font {
            family: bachuc_theme::ui_font_family(cx),
            ..Default::default()
        };
        let gap = zoomed(SPACE_2, cx);
        let line_height = size * 1.618;
        let label = label.clone();
        let summary = summary.clone();
        canvas(
            |_, _, _| (),
            move |bounds, (), window, cx| {
                let run = |len: usize| TextRun {
                    len,
                    font: font.clone(),
                    color: base,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                };
                let label_line = window.text_system().shape_line(
                    label.clone().into(),
                    size,
                    &[run(label.len())],
                    None,
                );
                let summary_line = summary.as_deref().map(|text| {
                    window
                        .text_system()
                        .shape_line(text.into(), size, &[run(text.len())], None)
                });
                let label_width = label_line.width();
                let span_end = label_width
                    + if summary.is_some() { gap } else { Pixels::ZERO }
                    + summary_line
                        .as_ref()
                        .map_or(Pixels::ZERO, |line| line.width());
                let chars = motion::shimmer_chars(&label, summary.as_deref());
                let duration = motion::shimmer_duration(chars);
                let phase = motion::shimmer_phase(ms, duration);
                let band = motion::shimmer_band_width(f32::from(span_end));
                let band_left = motion::shimmer_band_left(0., f32::from(span_end), band, phase);
                let y = bounds.top() + (bounds.size.height - line_height) / 2.;
                let mut paint = |text: &str, geometry: &ShapedLine, x: f32| {
                    let runs = shimmer_runs(geometry, text, x, band_left, band, base, &font);
                    let colored = window
                        .text_system()
                        .shape_line(text.into(), size, &runs, None);
                    colored
                        .paint(
                            point(bounds.left() + px(x), y),
                            line_height,
                            TextAlign::Left,
                            None,
                            window,
                            cx,
                        )
                        .ok();
                };
                paint(&label, &label_line, 0.);
                if let Some(text) = summary.as_deref()
                    && let Some(line) = &summary_line
                {
                    paint(text, line, f32::from(label_width + gap));
                }
            },
        )
        .w_full()
        .h(line_height)
        .into_any_element()
    }
}

/// The text runs of one shimmer line: a glyph under the band takes white at the band's alpha,
/// and a glyph outside it keeps `base`. Equal colors share one run.
fn shimmer_runs(
    geometry: &ShapedLine,
    text: &str,
    x: f32,
    band_left: f32,
    band_width: f32,
    base: Hsla,
    font: &Font,
) -> Vec<TextRun> {
    let mut boundaries: Vec<(usize, f32)> = text
        .char_indices()
        .map(|(index, _)| (index, f32::from(geometry.x_for_index(index))))
        .collect();
    boundaries.push((text.len(), f32::from(geometry.x_for_index(text.len()))));
    let mut runs: Vec<TextRun> = Vec::new();
    let mut current: Option<(u8, Hsla, usize)> = None;
    for pair in boundaries.windows(2) {
        let (start, at) = pair[0];
        let (end, _) = pair[1];
        let position = if band_width > 0. {
            ((x + at - band_left) / band_width).clamp(0., 1.)
        } else {
            0.
        };
        let (key, color) = if position > 0. && position < 1. {
            let alpha = motion::shimmer_alpha(position);
            ((alpha * 24.).round() as u8 + 1, white().opacity(alpha))
        } else {
            (0, base)
        };
        match &mut current {
            Some((current_key, _, len)) if *current_key == key => *len += end - start,
            Some((_, current_color, len)) => {
                runs.push(TextRun {
                    len: *len,
                    font: font.clone(),
                    color: *current_color,
                    background_color: None,
                    underline: None,
                    strikethrough: None,
                });
                current = Some((key, color, end - start));
            }
            None => current = Some((key, color, end - start)),
        }
    }
    if let Some((_, color, len)) = current {
        runs.push(TextRun {
            len,
            font: font.clone(),
            color,
            background_color: None,
            underline: None,
            strikethrough: None,
        });
    }
    runs
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
            &code_styles(language, text, syntax_colors(cx)),
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
        // A `hook` row is itself one hook, so it shows no badge or Hooks section of its own.
        let hooks = if item["name"]
            .as_str()
            .is_some_and(|name| name.eq_ignore_ascii_case("hook"))
        {
            Vec::new()
        } else {
            tools::hook_lines(item)
        };
        let can_open = !sections.is_empty() || !hooks.is_empty();
        let expanded = can_open && self.expanded.contains(&key);
        let status = item["status"].as_str().unwrap_or("");
        let is_loading = matches!(status, "running" | "executing");
        let is_error = status == "failed";
        let group: SharedString = format!("badge-{key}").into();
        let size = ui_font_size(FONT_BASE, cx);

        let tool_icon = if is_error {
            icon(IconName::TriangleAlert)
                .with_size(zoomed(ICON_XS, cx))
                .text_color(t.destructive.opacity(0.8))
        } else {
            icon(shown.icon)
                .with_size(zoomed(ICON_XS, cx))
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
        .with_size(zoomed(ICON_SM, cx))
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
            .size(zoomed(ICON_SLOT, cx))
            .flex_none()
            .mr(zoomed(SPACE_1, cx))
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
        let shimmer = is_loading
            .then(|| self.motions.get(&MotionKey::Badge(key)).cloned())
            .flatten()
            .map(|(_, motion)| motion.into_any_element());
        let has_shimmer = shimmer.is_some();
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
                .ml(zoomed(SPACE_2, cx))
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
                cx,
            )
            .ml(zoomed(SPACE_1, cx))
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
        let hook_failed = hooks.iter().any(|hook| hook.status == "error");
        let hook_badge = (!hooks.is_empty()).then(|| {
            let color = if hook_failed {
                t.destructive
            } else {
                t.foreground_muted
            };
            h_flex()
                .flex_none()
                .items_center()
                .gap(zoomed(SPACE_1, cx))
                .ml(zoomed(SPACE_2, cx))
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(color)
                .child(
                    icon(IconName::Zap)
                        .with_size(zoomed(ICON_XS, cx))
                        .text_color(color),
                )
                .child(hooks.len().to_string())
        });
        let mut aria = shown.name.clone();
        if let Some(summary) = &shown.summary {
            aria.push_str(&format!(": {summary}"));
        }
        if !hooks.is_empty() {
            aria.push_str(&format!(", {} hooks", hooks.len()));
            if hook_failed {
                aria.push_str(" (failed)");
            }
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
            .px(zoomed(SPACE_2, cx))
            .py(zoomed(SPACE_1, cx))
            .rounded(zoomed(RADIUS_LG, cx))
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
            .when_some(shimmer, |header, shimmer| {
                header.child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .h_full()
                        .overflow_hidden()
                        .child(shimmer),
                )
            })
            .when(!has_shimmer, |header| header.child(label).children(summary))
            .children(hook_badge)
            .children(open_file);
        let details = expanded.then(|| {
            div()
                .id(("badge-detail", key))
                .role(Role::Group)
                .aria_label(format!("{} details", shown.name))
                .rounded_b(zoomed(RADIUS_LG, cx))
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
                .when(!hooks.is_empty(), |details| {
                    details.child(self.render_hooks(key, &hooks, !sections.is_empty(), &t, cx))
                })
        });
        v_flex()
            .id(("tool", key))
            .group(group)
            .role(Role::Group)
            .aria_label(format!("Tool call {aria}"))
            .mx(zoomed(-BADGE_BLEED, cx))
            .child(header)
            .children(details)
            .into_any_element()
    }

    /// The Hooks section after the detail sections: one block per hook with its name, status,
    /// exit code, and output.
    fn render_hooks(
        &self,
        key: u64,
        hooks: &[HookLine],
        after_sections: bool,
        t: &Tokens,
        cx: &App,
    ) -> AnyElement {
        let mono = mono_font_family(cx);
        div()
            .id(("hooks", key))
            .when(after_sections, |area| {
                area.border_t_1().border_color(t.border)
            })
            .max_h(zoomed(TOOL_DETAIL_MAX_HEIGHT, cx))
            .overflow_y_scroll()
            .p(zoomed(SPACE_3, cx))
            .bg(t.surface1)
            .child(
                v_flex()
                    .gap(zoomed(SPACE_2, cx))
                    .child(
                        div()
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .text_color(t.foreground_muted)
                            .child("Hooks"),
                    )
                    .children(hooks.iter().map(|hook| {
                        let status_color = if hook.status == "error" {
                            t.destructive
                        } else {
                            t.foreground_muted
                        };
                        v_flex()
                            .gap(zoomed(SPACE_1, cx))
                            .font_family(mono)
                            .text_size(code_font_size(FONT_CODE, cx))
                            .child(
                                h_flex()
                                    .gap(zoomed(SPACE_2, cx))
                                    .child(div().text_color(t.foreground).child(hook.name.clone()))
                                    .child(
                                        div().text_color(status_color).child(hook.status.clone()),
                                    )
                                    .children(hook.exit_code.map(|code| {
                                        div()
                                            .text_color(t.foreground_muted)
                                            .child(format!("exit {code}"))
                                    })),
                            )
                            .children(hook.output.as_ref().map(|output| {
                                div()
                                    .whitespace_nowrap()
                                    .text_color(t.foreground_muted)
                                    .child(cut_lines(output))
                            }))
                    })),
            )
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
            .gap(zoomed(if full_bleed { SPACE_2 } else { SPACE_4 }, cx))
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
                                    .mb(zoomed(SPACE_2, cx))
                                    .text_color(t.foreground_muted)
                                    .child(format!("session {session}"))
                            }))
                            .when(!actions.is_empty(), |area| {
                                area.child(
                                    v_flex()
                                        .gap(zoomed(SPACE_1, cx))
                                        .mb(zoomed(SPACE_2, cx))
                                        .children(actions.iter().map(|(tool, summary)| {
                                            h_flex()
                                                .gap(zoomed(SPACE_2, cx))
                                                .font_family(mono)
                                                .child(
                                                    div()
                                                        .text_color(t.foreground_muted)
                                                        .child(tool.clone()),
                                                )
                                                .children(summary.clone())
                                        })),
                                )
                            })
                            .when(!log.is_empty(), |area| area.child(cut_lines(log)))
                            .into_any_element()
                    }
                    Section::Plain(text) => div()
                        .id(id)
                        .max_h(zoomed(max_height, cx))
                        .overflow_y_scroll()
                        .p(zoomed(SPACE_3, cx))
                        .rounded(zoomed(RADIUS_BASE, cx))
                        .border_1()
                        .border_color(t.border)
                        .bg(t.surface2)
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .line_height(zoomed(22., cx))
                        .text_color(t.foreground)
                        .child(text.clone())
                        .into_any_element(),
                    Section::Header(title) => div()
                        .px(zoomed(SPACE_3, cx))
                        .py(zoomed(SPACE_2, cx))
                        .border_b_1()
                        .border_color(t.border)
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .text_color(t.foreground_muted)
                        .child(title.clone())
                        .into_any_element(),
                    Section::Error(error) => v_flex()
                        .gap(zoomed(SPACE_2, cx))
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
                                .p(zoomed(SPACE_3, cx))
                                .rounded(zoomed(RADIUS_BASE, cx))
                                .border_1()
                                .border_color(t.destructive)
                                .bg(t.surface2)
                                .font_family(mono_font_family(cx))
                                .text_size(code_font_size(FONT_CODE, cx))
                                .line_height(zoomed(CODE_LINE_HEIGHT, cx))
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
            .max_h(zoomed(max_height, cx))
            .overflow_scroll()
            .p(zoomed(SPACE_3, cx))
            .font_family(mono_font_family(cx))
            .text_size(code_font_size(FONT_CODE, cx))
            .line_height(zoomed(CODE_LINE_HEIGHT, cx))
            .whitespace_nowrap()
            .text_color(t.foreground)
            .map(|area| {
                if full_bleed {
                    area.bg(t.surface1)
                } else {
                    area.rounded(zoomed(RADIUS_BASE, cx))
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
                            .pr(zoomed(SPACE_3, cx))
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
            .max_h(zoomed(max_height, cx))
            .overflow_scroll()
            .bg(t.surface1)
            .p(zoomed(SPACE_3, cx))
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
                    .py(zoomed(SPACE_1, cx))
                    .bg(bg)
                    .text_color(color)
                    .child(sign)
                    .child(styled_line(&line.text, line_runs))
            })),
        )
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
