//! The `next-prompt-panel` row, as `NextPromptPanel` of `client/panel.tsx`: the Recap header,
//! then the What Next title, intro, and one card per prompt. Only the latest reply's prompts
//! match candidates, so only they get buttons.

use crate::contracts::{Candidate, CandidateState};
use crate::model::{Models, PanelModel};
use crate::next_prompts::selection_allowed;
use crate::prompts::{PromptBlock, join_prompts, parse_prompts};
use crate::rules::{
    FaceInput, candidate_for, card_face, footer_hint, git_label, group_title, is_compact,
    is_suggested, reserved_faces, row_status, send_selected_label, shows_selection, timestamp_ms,
};
use crate::section::{Recap, RecapSection, bold_runs, commit_chip, recap_sections};
use crate::selection::{Group, layout, reachable, toggle};
use crate::settings_view::back_to_board_now;
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::ui::markdown;
use bachuc_plugin_host::{PluginContext, PluginSettings, TimelineRenderInput};
use bachuc_theme::metrics::{
    CONTROL_SM, FONT_CONTENT, FONT_SM, ICON_MD, ICON_SM, ICON_XS, RADIUS_BASE, RADIUS_LG,
    RADIUS_MD, RADIUS_XL, SPACE_1, SPACE_1_5, SPACE_2, SPACE_3, SPACE_4, SPACE_6,
};
use bachuc_theme::{Tokens, tokens, ui_font_size};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde::Deserialize;
use std::cell::Cell;
use std::rc::Rc;

/// The `Suggested` badge fill (`panel.tsx:180`), fixed in Electron so white text keeps AA
/// contrast in both themes; no theme token has it.
const SUGGESTED_FILL: u32 = 0x0f7b5f;
/// The `Suggested` badge text (`panel.tsx:185`).
const SUGGESTED_TEXT: u32 = 0xffffff;
/// Sizes of `panel.tsx` that no metric has.
const WIDE_PAD: f32 = 20.;
const RECAP_PAD_Y: f32 = 14.;
const CHIP_PAD_Y: f32 = 3.;
const CHIP_GAP: f32 = 5.;
const CHIP_ICON: f32 = 13.;
const SECTION_LABEL: f32 = 12.5;
const SECTION_LINE: f32 = 20.;
const CARD_PAD_LEFT: f32 = 18.;
const CARD_ROW_GAP: f32 = 14.;
const TEXT_LINE: f32 = SPACE_6;
const SMALL_TEXT: f32 = 13.;
const SMALL_LINE: f32 = 20.;
const BUTTON_LINE: f32 = 16.;
const BUTTON_MIN_WIDTH: f32 = 80.;
const BADGE_TEXT: f32 = 11.5;
const BADGE_LINE: f32 = 15.;
const BADGE_HEIGHT: f32 = 20.;
const ROW_PAD: f32 = 14.;
const MARK_BOX: f32 = 18.;
const MARK_TOP: f32 = 3.;
const FOOTER_MIN_WIDTH: f32 = 140.;
const DISABLED_OPACITY: f32 = 0.5;

/// The row data the transformer writes (`panelSchema`).
#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct PanelData {
    pub title: String,
    pub intro: String,
    pub section: String,
    #[serde(default)]
    pub recap: Option<Recap>,
}

/// What every panel shares: the host context, the `send` settings, and the models.
pub(crate) struct Shared {
    pub ctx: PluginContext,
    pub settings: Entity<PluginSettings>,
    pub models: Rc<Models>,
}

/// The renderer's view factory. Data that does not decode renders nothing.
pub(crate) fn build(shared: &Rc<Shared>, input: TimelineRenderInput, cx: &mut App) -> AnyView {
    let data = match serde_json::from_value::<PanelData>(input.item.data.clone()) {
        Ok(data) => Some(data),
        Err(error) => {
            log::warn!("next-prompt-panel data does not decode: {error}");
            None
        }
    };
    let model = data
        .is_some()
        .then(|| shared.models.get(&shared.ctx, &input.agent_id, cx));
    let shared = shared.clone();
    cx.new(|cx| PanelView::new(shared, input, data, model, cx))
        .into()
}

#[derive(Clone, Copy, PartialEq, Eq)]
enum Look {
    Primary,
    Outline,
    Quiet,
}

