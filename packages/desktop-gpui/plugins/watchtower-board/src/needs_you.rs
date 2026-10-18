//! `NeedsYouCell` of `client/needs-you.tsx`: questions, proposed ADRs, and the manual checks row,
//! each with the actions that send a Watchtower prompt to the workspace agent. The cell draws
//! the page's `NeedsYouState`; every press goes back to the page as a `NeedsAction`.

use crate::cell::{Slice, cell_slice, cell_title, empty, limit, show_more};
use crate::model::{
    AgentTarget, NeedsYouKind, NeedsYouRow, NeedsYouTone, action_subject, actions_disabled,
    agent_note, is_decision_id, needs_warning, send_disabled,
};
use crate::page::WatchtowerPage;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_SM, WEIGHT_MEDIUM, WEIGHT_SEMIBOLD};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::Sizable as _;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// Rows: `gap: 8`, `paddingVertical: 7`; the title 12.5 with `lineHeight: 17`; the default
/// 11.5 with `lineHeight: 16` at `marginTop: 3`; the meta 11 at `marginTop: 1`.
const ROW_GAP: f32 = 8.;
const ROW_PADDING: f32 = 7.;
const ROW_ICON: f32 = 14.;
const ICON_TOP: f32 = 2.;
const TITLE_TEXT: f32 = 12.5;
const TITLE_LINE: f32 = 17.;
const DEFAULT_TEXT: f32 = 11.5;
const DEFAULT_LINE: f32 = 16.;
const DEFAULT_GAP: f32 = 3.;
const META_TEXT: f32 = 11.;
const META_GAP: f32 = 1.;
/// The buttons: `columnGap: 14`, `rowGap: 4`, `marginTop: 4`, `paddingVertical: 2`, disabled at
/// 40%.
const BUTTON_COLUMN_GAP: f32 = 14.;
const BUTTON_ROW_GAP: f32 = 4.;
const BUTTONS_TOP: f32 = 4.;
const BUTTON_PADDING: f32 = 2.;
const DISABLED_OPACITY: f32 = 0.4;
/// The answer form: `gap: 6`, `marginTop: 6`; the field 9 by 6 with radius 7 and text 12.5.
const FORM_GAP: f32 = 6.;
const FIELD_PADDING_X: f32 = 9.;
const FIELD_PADDING_Y: f32 = 6.;
const FIELD_RADIUS: f32 = 7.;
/// The agent note: `fontSize: 11`, `marginBottom: 2`.
const NOTE_TEXT: f32 = 11.;
const NOTE_GAP: f32 = 2.;

/// What the owner pressed in the cell.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum NeedsAction {
    /// Shows or hides a row's full text, by row key.
    Toggle(String),
    /// Opens the answer form of a question row.
    Answer(usize),
    SendAnswer(usize),
    CancelAnswer,
    UseDefault(usize),
    View {
        id: String,
        title: String,
    },
    Accept(usize),
    Reject(usize),
    CancelReject,
    MarkDone(usize),
    ShowMore,
    OpenAgent(String),
}

/// The cell's view state, kept by the page.
#[derive(Default)]
pub(crate) struct NeedsYouState {
    /// The one row whose full text and default show.
    pub expanded: Option<String>,
    /// The question row whose answer field is open.
    pub answering: Option<String>,
    pub input: Option<Entity<InputState>>,
    pub draft: String,
    /// The ADR row whose Reject waits for a second press.
    pub rejecting: Option<String>,
    pub sending: bool,
    pub show_all: bool,
    pub _input_subscription: Option<Subscription>,
}

impl NeedsYouState {
    pub(crate) fn close_answer(&mut self) {
        self.answering = None;
        self.input = None;
        self._input_subscription = None;
        self.draft.clear();
    }
}

pub(crate) fn row_key(row: &NeedsYouRow) -> String {
    format!("{:?}-{}", row.kind, row.id)
}

fn listener(
    action: NeedsAction,
    cx: &mut Context<WatchtowerPage>,
) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
    cx.listener(move |page, _, window, cx| page.needs_action(action.clone(), window, cx))
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Tone {
    Primary,
    Plain,
    Danger,
    Quiet,
}

