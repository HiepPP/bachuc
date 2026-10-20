//! The New agent defaults pane: the model, effort, and mode a new draft composer starts with.
//! The choices come from the active host; a pick is stored in `agent-defaults.json` at once.

use crate::agent_defaults::{AgentDefaults, load, save};
use crate::ui::{
    card, column, error_text, note, page_title, row, row_text, scroll_page, section, select_trigger,
};
use bachuc_client::{Client, ClientEvent};
use bachuc_theme::metrics::FONT_BASE;
use bachuc_theme::{ui_font_size, zoomed};
use gpui_kit::component::Side;
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenuItem};
use gpui_kit::component::theme::Theme;
use gpui_kit::*;

const DEFAULT: &str = "Default";

/// The label of `id` in `choices`; an id the host does not offer shows as is.
fn choice_label(id: Option<&str>, choices: &[(String, String)]) -> String {
    match id {
        None => DEFAULT.into(),
        Some(id) => choices
            .iter()
            .find(|(choice, _)| choice == id)
            .map_or(id, |(_, label)| label.as_str())
            .to_string(),
    }
}

/// The defaults after a model pick. Another provider drops the mode; a thinking option the new
/// model does not offer is dropped.
fn pick_model(
    current: &AgentDefaults,
    provider: Option<String>,
    model: Option<String>,
    thinking_options: &[(String, String)],
) -> AgentDefaults {
    let same_provider = current.provider == provider;
    AgentDefaults {
        thinking_option_id: current
            .thinking_option_id
            .clone()
            .filter(|id| thinking_options.iter().any(|(option, _)| option == id)),
        mode_id: current.mode_id.clone().filter(|_| same_provider),
        provider,
        model,
    }
}