struct ButtonSpec {
    id: SharedString,
    label: SharedString,
    icon: Option<IconName>,
    look: Look,
    disabled: bool,
    faces: Option<[(&'static str, IconName); 2]>,
}

/// The model state one render reads.
struct Live {
    candidates: Vec<Candidate>,
    busy: bool,
    pending: bool,
    notices: Vec<String>,
}

pub(crate) struct PanelView {
    shared: Rc<Shared>,
    agent_id: String,
    data: Option<PanelData>,
    blocks: Vec<PromptBlock>,
    item_ms: Option<i64>,
    model: Option<Entity<PanelModel>>,
    width: Rc<Cell<f32>>,
    /// The main button under the pointer, as (block, prompt).
    hovered: Option<(usize, usize)>,
    modifier: bool,
    /// The selection of each block; per panel view.
    selections: Vec<Vec<String>>,
    _observe: Vec<Subscription>,
}

impl PanelView {
    fn new(
        shared: Rc<Shared>,
        input: TimelineRenderInput,
        data: Option<PanelData>,
        model: Option<Entity<PanelModel>>,
        cx: &mut Context<Self>,
    ) -> Self {
        let blocks = data
            .as_ref()
            .map(|data| parse_prompts(&data.section))
            .unwrap_or_default();
        let mut observe = Vec::new();
        if let Some(model) = &model {
            observe.push(cx.observe(model, |_, _, cx| cx.notify()));
        }
        Self {
            agent_id: input.agent_id,
            item_ms: input.timestamp.as_deref().and_then(timestamp_ms),
            selections: vec![Vec::new(); blocks.len()],
            shared,
            data,
            blocks,
            model,
            width: Rc::new(Cell::new(0.)),
            hovered: None,
            modifier: false,
            _observe: observe,
        }
    }

    fn edit(&mut self, texts: Vec<String>, cx: &mut Context<Self>) {
        let text = join_prompts(&texts);
        self.shared.ctx.set_composer_text(&self.agent_id, text, cx);
    }

    fn send(&mut self, keys: Vec<String>, cx: &mut Context<Self>) {
        let back_to_board = back_to_board_now(self.shared.settings.read(cx).state());
        if let Some(model) = &self.model {
            model.update(cx, |model, cx| model.send(keys, back_to_board, cx));
        }
    }

    fn start(&mut self, key: String, cx: &mut Context<Self>) {
        if let Some(model) = &self.model {
            model.update(cx, |model, cx| model.start(key, cx));
        }
    }

    fn live(&self, cx: &App) -> Live {
        let Some(model) = &self.model else {
            return Live {
                candidates: Vec::new(),
                busy: true,
                pending: false,
                notices: Vec::new(),
            };
        };
        let model = model.read(cx);
        Live {
            candidates: model
                .snapshot()
                .map(|s| s.candidates.clone())
                .unwrap_or_default(),
            busy: model.busy(),
            pending: model.pending(),
            notices: model.notices(),
        }
    }

    fn button(
        &self,
        spec: ButtonSpec,
        on_press: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        cx: &Context<Self>,
    ) -> Stateful<Div> {
        let colors = tokens(cx);
        let (ink, border, fill) = match spec.look {
            Look::Primary => (colors.surface0, colors.foreground, colors.foreground),
            Look::Outline => (colors.foreground, colors.border, colors.surface0),
            Look::Quiet => (
                colors.foreground_muted,
                transparent_black(),
                transparent_black(),
            ),
        };
        let size = ui_font_size(SMALL_TEXT, cx);
        let face = move |label: SharedString, name: Option<IconName>| {
            h_flex()
                .flex_shrink_0()
                .items_center()
                .justify_center()
                .gap(px(SPACE_1_5))
                .children(name.map(|name| icon(name).with_size(px(ICON_SM)).text_color(ink)))
                .child(
                    div()
                        .whitespace_nowrap()
                        .text_size(size)
                        .line_height(px(BUTTON_LINE))
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(ink)
                        .child(label),
                )
        };
        let content = match spec.faces {
            // Both faces reserve width; only the current one paints.
            Some(faces) => div()
                .relative()
                .h(px(BUTTON_LINE))
                .child(
                    v_flex()
                        .invisible()
                        .h(px(BUTTON_LINE))
                        .overflow_hidden()
                        .items_center()
                        .children(
                            faces
                                .into_iter()
                                .map(|(label, name)| face(label.into(), Some(name))),
                        ),
                )
                .child(
                    div()
                        .absolute()
                        .inset_0()
                        .flex()
                        .items_center()
                        .justify_center()
                        .child(face(spec.label.clone(), spec.icon)),
                )
                .into_any_element(),
            None => face(spec.label.clone(), spec.icon).into_any_element(),
        };
        let min_width = spec.faces.is_some() || spec.look == Look::Primary;
        let disabled = spec.disabled;
        div()
            .id(spec.id)
            .role(Role::Button)
            .aria_label(spec.label)
            .flex()
            .flex_row()
            .flex_shrink_0()
            .items_center()
            .justify_center()
            .min_h(px(CONTROL_SM))
            .when(min_width, |this| this.min_w(px(BUTTON_MIN_WIDTH)))
            .rounded(px(RADIUS_LG))
            .px(px(if spec.look == Look::Quiet {
                SPACE_2
            } else {
                SPACE_3
            }))
            .border_1()
            .border_color(border)
            .bg(fill)
            .when(disabled, |this| this.opacity(DISABLED_OPACITY))
            .when(!disabled, |this| {
                this.cursor_pointer()
                    .on_click(cx.listener(move |this, _: &ClickEvent, _, cx| on_press(this, cx)))
            })
            .child(content)
    }

    fn suggested_badge(cx: &App) -> Div {
        div()
            .self_end()
            .min_h(px(BADGE_HEIGHT))
            .flex()
            .items_center()
            .px(px(SPACE_2))
            .rounded(px(RADIUS_MD))
            .bg(rgb(SUGGESTED_FILL))
            .child(
                div()
                    .text_size(ui_font_size(BADGE_TEXT, cx))
                    .line_height(px(BADGE_LINE))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(rgb(SUGGESTED_TEXT))
                    .child("Suggested"),
            )
    }

    /// The prompt text and its reason; the reason's `**bold**` runs take the foreground.
    fn prompt_text(text: &str, why: &str, colors: &Tokens, cx: &App) -> Div {
        let mut column = v_flex().min_w_0().gap(px(SPACE_1)).child(
            div()
                .text_size(ui_font_size(FONT_CONTENT, cx))
                .line_height(px(TEXT_LINE))
                .text_color(colors.foreground)
                .child(text.to_string()),
        );
        if !why.is_empty() {
            let mut plain = String::new();
            let mut bold = Vec::new();
            for (index, run) in bold_runs(why).iter().enumerate() {
                let start = plain.len();
                plain.push_str(run);
                if index % 2 == 1 {
                    bold.push((
                        start..plain.len(),
                        HighlightStyle {
                            color: Some(colors.foreground),
                            font_weight: Some(FontWeight::BOLD),
                            ..HighlightStyle::default()
                        },
                    ));
                }
            }
            column = column.child(
                div()
                    .text_size(ui_font_size(SMALL_TEXT, cx))
                    .line_height(px(SMALL_LINE))
                    .text_color(colors.foreground_muted)
                    .child(StyledText::new(plain).with_highlights(bold)),
            );
        }
        column
    }

    #[allow(clippy::too_many_arguments)]
    fn prompt_card(
        &self,
        at: (usize, usize),
        text: &str,
        why: &str,
        candidate: Option<&Candidate>,
        suggested: bool,
        live: &Live,
        compact: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let column = compact;
        let words = Self::prompt_text(text, why, &colors, cx)
            .when(column, |this| this.w_full())
            .when(!column, |this| this.flex_1());
        let content = if column {
            v_flex().gap(px(CARD_ROW_GAP))
        } else {
            h_flex().gap(px(SPACE_6))
        }
        .min_w_0()
        .items_end()
        .child(words);
        let buttons = candidate.map(|candidate| {
            let git = git_label(candidate);
            let face = card_face(FaceInput {
                state: Some(candidate.state),
                thread_new: candidate.thread_new(),
                git_label: git,
                hovered: self.hovered == Some(at),
                modifier: self.modifier,
                busy: live.busy,
                pending: live.pending,
            });
            let key = candidate.key.clone();
            let edit_text = text.to_string();
            let edit = (!candidate.thread_new()).then(|| {
                self.button(
                    ButtonSpec {
                        id: format!("np-edit-{}-{}", at.0, at.1).into(),
                        label: "Edit".into(),
                        icon: Some(IconName::Pencil),
                        look: Look::Outline,
                        disabled: live.busy,
                        faces: None,
                    },
                    move |this, cx| this.edit(vec![edit_text.clone()], cx),
                    cx,
                )
            });
            let new_thread = face.new_thread;
            let main = self
                .button(
                    ButtonSpec {
                        id: format!("np-send-{}-{}", at.0, at.1).into(),
                        label: face.label.into(),
                        icon: Some(face.icon),
                        look: if new_thread {
                            Look::Outline
                        } else {
                            Look::Primary
                        },
                        disabled: !face.ready,
                        faces: git
                            .is_none()
                            .then(|| reserved_faces(candidate.thread_new())),
                    },
                    move |this, cx| {
                        if new_thread {
                            this.start(key.clone(), cx)
                        } else {
                            this.send(vec![key.clone()], cx)
                        }
                    },
                    cx,
                )
                .on_hover(cx.listener(move |this, hovered: &bool, window, cx| {
                    if *hovered {
                        this.hovered = Some(at);
                    } else if this.hovered == Some(at) {
                        this.hovered = None;
                    }
                    this.modifier = window.modifiers().secondary();
                    cx.notify();
                }))
                // A modifier change reaches only the focused path, so a move over the button
                // reads it again.
                .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                    let held = event.modifiers.secondary();
                    if held != this.modifier {
                        this.modifier = held;
                        cx.notify();
                    }
                }));
            h_flex()
                .flex_shrink_0()
                .flex_wrap()
                .self_end()
                .items_center()
                .justify_end()
                .gap(px(SPACE_2))
                .children(edit)
                .child(main)
        });
        // The badge has its own row at the card's top right, so it never covers text or buttons.
        v_flex()
            .min_w_0()
            .gap(px(SPACE_2))
            .p(px(SPACE_4))
            .pl(px(CARD_PAD_LEFT))
            .border_1()
            .border_color(colors.border)
            .rounded(px(RADIUS_LG))
            .bg(colors.surface0)
            .when(suggested, |this| this.child(Self::suggested_badge(cx)))
            .child(content.children(buttons))
            .into_any_element()
    }

    fn selection_block(
        &self,
        index: usize,
        block: &PromptBlock,
        candidates: &[Option<&Candidate>],
        live: &Live,
        compact: bool,
        cx: &Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let declaration = block.declaration.as_ref().unwrap();
        let (groups, combos) = (
            &declaration.exclusive_groups,
            &declaration.allowed_combinations,
        );
        let selected = &self.selections[index];
        let ids: Vec<String> = declaration
            .prompts
            .iter()
            .filter(|p| !p.thread_new)
            .map(|p| p.id.clone())
            .collect();
        let index_of = |id: &str| declaration.prompts.iter().position(|p| p.id == id);
        let ordered: Vec<String> = ids
            .iter()
            .filter(|id| selected.contains(id))
            .cloned()
            .collect();
        let chosen: Vec<Option<&Candidate>> = ordered
            .iter()
            .map(|id| index_of(id).and_then(|at| candidates[at]))
            .collect();
        let present: Vec<&Candidate> = chosen.iter().flatten().copied().collect();
        let allowed_set = present.len() == chosen.len() && selection_allowed(&present);
        let can_send = !chosen.is_empty()
            && chosen
                .iter()
                .all(|c| c.is_some_and(|c| c.state == CandidateState::Ready))
            && allowed_set
            && !live.busy;
        let mut body = v_flex().min_w_0().gap(px(SPACE_4));
        for group in layout(&ids, groups, combos) {
            let radio = matches!(group, Group::Choice { .. });
            let rows = group.ids().iter().enumerate().map(|(row, id)| {
                let at = index_of(id).unwrap_or_default();
                let on = selected.contains(id);
                let allowed = on || reachable(selected, id, groups, combos);
                let candidate = candidates.get(at).copied().flatten();
                let state = candidate.map(|c| c.state);
                let disabled = !allowed || live.busy || state != Some(CandidateState::Ready);
                let status = row_status(state, allowed, combos.iter().any(|c| c.contains(id)));
                let suggested = is_suggested(block, at, candidate);
                let toggled = if on { Toggled::True } else { Toggled::False };
                let press = id.clone();
                let mark = div()
                    .flex_shrink_0()
                    .size(px(MARK_BOX))
                    .mt(px(MARK_TOP))
                    .rounded(px(if radio { MARK_BOX / 2. } else { RADIUS_BASE }))
                    .border_1()
                    .border_color(if on { colors.accent } else { colors.border })
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(on, |this| {
                        let name = if radio {
                            IconName::Circle
                        } else {
                            IconName::Check
                        };
                        this.child(icon(name).with_size(px(ICON_XS)).text_color(colors.accent))
                    });
                div()
                    .id(SharedString::from(format!("np-pick-{index}-{id}")))
                    .role(if radio {
                        Role::RadioButton
                    } else {
                        Role::CheckBox
                    })
                    .aria_label(block.prompts[at].clone())
                    .aria_toggled(toggled)
                    .when(suggested, |this| this.aria_description("Suggested"))
                    .flex()
                    .flex_col()
                    .gap(px(SPACE_2))
                    .p(px(ROW_PAD))
                    .min_w_0()
                    .when(row > 0, |this| {
                        this.border_t_1().border_color(colors.border)
                    })
                    .bg(if on { colors.surface1 } else { colors.surface0 })
                    .when(!allowed, |this| this.opacity(DISABLED_OPACITY))
                    .when(!disabled, |this| {
                        this.cursor_pointer().on_click(cx.listener(
                            move |this, _: &ClickEvent, _, cx| {
                                let groups = this.blocks[index]
                                    .declaration
                                    .as_ref()
                                    .map(|d| d.exclusive_groups.clone())
                                    .unwrap_or_default();
                                this.selections[index] =
                                    toggle(&this.selections[index], &press, &groups);
                                cx.notify();
                            },
                        ))
                    })
                    .when(suggested, |this| this.child(Self::suggested_badge(cx)))
                    .child(
                        h_flex()
                            .min_w_0()
                            .items_start()
                            .gap(px(SPACE_3))
                            .child(mark)
                            .child(
                                Self::prompt_text(&block.prompts[at], &block.whys[at], &colors, cx)
                                    .flex_1(),
                            )
                            .when(!status.is_empty(), |this| {
                                this.child(
                                    div()
                                        .flex_shrink_0()
                                        .text_size(ui_font_size(FONT_SM, cx))
                                        .text_color(colors.foreground_muted)
                                        .child(status),
                                )
                            }),
                    )
            });
            body = body.child(
                v_flex()
                    .gap(px(SPACE_2))
                    .child(
                        div()
                            .text_size(ui_font_size(SMALL_TEXT, cx))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.foreground)
                            .child(group_title(&group)),
                    )
                    .child(
                        v_flex()
                            .border_1()
                            .border_color(colors.border)
                            .rounded(px(RADIUS_LG))
                            .overflow_hidden()
                            .children(rows),
                    ),
            );
        }
        if ids.is_empty() {
            return body.into_any_element();
        }
        let pad = if compact { SPACE_4 } else { WIDE_PAD };
        let edit_texts: Vec<String> = ordered
            .iter()
            .filter_map(|id| index_of(id).map(|at| block.prompts[at].clone()))
            .collect();
        let send_keys: Vec<String> = present.iter().map(|c| c.key.clone()).collect();
        let states: Vec<Option<CandidateState>> =
            chosen.iter().map(|c| c.map(|c| c.state)).collect();
        let clear = self.button(
            ButtonSpec {
                id: format!("np-clear-{index}").into(),
                label: "Clear".into(),
                icon: None,
                look: Look::Quiet,
                disabled: live.pending || selected.is_empty(),
                faces: None,
            },
            move |this, cx| {
                this.selections[index].clear();
                cx.notify();
            },
            cx,
        );
        let edit = self.button(
            ButtonSpec {
                id: format!("np-edit-selected-{index}").into(),
                label: "Edit selected".into(),
                icon: Some(IconName::Pencil),
                look: Look::Outline,
                disabled: !can_send,
                faces: None,
            },
            move |this, cx| this.edit(edit_texts.clone(), cx),
            cx,
        );
        let send = self.button(
            ButtonSpec {
                id: format!("np-send-selected-{index}").into(),
                label: send_selected_label(live.pending, &states).into(),
                icon: Some(IconName::ArrowUp),
                look: Look::Primary,
                disabled: !can_send,
                faces: None,
            },
            move |this, cx| this.send(send_keys.clone(), cx),
            cx,
        );
        body.child(
            h_flex()
                .flex_wrap()
                .items_center()
                .justify_between()
                .gap_x(px(SPACE_4))
                .gap_y(px(SPACE_3))
                .py(px(SPACE_3))
                .px(px(pad))
                .mx(px(-pad))
                .border_t_1()
                .border_color(colors.border)
                .bg(colors.surface1)
                .child(
                    v_flex()
                        .min_w(px(FOOTER_MIN_WIDTH))
                        .child(
                            div()
                                .text_size(ui_font_size(SMALL_TEXT, cx))
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(colors.foreground)
                                .child(format!("{} selected", selected.len())),
                        )
                        .child(
                            div()
                                .text_size(ui_font_size(SECTION_LABEL, cx))
                                .text_color(colors.foreground_muted)
                                .child(footer_hint(ordered.len(), allowed_set)),
                        ),
                )
                .child(
                    h_flex()
                        .flex_wrap()
                        .items_center()
                        .justify_end()
                        .gap(px(SPACE_2))
                        .child(clear)
                        .child(edit)
                        .child(send),
                ),
        )
        .into_any_element()
    }

    fn recap_header(&self, recap: &Recap, done: bool, cx: &Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let commit = commit_chip(&recap.commit);
        let branch = recap
            .branch
            .strip_prefix('`')
            .unwrap_or(&recap.branch)
            .to_string();
        let branch = branch.strip_suffix('`').unwrap_or(&branch).to_string();
        let commit_icon = if commit.done {
            IconName::Check
        } else {
            IconName::GitCommitHorizontal
        };
        let mut chips: Vec<(String, Option<IconName>, Option<String>)> = vec![
            (branch, None, None),
            (commit.text, Some(commit_icon), Some(commit.label)),
        ];
        if done {
            chips.push(("Task done".into(), None, None));
        }
        let chip_row = h_flex()
            .flex_wrap()
            .items_center()
            .gap(px(SPACE_2))
            .child(
                icon(IconName::History)
                    .with_size(px(ICON_MD))
                    .text_color(colors.foreground_muted),
            )
            .child(
                div()
                    .mr_auto()
                    .text_size(ui_font_size(FONT_CONTENT, cx))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.foreground)
                    .child("Recap"),
            )
            .children(
                chips
                    .into_iter()
                    .enumerate()
                    .map(|(index, (text, name, label))| {
                        h_flex()
                            .id(("np-chip", index))
                            .role(Role::Label)
                            .aria_label(label.unwrap_or_else(|| text.clone()))
                            .max_w_full()
                            .items_center()
                            .gap(px(CHIP_GAP))
                            .px(px(SPACE_2))
                            .py(px(CHIP_PAD_Y))
                            .rounded(px(RADIUS_MD))
                            .bg(colors.surface2)
                            .children(name.map(|name| {
                                icon(name)
                                    .with_size(px(CHIP_ICON))
                                    .text_color(colors.foreground_muted)
                            }))
                            .child(
                                div()
                                    .min_w_0()
                                    .text_size(ui_font_size(FONT_SM, cx))
                                    .text_color(colors.foreground_muted)
                                    .child(text),
                            )
                    }),
            );
        // Labeled fields: each label above its value, divided from the one before.
        let sections = recap_sections(recap);
        let labeled = sections.is_some();
        let fields = sections.unwrap_or_else(|| {
            vec![RecapSection {
                label: String::new(),
                text: recap.did.clone(),
            }]
        });
        v_flex()
            .px(px(WIDE_PAD))
            .py(px(RECAP_PAD_Y))
            .gap(px(SPACE_1_5))
            .border_b_1()
            .border_color(colors.border)
            .bg(colors.surface1)
            .child(chip_row)
            .children(fields.into_iter().enumerate().map(|(index, field)| {
                let value = markdown(
                    SharedString::from(format!("np-recap-{index}")),
                    field.text,
                    cx,
                );
                if !labeled {
                    return div().child(value).into_any_element();
                }
                v_flex()
                    .when(index > 0, |this| {
                        this.mt(px(SPACE_1))
                            .pt(px(10.))
                            .border_t_1()
                            .border_color(colors.border)
                    })
                    .when(index == 0, |this| this.mt(px(2.)))
                    .child(
                        div()
                            .mb(px(2.))
                            .text_size(ui_font_size(SECTION_LABEL, cx))
                            .line_height(px(SECTION_LINE))
                            .font_weight(FontWeight::SEMIBOLD)
                            .text_color(colors.foreground)
                            .child(field.label),
                    )
                    .child(value)
                    .into_any_element()
            }))
            .into_any_element()
    }
}