/// `ActionButton`: accent text with no border; Cancel is quiet, Reject is danger.
fn action_button(
    label: &'static str,
    subject: &str,
    tone: Tone,
    disabled: bool,
    action: NeedsAction,
    cx: &mut Context<WatchtowerPage>,
) -> AnyElement {
    let colors = tokens(cx);
    let color = match tone {
        Tone::Danger => colors.status_danger,
        Tone::Quiet => colors.foreground_muted,
        Tone::Primary | Tone::Plain => colors.accent,
    };
    let on_click = listener(action, cx);
    div()
        .id(SharedString::from(format!("wt-needs-{label}-{subject}")))
        .role(Role::Button)
        .aria_label(SharedString::from(format!("{label} {subject}")))
        .py(bachuc_theme::zoomed(BUTTON_PADDING, cx))
        .text_size(ui_font_size(FONT_SM, cx))
        .font_weight(FontWeight(if tone == Tone::Primary {
            WEIGHT_SEMIBOLD
        } else {
            WEIGHT_MEDIUM
        }))
        .text_color(color)
        .when(disabled, |this| this.opacity(DISABLED_OPACITY))
        .when(!disabled, |this| {
            this.cursor_pointer()
                .active(|style| style.opacity(0.6))
                .on_click(on_click)
        })
        .child(label)
        .into_any_element()
}

fn row_icon(row: &NeedsYouRow, cx: &App) -> (IconName, Hsla) {
    let colors = tokens(cx);
    if row.tone == NeedsYouTone::Danger {
        return (IconName::CircleAlert, colors.status_danger);
    }
    match row.kind {
        NeedsYouKind::Adr => (IconName::Scale, colors.foreground_muted),
        NeedsYouKind::Checks => (IconName::ClipboardCheck, colors.foreground_muted),
        NeedsYouKind::Question => (
            IconName::MessageCircleQuestion,
            if row.tone == NeedsYouTone::Warning {
                colors.status_warning
            } else {
                colors.foreground_muted
            },
        ),
    }
}

fn buttons(
    index: usize,
    row: &NeedsYouRow,
    state: &NeedsYouState,
    target: &AgentTarget,
    cx: &mut Context<WatchtowerPage>,
) -> Vec<AnyElement> {
    let disabled = actions_disabled(target, state.sending);
    let subject = action_subject(row).to_string();
    match row.kind {
        NeedsYouKind::Question => {
            let mut out = vec![action_button(
                "Answer",
                &subject,
                Tone::Plain,
                disabled,
                NeedsAction::Answer(index),
                cx,
            )];
            if row.default_answer.is_some() {
                out.push(action_button(
                    "Use default",
                    &subject,
                    Tone::Plain,
                    disabled,
                    NeedsAction::UseDefault(index),
                    cx,
                ));
            }
            out
        }
        NeedsYouKind::Adr => {
            let confirming = state.rejecting.as_deref() == Some(row.id.as_str());
            let mut out = Vec::new();
            if is_decision_id(&row.id) {
                out.push(action_button(
                    "View",
                    &subject,
                    Tone::Plain,
                    false,
                    NeedsAction::View {
                        id: row.id.clone(),
                        title: row.title.clone(),
                    },
                    cx,
                ));
            }
            out.push(action_button(
                "Accept",
                &subject,
                Tone::Plain,
                disabled,
                NeedsAction::Accept(index),
                cx,
            ));
            out.push(action_button(
                if confirming {
                    "Confirm reject"
                } else {
                    "Reject"
                },
                &subject,
                Tone::Danger,
                disabled,
                NeedsAction::Reject(index),
                cx,
            ));
            if confirming {
                out.push(action_button(
                    "Cancel",
                    &subject,
                    Tone::Quiet,
                    state.sending,
                    NeedsAction::CancelReject,
                    cx,
                ));
            }
            out
        }
        NeedsYouKind::Checks => vec![action_button(
            "Mark done",
            &subject,
            Tone::Plain,
            disabled,
            NeedsAction::MarkDone(index),
            cx,
        )],
    }
}

