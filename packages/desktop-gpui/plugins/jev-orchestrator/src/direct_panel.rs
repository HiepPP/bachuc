//! The Direct route panel (`client/direct-panel.tsx`). `DirectState` in `model` holds the prompt,
//! the selected profiles and efforts, and the locked request; this file sends the requests and
//! draws one list row per visual row group.

use crate::model::{
    DIRECT_PROFILES, DIRECT_RUN, DIRECT_STATUS, DirectItem, DirectState, Effort, EffortCard,
    ProfileRow, ProfilesInput, ProfilesPayload, RecordStatus, RunResult, SelectionView,
    StatusPayload, clamp_prompt, created_line, new_request_id,
};
use bachuc_client::{Client, ClientEvent};
use bachuc_plugin_host::{PanelInput, PluginContext};
use bachuc_theme::metrics::{
    FONT_2XL, FONT_BASE, FONT_SM, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2, SPACE_3, WEIGHT_SEMIBOLD,
};
use bachuc_theme::{Tokens, tokens, ui_font_size, zoomed};
use gpui_kit::component::input::{InputEvent, Textarea, TextareaState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

const TITLE: &str = "Direct Jev routing";
const INTRO: &str = "Choose profiles, then explicitly allow effort levels for every selected model. Jev can only choose an allowed profile, model, and effort combination. Luna offers max effort only. The selected profile's mode and feature values stay unchanged. The current agent is not involved.";
const PROMPT_LABEL: &str = "Direct routing prompt";
const PROMPT_PLACEHOLDER: &str = "Describe the task for the new agent";
const PROFILES_HINT: &str = "Select at least one profile. Jev only chooses from this list.";
const EFFORT_HINT: &str =
    "Select at least one offered effort for each model. Only checked efforts are sent to Jev.";

/// Electron's `padding: compact ? 12 : 20` and `gap: 14`, on desktop.
const CONTENT_PADDING: f32 = 20.;
const CONTENT_GAP: f32 = 14.;
/// `effortSection.gap`: the heading, the hint, and the cards.
const EFFORT_GAP: f32 = 10.;
/// `lineHeight: 21` of every text style but the route label.
const LINE_HEIGHT: f32 = 21.;
const PROMPT_MIN_HEIGHT: f32 = 120.;
const PROMPT_MIN_ROWS: usize = 5;
const PROMPT_MAX_ROWS: usize = 12;
const OUTCOME_GAP: f32 = 6.;
const BUTTON_PADDING: f32 = 10.;
const DIMMED: f32 = 0.65;
const DISABLED_ROUTE: f32 = 0.55;
const OVERDRAW: f32 = 400.;

pub(crate) fn build(
    ctx: &PluginContext,
    input: PanelInput,
    window: &mut Window,
    cx: &mut App,
) -> AnyView {
    let ctx = ctx.clone();
    cx.new(|cx| DirectPanel::new(ctx, input.workspace_id, window, cx))
        .into()
}

pub(crate) struct DirectPanel {
    ctx: PluginContext,
    state: DirectState,
    items: Vec<DirectItem>,
    list: ListState,
    prompt: Entity<TextareaState>,
    /// The panel opened before `Connected`; the profiles load on the first one.
    waiting_for_connection: bool,
    /// Held strong: a subscription holds its emitter weakly.
    _client: Option<Entity<Client>>,
    _subscriptions: Vec<Subscription>,
    // Dropping the view drops these and cancels the requests.
    _profiles: Option<Task<()>>,
    _run: Option<Task<()>>,
    _status: Option<Task<()>>,
}

impl DirectPanel {
    fn new(
        ctx: PluginContext,
        workspace_id: String,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let client: Option<Entity<Client>> = ctx.client(cx);
        let connected = client.as_ref().is_some_and(|c| c.read(cx).is_connected());
        let prompt = cx.new(|cx| {
            TextareaState::new(window, cx)
                .placeholder(PROMPT_PLACEHOLDER)
                .auto_grow(PROMPT_MIN_ROWS, PROMPT_MAX_ROWS)
        });
        let mut subscriptions = vec![cx.subscribe_in(
            &prompt,
            window,
            |panel: &mut Self, input, event: &InputEvent, window, cx| {
                if !matches!(event, InputEvent::Change) {
                    return;
                }
                let mut value = input.read(cx).value().to_string();
                if let Some(cut) = clamp_prompt(&value) {
                    input.update(cx, |input, cx| input.set_value(cut.clone(), window, cx));
                    value = cut;
                }
                panel.state.set_prompt(value);
                panel.sync(cx);
            },
        )];
        if let Some(client) = &client {
            subscriptions.push(cx.subscribe(
                client,
                |panel: &mut Self, _, event: &ClientEvent, cx| {
                    if matches!(event, ClientEvent::Connected) && panel.waiting_for_connection {
                        panel.waiting_for_connection = false;
                        panel.state.profiles_fetching = true;
                        panel.load_profiles(cx);
                        panel.sync(cx);
                    }
                },
            ));
        }
        let mut panel = Self {
            ctx,
            state: DirectState::new(workspace_id),
            items: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            prompt,
            waiting_for_connection: !connected,
            _client: client,
            _subscriptions: subscriptions,
            _profiles: None,
            _run: None,
            _status: None,
        };
        if connected {
            panel.load_profiles(cx);
        } else {
            // A request sent before `Connected` fails and is not retried: show the error now.
            panel.state.profiles_loaded(Err(()));
        }
        panel.sync(cx);
        panel
    }

    fn load_profiles(&mut self, cx: &mut Context<Self>) {
        let input = ProfilesInput {
            workspace_id: self.state.workspace_id.clone(),
        };
        let answer = self
            .ctx
            .rpc_as::<_, ProfilesPayload>(DIRECT_PROFILES, &input, cx);
        self._profiles = Some(cx.spawn(async move |this, cx| {
            let result = answer.await.map(|payload| payload.profiles);
            this.update(cx, |panel, cx| {
                panel
                    .state
                    .profiles_loaded(log_failure("direct.profiles", result));
                panel.sync(cx);
            })
            .ok();
        }));
    }

    fn retry_profiles(&mut self, cx: &mut Context<Self>) {
        if self.state.retry_profiles() {
            self.load_profiles(cx);
            self.sync(cx);
        }
    }

    fn route(&mut self, cx: &mut Context<Self>) {
        let Some(request) = self.state.route(new_request_id) else {
            return;
        };
        self.sync(cx);
        let answer = self.ctx.rpc_as::<_, RunResult>(DIRECT_RUN, &request, cx);
        self._run = Some(cx.spawn(async move |this, cx| {
            let result = answer.await;
            this.update(cx, |panel, cx| {
                panel.state.run_finished(log_failure("direct.run", result));
                panel.sync(cx);
            })
            .ok();
        }));
    }

    fn check_status(&mut self, cx: &mut Context<Self>) {
        let Some(input) = self.state.check_status() else {
            return;
        };
        self.sync(cx);
        let answer = self
            .ctx
            .rpc_as::<_, StatusPayload>(DIRECT_STATUS, &input, cx);
        self._status = Some(cx.spawn(async move |this, cx| {
            let result = answer.await.map(|payload| payload.records);
            this.update(cx, |panel, cx| {
                panel
                    .state
                    .status_finished(log_failure("direct.status", result));
                panel.sync(cx);
            })
            .ok();
        }));
    }

    fn reset(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.state.reset();
        self._run = None;
        self._status = None;
        self.prompt
            .update(cx, |prompt, cx| prompt.set_value(String::new(), window, cx));
        self.sync(cx);
    }

    /// Measures the rows again only when the rows differ.
    fn sync(&mut self, cx: &mut Context<Self>) {
        let items = self.state.items();
        if items != self.items {
            self.list.splice(0..self.items.len(), items.len());
            self.items = items;
        }
        cx.notify();
    }

    fn gap_after(&self, index: usize) -> f32 {
        match (&self.items[index], self.items.get(index + 1)) {
            (DirectItem::EffortIntro, _) => EFFORT_GAP,
            (DirectItem::EffortGroup(_), Some(DirectItem::EffortGroup(_))) => EFFORT_GAP,
            _ => CONTENT_GAP,
        }
    }

    fn render_row(&mut self, index: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(item) = self.items.get(index).cloned() else {
            return div().into_any_element();
        };
        let colors = tokens(cx);
        let last = index + 1 == self.items.len();
        let body = match item {
            DirectItem::Title => v_stack(CONTENT_GAP, cx)
                .child(
                    div()
                        .id("jev-direct-title")
                        .role(Role::Heading)
                        .aria_label(TITLE)
                        .text_size(ui_font_size(FONT_2XL, cx))
                        .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                        .child(TITLE),
                )
                .child(muted(INTRO, colors, cx))
                .into_any_element(),
            DirectItem::Prompt { locked } => v_stack(CONTENT_GAP, cx)
                .child(heading("Prompt", cx))
                .child(
                    Textarea::new(&self.prompt)
                        .readonly(locked)
                        .min_h(zoomed(PROMPT_MIN_HEIGHT, cx))
                        .bg(colors.surface1)
                        .border_color(colors.border)
                        .rounded(zoomed(RADIUS_LG, cx))
                        .accessibility_id("jev-direct-prompt")
                        .aria_label(PROMPT_LABEL),
                )
                .into_any_element(),
            DirectItem::ProfilesIntro => v_stack(CONTENT_GAP, cx)
                .child(heading("Allowed profiles", cx))
                .child(muted(PROFILES_HINT, colors, cx))
                .into_any_element(),
            DirectItem::ProfilesLoading => muted("Loading profiles…", colors, cx),
            DirectItem::ProfilesError { fetching } => v_stack(SPACE_2, cx)
                .child(
                    danger("Could not load profiles.", colors, cx)
                        .id("jev-direct-profiles-error")
                        .role(Role::Alert)
                        .aria_label("Could not load profiles."),
                )
                .child(self.secondary_button(
                    "jev-direct-retry-profiles",
                    if fetching { "Retrying…" } else { "Retry profiles" },
                    !fetching,
                    colors,
                    cx,
                    |panel, _, cx| panel.retry_profiles(cx),
                ))
                .into_any_element(),
            DirectItem::ProfilesEmpty => muted(
                "No direct-routing profiles are available for this workspace.",
                colors,
                cx,
            ),
            DirectItem::Profile(row) => self.profile_row(row, colors, cx),
            DirectItem::EffortIntro => v_stack(EFFORT_GAP, cx)
                .child(heading("Allowed model efforts", cx))
                .child(muted(EFFORT_HINT, colors, cx))
                .into_any_element(),
            DirectItem::EffortGroup(card) => self.effort_card(card, colors, cx),
            DirectItem::Route {
                enabled,
                label,
                aria,
            } => div()
                .id("jev-direct-route")
                .role(Role::Button)
                .aria_label(aria)
                .p(zoomed(SPACE_3, cx))
                .rounded(zoomed(RADIUS_MD, cx))
                .bg(colors.accent)
                .text_size(ui_font_size(FONT_BASE, cx))
                .line_height(relative(1.2))
                .font_weight(FontWeight(WEIGHT_SEMIBOLD))
                .text_color(colors.accent_foreground)
                .text_center()
                .when(!enabled, |button| {
                    button
                        .opacity(DISABLED_ROUTE)
                        .aria_description("Disabled")
                })
                .when(enabled, |button| {
                    button
                        .cursor_pointer()
                        .on_click(cx.listener(|panel, _: &ClickEvent, _, cx| panel.route(cx)))
                })
                .child(label)
                .into_any_element(),
            DirectItem::RequestId(id) => muted(&format!("Request {id}"), colors, cx),
            DirectItem::RunError { checking } => v_stack(SPACE_2, cx)
                .child(
                    danger(
                        "The result is uncertain. Check the saved request before retrying or starting another task.",
                        colors,
                        cx,
                    )
                    .id("jev-direct-run-error")
                    .role(Role::Alert)
                    .aria_label("The result is uncertain"),
                )
                .child(self.secondary_button(
                    "jev-direct-check-status",
                    if checking { "Checking…" } else { "Check request status" },
                    !checking,
                    colors,
                    cx,
                    |panel, _, cx| panel.check_status(cx),
                ))
                .into_any_element(),
            DirectItem::StatusError => danger(
                "Could not check the saved request. Try checking again with the same request ID.",
                colors,
                cx,
            )
            .id("jev-direct-status-error")
            .role(Role::Alert)
            .aria_label("Could not check the saved request")
            .into_any_element(),
            DirectItem::StatusNone => muted(
                "No saved record was found. A retry will reuse the same request ID.",
                colors,
                cx,
            ),
            DirectItem::Record { status, error } => {
                self.record_card(status, error, colors, cx)
            }
            DirectItem::Created {
                agent_id,
                selection,
            } => self.created_card(agent_id, &selection, colors, cx),
            DirectItem::Reset => self.secondary_button(
                "jev-direct-reset",
                "Reset for a new task",
                true,
                colors,
                cx,
                |panel, window, cx| panel.reset(window, cx),
            ),
        };
        div()
            .w_full()
            .px(zoomed(CONTENT_PADDING, cx))
            .pt(zoomed(if index == 0 { CONTENT_PADDING } else { 0. }, cx))
            .pb(zoomed(
                if last {
                    CONTENT_PADDING
                } else {
                    self.gap_after(index)
                },
                cx,
            ))
            .line_height(ui_font_size(LINE_HEIGHT, cx))
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_color(colors.foreground)
            .child(body)
            .into_any_element()
    }

    /// `secondaryButton`. A disabled one keeps its look and loses its click.
    fn secondary_button(
        &self,
        id: &'static str,
        label: &'static str,
        enabled: bool,
        colors: Tokens,
        cx: &mut Context<Self>,
        on_press: impl Fn(&mut Self, &mut Window, &mut Context<Self>) + 'static,
    ) -> AnyElement {
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .p(zoomed(BUTTON_PADDING, cx))
            .rounded(zoomed(RADIUS_MD, cx))
            .bg(colors.surface2)
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_color(colors.foreground)
            .when(!enabled, |button| button.aria_description("Disabled"))
            .when(enabled, |button| {
                button.cursor_pointer().on_click(
                    cx.listener(move |panel, _: &ClickEvent, window, cx| {
                        on_press(panel, window, cx)
                    }),
                )
            })
            .child(label)
            .into_any_element()
    }

    fn profile_row(&self, row: ProfileRow, colors: Tokens, cx: &mut Context<Self>) -> AnyElement {
        let id = row.id.clone();
        div()
            .id(SharedString::from(format!("jev-direct-profile-{}", row.id)))
            .role(Role::CheckBox)
            .aria_label(row.title.clone())
            .aria_description(row.subtitle.clone())
            .aria_toggled(toggled(row.selected))
            .flex()
            .flex_col()
            .gap(zoomed(SPACE_1, cx))
            .p(zoomed(SPACE_3, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .border_1()
            .border_color(if row.selected {
                colors.accent
            } else {
                colors.border
            })
            .bg(if row.selected {
                colors.surface2
            } else {
                colors.surface1
            })
            .when(row.disabled, |card| card.opacity(DIMMED))
            .when(!row.disabled, |card| {
                card.cursor_pointer()
                    .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                        panel.state.toggle_profile(&id);
                        panel.sync(cx);
                    }))
            })
            .child(heading(&row.title, cx))
            .child(muted(&row.subtitle, colors, cx))
            .when(row.no_effort, |card| {
                card.child(
                    muted_div(colors, cx)
                        .text_color(colors.status_danger)
                        .child("No effort levels are currently available for this profile."),
                )
            })
            .children(row.notes.map(|notes| div().child(notes)))
            .into_any_element()
    }

    fn effort_card(&self, card: EffortCard, colors: Tokens, cx: &mut Context<Self>) -> AnyElement {
        let chips: Vec<AnyElement> = card
            .chips
            .iter()
            .map(|(effort, selected)| {
                self.effort_chip(&card.key, *effort, *selected, card.locked, colors, cx)
            })
            .collect();
        v_stack(SPACE_2, cx)
            .p(zoomed(SPACE_3, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .border_1()
            .border_color(colors.border)
            .bg(colors.surface1)
            .child(heading(&card.title, cx))
            .children(card.note.map(|note| muted(&note, colors, cx)))
            .child(
                div()
                    .flex()
                    .flex_row()
                    .flex_wrap()
                    .gap(zoomed(SPACE_2, cx))
                    .children(chips),
            )
            .into_any_element()
    }

    fn effort_chip(
        &self,
        key: &str,
        effort: Effort,
        selected: bool,
        locked: bool,
        colors: Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let key = key.to_string();
        let label = effort.as_str();
        div()
            .id(SharedString::from(format!(
                "jev-direct-effort-{key}-{label}"
            )))
            .role(Role::CheckBox)
            .aria_label(label)
            .aria_toggled(toggled(selected))
            .px(zoomed(SPACE_3, cx))
            .py(zoomed(SPACE_2, cx))
            .rounded(zoomed(RADIUS_MD, cx))
            .border_1()
            .border_color(if selected {
                colors.accent
            } else {
                colors.border
            })
            .bg(if selected {
                colors.surface2
            } else {
                colors.surface0
            })
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_color(colors.foreground)
            .when(locked, |chip| {
                chip.opacity(DIMMED).aria_description("Disabled")
            })
            .when(!locked, |chip| {
                chip.cursor_pointer()
                    .on_click(cx.listener(move |panel, _: &ClickEvent, _, cx| {
                        panel.state.toggle_effort(&key, effort);
                        panel.sync(cx);
                    }))
            })
            .child(if selected {
                format!("Selected · {label}")
            } else {
                label.to_string()
            })
            .into_any_element()
    }

    fn record_card(
        &self,
        status: RecordStatus,
        error: Option<String>,
        colors: Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let title = format!("Saved request · {}", status.as_str());
        outcome_card(colors, cx)
            .id("jev-direct-record")
            .role(Role::Group)
            .aria_label(title.clone())
            .child(heading(&title, cx))
            .children(error.map(|error| div().child(error)))
            .when(status == RecordStatus::Interrupted, |card| {
                card.child(muted(
                    "This outcome is uncertain. Keep this request ID and inspect the saved agent labels.",
                    colors,
                    cx,
                ))
            })
            .into_any_element()
    }

    fn created_card(
        &self,
        agent_id: Option<String>,
        selection: &SelectionView,
        colors: Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let line = created_line(selection);
        outcome_card(colors, cx)
            .id("jev-direct-created")
            .role(Role::Group)
            .aria_label("Agent created")
            .aria_description(line.clone())
            .child(heading("Agent created", cx))
            .child(div().child(line))
            .children(agent_id.map(|agent_id| {
                self.secondary_button(
                    "jev-direct-open-agent",
                    "Open agent",
                    true,
                    colors,
                    cx,
                    move |panel, _, cx| panel.ctx.open_agent(&agent_id, cx),
                )
            }))
            .into_any_element()
    }
}

impl Render for DirectPanel {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        div()
            .id("jev-direct-panel")
            .role(Role::Group)
            .aria_label("Direct route")
            .size_full()
            .bg(colors.surface0)
            .child(list(self.list.clone(), cx.processor(Self::render_row)).size_full())
    }
}

/// Logs a failed call and drops its error: the panel shows a fixed text.
fn log_failure<T>(method: &str, result: anyhow::Result<T>) -> Result<T, ()> {
    result.map_err(|error| log::warn!("jev-orchestrator: {method} failed: {error:#}"))
}

fn toggled(on: bool) -> Toggled {
    if on { Toggled::True } else { Toggled::False }
}

fn v_stack(gap: f32, cx: &App) -> Div {
    div().flex().flex_col().gap(zoomed(gap, cx))
}

fn heading(text: &str, cx: &App) -> Div {
    div()
        .text_size(ui_font_size(FONT_BASE, cx))
        .font_weight(FontWeight(WEIGHT_SEMIBOLD))
        .child(text.to_string())
}

fn muted_div(colors: Tokens, cx: &App) -> Div {
    div()
        .text_size(ui_font_size(FONT_SM, cx))
        .text_color(colors.foreground_muted)
}

fn muted(text: &str, colors: Tokens, cx: &App) -> AnyElement {
    muted_div(colors, cx)
        .child(text.to_string())
        .into_any_element()
}

fn danger(text: &str, colors: Tokens, cx: &App) -> Div {
    div()
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_color(colors.status_danger)
        .child(text.to_string())
}

fn outcome_card(colors: Tokens, cx: &App) -> Div {
    v_stack(OUTCOME_GAP, cx)
        .p(zoomed(SPACE_3, cx))
        .rounded(zoomed(RADIUS_LG, cx))
        .border_1()
        .border_color(colors.border)
}
