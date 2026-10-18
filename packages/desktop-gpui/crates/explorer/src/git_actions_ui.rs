//! The git actions split button of the workspace header, drawn as `actions-split-button.tsx`
//! draws it: the primary action, a caret with the secondary list, and a spinner while the
//! primary action runs. `git_actions` holds the rules.

use crate::git_actions::{
    Action, ActionId, GitActions, PolicyState, Press, RefreshGate, ShipDefault, Step,
    build_git_actions, failure_message, press, ship_default_for, step, success_message,
    suggests_archive,
};
use bachuc_client::{Client, ClientEvent, GitStatus, GitStatusEvent, PrStatusReply};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{FONT_BASE, ICON_MD, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2};
use bachuc_theme::{ShadowSize, popup_offset, shadow, tokens, ui_font_size, zoomed};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;
use std::time::Duration;

/// `HEADER_CONTROL_HEIGHT` of `control-geometry.ts`.
const HEADER_CONTROL_HEIGHT: f32 = 26.;
/// `MENU_ITEM_HEIGHT.md` of `menu-geometry.ts`.
const MENU_ITEM_HEIGHT: f32 = 28.;
const MENU_MIN_WIDTH: f32 = 200.;
const TEXT_LINE_FACTOR: f32 = 1.5;
const REFRESH_COALESCE: Duration = Duration::from_millis(500);
const SPIN: Duration = Duration::from_millis(1000);

/// What the button asks the app to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum GitActionsEvent {
    /// An action ended, or an unavailable action was pressed; show `message`.
    Finished { ok: bool, message: String },
    /// View PR.
    OpenUrl { url: String },
    /// Run the app's archive flow for this workspace.
    ArchiveWorkspace,
    /// The user picked an action that changes the ship default; save it for this repo.
    ShipDefaultChanged { cwd: String, default: ShipDefault },
}

pub struct GitActionsButton {
    client: Entity<Client>,
    cwd: String,
    /// `None` until the first status arrives.
    status: Option<GitStatus>,
    status_failed: bool,
    pr: Option<PrStatusReply>,
    ship_default: ShipDefault,
    post_ship_archive_suggested: bool,
    running: Option<ActionId>,
    menu_open: bool,
    /// The status request in flight or last applied; an older answer is dropped.
    generation: u64,
    refresh_gate: RefreshGate,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<GitActionsEvent> for GitActionsButton {}

impl GitActionsButton {
    pub fn new(
        client: Entity<Client>,
        cwd: String,
        cx: &mut Context<GitActionsButton>,
    ) -> GitActionsButton {
        let subscriptions = vec![
            cx.subscribe(
                &client,
                |this: &mut GitActionsButton, _, event: &GitStatusEvent, cx| {
                    if let GitStatusEvent::Changed { cwd } = event
                        && *cwd == this.cwd
                    {
                        this.schedule_refresh(cx);
                    }
                },
            ),
            // A request sent while the client connects fails, so load again on each connection.
            cx.subscribe(
                &client,
                |this: &mut GitActionsButton, _, event: &ClientEvent, cx| {
                    if let ClientEvent::Connected = event {
                        this.refresh(cx);
                    }
                },
            ),
        ];
        let mut button = GitActionsButton {
            client,
            cwd,
            status: None,
            status_failed: false,
            pr: None,
            ship_default: ShipDefault::default(),
            post_ship_archive_suggested: false,
            running: None,
            menu_open: false,
            generation: 0,
            refresh_gate: RefreshGate::default(),
            _subscriptions: subscriptions,
        };
        button.refresh(cx);
        button
    }

    /// The saved ship default of this repo.
    pub fn set_ship_default(&mut self, default: ShipDefault, cx: &mut Context<GitActionsButton>) {
        if self.ship_default != default {
            self.ship_default = default;
            cx.notify();
        }
    }