fn answer_form(
    index: usize,
    row: &NeedsYouRow,
    state: &NeedsYouState,
    target: &AgentTarget,
    cx: &mut Context<WatchtowerPage>,
) -> AnyElement {
    let colors = tokens(cx);
    let subject = action_subject(row).to_string();
    let field = state.input.as_ref().map(|input| {
        div()
            .px(bachuc_theme::zoomed(FIELD_PADDING_X, cx))
            .py(bachuc_theme::zoomed(FIELD_PADDING_Y, cx))
            .rounded(bachuc_theme::zoomed(FIELD_RADIUS, cx))
            .border_1()
            .border_color(colors.border)
            .child(
                Input::new(input)
                    .appearance(false)
                    .bordered(false)
                    .focus_bordered(false)
                    .px_0()
                    .py_0()
                    .text_size(ui_font_size(TITLE_TEXT, cx))
                    .aria_label(SharedString::from(format!("Answer {}", row.id))),
            )
    });
    div()
        .mt(bachuc_theme::zoomed(FORM_GAP, cx))
        .flex()
        .flex_col()
        .gap(bachuc_theme::zoomed(FORM_GAP, cx))
        .children(field)
        .child(
            div()
                .flex()
                .flex_row()
                .gap(bachuc_theme::zoomed(BUTTON_COLUMN_GAP, cx))
                .child(action_button(
                    "Send",
                    &subject,
                    Tone::Primary,
                    send_disabled(target, state.sending, &state.draft),
                    NeedsAction::SendAnswer(index),
                    cx,
                ))
                .child(action_button(
                    "Cancel",
                    &subject,
                    Tone::Quiet,
                    state.sending,
                    NeedsAction::CancelAnswer,
                    cx,
                )),
        )
        .into_any_element()
}

/// One row: its icon, the title that toggles its full text, the meta line, and the buttons or
/// the answer form.
pub(crate) fn needs_row(
    index: usize,
    row: &NeedsYouRow,
    state: &NeedsYouState,
    target: &AgentTarget,
    cx: &mut Context<WatchtowerPage>,
) -> AnyElement {
    let colors = tokens(cx);
    let (glyph, glyph_color) = row_icon(row, cx);
    let key = row_key(row);
    let open = state.expanded.as_deref() == Some(key.as_str());
    let answering = state.answering.as_deref() == Some(row.id.as_str());
    let toggle = listener(NeedsAction::Toggle(key.clone()), cx);
    let default_line = row
        .default_answer
        .as_ref()
        .filter(|_| open && row.kind == NeedsYouKind::Question)
        .map(|default| {
            let text = format!("Default: {default}");
            let bold = HighlightStyle {
                font_weight: Some(FontWeight(WEIGHT_SEMIBOLD)),
                ..HighlightStyle::default()
            };
            div()
                .pt(bachuc_theme::zoomed(DEFAULT_GAP, cx))
                .text_size(ui_font_size(DEFAULT_TEXT, cx))
                .line_height(bachuc_theme::zoomed(DEFAULT_LINE, cx))
                .text_color(colors.foreground)
                .child(StyledText::new(text).with_highlights([(0.."Default:".len(), bold)]))
        });
    let actions = if answering {
        answer_form(index, row, state, target, cx)
    } else {
        div()
            .mt(bachuc_theme::zoomed(BUTTONS_TOP, cx))
            .flex()
            .flex_row()
            .flex_wrap()
            .gap_x(bachuc_theme::zoomed(BUTTON_COLUMN_GAP, cx))
            .gap_y(bachuc_theme::zoomed(BUTTON_ROW_GAP, cx))
            .children(buttons(index, row, state, target, cx))
            .into_any_element()
    };
    div()
        .w_full()
        .flex()
        .flex_row()
        .items_start()
        .gap(bachuc_theme::zoomed(ROW_GAP, cx))
        .py(bachuc_theme::zoomed(ROW_PADDING, cx))
        .when(index > 0, |this| {
            this.border_t_1().border_color(colors.border)
        })
        .child(
            div()
                .mt(bachuc_theme::zoomed(ICON_TOP, cx))
                .flex_shrink_0()
                .child(
                    icon(glyph)
                        .with_size(bachuc_theme::zoomed(ROW_ICON, cx))
                        .text_color(glyph_color),
                ),
        )
        .child(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .flex_col()
                .child(
                    div()
                        .id(SharedString::from(format!("wt-needs-row-{key}")))
                        .role(Role::Button)
                        .aria_label(SharedString::from(format!(
                            "{} {}: {}",
                            if open { "Collapse" } else { "Show all of" },
                            row.id,
                            row.title
                        )))
                        .aria_expanded(open)
                        .cursor_pointer()
                        .on_click(toggle)
                        .child(
                            div()
                                .when(!open, |this| this.line_clamp(2))
                                .text_size(ui_font_size(TITLE_TEXT, cx))
                                .line_height(bachuc_theme::zoomed(TITLE_LINE, cx))
                                .text_color(colors.foreground)
                                .child(row.title.clone()),
                        )
                        .children(default_line),
                )
                .child(
                    div()
                        .pt(bachuc_theme::zoomed(META_GAP, cx))
                        .truncate()
                        .text_size(ui_font_size(META_TEXT, cx))
                        .text_color(colors.foreground_muted)
                        .child(format!(
                            "{}{}",
                            row.meta,
                            if open { "" } else { " · press to read all" }
                        )),
                )
                .child(actions),
        )
        .into_any_element()
}

