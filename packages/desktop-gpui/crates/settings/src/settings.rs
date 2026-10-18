//! The settings view: a section list on the left and the pane of the picked section on the right,
//! as `settings-screen.tsx` lays them out on desktop.

use crate::pane_appearance::AppearancePane;
use crate::pane_hosts::{HostsChanged, HostsPane};
use crate::pane_projects::ProjectsPane;
use crate::pane_providers::ProvidersPane;
use crate::pane_shortcuts::{ShortcutsChanged, ShortcutsPane};
use crate::plugins::{
    GROUP_LABEL, ScreenId, ScreenRow, UNAVAILABLE, page_header, page_state, prune_views,
    screen_rows, state_text,
};
use crate::ui::{LINE_HEIGHT_NORMAL, column, note, page_title, scroll_page};
use bachuc_client::{Client, ClientEvent};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{Contributed, PluginHostEvent, SettingsScreen, ViewFactory};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_MD, RADIUS_LG, SETTINGS_DETAIL_MIN_WIDTH, SETTINGS_SIDEBAR_WIDTH,
    SPACE_0_5, SPACE_1, SPACE_2, SPACE_3,
};
use bachuc_theme::{tokens, ui_font_size, zoomed};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::HashMap;
use std::rc::Rc;

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

/// The open page: one of the fixed sections or a plugin screen.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Page {
    Section(Section),
    Plugin(ScreenId),
}

pub struct Settings {
    page: Page,
    client: Entity<Client>,
    /// The screens of the active plugins, read from the host index when its contributions
    /// change.
    screens: Vec<Contributed<SettingsScreen>>,
    /// The view of each opened plugin screen, with the factory that built it.
    views: HashMap<ScreenId, (ViewFactory<()>, AnyView)>,
    scroll: ScrollHandle,
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
        let projects = cx.new(|cx| ProjectsPane::new(client.clone(), cx));
        let mut subscriptions = vec![
            cx.subscribe(&hosts, |_, _, _: &HostsChanged, cx| {
                cx.emit(SettingsEvent::HostsChanged)
            }),
            cx.subscribe(&shortcuts, |_, _, _: &ShortcutsChanged, cx| {
                cx.emit(SettingsEvent::ShortcutsChanged)
            }),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
            // A plugin page shows the offline text while the client is not connected.
            cx.subscribe(&client, |_, _, event: &ClientEvent, cx| {
                if matches!(
                    event,
                    ClientEvent::Connected | ClientEvent::Disconnected { .. }
                ) {
                    cx.notify();
                }
            }),
        ];
        let mut screens = Vec::new();
        if let Some(host) = bachuc_plugin_host::host(cx) {
            screens = host.read(cx).index().settings_screens().to_vec();
            subscriptions.push(
                cx.subscribe(
                    &host,
                    |this, host, event: &PluginHostEvent, cx| match event {
                        PluginHostEvent::ContributionsChanged => {
                            this.screens = host.read(cx).index().settings_screens().to_vec();
                            prune_views(&mut this.views, &screen_rows(&this.screens));
                            cx.notify();
                        }
                        PluginHostEvent::PluginStateChanged { .. } => cx.notify(),
                        _ => {}
                    },
                ),
            );
        }
        Settings {
            page: Page::Section(Section::Hosts),
            client,
            screens,
            views: HashMap::new(),
            scroll: ScrollHandle::new(),
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
        self.page = Page::Section(Section::Projects);
        self.projects
            .update(cx, |projects, cx| projects.open(root, name, window, cx));
        cx.notify();
    }

    /// Shows the settings screen `settings_id` of the plugin `plugin_id`. An id that the host
    /// index lacks shows the unavailable text.
    pub fn open_plugin_screen(
        &mut self,
        plugin_id: &str,
        settings_id: &str,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.page = Page::Plugin(ScreenId::new(plugin_id, settings_id));
        cx.notify();
    }

    fn nav_row(
        &self,
        id: ElementId,
        icon_name: IconName,
        label: SharedString,
        page: Page,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let colors = tokens(cx);
        let selected = self.page == page;
        div()
            .id(id)
            .role(Role::Tab)
            .aria_label(label.clone())
            .aria_selected(selected)
            .min_h(zoomed(28., cx))
            .px(zoomed(SPACE_2, cx))
            .py(zoomed(bachuc_theme::metrics::SPACE_1, cx))
            .flex()
            .flex_row()
            .items_center()
            .gap(zoomed(SPACE_2, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .cursor_pointer()
            .when(selected, |this| this.bg(colors.surface_sidebar_hover))
            .hover(|style| style.bg(colors.surface_sidebar_hover))
            .child(
                icon(icon_name)
                    .with_size(zoomed(ICON_MD, cx))
                    .text_color(colors.foreground_muted),
            )
            .child(
                div()
                    .flex_1()
                    .truncate()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(if selected {
                        colors.foreground
                    } else {
                        colors.foreground_muted
                    })
                    .child(label),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.page = page.clone();
                cx.notify();
            }))
    }

