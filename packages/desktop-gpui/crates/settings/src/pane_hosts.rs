//! The Hosts pane: add, edit, and remove the saved `ws://` hosts (ADR-0040).

use crate::hosts::validate_host;
use crate::ui::{
    callout, card, column, error_text, field_label, icon_button, note, page_title, row, row_text,
    scroll_page, section, tag,
};
use bachuc_client::{Client, HostConfig, load_hosts, save_hosts};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{ICON_MD, SPACE_2, SPACE_3, SPACE_4};
use bachuc_theme::{tokens, zoomed};
use gpui_kit::component::button::{Button, ButtonVariants as _};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// The saved hosts changed in `hosts.json`. The app reconnects when it must.
pub struct HostsChanged;

struct HostForm {
    editing: Option<usize>,
    name: Entity<InputState>,
    url: Entity<InputState>,
    password: Entity<InputState>,
    error: Option<SharedString>,
    _subscriptions: Vec<Subscription>,
}

pub struct HostsPane {
    client: Entity<Client>,
    hosts: Vec<HostConfig>,
    form: Option<HostForm>,
    /// The index of the host that waits for a remove confirmation.
    removing: Option<usize>,
    error: Option<SharedString>,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<HostsChanged> for HostsPane {}

impl HostsPane {
    pub fn new(client: Entity<Client>, cx: &mut Context<Self>) -> Self {
        let subscriptions = vec![
            cx.observe(&client, |_, _, cx| cx.notify()),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
        ];
        Self {
            client,
            hosts: load_hosts(),
            form: None,
            removing: None,
            error: None,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        }
    }

    fn open_form(&mut self, editing: Option<usize>, window: &mut Window, cx: &mut Context<Self>) {
        let existing = editing.and_then(|index| self.hosts.get(index)).cloned();
        let name = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("release")
                .default_value(
                    existing
                        .as_ref()
                        .map(|h| h.name.clone())
                        .unwrap_or_default(),
                )
        });
        let url = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("ws://127.0.0.1:6768/ws")
                .default_value(existing.as_ref().map(|h| h.url.clone()).unwrap_or_default())
        });
        let password = cx.new(|cx| {
            InputState::new(window, cx)
                .placeholder("Optional")
                .masked(true)
                .default_value(
                    existing
                        .as_ref()
                        .and_then(|h| h.password.clone())
                        .unwrap_or_default(),
                )
        });
        let subscriptions = [&name, &url, &password]
            .into_iter()
            .map(|input| {
                cx.subscribe_in(
                    input,
                    window,
                    |this: &mut HostsPane, _, event: &InputEvent, _, cx| {
                        if let InputEvent::PressEnter { .. } = event {
                            this.submit(cx);
                        }
                    },
                )
            })
            .collect();
        name.update(cx, |input, cx| input.focus(window, cx));
        self.removing = None;
        self.error = None;
        self.form = Some(HostForm {
            editing,
            name,
            url,
            password,
            error: None,
            _subscriptions: subscriptions,
        });
        cx.notify();
    }

    fn submit(&mut self, cx: &mut Context<Self>) {
        let Some(form) = &self.form else { return };
        let built = validate_host(
            &form.name.read(cx).value(),
            &form.url.read(cx).value(),
            &form.password.read(cx).value(),
            &self.hosts,
            form.editing,
        );
        let host = match built {
            Ok(host) => host,
            Err(error) => {
                if let Some(form) = &mut self.form {
                    form.error = Some(error.to_string().into());
                }
                cx.notify();
                return;
            }
        };
        let editing = form.editing;
        let mut hosts = self.hosts.clone();
        match editing {
            Some(index) if index < hosts.len() => hosts[index] = host,
            _ => hosts.push(host),
        }
        self.commit(hosts, cx);
        if self.error.is_none() {
            self.form = None;
        }
        cx.notify();
    }

    /// Saves `hosts` to `hosts.json` and, on success, adopts them and tells the app.
    fn commit(&mut self, hosts: Vec<HostConfig>, cx: &mut Context<Self>) {
        match save_hosts(&hosts) {
            Ok(()) => {
                self.hosts = hosts;
                self.error = None;
                cx.emit(HostsChanged);
            }
            Err(error) => {
                log::warn!("cannot save hosts: {error:#}");
                self.error = Some(format!("Could not save the hosts: {error}").into());
            }
        }
    }

    fn confirm_remove(&mut self, cx: &mut Context<Self>) {
        let Some(index) = self.removing.take() else {
            return;
        };
        if index < self.hosts.len() {
            let mut hosts = self.hosts.clone();
            hosts.remove(index);
            self.commit(hosts, cx);
        }
        cx.notify();
    }

    fn request_remove(&mut self, index: usize, cx: &mut Context<Self>) {
        self.form = None;
        if self.hosts.len() <= 1 {
            // An empty hosts.json is read back as the release and live seeds.
            self.removing = None;
            self.error = Some("Keep at least one host".into());
        } else {
            self.error = None;
            self.removing = Some(index);
        }
        cx.notify();
    }

    fn host_row(&self, index: usize, host: &HostConfig, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let client = self.client.read(cx);
        let current = client.host().url == host.url;
        let online = client.is_connected();
        let name = host.name.clone();
        let removing = self.removing == Some(index);
        let mut item = v_flex()
            .id(("host-row", index))
            .role(Role::ListItem)
            .aria_label(format!("Host {name}"))
            .child(
                row(index == 0, cx)
                    .child(
                        icon(IconName::Server)
                            .with_size(zoomed(ICON_MD, cx))
                            .text_color(colors.foreground_muted),
                    )
                    .child(row_text(
                        host.name.clone(),
                        Some(host.url.clone().into()),
                        cx,
                    ))
                    .when(current, |this| {
                        this.child(if online {
                            tag("Connected", colors.status_success, colors.tint_success, cx)
                        } else {
                            tag("Connecting", colors.status_warning, colors.tint_warning, cx)
                        })
                    })
                    .child(
                        icon_button(
                            ("host-edit", index),
                            IconName::Pencil,
                            format!("Edit {name}"),
                            cx,
                        )
                        .on_click(cx.listener(
                            move |this, _, window, cx| this.open_form(Some(index), window, cx),
                        )),
                    )
                    .child(
                        icon_button(
                            ("host-remove", index),
                            IconName::Trash2,
                            format!("Remove {name}"),
                            cx,
                        )
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.request_remove(index, cx)),
                        ),
                    ),
            );
        if removing {
            let message = if current {
                format!(
                    "Remove {name}? This host is connected. Removing it disconnects the client."
                )
            } else {
                format!("Remove {name}? The host leaves hosts.json.")
            };
            item = item.child(
                div().px(zoomed(SPACE_4, cx)).pb(zoomed(SPACE_4, cx)).child(
                    callout(cx)
                        .id(("host-remove-confirm", index))
                        .role(Role::AlertDialog)
                        .aria_label(format!("Confirm remove {name}"))
                        .child(div().child(message))
                        .child(
                            h_flex()
                                .gap(zoomed(SPACE_2, cx))
                                .justify_end()
                                .child(
                                    Button::new("host-remove-cancel")
                                        .ghost()
                                        .small()
                                        .label("Cancel")
                                        .accessibility_label("Cancel remove")
                                        .on_click(cx.listener(|this, _, _, cx| {
                                            this.removing = None;
                                            cx.notify();
                                        })),
                                )
                                .child(
                                    Button::new("host-remove-confirm-button")
                                        .danger()
                                        .small()
                                        .label("Remove")
                                        .accessibility_label("Confirm remove host")
                                        .on_click(
                                            cx.listener(|this, _, _, cx| this.confirm_remove(cx)),
                                        ),
                                ),
                        ),
                ),
            );
        }
        item.into_any_element()
    }

    fn render_form(&self, form: &HostForm, cx: &mut Context<Self>) -> AnyElement {
        let title = if form.editing.is_some() {
            "Edit host"
        } else {
            "Add host"
        };
        let field = |label: &'static str, input: Input, cx: &App| {
            v_flex()
                .gap(zoomed(bachuc_theme::metrics::SPACE_1, cx))
                .child(field_label(label, cx))
                .child(input)
        };
        section(
            title,
            None,
            card(cx)
                .id("host-form")
                .role(Role::Form)
                .aria_label(title)
                .p(zoomed(SPACE_4, cx))
                .gap(zoomed(SPACE_3, cx))
                .child(field(
                    "Name",
                    Input::new(&form.name)
                        .accessibility_id("host-name-input")
                        .aria_label("Host name"),
                    cx,
                ))
                .child(field(
                    "URL",
                    Input::new(&form.url)
                        .accessibility_id("host-url-input")
                        .aria_label("Host URL"),
                    cx,
                ))
                .child(field(
                    "Password",
                    Input::new(&form.password)
                        .mask_toggle()
                        .accessibility_id("host-password-input")
                        .aria_label("Host password"),
                    cx,
                ))
                .children(form.error.clone().map(|error| error_text(error, cx)))
                .child(
                    h_flex()
                        .gap(zoomed(SPACE_2, cx))
                        .justify_end()
                        .child(
                            Button::new("host-form-cancel")
                                .ghost()
                                .label("Cancel")
                                .accessibility_label("Cancel host form")
                                .on_click(cx.listener(|this, _, _, cx| {
                                    this.form = None;
                                    cx.notify();
                                })),
                        )
                        .child(
                            Button::new("host-form-save")
                                .primary()
                                .label("Save")
                                .accessibility_label("Save host")
                                .on_click(cx.listener(|this, _, _, cx| this.submit(cx))),
                        ),
                ),
            cx,
        )
        .into_any_element()
    }
}