/// The empty text, or the note that names where the actions go.
fn needs_note(
    rows: &[NeedsYouRow],
    target: &AgentTarget,
    cx: &mut Context<WatchtowerPage>,
) -> Option<AnyElement> {
    let colors = tokens(cx);
    if rows.is_empty() {
        return Some(empty("Nothing waits on you.", cx).into_any_element());
    }
    if let AgentTarget::Ready { id, name } = target {
        let link = HighlightStyle {
            color: Some(colors.accent),
            underline: Some(UnderlineStyle {
                thickness: px(1.),
                color: Some(colors.accent),
                wavy: false,
            }),
            ..HighlightStyle::default()
        };
        let prefix = "Actions go to ";
        let text = format!("{prefix}{name}");
        let len = text.len();
        return Some(
            div()
                .id("wt-needs-agent")
                .role(Role::Link)
                .aria_label(SharedString::from(format!(
                    "Open {name}, the thread that receives these actions"
                )))
                .self_start()
                .pb(bachuc_theme::zoomed(NOTE_GAP, cx))
                .cursor_pointer()
                .active(|style| style.opacity(0.6))
                .on_click(listener(NeedsAction::OpenAgent(id.clone()), cx))
                .text_size(ui_font_size(NOTE_TEXT, cx))
                .text_color(colors.foreground_muted)
                .child(StyledText::new(text).with_highlights([(prefix.len()..len, link)]))
                .into_any_element(),
        );
    }
    let (text, warning) = agent_note(target);
    Some(
        div()
            .pb(bachuc_theme::zoomed(NOTE_GAP, cx))
            .truncate()
            .text_size(ui_font_size(NOTE_TEXT, cx))
            .text_color(if warning {
                colors.status_warning
            } else {
                colors.foreground_muted
            })
            .child(text)
            .into_any_element(),
    )
}

/// The top slice: the title with the row count, and the note.
pub(crate) fn needs_top(
    rows: &[NeedsYouRow],
    target: &AgentTarget,
    slice: Slice,
    cx: &mut Context<WatchtowerPage>,
) -> Div {
    let aside = (!rows.is_empty()).then(|| SharedString::from(rows.len().to_string()));
    let note = needs_note(rows, target, cx);
    cell_slice(slice, needs_warning(rows), cx)
        .child(cell_title("Needs you", aside, cx))
        .children(note)
}

pub(crate) fn needs_more(
    rows: &[NeedsYouRow],
    state: &NeedsYouState,
    cx: &mut Context<WatchtowerPage>,
) -> Option<AnyElement> {
    let toggle = listener(NeedsAction::ShowMore, cx);
    show_more("wt-needs-more", rows.len(), state.show_all, toggle, cx)
}

/// The whole cell in one item, for a grid row.
pub(crate) fn needs_cell(
    rows: &[NeedsYouRow],
    state: &NeedsYouState,
    target: &AgentTarget,
    cx: &mut Context<WatchtowerPage>,
) -> AnyElement {
    let shown: Vec<AnyElement> = limit(rows, state.show_all)
        .iter()
        .enumerate()
        .map(|(index, row)| needs_row(index, row, state, target, cx))
        .collect();
    let more = needs_more(rows, state, cx);
    needs_top(rows, target, Slice::Whole, cx)
        .flex_grow_1()
        .children(shown)
        .children(more)
        .into_any_element()
}

/// A middle slice that holds one row, or the Show more toggle.
pub(crate) fn needs_slice(rows: &[NeedsYouRow], child: Option<AnyElement>, cx: &App) -> AnyElement {
    cell_slice(Slice::Middle, needs_warning(rows), cx)
        .children(child)
        .into_any_element()
}