    /// The page of a plugin screen: the header, then the screen's view or the text of its
    /// state. The view is built once per screen and kept until the contribution goes.
    fn plugin_page(
        &mut self,
        id: &ScreenId,
        rows: &[ScreenRow],
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let connected = self.client.read(cx).is_connected();
        let state = page_state(connected, rows, id);
        let body: AnyElement = match state_text(state) {
            Some(text) => note(text, cx).into_any_element(),
            None => match self.screen_view(id, window, cx) {
                Some(view) => view.into_any_element(),
                None => note(UNAVAILABLE, cx).into_any_element(),
            },
        };
        scroll_page(
            "plugin-screen-pane",
            &self.scroll,
            column(cx)
                .child(page_title(page_header(rows, id), cx))
                .child(body),
            cx,
        )
        .into_any_element()
    }

    /// The cached view of a screen, built again when the plugin contributed again.
    fn screen_view(
        &mut self,
        id: &ScreenId,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyView> {
        let factory = self
            .screens
            .iter()
            .find(|screen| screen.plugin_id == id.plugin_id && screen.value.id == id.screen_id)?
            .value
            .view
            .clone();
        if let Some((built_by, view)) = self.views.get(id)
            && Rc::ptr_eq(built_by, &factory)
        {
            return Some(view.clone());
        }
        let view = factory((), window, cx);
        self.views.insert(id.clone(), (factory, view.clone()));
        Some(view)
    }
}

impl Render for Settings {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let rows: Vec<Stateful<Div>> = Section::ALL
            .into_iter()
            .map(|section| {
                self.nav_row(
                    ("settings-section", section as usize).into(),
                    section.icon(),
                    section.label().into(),
                    Page::Section(section),
                    cx,
                )
            })
            .collect();
        let screen_rows = screen_rows(&self.screens);
        let plugin_rows: Vec<Stateful<Div>> = screen_rows
            .iter()
            .enumerate()
            .map(|(index, row)| {
                self.nav_row(
                    ("settings-plugin-screen", index).into(),
                    row.icon,
                    row.title.clone(),
                    Page::Plugin(row.id.clone()),
                    cx,
                )
            })
            .collect();
        let pane: AnyElement = match self.page.clone() {
            Page::Section(Section::Hosts) => self.hosts.clone().into_any_element(),
            Page::Section(Section::Appearance) => self.appearance.clone().into_any_element(),
            Page::Section(Section::Shortcuts) => self.shortcuts.clone().into_any_element(),
            Page::Section(Section::Providers) => self.providers.clone().into_any_element(),
            Page::Section(Section::Projects) => self.projects.clone().into_any_element(),
            Page::Plugin(id) => self.plugin_page(&id, &screen_rows, window, cx),
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
                    .w(zoomed(SETTINGS_SIDEBAR_WIDTH, cx))
                    .h_full()
                    .flex_shrink_0()
                    .border_r_1()
                    .border_color(colors.border)
                    .bg(colors.surface_sidebar)
                    .p(zoomed(SPACE_2, cx))
                    .gap(zoomed(SPACE_0_5, cx))
                    .child(
                        div()
                            .id("settings-back")
                            .role(Role::Button)
                            .aria_label("Back")
                            .min_h(zoomed(28., cx))
                            .px(zoomed(SPACE_2, cx))
                            .mb(zoomed(SPACE_2, cx))
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(zoomed(SPACE_2, cx))
                            .rounded(zoomed(RADIUS_LG, cx))
                            .cursor_pointer()
                            .hover(|style| style.bg(colors.surface_sidebar_hover))
                            .child(
                                icon(IconName::ArrowLeft)
                                    .with_size(zoomed(ICON_MD, cx))
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
                            .gap(zoomed(SPACE_0_5, cx))
                            .children(rows),
                    )
                    .when(!plugin_rows.is_empty(), |this| {
                        this.child(
                            div()
                                .id("settings-plugins-label")
                                .role(Role::Heading)
                                .aria_label(GROUP_LABEL)
                                .mt(zoomed(SPACE_3, cx))
                                .mb(zoomed(SPACE_1, cx))
                                .px(zoomed(SPACE_2, cx))
                                .text_size(ui_font_size(FONT_SM, cx))
                                .text_color(colors.foreground_muted)
                                .child(GROUP_LABEL),
                        )
                        .child(
                            v_flex()
                                .id("settings-plugin-screens")
                                .role(Role::TabList)
                                .aria_label("Plugin screens")
                                .gap(zoomed(SPACE_0_5, cx))
                                .children(plugin_rows),
                        )
                    }),
            )
            .child(
                div()
                    .flex_1()
                    .h_full()
                    .min_w(zoomed(SETTINGS_DETAIL_MIN_WIDTH, cx))
                    .child(pane),
            )
    }
}
