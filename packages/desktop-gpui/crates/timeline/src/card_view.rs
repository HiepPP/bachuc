//! Permission cards: `PermissionRequestCard` in `packages/app/src/agent-stream/view.tsx`, the
//! plan card of `plan-card.tsx`, and the question form of `question-form-card.tsx`. Every answer
//! goes out through `respond_permission_with` with the payload the app sends.

use std::hash::{Hash, Hasher};

use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, ICON_MD, ICON_SM, ICON_XS, RADIUS_BASE, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2,
    SPACE_3,
};
use bachuc_theme::{Tokens, tokens, ui_font_size, zoomed};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::text::TextView;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use serde_json::Value;

use crate::TimelineView;
use crate::cards::{self, Action, Question, QuestionForm};
use crate::tools::{self, PERMISSION_DETAIL_MAX_HEIGHT};

/// The text block line heights of the cards.
const TITLE_LINE_HEIGHT: f32 = 22.;
const DESCRIPTION_LINE_HEIGHT: f32 = 20.;
/// The question form's option control.
const CONTROL_SIZE: f32 = 18.;
const RADIO_DOT: f32 = 8.;
const NAV_HEIGHT: f32 = 28.;
const PRESSED_OPACITY: f32 = 0.5;

/// A question form being filled in: its questions, answers, and the text field of the active
/// question.
pub(crate) struct FormState {
    pub questions: Vec<Question>,
    pub form: QuestionForm,
    pub input: Entity<InputState>,
    _subscription: Subscription,
}

impl TimelineView {
    /// Creates the form state of each question request that has none. Needs the window for the
    /// text field, so the row renderer calls it.
    pub(crate) fn ensure_form(
        &mut self,
        request: &Value,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let id = request["id"].as_str().unwrap_or("").to_string();
        if self.forms.contains_key(&id) {
            return;
        }
        let Some(questions) = cards::parse_questions(&request["input"]) else {
            return;
        };
        let placeholder = cards::placeholder(&questions[0]);
        let input = cx.new(|cx| InputState::new(window, cx).placeholder(placeholder));
        let request_id = id.clone();
        let subscription = cx.subscribe_in(
            &input,
            window,
            move |this, input, event: &InputEvent, window, cx| match event {
                InputEvent::Change => {
                    let text = input.read(cx).value().to_string();
                    if let Some(state) = this.forms.get_mut(&request_id) {
                        let active = state.form.active;
                        state.form.set_other(&state.questions, active, text);
                        this.refresh(cx);
                    }
                }
                InputEvent::PressEnter { .. } => this.form_primary(&request_id, window, cx),
                _ => {}
            },
        );
        let count = questions.len();
        self.forms.insert(
            id,
            FormState {
                questions,
                form: QuestionForm::new(count),
                input,
                _subscription: subscription,
            },
        );
    }

    /// Hashes what a permission row draws, so the list measures it again when it changes.
    pub(crate) fn hash_permission(&self, id: &str, hasher: &mut impl Hasher) {
        id.hash(hasher);
        self.responding.get(id).hash(hasher);
        self.plan_toggled.contains(id).hash(hasher);
        if let Some(state) = self.forms.get(id) {
            state.form.hash(hasher);
        }
    }

    fn respond_with(
        &mut self,
        request_id: String,
        action: String,
        response: Value,
        cx: &mut Context<Self>,
    ) {
        let agent_id = self.agent_id.clone();
        self.client.update(cx, |client, cx| {
            client.respond_permission_with(&agent_id, &request_id, response, cx)
        });
        self.responding.insert(request_id, action);
        self.refresh(cx);
    }

    fn toggle_plan(&mut self, id: String, cx: &mut Context<Self>) {
        if !self.plan_toggled.remove(&id) {
            self.plan_toggled.insert(id);
        }
        self.refresh(cx);
    }

    fn select_question(
        &mut self,
        request_id: &str,
        index: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.forms.get_mut(request_id) else {
            return;
        };
        state.form.active = index;
        let text = state.form.others[index].clone();
        let placeholder = cards::placeholder(&state.questions[index]);
        state.input.update(cx, |input, cx| {
            input.set_value(text, window, cx);
            input.set_placeholder(placeholder, window, cx);
        });
        self.refresh(cx);
    }

