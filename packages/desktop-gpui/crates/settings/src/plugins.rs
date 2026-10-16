//! What the plugins add to Settings: their screens in the sidebar and on a page, and their
//! themes in the Appearance picker. The rules are pure functions; `Settings` and
//! `AppearancePane` draw them. Everything is read from `PluginHost::index` (ADR-0045).

use crate::appearance::theme_label;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    Contributed, PluginHostEvent, SettingsScreen, ThemeAppearance, ThemeContribution,
};
use bachuc_theme::{CustomTheme, CustomThemeColors, Scheme, ThemeChoice, active_custom};
use gpui_kit::{App, Hsla, SharedString};
use std::collections::HashMap;

/// `settings.plugins.screens.offline`.
pub const OFFLINE: &str = "Connect to this host to open plugin settings.";
/// `settings.plugins.screens.unavailable`.
pub const UNAVAILABLE: &str = "This plugin settings screen is unavailable.";
/// The sidebar label above the plugin screen rows (`settings.groups.plugins`).
pub const GROUP_LABEL: &str = "Plugins";
/// `t("settings.title")`, the header text when the screen is gone.
const FALLBACK_TITLE: &str = "Settings";

/// One plugin screen: the plugin id and the screen id.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub struct ScreenId {
    pub plugin_id: String,
    pub screen_id: String,
}

impl ScreenId {
    pub fn new(plugin_id: &str, screen_id: &str) -> Self {
        Self {
            plugin_id: plugin_id.to_string(),
            screen_id: screen_id.to_string(),
        }
    }
}

/// One sidebar row of a plugin screen.
#[derive(Clone, Debug, PartialEq)]
pub struct ScreenRow {
    pub id: ScreenId,
    pub title: SharedString,
    pub icon: IconName,
}

/// The sidebar rows in index order. No screen gives no rows, and the "Plugins" label goes with
/// them.
pub fn screen_rows(screens: &[Contributed<SettingsScreen>]) -> Vec<ScreenRow> {
    screens
        .iter()
        .map(|screen| ScreenRow {
            id: ScreenId::new(screen.plugin_id, screen.value.id),
            title: screen.value.title.clone(),
            icon: screen.value.icon,
        })
        .collect()
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum PageState {
    /// The client is not connected.
    Offline,
    /// The plugin is not active or has no such screen.
    Unavailable,
    Ready,
}

/// The state of an open plugin page: offline first, then unavailable, else the screen's view.
pub fn page_state(connected: bool, rows: &[ScreenRow], id: &ScreenId) -> PageState {
    if !connected {
        PageState::Offline
    } else if rows.iter().any(|row| &row.id == id) {
        PageState::Ready
    } else {
        PageState::Unavailable
    }
}

/// The note under the header for a state that has no view.
pub fn state_text(state: PageState) -> Option<&'static str> {
    match state {
        PageState::Offline => Some(OFFLINE),
        PageState::Unavailable => Some(UNAVAILABLE),
        PageState::Ready => None,
    }
}

/// The page header: the plugin id and the screen title joined by a middle dot
/// (`settings-screen.tsx`).
pub fn page_header(rows: &[ScreenRow], id: &ScreenId) -> String {
    let title = rows
        .iter()
        .find(|row| &row.id == id)
        .map_or(FALLBACK_TITLE, |row| row.title.as_ref());
    format!("{} \u{b7} {title}", id.plugin_id)
}

/// Drops the cached views of screens that left the index, so a screen that comes back is built
/// anew.
pub fn prune_views<V>(cache: &mut HashMap<ScreenId, V>, rows: &[ScreenRow]) {
    cache.retain(|id, _| rows.iter().any(|row| &row.id == id));
}

/// `<plugin_id>/theme/<id>`, the id a plugin theme has in `appearance.json`.
pub fn theme_id(plugin_id: &str, theme: &str) -> String {
    format!("{plugin_id}/theme/{theme}")
}

fn scheme(appearance: ThemeAppearance) -> Scheme {
    match appearance {
        ThemeAppearance::Light => Scheme::Light,
        ThemeAppearance::Dark => Scheme::Dark,
    }
}

/// The plugin themes as `bachuc_theme` themes, in index order.
pub fn custom_themes_of(themes: &[Contributed<ThemeContribution>]) -> Vec<CustomTheme> {
    themes
        .iter()
        .map(|theme| {
            let colors = theme.value.colors;
            CustomTheme {
                id: theme_id(theme.plugin_id, theme.value.id).into(),
                name: theme.value.name.clone(),
                scheme: scheme(theme.value.appearance),
                colors: CustomThemeColors {
                    background: colors.background,
                    foreground: colors.foreground,
                    raised: colors.raised,
                    control: colors.control,
                    border: colors.border,
                    muted_foreground: colors.muted_foreground,
                    ring: colors.ring,
                    accent: colors.accent,
                },
            }
        })
        .collect()
}

