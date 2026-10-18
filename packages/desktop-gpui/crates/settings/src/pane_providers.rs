//! The Providers pane: which providers the daemon reports, and their models. Read only.

use crate::ui::{card, column, note, page_title, row, row_text, scroll_page, section, tag};
use bachuc_client::{Client, ProviderSummary};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_SM, ICON_SM, SPACE_1, SPACE_4};
use bachuc_theme::{tokens, ui_font_size, zoomed};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::HashSet;

pub struct ProvidersPane {
    client: Entity<Client>,
    /// The providers whose model list is open.
    open: HashSet<String>,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl ProvidersPane {
    pub fn new(client: Entity<Client>, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![
            cx.observe(&client, |_, _, cx| cx.notify()),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
        ];
        Self {
            client,
            open: HashSet::new(),
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    fn toggle(&mut self, id: &str, cx: &mut Context<Self>) {
        if !self.open.remove(id) {
            self.open.insert(id.to_string());
        }
        cx.notify();
    }

    fn provider_row(
        &self,
        index: usize,
        provider: &ProviderSummary,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let open = self.open.contains(&provider.id);
        let id = provider.id.clone();
        let count = provider.models.len();
        let hint = if !provider.available {
            None
        } else if count == 1 {
            Some("1 model".into())
        } else if count > 1 {
            Some(format!("{count} models").into())
        } else {
            None
        };
        let status = if provider.available {
            tag("Available", colors.status_success, colors.tint_success, cx)
        } else {
            tag(
                "Not installed",
                colors.status_warning,
                colors.tint_warning,
                cx,
            )
        };
        let expandable = provider.available && count > 0;
        let header = row(index == 0, cx)
            .id(("provider-row", index))
            .role(Role::Button)
            .aria_label(format!(
                "{} {}",
                provider.label,
                if provider.available {
                    "available"
                } else {
                    "not installed"
                }
            ))
            .aria_expanded(open)
            .when(expandable, |this| {
                this.cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| this.toggle(&id, cx)))
            })
            .child(row_text(provider.label.clone(), hint, cx))
            .child(status)
            .when(expandable, |this| {
                this.child(
                    icon(if open {
                        IconName::ChevronUp
                    } else {
                        IconName::ChevronDown
                    })
                    .with_size(zoomed(ICON_SM, cx))
                    .text_color(colors.foreground_muted),
                )
            });
        let models = open.then(|| {
            v_flex()
                .id(("provider-models", index))
                .role(Role::List)
                .aria_label(format!("{} models", provider.label))
                .px(zoomed(SPACE_4, cx))
                .pb(zoomed(SPACE_4, cx))
                .gap(zoomed(SPACE_1, cx))
                .children(provider.models.iter().map(|(model_id, label)| {
                    div()
                        .id(SharedString::from(format!("model-{model_id}")))
                        .role(Role::ListItem)
                        .aria_label(label.clone())
                        .flex()
                        .flex_row()
                        .justify_between()
                        .gap(zoomed(SPACE_4, cx))
                        .text_size(ui_font_size(FONT_SM, cx))
                        .child(div().text_color(colors.foreground).child(label.clone()))
                        .child(
                            div()
                                .text_color(colors.foreground_muted)
                                .truncate()
                                .child(model_id.clone()),
                        )
                }))
        });
        v_flex().child(header).children(models).into_any_element()
    }
}

impl Render for ProvidersPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let providers = self.client.read(cx).providers().to_vec();
        let rows: Vec<AnyElement> = providers
            .iter()
            .enumerate()
            .map(|(index, provider)| self.provider_row(index, provider, cx))
            .collect();
        let body = if rows.is_empty() {
            card(cx)
                .child(
                    div()
                        .p(zoomed(SPACE_4, cx))
                        .child(note("No providers reported yet", cx)),
                )
                .into_any_element()
        } else {
            card(cx)
                .id("provider-list")
                .role(Role::List)
                .aria_label("Providers")
                .children(rows)
                .into_any_element()
        };
        scroll_page(
            "providers-pane",
            &self.scroll,
            column(cx).child(page_title("Providers", cx)).child(section(
                "Providers",
                None,
                body,
                cx,
            )),
            cx,
        )
    }
}