    fn schedule_refresh(&mut self, cx: &mut Context<GitActionsButton>) {
        if !self.refresh_gate.request() {
            return;
        }
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(REFRESH_COALESCE).await;
            this.update(cx, |this, cx| {
                this.refresh_gate.fire();
                this.refresh(cx);
            })
            .ok();
        })
        .detach();
    }

    fn refresh(&mut self, cx: &mut Context<GitActionsButton>) {
        self.generation += 1;
        if !self.client.read(cx).is_connected() {
            // `ClientEvent::Connected` loads it.
            return;
        }
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let (status, pr) = self.client.update(cx, |client, cx| {
            (client.git_status(&cwd, cx), client.pr_status(&cwd, cx))
        });
        cx.spawn(async move |this, cx| {
            let status = status.await;
            let pr = pr.await;
            this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                match status {
                    Ok(status) => {
                        this.status = Some(status);
                        this.status_failed = false;
                    }
                    Err(error) => {
                        log::warn!("git_status {}: {error:#}", this.cwd);
                        this.status_failed = true;
                    }
                }
                this.pr = pr
                    .inspect_err(|error| log::debug!("pr_status {}: {error:#}", this.cwd))
                    .ok();
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn actions(&self, cx: &App) -> Option<GitActions> {
        let status = self.status.as_ref()?;
        let auto_merge_actions_enabled = self
            .client
            .read(cx)
            .server_info()
            .and_then(|info| info.features.as_ref())
            .is_some_and(|features| {
                features.checkout_forge_set_auto_merge == Some(true)
                    || features.checkout_github_set_auto_merge == Some(true)
            });
        let state = PolicyState {
            auto_merge_actions_enabled,
            ship_default: self.ship_default,
            post_ship_archive_suggested: self.post_ship_archive_suggested,
            actions_disabled: self.status_failed || self.running.is_some(),
        };
        Some(build_git_actions(status, self.pr.as_ref(), state))
    }

    fn pr_url(&self) -> Option<String> {
        self.pr
            .as_ref()?
            .status
            .as_ref()
            .map(|pr| pr.url.clone())
            .filter(|url| !url.is_empty())
    }

    fn on_press(&mut self, action: &Action, cx: &mut Context<GitActionsButton>) {
        match press(action) {
            Press::Ignore => {}
            Press::Explain(message) => {
                cx.emit(GitActionsEvent::Finished { ok: false, message });
            }
            Press::Run(id) => self.run(id, cx),
        }
    }

    fn run(&mut self, id: ActionId, cx: &mut Context<GitActionsButton>) {
        if let Some(default) = ship_default_for(id)
            && default != self.ship_default
        {
            self.ship_default = default;
            cx.emit(GitActionsEvent::ShipDefaultChanged {
                cwd: self.cwd.clone(),
                default,
            });
        }
        let pr_url = self.pr_url();
        let action = match step(id, pr_url.is_some()) {
            Step::OpenPr => {
                self.menu_open = false;
                if let Some(url) = pr_url {
                    cx.emit(GitActionsEvent::OpenUrl { url });
                }
                cx.notify();
                return;
            }
            Step::Archive => {
                self.menu_open = false;
                cx.emit(GitActionsEvent::ArchiveWorkspace);
                cx.notify();
                return;
            }
            Step::Daemon(action) => action,
        };
        self.running = Some(id);
        cx.notify();
        let cwd = self.cwd.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.git_action(&cwd, action, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                this.running = None;
                let (ok, message) = match result {
                    Ok(_) => {
                        if suggests_archive(id) {
                            this.post_ship_archive_suggested = true;
                        }
                        (true, success_message(id).to_string())
                    }
                    Err(error) => (false, failure_message(id, &format!("{error:#}"))),
                };
                cx.emit(GitActionsEvent::Finished { ok, message });
                this.refresh(cx);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn toggle_menu(&mut self, cx: &mut Context<GitActionsButton>) {
        self.menu_open = !self.menu_open;
        cx.notify();
    }

    fn spinner(&self, id: &'static str, cx: &App) -> AnyElement {
        icon(IconName::LoaderCircle)
            .with_size(zoomed(ICON_MD, cx))
            .text_color(tokens(cx).foreground)
            .with_animation(id, Animation::new(SPIN).repeat(), |icon, delta| {
                icon.transform(Transformation::rotate(percentage(delta)))
            })
            .into_any_element()
    }

    fn primary(&self, action: Action, cx: &mut Context<GitActionsButton>) -> AnyElement {
        let colors = tokens(cx);
        let pending = self.running == Some(action.id);
        let text_size = ui_font_size(FONT_BASE, cx);
        let content = if pending {
            self.spinner("git-actions-primary-spin", cx)
        } else {
            h_flex()
                .items_center()
                .gap(zoomed(SPACE_1, cx))
                .child(
                    icon(action_icon(action.id))
                        .with_size(zoomed(ICON_MD, cx))
                        .text_color(colors.foreground_muted),
                )
                .child(
                    div()
                        .text_size(text_size)
                        .line_height(text_size * TEXT_LINE_FACTOR)
                        .text_color(colors.foreground)
                        .whitespace_nowrap()
                        .child(action.label.clone()),
                )
                .into_any_element()
        };
        let disabled = action.disabled;
        let action = Rc::new(action);
        let ax_action = action.clone();
        let view = cx.weak_entity();
        div()
            .id("git-actions-primary")
            .role(Role::Button)
            .aria_label(action.label.clone())
            .h_full()
            .px(zoomed(SPACE_2, cx))
            .flex()
            .items_center()
            .justify_center()
            .when(disabled, |el| el.opacity(0.6))
            .when(!disabled, |el| {
                el.cursor_pointer()
                    .hover(move |style| style.bg(colors.surface2))
                    .on_click(cx.listener(move |this, _, _, cx| this.on_press(&action, cx)))
                    .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                        view.update(cx, |this, cx| this.on_press(&ax_action, cx))
                            .ok();
                    })
            })
            .child(content)
            .into_any_element()
    }

    fn caret(&self, cx: &mut Context<GitActionsButton>) -> AnyElement {
        let colors = tokens(cx);
        let open = self.menu_open;
        let view = cx.weak_entity();
        div()
            .id("git-actions-caret")
            .role(Role::Button)
            .aria_label("More options")
            .w(zoomed(HEADER_CONTROL_HEIGHT, cx))
            .h_full()
            .flex()
            .items_center()
            .justify_center()
            .border_l_1()
            .border_color(colors.border_accent)
            .cursor_pointer()
            .when(open, |el| el.bg(colors.surface2))
            .hover(move |style| style.bg(colors.surface2))
            .on_click(cx.listener(|this, _, _, cx| this.toggle_menu(cx)))
            .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                view.update(cx, |this, cx| this.toggle_menu(cx)).ok();
            })
            .child(
                icon(IconName::ChevronDown)
                    .with_size(zoomed(ICON_MD, cx))
                    .text_color(colors.foreground_extra_muted),
            )
            .into_any_element()
    }

    fn menu_item(
        &self,
        index: usize,
        action: Action,
        cx: &mut Context<GitActionsButton>,
    ) -> AnyElement {
        let colors = tokens(cx);
        let pending = self.running == Some(action.id);
        let text_size = ui_font_size(FONT_BASE, cx);
        // An item with an unavailable message stays pressable to show it (`muted`).
        let pressable = !pending && (!action.disabled || action.unavailable.is_some());
        let dimmed = action.disabled || pending;
        let muted = !dimmed && action.unavailable.is_some();
        let label = if pending {
            action.pending_label.clone()
        } else {
            action.label.clone()
        };
        let leading = if pending {
            self.spinner("git-actions-menu-spin", cx)
        } else {
            icon(action_icon(action.id))
                .with_size(zoomed(ICON_MD, cx))
                .text_color(colors.foreground_muted)
                .into_any_element()
        };
        let separator = (action.starts_group && index > 0).then(|| {
            div()
                .h(px(1.))
                .my(zoomed(SPACE_1, cx))
                .bg(colors.border_accent)
        });
        let action = Rc::new(action);
        let ax_action = action.clone();
        let view = cx.weak_entity();
        let row = div()
            .id(SharedString::from(format!("git-actions-item-{index}")))
            .role(Role::MenuItem)
            .aria_label(label.clone())
            .min_h(zoomed(MENU_ITEM_HEIGHT, cx))
            .mx(zoomed(SPACE_1, cx))
            .px(zoomed(SPACE_2, cx))
            .py(zoomed(SPACE_1, cx))
            .flex()
            .items_center()
            .gap(zoomed(SPACE_2, cx))
            .rounded(zoomed(RADIUS_MD, cx))
            .when(dimmed, |el| el.opacity(0.5))
            .when(muted, |el| el.opacity(0.72))
            .when(pressable, |el| {
                el.cursor_pointer()
                    .hover(move |style| style.bg(colors.surface2))
                    .on_click(cx.listener(move |this, _, _, cx| this.on_press(&action, cx)))
                    .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                        view.update(cx, |this, cx| this.on_press(&ax_action, cx))
                            .ok();
                    })
            })
            .child(leading)
            .child(
                div()
                    .text_size(text_size)
                    .line_height(text_size * TEXT_LINE_FACTOR)
                    .text_color(if muted {
                        colors.foreground_muted
                    } else {
                        colors.foreground
                    })
                    .whitespace_nowrap()
                    .child(label),
            );
        v_flex().children(separator).child(row).into_any_element()
    }

    fn menu(&self, secondary: Vec<Action>, cx: &mut Context<GitActionsButton>) -> AnyElement {
        let colors = tokens(cx);
        let items: Vec<AnyElement> = secondary
            .into_iter()
            .enumerate()
            .map(|(index, action)| self.menu_item(index, action, cx))
            .collect();
        div()
            .absolute()
            .top(zoomed(popup_offset(HEADER_CONTROL_HEIGHT), cx))
            .right_0()
            .child(
                deferred(
                    anchored()
                        .anchor(Anchor::TopRight)
                        .snap_to_window_with_margin(zoomed(8., cx))
                        .child(
                            v_flex()
                                .id("git-actions-menu")
                                .role(Role::Menu)
                                .aria_label("More options")
                                .min_w(zoomed(MENU_MIN_WIDTH, cx))
                                .py(zoomed(SPACE_1, cx))
                                .bg(colors.popover)
                                .border_1()
                                .border_color(colors.border_accent)
                                .rounded(zoomed(RADIUS_LG, cx))
                                .shadow(vec![shadow(ShadowSize::Md, cx)])
                                .occlude()
                                .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                                    this.menu_open = false;
                                    cx.notify();
                                }))
                                .children(items),
                        ),
                )
                .with_priority(gpui_kit::base::POPUP_PRIORITY),
            )
            .into_any_element()
    }
}