    fn toggle_option(
        &mut self,
        request_id: &str,
        question: usize,
        option: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(state) = self.forms.get_mut(request_id) else {
            return;
        };
        let before = state.form.active;
        state.form.toggle(&state.questions, question, option);
        let cleared = !state.questions[question].multi_select;
        let after = state.form.active;
        if cleared {
            state
                .input
                .update(cx, |input, cx| input.set_value("", window, cx));
        }
        if after != before {
            self.select_question(request_id, after, window, cx);
        } else {
            self.refresh(cx);
        }
    }

    /// Next, or Submit on the last question.
    fn form_primary(&mut self, request_id: &str, window: &mut Window, cx: &mut Context<Self>) {
        if self.responding.contains_key(request_id) {
            return;
        }
        let Some(state) = self.forms.get(request_id) else {
            return;
        };
        if !state.form.can_advance(&state.questions) {
            return;
        }
        if !state.form.is_last(&state.questions) {
            let next = state.form.active + 1;
            self.select_question(request_id, next, window, cx);
            return;
        }
        let Some(input) = self.permission_input(request_id) else {
            return;
        };
        let response = state.form.submit_response(&state.questions, &input);
        self.respond_with(request_id.to_string(), "submit".into(), response, cx);
    }

    fn form_dismiss(&mut self, request_id: &str, cx: &mut Context<Self>) {
        let Some(state) = self.forms.get(request_id) else {
            return;
        };
        let input = self.permission_input(request_id).unwrap_or(Value::Null);
        let response = state.form.dismiss_response(&state.questions, &input);
        self.respond_with(request_id.to_string(), "dismiss".into(), response, cx);
    }

    fn permission_input(&self, request_id: &str) -> Option<Value> {
        self.rows.iter().find_map(|row| match &row.kind {
            crate::RowKind::Permission(request) if request["id"] == request_id => {
                Some(request["input"].clone())
            }
            _ => None,
        })
    }