impl Render for HostsPane {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let current = self.client.read(cx).host().clone();
        let removed_current = !self.hosts.iter().any(|h| h.url == current.url);
        let rows: Vec<AnyElement> = self
            .hosts
            .clone()
            .iter()
            .enumerate()
            .map(|(index, host)| self.host_row(index, host, cx))
            .collect();

        let add = Button::new("host-add")
            .ghost()
            .small()
            .icon(icon(IconName::Plus))
            .label("Add host")
            .accessibility_label("Add host")
            .on_click(cx.listener(|this, _, window, cx| this.open_form(None, window, cx)))
            .into_any_element();

        let mut body = column(cx).child(page_title("Hosts", cx));
        if removed_current {
            body = body.child(
                div().mb(zoomed(SPACE_4, cx)).child(
                    callout(cx)
                        .id("host-disconnected")
                        .role(Role::Status)
                        .aria_label("Disconnected")
                        .child(div().child("Disconnected"))
                        .child(note(
                            format!(
                                "{} is no longer a saved host. Pick another host to connect.",
                                current.name
                            ),
                            cx,
                        )),
                ),
            );
        }
        if let Some(form) = &self.form {
            body = body.child(self.render_form(form, cx));
        }
        body = body.child(section(
            "Hosts",
            Some(add),
            v_flex()
                .gap(zoomed(SPACE_2, cx))
                .children(self.error.clone().map(|error| error_text(error, cx)))
                .child(
                    card(cx)
                        .id("host-list")
                        .role(Role::List)
                        .aria_label("Hosts")
                        .children(rows),
                ),
            cx,
        ));
        scroll_page("hosts-pane", &self.scroll, body, cx)
    }
}