fn action_icon(id: ActionId) -> IconName {
    match id {
        ActionId::Commit => IconName::GitCommitHorizontal,
        ActionId::Pull => IconName::Download,
        ActionId::Push => IconName::Upload,
        ActionId::PullAndPush => IconName::ArrowDownUp,
        ActionId::Pr
        | ActionId::MergePr(_)
        | ActionId::EnableAutoMerge(_)
        | ActionId::DisableAutoMerge => IconName::Github,
        ActionId::MergeBranch => IconName::GitMerge,
        ActionId::MergeFromBase => IconName::RefreshCcw,
        ActionId::ArchiveWorkspace => IconName::Archive,
    }
}

impl Render for GitActionsButton {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // As in Electron, no primary action (not git, no status yet) draws no button.
        let Some(GitActions {
            primary: Some(primary),
            secondary,
        }) = self.actions(cx)
        else {
            return div().id("git-actions").into_any_element();
        };
        let colors = tokens(cx);
        let has_menu = !secondary.is_empty();
        let menu = (self.menu_open && has_menu).then(|| self.menu(secondary, cx));
        let primary = self.primary(primary, cx);
        let caret = has_menu.then(|| self.caret(cx));
        div()
            .id("git-actions")
            .relative()
            .flex_shrink_0()
            .child(
                h_flex()
                    .h(zoomed(HEADER_CONTROL_HEIGHT, cx))
                    .items_stretch()
                    .rounded(zoomed(RADIUS_MD, cx))
                    .border_1()
                    .border_color(colors.border_accent)
                    .overflow_hidden()
                    .child(primary)
                    .children(caret),
            )
            .children(menu)
            .into_any_element()
    }
}
