//! The settings view: a section list on the left and the pane of the picked section on the right,
//! as `settings-screen.tsx` lays them out on desktop.

use crate::pane_appearance::AppearancePane;
use crate::pane_hosts::{HostsChanged, HostsPane};
use crate::pane_projects::ProjectsPane;
use crate::pane_providers::ProvidersPane;
use crate::pane_shortcuts::{ShortcutsChanged, ShortcutsPane};
use crate::ui::LINE_HEIGHT_NORMAL;
use bachuc_client::Client;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, ICON_MD, RADIUS_LG, SETTINGS_DETAIL_MIN_WIDTH, SETTINGS_SIDEBAR_WIDTH, SPACE_0_5,
    SPACE_2,
};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// What the settings view tells the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SettingsEvent {
    /// `hosts.json` changed: reconnect, or refresh the host switcher.
    HostsChanged,
    /// `shortcuts.json` changed: bind the keys again.
    ShortcutsChanged,
    /// The user left the settings screen.
    Closed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Section {
    Hosts,
    Appearance,
    Shortcuts,
    Providers,
    Projects,
}

impl Section {
    const ALL: [Section; 5] = [
        Section::Hosts,
        Section::Appearance,
        Section::Shortcuts,
        Section::Providers,
        Section::Projects,
    ];

    fn label(self) -> &'static str {
        match self {
            Section::Hosts => "Hosts",
            Section::Appearance => "Appearance",
            Section::Shortcuts => "Shortcuts",
            Section::Providers => "Providers",
            Section::Projects => "Projects",
        }
    }

    fn icon(self) -> IconName {
        match self {
            Section::Hosts => IconName::Server,
            Section::Appearance => IconName::Palette,
            Section::Shortcuts => IconName::Keyboard,
            Section::Providers => IconName::Boxes,
            Section::Projects => IconName::FolderGit2,
        }
    }
}

pub struct Settings {
    section: Section,
    hosts: Entity<HostsPane>,
    appearance: Entity<AppearancePane>,
    shortcuts: Entity<ShortcutsPane>,
    providers: Entity<ProvidersPane>,
    projects: Entity<ProjectsPane>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<SettingsEvent> for Settings {}

impl Settings {
    pub fn new(
        client: Entity<Client>,
        window: &mut Window,
        cx: &mut Context<Settings>,
    ) -> Settings {
        let hosts = cx.new(|cx| HostsPane::new(client.clone(), cx));
        let appearance = cx.new(|cx| AppearancePane::new(window, cx));
        let shortcuts = cx.new(ShortcutsPane::new);
        let providers = cx.new(|cx| ProvidersPane::new(client.clone(), cx));
        let projects = cx.new(|cx| ProjectsPane::new(client, cx));
        let subscriptions = vec![
            cx.subscribe(&hosts, |_, _, _: &HostsChanged, cx| {
                cx.emit(SettingsEvent::HostsChanged)
            }),
            cx.subscribe(&shortcuts, |_, _, _: &ShortcutsChanged, cx| {
                cx.emit(SettingsEvent::ShortcutsChanged)
            }),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
        ];
        Settings {
            section: Section::Hosts,
            hosts,
            appearance,
            shortcuts,
            providers,
            projects,
            _subscriptions: subscriptions,
        }
    }

    /// Shows the editor of the project with this root folder.
    pub fn open_project(
        &mut self,
        root: &str,
        name: &str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.section = Section::Projects;
        self.projects
            .update(cx, |projects, cx| projects.open(root, name, window, cx));
        cx.notify();
    }

    fn nav_row(&self, section: Section, cx: &mut Context<Self>) -> Stateful<Div> {
        let colors = tokens(cx);
        let selected = self.section == section;
        let label = section.label();
        div()
            .id(("settings-section", section as usize))
            .role(Role::Tab)
            .aria_label(label)
            .aria_selected(selected)
            .min_h(px(28.))
            .px(px(SPACE_2))
            .py(px(bachuc_theme::metrics::SPACE_1))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACE_2))
            .rounded(px(RADIUS_LG))
            .cursor_pointer()
            .when(selected, |this| this.bg(colors.surface_sidebar_hover))
            .hover(|style| style.bg(colors.surface_sidebar_hover))
            .child(
                icon(section.icon())
                    .with_size(px(ICON_MD))
                    .text_color(colors.foreground_muted),
            )
            .child(
                div()
                    .flex_1()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(if selected {
                        colors.foreground
                    } else {
                        colors.foreground_muted
                    })
                    .child(label),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.section = section;
                cx.notify();
            }))
    }
}

impl Render for Settings {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let rows: Vec<Stateful<Div>> = Section::ALL
            .into_iter()
            .map(|section| self.nav_row(section, cx))
            .collect();
        let pane: AnyElement = match self.section {
            Section::Hosts => self.hosts.clone().into_any_element(),
            Section::Appearance => self.appearance.clone().into_any_element(),
            Section::Shortcuts => self.shortcuts.clone().into_any_element(),
            Section::Providers => self.providers.clone().into_any_element(),
            Section::Projects => self.projects.clone().into_any_element(),
        };
        h_flex()
            .id("settings")
            .role(Role::Group)
            .aria_label("Settings")
            .size_full()
            .bg(colors.surface0)
            .text_color(colors.foreground)
            .line_height(relative(LINE_HEIGHT_NORMAL))
            .child(
                v_flex()
                    .id("settings-sidebar")
                    .role(Role::Navigation)
                    .aria_label("Settings sections")
                    .w(px(SETTINGS_SIDEBAR_WIDTH))
                    .h_full()
                    .flex_shrink_0()
                    .border_r_1()
                    .border_color(colors.border)
                    .bg(colors.surface_sidebar)
                    .p(px(SPACE_2))
                    .gap(px(SPACE_0_5))
                    .child(
                        div()
                            .id("settings-back")
                            .role(Role::Button)
                            .aria_label("Back")
                            .min_h(px(28.))
                            .px(px(SPACE_2))
                            .mb(px(SPACE_2))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(SPACE_2))
                            .rounded(px(RADIUS_LG))
                            .cursor_pointer()
                            .hover(|style| style.bg(colors.surface_sidebar_hover))
                            .child(
                                icon(IconName::ArrowLeft)
                                    .with_size(px(ICON_MD))
                                    .text_color(colors.foreground_muted),
                            )
                            .child(
                                div()
                                    .text_size(ui_font_size(FONT_BASE, cx))
                                    .text_color(colors.foreground_muted)
                                    .child("Back"),
                            )
                            .on_click(cx.listener(|_, _, _, cx| cx.emit(SettingsEvent::Closed))),
                    )
                    .child(
                        v_flex()
                            .id("settings-sections")
                            .role(Role::TabList)
                            .aria_label("Sections")
                            .gap(px(SPACE_0_5))
                            .children(rows),
                    ),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .min_w(px(SETTINGS_DETAIL_MIN_WIDTH))
                    .child(pane),
            )
    }
}