/// The plugin themes of the active plugins; empty with no host.
pub fn plugin_themes(cx: &App) -> Vec<CustomTheme> {
    bachuc_plugin_host::host(cx)
        .map(|host| custom_themes_of(host.read(cx).index().themes()))
        .unwrap_or_default()
}

/// Hands the plugin themes to `bachuc_theme` now and again on each contribution change. Call it
/// once after `bachuc_plugin_host::init`. With no host it does nothing.
pub fn watch_plugin_themes(cx: &mut App) {
    let Some(host) = bachuc_plugin_host::host(cx) else {
        return;
    };
    bachuc_theme::set_custom_themes(plugin_themes(cx), cx);
    cx.subscribe(&host, |_, event: &PluginHostEvent, cx| {
        if matches!(event, PluginHostEvent::ContributionsChanged) {
            bachuc_theme::set_custom_themes(plugin_themes(cx), cx);
        }
    })
    .detach();
}

/// One entry of the theme picker menu.
#[derive(Clone, Debug, PartialEq)]
pub enum ThemeItem {
    Builtin {
        theme: ThemeChoice,
        checked: bool,
    },
    Separator,
    Plugin {
        id: SharedString,
        name: SharedString,
        swatch: Hsla,
        checked: bool,
    },
}

/// The picker entries: the built-in themes, then a separator and each plugin theme. A selected
/// id that the list lacks checks the stored built-in choice, which is what applies.
pub fn theme_items(
    current: ThemeChoice,
    selected: Option<&str>,
    themes: &[CustomTheme],
    builtins: &[ThemeChoice],
) -> Vec<ThemeItem> {
    let active = active_custom(selected, themes).map(|theme| theme.id.clone());
    let mut items: Vec<ThemeItem> = builtins
        .iter()
        .map(|&theme| ThemeItem::Builtin {
            theme,
            checked: active.is_none() && theme == current,
        })
        .collect();
    if !themes.is_empty() {
        items.push(ThemeItem::Separator);
    }
    items.extend(themes.iter().map(|theme| ThemeItem::Plugin {
        id: theme.id.clone(),
        name: theme.name.clone(),
        swatch: theme.colors.background,
        checked: active.as_ref() == Some(&theme.id),
    }));
    items
}

/// What the picker trigger shows: the plugin theme's name and swatch, else the built-in label.
#[derive(Clone, Debug, PartialEq)]
pub enum TriggerChoice {
    Builtin(ThemeChoice),
    Plugin { name: SharedString, swatch: Hsla },
}

pub fn trigger_choice(
    current: ThemeChoice,
    selected: Option<&str>,
    themes: &[CustomTheme],
) -> TriggerChoice {
    match active_custom(selected, themes) {
        Some(theme) => TriggerChoice::Plugin {
            name: theme.name.clone(),
            swatch: theme.colors.background,
        },
        None => TriggerChoice::Builtin(current),
    }
}