    /// A pending permission: the question form, the plan card, or the generic card.
    pub(crate) fn render_permission(&self, request: &Value, cx: &mut Context<Self>) -> AnyElement {
        let id = request["id"].as_str().unwrap_or("").to_string();
        if cards::is_question(request)
            && let Some(state) = self.forms.get(&id)
        {
            return self.render_question_form(&id, state, cx);
        }
        let t = tokens(cx);
        let footer = self.render_actions(&id, request, &t, cx);
        let plan = cards::plan_text(request);
        if cards::is_plan(request)
            && let Some(plan) = plan
        {
            let open = !self.plan_toggled.contains(&id);
            let description = request["description"].as_str().map(str::to_string);
            return self.plan_card(
                &id,
                "Plan".into(),
                description,
                plan,
                open,
                Some(footer),
                true,
                cx,
            );
        }
        let title = cards::title(request);
        let detail = if request["detail"].is_object() {
            request["detail"].clone()
        } else {
            serde_json::json!({"type": "unknown", "input": request["input"], "output": null})
        };
        let sections = tools::sections(&detail, None);
        let size = ui_font_size(FONT_BASE, cx);
        let key = permission_key(&id);
        let proposed = plan.map(|plan| {
            let open = !self.plan_toggled.contains(&id);
            self.plan_card(
                &id,
                "Proposed plan".into(),
                None,
                plan,
                open,
                None,
                true,
                cx,
            )
        });
        v_flex()
            .id(("permission", key))
            .role(Role::Group)
            .aria_label(format!("Permission request: {title}"))
            .my(zoomed(SPACE_3, cx))
            .p(zoomed(SPACE_3, cx))
            .gap(zoomed(SPACE_2, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .border_1()
            .border_color(t.border)
            .bg(t.surface1)
            .text_size(size)
            .child(
                div()
                    .line_height(zoomed(TITLE_LINE_HEIGHT, cx))
                    .text_color(t.foreground)
                    .child(title),
            )
            .children(
                request["description"]
                    .as_str()
                    .filter(|d| !d.is_empty())
                    .map(|text| {
                        div()
                            .line_height(zoomed(DESCRIPTION_LINE_HEIGHT, cx))
                            .text_color(t.foreground_muted)
                            .child(text.to_string())
                    }),
            )
            .children(proposed)
            .when(!cards::is_plan(request) && !sections.is_empty(), |card| {
                card.child(self.render_sections(
                    key,
                    &sections,
                    &detail,
                    PERMISSION_DETAIL_MAX_HEIGHT,
                    cx,
                ))
            })
            .child(footer)
            .into_any_element()
    }

    /// "How would you like to proceed?" and one button per action.
    fn render_actions(
        &self,
        id: &str,
        request: &Value,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let responding = self.responding.get(id).cloned();
        let key = permission_key(id);
        let buttons = cards::actions(request)
            .into_iter()
            .enumerate()
            .map(|(position, action)| {
                let is_responding = responding.as_deref() == Some(action.id.as_str());
                let request_id = id.to_string();
                let color = if action.primary {
                    t.foreground
                } else {
                    t.foreground_muted
                };
                let fill = t.surface2;
                let Action {
                    id: action_id,
                    label,
                    allow,
                    ..
                } = action.clone();
                h_flex()
                    .id(("permission-action", key.wrapping_add(position as u64)))
                    .role(Role::Button)
                    .aria_label(label.clone())
                    .max_w_full()
                    .min_w_0()
                    .gap(zoomed(SPACE_2, cx))
                    .items_center()
                    .py(zoomed(SPACE_2, cx))
                    .px(zoomed(SPACE_3, cx))
                    .rounded(zoomed(RADIUS_MD, cx))
                    .border_1()
                    .border_color(t.border_accent)
                    .bg(t.surface1)
                    .when(responding.is_none(), |button| {
                        button
                            .cursor_pointer()
                            .hover(move |button| button.bg(fill))
                            .on_click(cx.listener(move |this, _, _, cx| {
                                let response = cards::action_response(&action);
                                this.respond_with(
                                    request_id.clone(),
                                    action_id.clone(),
                                    response,
                                    cx,
                                );
                            }))
                    })
                    .map(|button| {
                        if is_responding {
                            button.child(Spinner::new().with_size(zoomed(ICON_SM, cx)).color(color))
                        } else {
                            button
                                .child(
                                    icon(if allow { IconName::Check } else { IconName::X })
                                        .with_size(zoomed(ICON_SM, cx))
                                        .text_color(color),
                                )
                                .child(div().min_w_0().text_color(color).child(label))
                        }
                    })
            });
        v_flex()
            .gap(zoomed(SPACE_2, cx))
            .child(
                div()
                    .my(zoomed(SPACE_1, cx))
                    .text_color(t.foreground_muted)
                    .child("How would you like to proceed?"),
            )
            .child(
                h_flex()
                    .flex_wrap()
                    .gap(zoomed(SPACE_2, cx))
                    .items_center()
                    .children(buttons),
            )
            .into_any_element()
    }

    /// A plan tool call: the plan card with its outcome as the title.
    pub(crate) fn render_plan_tool(
        &self,
        key: u64,
        item: &Value,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let outcome = cards::plan_outcome(item);
        let id = format!("tool-{key}");
        let toggled = self.plan_toggled.contains(&id);
        let open = cards::plan_starts_open(outcome) != toggled;
        let text = item["detail"]["text"].as_str().unwrap_or("").to_string();
        let title = cards::plan_title(outcome, "Plan");
        self.plan_card(&id, title, None, text, open, None, false, cx)
    }

    /// `PlanCard`: a header button that folds the plan, the plan markdown, and a footer.
    #[allow(clippy::too_many_arguments)]
    fn plan_card(
        &self,
        id: &str,
        title: String,
        description: Option<String>,
        text: String,
        open: bool,
        footer: Option<AnyElement>,
        compact: bool,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let key = permission_key(id);
        let toggle_id = id.to_string();
        v_flex()
            .id(("plan", key))
            .role(Role::Group)
            .aria_label(title.clone())
            .when(!compact, |card| card.my(zoomed(SPACE_3, cx)))
            .p(zoomed(SPACE_3, cx))
            .gap(zoomed(SPACE_2, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .border_1()
            .border_color(t.border)
            .bg(t.surface1)
            .text_size(ui_font_size(FONT_BASE, cx))
            .child(
                h_flex()
                    .id(("plan-toggle", key))
                    .role(Role::Button)
                    .aria_label(title.clone())
                    .aria_expanded(open)
                    .min_h(zoomed(24., cx))
                    .gap(zoomed(SPACE_2, cx))
                    .items_center()
                    .cursor_pointer()
                    .on_click(
                        cx.listener(move |this, _, _, cx| this.toggle_plan(toggle_id.clone(), cx)),
                    )
                    .child(
                        icon(if open {
                            IconName::ChevronDown
                        } else {
                            IconName::ChevronRight
                        })
                        .with_size(zoomed(ICON_MD, cx))
                        .text_color(t.foreground_muted),
                    )
                    .child(
                        div()
                            .flex_shrink(1.)
                            .min_w_0()
                            .line_height(zoomed(TITLE_LINE_HEIGHT, cx))
                            .text_color(t.foreground)
                            .child(title),
                    ),
            )
            .when(open, |card| {
                card.child(
                    v_flex()
                        .gap(zoomed(SPACE_2, cx))
                        .children(description.filter(|d| !d.is_empty()).map(|text| {
                            div()
                                .line_height(zoomed(DESCRIPTION_LINE_HEIGHT, cx))
                                .text_color(t.foreground_muted)
                                .child(text)
                        }))
                        .child(TextView::markdown(
                            ElementId::Name(format!("plan-text-{id}").into()),
                            text,
                        )),
                )
            })
            .children(footer)
            .into_any_element()
    }

    /// `QuestionFormCard`: question tabs, the active question's options and text field, and
    /// Dismiss and Next or Submit.
    fn render_question_form(
        &self,
        id: &str,
        state: &FormState,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let key = permission_key(id);
        let questions = &state.questions;
        let form = &state.form;
        let active = form.active.min(questions.len() - 1);
        let question = &questions[active];
        let responding = self.responding.get(id).cloned();
        let is_responding = responding.is_some();
        let size = ui_font_size(FONT_BASE, cx);

        let nav = (questions.len() > 1).then(|| {
            h_flex()
                .id(("question-tabs", key))
                .role(Role::TabList)
                .aria_label("Questions")
                .flex_wrap()
                .items_center()
                .gap(zoomed(SPACE_1, cx))
                .px(zoomed(SPACE_3, cx))
                .children(questions.iter().enumerate().map(|(index, q)| {
                    let selected = index == active;
                    let answered = form.is_answered(questions, index);
                    let request_id = id.to_string();
                    let hover = t.surface2;
                    h_flex()
                        .id(("question-tab", key.wrapping_add(index as u64)))
                        .role(Role::Tab)
                        .aria_label(format!("Question {} of {}", index + 1, questions.len()))
                        .aria_selected(selected)
                        .min_h(zoomed(NAV_HEIGHT, cx))
                        .max_w_full()
                        .min_w_0()
                        .gap(zoomed(SPACE_1, cx))
                        .items_center()
                        .px(zoomed(SPACE_2, cx))
                        .py(zoomed(SPACE_1, cx))
                        .rounded(zoomed(RADIUS_MD, cx))
                        .border_1()
                        .border_color(if selected {
                            t.foreground_muted
                        } else {
                            t.border
                        })
                        .bg(if selected { t.surface2 } else { t.surface1 })
                        .when(!is_responding, |tab| {
                            tab.cursor_pointer()
                                .hover(move |tab| tab.bg(hover))
                                .on_click(cx.listener(move |this, _, window, cx| {
                                    this.select_question(&request_id, index, window, cx)
                                }))
                        })
                        .when(answered, |tab| {
                            tab.child(
                                icon(IconName::Check)
                                    .with_size(zoomed(ICON_XS, cx))
                                    .text_color(if selected {
                                        t.foreground
                                    } else {
                                        t.foreground_muted
                                    }),
                            )
                        })
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_color(if selected {
                                    t.foreground
                                } else {
                                    t.foreground_muted
                                })
                                .child(q.header.clone()),
                        )
                }))
        });

        let selected_options = &form.selections[active];
        let options = (!question.options.is_empty()).then(|| {
            v_flex()
                .id(("question-options", key))
                .role(if question.multi_select {
                    Role::Group
                } else {
                    Role::RadioGroup
                })
                .aria_label(question.question.clone())
                .gap(zoomed(SPACE_1, cx))
                .children(
                    question
                        .options
                        .iter()
                        .enumerate()
                        .map(|(option_index, option)| {
                            let selected = selected_options.contains(&option_index);
                            let multi = question.multi_select;
                            let request_id = id.to_string();
                            let hover = t.surface2;
                            let control = div()
                                .size(zoomed(CONTROL_SIZE, cx))
                                .mt(zoomed(2., cx))
                                .flex_none()
                                .flex()
                                .items_center()
                                .justify_center()
                                .border_1()
                                .border_color(if selected {
                                    t.accent
                                } else {
                                    t.foreground_extra_muted
                                })
                                .map(|control| {
                                    if multi {
                                        control.rounded(zoomed(RADIUS_BASE, cx)).when(
                                            selected,
                                            |c| {
                                                c.bg(t.accent).child(
                                                    icon(IconName::Check)
                                                        .with_size(zoomed(ICON_XS, cx))
                                                        .text_color(t.accent_foreground),
                                                )
                                            },
                                        )
                                    } else {
                                        control.rounded_full().when(selected, |c| {
                                            c.child(
                                                div()
                                                    .size(zoomed(RADIO_DOT, cx))
                                                    .rounded_full()
                                                    .bg(t.accent),
                                            )
                                        })
                                    }
                                });
                            h_flex()
                                .id(("question-option", key.wrapping_add(option_index as u64)))
                                .role(if multi {
                                    Role::CheckBox
                                } else {
                                    Role::RadioButton
                                })
                                .aria_label(option.label.clone())
                                .aria_toggled(if selected {
                                    Toggled::True
                                } else {
                                    Toggled::False
                                })
                                .items_start()
                                .gap(zoomed(SPACE_2, cx))
                                .px(zoomed(SPACE_3, cx))
                                .py(zoomed(SPACE_2, cx))
                                .rounded(zoomed(RADIUS_MD, cx))
                                .when(selected, |row| row.bg(t.surface2))
                                .when(!is_responding, |row| {
                                    row.cursor_pointer()
                                        .hover(move |row| row.bg(hover))
                                        .on_click(cx.listener(move |this, _, window, cx| {
                                            this.toggle_option(
                                                &request_id,
                                                active,
                                                option_index,
                                                window,
                                                cx,
                                            )
                                        }))
                                })
                                .child(control)
                                .child(
                                    v_flex()
                                        .flex_1()
                                        .min_w_0()
                                        .gap(zoomed(SPACE_1, cx))
                                        .child(
                                            div()
                                                .line_height(zoomed(TITLE_LINE_HEIGHT, cx))
                                                .text_color(if selected {
                                                    t.foreground
                                                } else {
                                                    t.foreground_muted
                                                })
                                                .child(option.label.clone()),
                                        )
                                        .children(option.description.clone().map(|text| {
                                            div()
                                                .line_height(zoomed(DESCRIPTION_LINE_HEIGHT, cx))
                                                .text_color(t.foreground_muted)
                                                .child(text)
                                        })),
                                )
                        }),
                )
        });

        let other = question.shows_text_input().then(|| {
            let has_text = !form.others[active].is_empty();
            div()
                .px(zoomed(SPACE_3, cx))
                .py(zoomed(SPACE_3, cx))
                .rounded(zoomed(RADIUS_LG, cx))
                .border_1()
                .border_color(if has_text { t.border_accent } else { t.border })
                .bg(t.surface2)
                .child(
                    Input::new(&state.input)
                        .appearance(false)
                        .bordered(false)
                        .focus_bordered(false)
                        .px_0()
                        .py_0()
                        .text_size(size)
                        .disabled(is_responding)
                        .aria_label(question.question.clone()),
                )
        });

        let dismiss_label = cards::dismiss_label(questions);
        let is_last = form.is_last(questions);
        let primary_label = if is_last { "Submit" } else { "Next" };
        let primary_disabled = is_responding || !form.can_advance(questions);
        let dismiss_id = id.to_string();
        let primary_id = id.to_string();
        let hover = t.surface2;
        let action_button = |button: Stateful<Div>| {
            button
                .gap(zoomed(SPACE_2, cx))
                .items_center()
                .py(zoomed(SPACE_2, cx))
                .px(zoomed(SPACE_3, cx))
                .rounded(zoomed(RADIUS_MD, cx))
                .border_1()
        };
        let dismiss = action_button(h_flex().id(("question-dismiss", key)))
            .role(Role::Button)
            .aria_label(dismiss_label.clone())
            .border_color(t.border_accent)
            .bg(t.surface1)
            .when(!is_responding, |button| {
                button
                    .cursor_pointer()
                    .hover(move |button| button.bg(hover))
                    .on_click(cx.listener(move |this, _, _, cx| this.form_dismiss(&dismiss_id, cx)))
            })
            .map(|button| {
                if responding.as_deref() == Some("dismiss") {
                    button.child(
                        Spinner::new()
                            .with_size(zoomed(ICON_SM, cx))
                            .color(t.foreground_muted),
                    )
                } else {
                    button
                        .child(
                            icon(IconName::X)
                                .with_size(zoomed(ICON_SM, cx))
                                .text_color(t.foreground_muted),
                        )
                        .child(div().text_color(t.foreground_muted).child(dismiss_label))
                }
            });
        let primary = action_button(h_flex().id(("question-primary", key)))
            .role(Role::Button)
            .aria_label(primary_label)
            .when(primary_disabled, |button| button.opacity(PRESSED_OPACITY))
            .border_color(t.accent)
            .bg(t.accent)
            .when(!primary_disabled, |button| {
                button
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.form_primary(&primary_id, window, cx)
                    }))
            })
            .map(|button| {
                if responding.as_deref() == Some("submit") {
                    button.child(
                        Spinner::new()
                            .with_size(zoomed(ICON_SM, cx))
                            .color(t.accent_foreground),
                    )
                } else {
                    button
                        .child(
                            icon(IconName::Check)
                                .with_size(zoomed(ICON_SM, cx))
                                .text_color(t.accent_foreground),
                        )
                        .child(div().text_color(t.accent_foreground).child(primary_label))
                }
            });

        v_flex()
            .id(("question-form", key))
            .role(Role::Group)
            .aria_label(format!("Question: {}", question.question))
            .p(zoomed(SPACE_3, cx))
            .gap(zoomed(SPACE_3, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .border_1()
            .border_color(t.border)
            .bg(t.surface1)
            .text_size(size)
            .children(nav)
            .child(
                div()
                    .px(zoomed(SPACE_3, cx))
                    .pb(zoomed(SPACE_1, cx))
                    .line_height(zoomed(TITLE_LINE_HEIGHT, cx))
                    .text_color(t.foreground)
                    .child(question.question.clone()),
            )
            .child(
                v_flex()
                    .gap(zoomed(SPACE_2, cx))
                    .children(options)
                    .children(other),
            )
            .child(
                h_flex()
                    .gap(zoomed(SPACE_2, cx))
                    .items_center()
                    .child(dismiss)
                    .child(primary),
            )
            .into_any_element()
    }
}

/// A stable number for element ids from a request id.
fn permission_key(id: &str) -> u64 {
    let mut hasher = std::collections::hash_map::DefaultHasher::new();
    id.hash(&mut hasher);
    hasher.finish()
}