pub struct AgentDefaultsPane {
    client: Entity<Client>,
    current: AgentDefaults,
    /// The modes of the stored provider, from the host.
    modes: Vec<(String, String)>,
    modes_request: u64,
    error: Option<SharedString>,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl AgentDefaultsPane {
    pub fn new(client: Entity<Client>, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![
            cx.observe(&client, |_, _, cx| cx.notify()),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
            cx.subscribe(&client, |this, _, event: &ClientEvent, cx| {
                if matches!(event, ClientEvent::Connected) {
                    this.load_modes(cx);
                }
            }),
        ];
        let mut pane = Self {
            client,
            current: load(),
            modes: Vec::new(),
            modes_request: 0,
            error: None,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        };
        pane.load_modes(cx);
        pane
    }

    fn load_modes(&mut self, cx: &mut Context<Self>) {
        self.modes_request += 1;
        let request = self.modes_request;
        self.modes.clear();
        cx.notify();
        let Some(provider) = self.current.provider.clone() else {
            return;
        };
        if !self.client.read(cx).is_connected() {
            return;
        }
        let modes = self
            .client
            .update(cx, |client, cx| client.provider_modes(&provider, None, cx));
        cx.spawn(async move |this, cx| {
            let modes = modes.await;
            let _ = this.update(cx, |this, cx| {
                if this.modes_request != request {
                    return;
                }
                match modes {
                    Ok(modes) => this.modes = modes,
                    Err(error) => log::warn!("cannot list modes of {provider}: {error}"),
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn persist(&mut self, next: AgentDefaults, cx: &mut Context<Self>) {
        let provider_changed = next.provider != self.current.provider;
        self.error = save(&next)
            .err()
            .map(|error| format!("Cannot save the defaults: {error:#}").into());
        self.current = next;
        if provider_changed {
            self.load_modes(cx);
        }
        cx.notify();
    }

    fn thinking_options(&self, provider: &str, model: &str, cx: &App) -> Vec<(String, String)> {
        self.client
            .read(cx)
            .model_thinking(provider, model)
            .map(|thinking| thinking.options)
            .unwrap_or_default()
    }

    fn set_model(&mut self, pick: Option<(String, String)>, cx: &mut Context<Self>) {
        let (provider, model) = pick.unzip();
        let options = match (&provider, &model) {
            (Some(provider), Some(model)) => self.thinking_options(provider, model, cx),
            _ => Vec::new(),
        };
        let next = pick_model(&self.current, provider, model, &options);
        self.persist(next, cx);
    }

    fn set_thinking(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        let next = AgentDefaults {
            thinking_option_id: id,
            ..self.current.clone()
        };
        self.persist(next, cx);
    }

    fn set_mode(&mut self, id: Option<String>, cx: &mut Context<Self>) {
        let next = AgentDefaults {
            mode_id: id,
            ..self.current.clone()
        };
        self.persist(next, cx);
    }

    /// A menu row at the interface size, checked when it is the current choice.
    fn text_item(
        label: SharedString,
        checked: bool,
        on_pick: impl Fn(&mut Self, &mut Context<Self>) + 'static,
        weak: WeakEntity<Self>,
    ) -> PopupMenuItem {
        PopupMenuItem::element(move |_, cx| {
            div()
                .flex_1()
                .truncate()
                .text_size(ui_font_size(FONT_BASE, cx))
                .child(label.clone())
        })
        .checked(checked)
        .on_click(move |_, _, cx| {
            weak.update(cx, |this, cx| on_pick(this, cx)).ok();
        })
    }

    /// Default, then each available provider's models under its name.
    fn model_dropdown(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let providers: Vec<_> = self
            .client
            .read(cx)
            .providers()
            .iter()
            .filter(|provider| provider.available)
            .cloned()
            .collect();
        let current = self
            .current
            .provider
            .clone()
            .zip(self.current.model.clone());
        let shown = match &current {
            None => DEFAULT.to_string(),
            Some((provider, model)) => {
                let models = providers
                    .iter()
                    .find(|summary| summary.id == *provider)
                    .map(|summary| summary.models.as_slice())
                    .unwrap_or_default();
                choice_label(Some(model), models)
            }
        };
        let weak = cx.weak_entity();
        select_trigger(
            "agent-defaults-model",
            None,
            shown.clone(),
            format!("Model: {shown}"),
            cx,
        )
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, app| {
            let menu = menu
                .min_w(zoomed(240., app))
                .max_h(zoomed(360., app))
                .scrollable(true)
                .check_side(Side::Right)
                .item(Self::text_item(
                    DEFAULT.into(),
                    current.is_none(),
                    |this, cx| this.set_model(None, cx),
                    weak.clone(),
                ));
            providers.iter().fold(menu, |menu, provider| {
                provider.models.iter().fold(
                    menu.separator().label(provider.label.clone()),
                    |menu, (model, label)| {
                        let pick = (provider.id.clone(), model.clone());
                        let checked = current.as_ref() == Some(&pick);
                        menu.item(Self::text_item(
                            label.clone().into(),
                            checked,
                            move |this, cx| this.set_model(Some(pick.clone()), cx),
                            weak.clone(),
                        ))
                    },
                )
            })
        })
    }

    /// Default, then `choices`; `current` is checked.
    fn choice_dropdown(
        id: &'static str,
        name: &'static str,
        current: Option<String>,
        choices: Vec<(String, String)>,
        set: fn(&mut Self, Option<String>, &mut Context<Self>),
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let shown = choice_label(current.as_deref(), &choices);
        let weak = cx.weak_entity();
        select_trigger(id, None, shown.clone(), format!("{name}: {shown}"), cx)
            .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, app| {
                let menu =
                    menu.min_w(zoomed(200., app))
                        .check_side(Side::Right)
                        .item(Self::text_item(
                            DEFAULT.into(),
                            current.is_none(),
                            move |this, cx| set(this, None, cx),
                            weak.clone(),
                        ));
                choices.iter().fold(menu, |menu, (choice, label)| {
                    let pick = choice.clone();
                    menu.item(Self::text_item(
                        label.clone().into(),
                        current.as_ref() == Some(choice),
                        move |this, cx| set(this, Some(pick.clone()), cx),
                        weak.clone(),
                    ))
                })
            })
    }
}

impl Render for AgentDefaultsPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let thinking = match (&self.current.provider, &self.current.model) {
            (Some(provider), Some(model)) => self.thinking_options(provider, model, cx),
            _ => Vec::new(),
        };
        let model = self.model_dropdown(cx);
        let effort = Self::choice_dropdown(
            "agent-defaults-effort",
            "Effort",
            self.current.thinking_option_id.clone(),
            thinking,
            Self::set_thinking,
            cx,
        );
        let mode = Self::choice_dropdown(
            "agent-defaults-mode",
            "Mode",
            self.current.mode_id.clone(),
            self.modes.clone(),
            Self::set_mode,
            cx,
        );
        let body = card(cx)
            .id("agent-defaults")
            .role(Role::Group)
            .aria_label("New agent defaults")
            .child(
                row(true, cx)
                    .child(row_text("Model", Some("The provider and model".into()), cx))
                    .child(model),
            )
            .child(
                row(false, cx)
                    .child(row_text(
                        "Effort",
                        Some("The thinking option of the model".into()),
                        cx,
                    ))
                    .child(effort),
            )
            .child(
                row(false, cx)
                    .child(row_text("Mode", Some("The permission mode".into()), cx))
                    .child(mode),
            );
        scroll_page(
            "agent-defaults-pane",
            &self.scroll,
            column(cx)
                .child(page_title("New agent defaults", cx))
                .child(section("Draft composer", None, body, cx))
                .child(note(
                    "New workspace drafts start with these values. A value the host does not \
                     offer falls back to the built-in default.",
                    cx,
                ))
                .children(self.error.clone().map(|error| error_text(error, cx))),
            cx,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::{AgentDefaults, choice_label, pick_model};

    fn options(ids: &[&str]) -> Vec<(String, String)> {
        ids.iter()
            .map(|id| (id.to_string(), id.to_uppercase()))
            .collect()
    }

    #[test]
    fn labels_show_default_the_host_label_or_the_raw_id() {
        assert_eq!(choice_label(None, &options(&["high"])), "Default");
        assert_eq!(choice_label(Some("high"), &options(&["high"])), "HIGH");
        assert_eq!(choice_label(Some("gone"), &options(&["high"])), "gone");
    }

    #[test]
    fn a_model_pick_keeps_what_still_applies() {
        let current = AgentDefaults {
            provider: Some("claude".into()),
            model: Some("claude-opus-5-5".into()),
            thinking_option_id: Some("high".into()),
            mode_id: Some("bypassPermissions".into()),
        };
        let same = pick_model(
            &current,
            Some("claude".into()),
            Some("claude-sonnet-5-5".into()),
            &options(&["medium", "high"]),
        );
        assert_eq!(same.thinking_option_id.as_deref(), Some("high"));
        assert_eq!(same.mode_id.as_deref(), Some("bypassPermissions"));
        let other = pick_model(
            &current,
            Some("codex".into()),
            Some("gpt".into()),
            &options(&["low"]),
        );
        assert_eq!(other.thinking_option_id, None);
        assert_eq!(other.mode_id, None);
        assert_eq!(
            pick_model(&current, None, None, &[]),
            AgentDefaults::default()
        );
    }
}