impl TriggerChoice {
    pub fn label(&self) -> SharedString {
        match self {
            TriggerChoice::Builtin(theme) => theme_label(*theme).into(),
            TriggerChoice::Plugin { name, .. } => name.clone(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{
        OFFLINE, PageState, ScreenId, ThemeItem, TriggerChoice, UNAVAILABLE, custom_themes_of,
        page_header, page_state, prune_views, screen_rows, state_text, theme_id, theme_items,
        trigger_choice,
    };
    use bachuc_icons::IconName;
    use bachuc_plugin_host::{
        Contributed, PluginThemeColors, SettingsScreen, ThemeAppearance, ThemeContribution,
        ViewFactory,
    };
    use bachuc_theme::{CustomTheme, Scheme, ThemeChoice};
    use gpui_kit::{Hsla, hsla};
    use std::collections::HashMap;
    use std::rc::Rc;

    const BUILTINS: [ThemeChoice; 3] = [
        ThemeChoice::Light,
        ThemeChoice::ClaudeDark,
        ThemeChoice::System,
    ];

    fn screen(
        plugin_id: &'static str,
        id: &'static str,
        title: &str,
    ) -> Contributed<SettingsScreen> {
        let view: ViewFactory<()> = Rc::new(|_, _, _| panic!("a test never builds the view"));
        Contributed {
            plugin_id,
            value: SettingsScreen {
                id,
                title: title.to_string().into(),
                icon: IconName::Palette,
                view,
            },
        }
    }

    fn colors(background: Hsla) -> PluginThemeColors {
        PluginThemeColors {
            background,
            foreground: hsla(0., 0., 0.1, 1.),
            raised: hsla(0., 0., 0.95, 1.),
            control: hsla(0., 0., 0.9, 1.),
            border: hsla(0., 0., 0.8, 1.),
            muted_foreground: hsla(0., 0., 0.4, 1.),
            ring: hsla(0.6, 0.5, 0.5, 1.),
            accent: Some(hsla(0.1, 0.9, 0.5, 1.)),
        }
    }

    fn theme(
        plugin_id: &'static str,
        id: &'static str,
        appearance: ThemeAppearance,
        background: Hsla,
    ) -> Contributed<ThemeContribution> {
        Contributed {
            plugin_id,
            value: ThemeContribution {
                id,
                name: format!("Theme {id}").into(),
                appearance,
                colors: colors(background),
            },
        }
    }

    fn two_themes() -> Vec<CustomTheme> {
        custom_themes_of(&[
            theme(
                "claude-look",
                "claude-light",
                ThemeAppearance::Light,
                hsla(0.1, 0.5, 0.9, 1.),
            ),
            theme(
                "claude-look",
                "claude-dark",
                ThemeAppearance::Dark,
                hsla(0.1, 0.2, 0.1, 1.),
            ),
        ])
    }

    #[test]
    fn no_screens_give_no_rows() {
        assert!(screen_rows(&[]).is_empty());
    }

    #[test]
    fn rows_follow_the_index_order_with_title_and_icon() {
        let rows = screen_rows(&[
            screen("edit-diffs", "display", "Display"),
            screen("claude-look", "palette", "Palette"),
            screen("edit-diffs", "limits", "Limits"),
        ]);
        let ids: Vec<_> = rows
            .iter()
            .map(|row| (row.id.plugin_id.as_str(), row.id.screen_id.as_str()))
            .collect();
        assert_eq!(
            ids,
            [
                ("edit-diffs", "display"),
                ("claude-look", "palette"),
                ("edit-diffs", "limits")
            ]
        );
        assert_eq!(rows[1].title.as_ref(), "Palette");
        assert_eq!(rows[1].icon, IconName::Palette);
    }

    #[test]
    fn the_page_state_is_offline_unavailable_or_ready() {
        let rows = screen_rows(&[screen("edit-diffs", "display", "Display")]);
        let known = ScreenId::new("edit-diffs", "display");
        let unknown = ScreenId::new("edit-diffs", "nope");
        let other_plugin = ScreenId::new("claude-look", "display");
        assert_eq!(page_state(true, &rows, &known), PageState::Ready);
        assert_eq!(page_state(true, &rows, &unknown), PageState::Unavailable);
        assert_eq!(
            page_state(true, &rows, &other_plugin),
            PageState::Unavailable,
            "a screen id of another plugin is not found"
        );
        assert_eq!(page_state(false, &rows, &known), PageState::Offline);
        assert_eq!(page_state(false, &[], &unknown), PageState::Offline);
        assert_eq!(page_state(true, &[], &known), PageState::Unavailable);
        assert_eq!(state_text(PageState::Offline), Some(OFFLINE));
        assert_eq!(state_text(PageState::Unavailable), Some(UNAVAILABLE));
        assert_eq!(state_text(PageState::Ready), None);
    }

    #[test]
    fn a_plugin_that_goes_inactive_makes_its_open_page_unavailable() {
        let id = ScreenId::new("edit-diffs", "display");
        let before = screen_rows(&[screen("edit-diffs", "display", "Display")]);
        assert_eq!(page_state(true, &before, &id), PageState::Ready);
        assert_eq!(page_state(true, &[], &id), PageState::Unavailable);
        assert_eq!(page_state(true, &before, &id), PageState::Ready);
    }

    #[test]
    fn the_header_joins_the_plugin_id_and_the_title() {
        let rows = screen_rows(&[screen("edit-diffs", "display", "Display")]);
        assert_eq!(
            page_header(&rows, &ScreenId::new("edit-diffs", "display")),
            "edit-diffs \u{b7} Display"
        );
        assert_eq!(
            page_header(&rows, &ScreenId::new("edit-diffs", "gone")),
            "edit-diffs \u{b7} Settings"
        );
    }

    #[test]
    fn views_of_screens_that_left_are_dropped() {
        let rows = screen_rows(&[screen("a", "one", "One")]);
        let mut cache: HashMap<ScreenId, u8> = HashMap::new();
        cache.insert(ScreenId::new("a", "one"), 1);
        cache.insert(ScreenId::new("a", "two"), 2);
        cache.insert(ScreenId::new("b", "one"), 3);
        prune_views(&mut cache, &rows);
        assert_eq!(cache.len(), 1);
        assert_eq!(cache[&ScreenId::new("a", "one")], 1);
        prune_views(&mut cache, &[]);
        assert!(cache.is_empty());
    }

    #[test]
    fn a_contribution_maps_to_a_custom_theme_with_id_and_scheme() {
        let themes = two_themes();
        assert_eq!(themes.len(), 2);
        assert_eq!(themes[0].id.as_ref(), "claude-look/theme/claude-light");
        assert_eq!(
            themes[0].id.as_ref(),
            theme_id("claude-look", "claude-light")
        );
        assert_eq!(themes[0].name.as_ref(), "Theme claude-light");
        assert_eq!(themes[0].scheme, Scheme::Light);
        assert_eq!(themes[1].id.as_ref(), "claude-look/theme/claude-dark");
        assert_eq!(themes[1].scheme, Scheme::Dark);
        let source = colors(hsla(0.1, 0.5, 0.9, 1.));
        let mapped = themes[0].colors;
        assert_eq!(mapped.background, source.background);
        assert_eq!(mapped.foreground, source.foreground);
        assert_eq!(mapped.raised, source.raised);
        assert_eq!(mapped.control, source.control);
        assert_eq!(mapped.border, source.border);
        assert_eq!(mapped.muted_foreground, source.muted_foreground);
        assert_eq!(mapped.ring, source.ring);
        assert_eq!(mapped.accent, source.accent);
        assert!(custom_themes_of(&[]).is_empty());
    }

    #[test]
    fn the_picker_lists_built_ins_then_a_separator_then_plugin_themes() {
        let themes = two_themes();
        let items = theme_items(ThemeChoice::System, None, &themes, &BUILTINS);
        assert_eq!(items.len(), 6);
        assert!(matches!(
            items[0],
            ThemeItem::Builtin {
                theme: ThemeChoice::Light,
                checked: false
            }
        ));
        assert!(matches!(
            items[2],
            ThemeItem::Builtin {
                theme: ThemeChoice::System,
                checked: true
            }
        ));
        assert_eq!(items[3], ThemeItem::Separator);
        assert!(
            matches!(&items[4], ThemeItem::Plugin { id, checked: false, .. }
            if id.as_ref() == "claude-look/theme/claude-light")
        );
        assert!(
            matches!(&items[5], ThemeItem::Plugin { id, checked: false, .. }
            if id.as_ref() == "claude-look/theme/claude-dark")
        );
    }

    #[test]
    fn the_picker_has_no_separator_without_plugin_themes() {
        let items = theme_items(ThemeChoice::Light, None, &[], &BUILTINS);
        assert_eq!(items.len(), 3);
        assert!(!items.contains(&ThemeItem::Separator));
    }

    #[test]
    fn a_selected_plugin_theme_is_the_only_checked_item() {
        let themes = two_themes();
        let items = theme_items(
            ThemeChoice::Light,
            Some("claude-look/theme/claude-dark"),
            &themes,
            &BUILTINS,
        );
        let checked: Vec<_> = items
            .iter()
            .filter(|item| {
                matches!(
                    item,
                    ThemeItem::Builtin { checked: true, .. }
                        | ThemeItem::Plugin { checked: true, .. }
                )
            })
            .collect();
        assert_eq!(checked.len(), 1);
        assert!(matches!(checked[0], ThemeItem::Plugin { id, .. }
            if id.as_ref() == "claude-look/theme/claude-dark"));
    }

    #[test]
    fn a_selected_id_the_list_lacks_checks_the_built_in_choice() {
        let items = theme_items(
            ThemeChoice::ClaudeDark,
            Some("gone/theme/x"),
            &two_themes(),
            &BUILTINS,
        );
        assert!(matches!(
            items[1],
            ThemeItem::Builtin {
                theme: ThemeChoice::ClaudeDark,
                checked: true
            }
        ));
        let none = theme_items(
            ThemeChoice::ClaudeDark,
            Some("gone/theme/x"),
            &[],
            &BUILTINS,
        );
        assert!(matches!(none[1], ThemeItem::Builtin { checked: true, .. }));
    }

    #[test]
    fn the_trigger_shows_the_selected_plugin_theme_or_the_built_in_label() {
        let themes = two_themes();
        match trigger_choice(
            ThemeChoice::Light,
            Some("claude-look/theme/claude-light"),
            &themes,
        ) {
            TriggerChoice::Plugin { name, swatch } => {
                assert_eq!(name.as_ref(), "Theme claude-light");
                assert_eq!(swatch, hsla(0.1, 0.5, 0.9, 1.));
            }
            other => panic!("{other:?}"),
        }
        let missing = trigger_choice(ThemeChoice::ClaudeDark, Some("gone/theme/x"), &themes);
        assert_eq!(missing, TriggerChoice::Builtin(ThemeChoice::ClaudeDark));
        assert_eq!(missing.label().as_ref(), "Claude dark");
        assert_eq!(
            trigger_choice(ThemeChoice::Light, None, &themes),
            TriggerChoice::Builtin(ThemeChoice::Light)
        );
    }
}