impl Render for PanelView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(data) = self.data.clone() else {
            return div().into_any_element();
        };
        let colors = tokens(cx);
        let live = self.live(cx);
        let compact = is_compact(self.width.get());
        let done = self
            .blocks
            .iter()
            .any(|block| block.declaration.as_ref().is_some_and(|d| d.goal_done));
        let mut cards: Vec<AnyElement> = Vec::new();
        let mut other_work: Vec<AnyElement> = Vec::new();
        for (index, block) in self.blocks.iter().enumerate() {
            let candidates: Vec<Option<&Candidate>> = block
                .prompts
                .iter()
                .map(|text| candidate_for(&live.candidates, self.item_ms, &block.block, text))
                .collect();
            if shows_selection(block, &candidates) {
                cards.push(self.selection_block(index, block, &candidates, &live, compact, cx));
            } else {
                for (at, text) in block.prompts.iter().enumerate() {
                    if block.threads[at] {
                        continue;
                    }
                    let suggested = is_suggested(block, at, candidates[at]);
                    cards.push(self.prompt_card(
                        (index, at),
                        text,
                        &block.whys[at],
                        candidates[at],
                        suggested,
                        &live,
                        compact,
                        cx,
                    ));
                }
            }
            for (at, text) in block.prompts.iter().enumerate() {
                if block.threads[at] {
                    let suggested = is_suggested(block, at, candidates[at]);
                    other_work.push(self.prompt_card(
                        (index, at),
                        text,
                        &block.whys[at],
                        candidates[at],
                        suggested,
                        &live,
                        compact,
                        cx,
                    ));
                }
            }
        }
        let other_work = (!other_work.is_empty()).then(|| {
            v_flex()
                .id("np-other-work")
                .role(Role::Group)
                .aria_label("Other work")
                .gap(px(SPACE_2))
                .child(
                    div()
                        .text_size(ui_font_size(SMALL_TEXT, cx))
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(colors.foreground_muted)
                        .child("Other work"),
                )
                .child(
                    div()
                        .text_size(ui_font_size(FONT_SM, cx))
                        .text_color(colors.foreground_muted)
                        .child("Not part of this task. Starts a separate thread."),
                )
                .children(other_work)
        });
        let title = h_flex()
            .items_center()
            .gap(px(SPACE_2))
            .child(
                icon(IconName::CornerDownRight)
                    .with_size(px(ICON_MD))
                    .text_color(colors.accent),
            )
            .child(
                div()
                    .text_size(ui_font_size(FONT_CONTENT, cx))
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(colors.foreground)
                    .child(data.title.clone()),
            )
            .when(done && data.recap.is_none(), |this| {
                this.child(
                    div()
                        .text_size(ui_font_size(FONT_SM, cx))
                        .text_color(colors.foreground_muted)
                        .child("Task done"),
                )
            });
        let intro = (!data.intro.is_empty()).then(|| markdown("np-intro", data.intro.clone(), cx));
        let notices = live.notices.iter().map(|notice| {
            div()
                .text_size(ui_font_size(SMALL_TEXT, cx))
                .text_color(colors.foreground_muted)
                .child(notice.clone())
        });
        let body = v_flex()
            .min_w_0()
            .p(px(if compact { SPACE_4 } else { WIDE_PAD }))
            .gap(px(SPACE_4))
            .child(v_flex().gap(px(SPACE_1)).child(title).children(intro))
            .children(cards)
            .children(other_work)
            .children(notices);
        let width = self.width.clone();
        let measure = canvas(
            move |bounds, window, _| {
                let measured = f32::from(bounds.size.width);
                let before = width.replace(measured);
                if is_compact(before) != is_compact(measured) {
                    window.request_animation_frame();
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        div()
            .relative()
            .w_full()
            .py(px(SPACE_1_5))
            .child(
                v_flex()
                    .id("next-prompt-panel")
                    .role(Role::Group)
                    .aria_label(SharedString::from(data.title.clone()))
                    .relative()
                    .w_full()
                    .min_w_0()
                    .rounded(px(RADIUS_XL))
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.surface0)
                    .overflow_hidden()
                    .children(
                        data.recap
                            .as_ref()
                            .map(|recap| self.recap_header(recap, done, cx)),
                    )
                    .child(body)
                    .child(measure),
            )
            .into_any_element()
    }
}
