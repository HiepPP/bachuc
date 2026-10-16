//! Plugin themes. A plugin theme gives eight colors and a scheme; the rest of the tokens come
//! from the base theme of the same scheme, as `buildLightPluginTheme` and `buildDarkPluginTheme`
//! in `packages/app/src/plugins/themes/index.ts` do. The eight colors are the ones of
//! `bachuc_plugin_host::PluginThemeColors`, which this crate cannot depend on (the plugin host
//! depends on this crate).

use gpui_kit::{Hsla, SharedString};

use crate::{Scheme, ThemeChoice, Tokens};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct CustomThemeColors {
    pub background: Hsla,
    pub foreground: Hsla,
    pub raised: Hsla,
    pub control: Hsla,
    pub border: Hsla,
    pub muted_foreground: Hsla,
    pub ring: Hsla,
    pub accent: Option<Hsla>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CustomTheme {
    pub id: SharedString,
    pub name: SharedString,
    pub scheme: Scheme,
    pub colors: CustomThemeColors,
}

impl Tokens {
    /// The tokens of a plugin theme. Status, tint, dot, diff, destructive, and selection tokens
    /// keep the value of the base theme of the same scheme.
    pub fn from_custom(colors: &CustomThemeColors, scheme: Scheme) -> Tokens {
        let mut t = match scheme {
            Scheme::Light => Tokens::light(),
            Scheme::Dark => Tokens::claude_dark(),
        };
        let accent = colors.accent.unwrap_or(colors.foreground);
        t.surface0 = colors.background;
        t.surface1 = colors.raised;
        t.surface2 = colors.control;
        t.surface3 = colors.border;
        t.surface4 = colors.ring;
        t.surface_diff_empty = colors.raised;
        t.surface_sidebar_hover = colors.raised;
        t.foreground = colors.foreground;
        t.foreground_muted = colors.muted_foreground;
        t.foreground_extra_muted = colors.ring;
        t.border = colors.border;
        t.border_accent = colors.border;
        t.accent = accent;
        t.accent_bright = accent;
        t.accent_foreground = colors.background;
        t.ring = colors.ring;
        t.terminal_background = colors.background;
        t.terminal_foreground = colors.foreground;
        t.terminal_cursor = colors.foreground;
        t.ansi[8] = colors.ring;
        match scheme {
            Scheme::Light => {
                t.surface_sidebar = colors.control;
                t.surface_sidebar_selected = colors.border;
                t.surface_workspace = colors.background;
                t.popover = colors.background;
                t.ansi[0] = colors.foreground;
            }
            Scheme::Dark => {
                t.surface_sidebar = colors.background;
                t.surface_sidebar_selected = colors.control;
                t.surface_workspace = colors.raised;
                t.popover = colors.control;
                t.ansi[0] = colors.control;
            }
        }
        t
    }
}

/// The selected theme when the list holds it. A missing id gives `None`, so the built-in choice
/// applies.
pub fn active_custom<'a>(
    selected: Option<&str>,
    themes: &'a [CustomTheme],
) -> Option<&'a CustomTheme> {
    let selected = selected?;
    themes.iter().find(|theme| theme.id == selected)
}

/// The built-in choice after `current`, for the cycle-theme shortcut.
pub fn next_builtin_choice(current: ThemeChoice) -> ThemeChoice {
    match current {
        ThemeChoice::Light => ThemeChoice::ClaudeDark,
        ThemeChoice::ClaudeDark => ThemeChoice::System,
        ThemeChoice::System => ThemeChoice::Light,
    }
}

/// The cycle-theme shortcut: leaves the plugin theme, if one is selected, and returns the
/// built-in choice after `choice`. Without it the shortcut would change nothing on screen,
/// because the plugin theme wins over the choice (Q-103).
pub(crate) fn cycle_builtin(choice: ThemeChoice, custom: &mut CustomState) -> ThemeChoice {
    custom.select(None);
    next_builtin_choice(choice)
}

/// The active plugin theme with its tokens.
#[derive(Clone, Debug)]
pub(crate) struct ActiveCustom {
    pub theme: CustomTheme,
    pub tokens: Tokens,
}

/// The plugin theme list, the stored selection, and the active theme that both give. The
/// selection stays when the list drops its theme, so the theme returns with the list.
#[derive(Debug, Default)]
pub(crate) struct CustomState {
    themes: Vec<CustomTheme>,
    selected: Option<SharedString>,
    active: Option<ActiveCustom>,
}

impl CustomState {
    pub fn themes(&self) -> &[CustomTheme] {
        &self.themes
    }

    pub fn selected(&self) -> Option<&SharedString> {
        self.selected.as_ref()
    }

    pub fn active(&self) -> Option<&ActiveCustom> {
        self.active.as_ref()
    }

    /// Replaces the list. Returns whether the active theme or its colors changed.
    pub fn set_themes(&mut self, themes: Vec<CustomTheme>) -> bool {
        self.themes = themes;
        self.refresh()
    }

    /// Stores the selection. Returns whether the active theme or its colors changed.
    pub fn select(&mut self, id: Option<SharedString>) -> bool {
        self.selected = id;
        self.refresh()
    }

    fn refresh(&mut self) -> bool {
        let next = active_custom(self.selected.as_deref(), &self.themes);
        if next == self.active.as_ref().map(|active| &active.theme) {
            return false;
        }
        self.active = next.map(|theme| ActiveCustom {
            tokens: Tokens::from_custom(&theme.colors, theme.scheme),
            theme: theme.clone(),
        });
        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::Rgba;

    fn color(rgb: u32) -> Hsla {
        Rgba {
            r: ((rgb >> 16) & 0xff) as f32 / 255.,
            g: ((rgb >> 8) & 0xff) as f32 / 255.,
            b: (rgb & 0xff) as f32 / 255.,
            a: 1.,
        }
        .into()
    }

    fn colors(accent: Option<Hsla>) -> CustomThemeColors {
        CustomThemeColors {
            background: color(0x101010),
            foreground: color(0x202020),
            raised: color(0x303030),
            control: color(0x404040),
            border: color(0x505050),
            muted_foreground: color(0x606060),
            ring: color(0x707070),
            accent,
        }
    }

    fn theme(id: &str, scheme: Scheme, accent: Option<Hsla>) -> CustomTheme {
        CustomTheme {
            id: id.to_string().into(),
            name: id.to_string().into(),
            scheme,
            colors: colors(accent),
        }
    }

    #[test]
    fn light_mapping_sets_every_listed_token() {
        let c = colors(Some(color(0xaa0000)));
        let t = Tokens::from_custom(&c, Scheme::Light);
        assert_eq!(t.surface0, c.background);
        assert_eq!(t.surface1, c.raised);
        assert_eq!(t.surface2, c.control);
        assert_eq!(t.surface3, c.border);
        assert_eq!(t.surface4, c.ring);
        assert_eq!(t.surface_diff_empty, c.raised);
        assert_eq!(t.surface_sidebar_hover, c.raised);
        assert_eq!(t.surface_sidebar, c.control);
        assert_eq!(t.surface_sidebar_selected, c.border);
        assert_eq!(t.surface_workspace, c.background);
        assert_eq!(t.popover, c.background);
        assert_eq!(t.foreground, c.foreground);
        assert_eq!(t.foreground_muted, c.muted_foreground);
        assert_eq!(t.foreground_extra_muted, c.ring);
        assert_eq!(t.border, c.border);
        assert_eq!(t.border_accent, c.border);
        assert_eq!(t.accent, color(0xaa0000));
        assert_eq!(t.accent_bright, color(0xaa0000));
        assert_eq!(t.accent_foreground, c.background);
        assert_eq!(t.ring, c.ring);
        assert_eq!(t.terminal_background, c.background);
        assert_eq!(t.terminal_foreground, c.foreground);
        assert_eq!(t.terminal_cursor, c.foreground);
        assert_eq!(t.ansi[0], c.foreground);
        assert_eq!(t.ansi[8], c.ring);
    }

    #[test]
    fn dark_mapping_sets_every_listed_token() {
        let c = colors(Some(color(0x00aa00)));
        let t = Tokens::from_custom(&c, Scheme::Dark);
        assert_eq!(t.surface0, c.background);
        assert_eq!(t.surface1, c.raised);
        assert_eq!(t.surface2, c.control);
        assert_eq!(t.surface3, c.border);
        assert_eq!(t.surface4, c.ring);
        assert_eq!(t.surface_diff_empty, c.raised);
        assert_eq!(t.surface_sidebar_hover, c.raised);
        assert_eq!(t.surface_sidebar, c.background);
        assert_eq!(t.surface_sidebar_selected, c.control);
        assert_eq!(t.surface_workspace, c.raised);
        assert_eq!(t.popover, c.control);
        assert_eq!(t.foreground, c.foreground);
        assert_eq!(t.foreground_muted, c.muted_foreground);
        assert_eq!(t.foreground_extra_muted, c.ring);
        assert_eq!(t.border, c.border);
        assert_eq!(t.border_accent, c.border);
        assert_eq!(t.accent, color(0x00aa00));
        assert_eq!(t.accent_bright, color(0x00aa00));
        assert_eq!(t.accent_foreground, c.background);
        assert_eq!(t.ring, c.ring);
        assert_eq!(t.terminal_background, c.background);
        assert_eq!(t.terminal_foreground, c.foreground);
        assert_eq!(t.terminal_cursor, c.foreground);
        assert_eq!(t.ansi[0], c.control);
        assert_eq!(t.ansi[8], c.ring);
    }

    #[test]
    fn a_missing_accent_falls_back_to_the_foreground() {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let c = colors(None);
            let t = Tokens::from_custom(&c, scheme);
            assert_eq!(t.accent, c.foreground);
            assert_eq!(t.accent_bright, c.foreground);
        }
    }

    #[test]
    fn tokens_outside_the_mapping_keep_the_base_value() {
        for (scheme, base) in [
            (Scheme::Light, Tokens::light()),
            (Scheme::Dark, Tokens::claude_dark()),
        ] {
            let t = Tokens::from_custom(&colors(None), scheme);
            assert_eq!(t.status_success, base.status_success);
            assert_eq!(t.status_danger, base.status_danger);
            assert_eq!(t.status_warning, base.status_warning);
            assert_eq!(t.status_merged, base.status_merged);
            assert_eq!(t.tint_success, base.tint_success);
            assert_eq!(t.tint_merged, base.tint_merged);
            assert_eq!(t.dot_running, base.dot_running);
            assert_eq!(t.diff_addition, base.diff_addition);
            assert_eq!(t.diff_deletion, base.diff_deletion);
            assert_eq!(t.destructive, base.destructive);
            assert_eq!(t.destructive_foreground, base.destructive_foreground);
            assert_eq!(t.interaction_highlight, base.interaction_highlight);
            assert_eq!(t.terminal_selection, base.terminal_selection);
            assert_eq!(t.ansi[1], base.ansi[1]);
            assert_eq!(t.ansi[15], base.ansi[15]);
        }
    }

    #[test]
    fn active_custom_finds_a_present_id_only() {
        let themes = vec![
            theme("a", Scheme::Light, None),
            theme("b", Scheme::Dark, None),
        ];
        assert_eq!(active_custom(Some("b"), &themes), Some(&themes[1]));
        assert_eq!(active_custom(Some("missing"), &themes), None);
        assert_eq!(active_custom(None, &themes), None);
        assert_eq!(active_custom(Some("a"), &[]), None);
    }

    #[test]
    fn a_selection_before_the_list_applies_when_the_list_arrives() {
        let mut state = CustomState::default();
        assert!(!state.select(Some("dusk".into())));
        assert!(state.active().is_none());
        let dusk = theme("dusk", Scheme::Dark, None);
        assert!(state.set_themes(vec![dusk.clone()]));
        let active = state.active().expect("dusk is active");
        assert_eq!(active.theme, dusk);
        assert_eq!(active.tokens.surface0, dusk.colors.background);
        assert_eq!(
            active.tokens.status_success,
            Tokens::claude_dark().status_success
        );
    }

    #[test]
    fn dropping_the_active_theme_keeps_the_selection() {
        let mut state = CustomState::default();
        state.set_themes(vec![theme("dusk", Scheme::Dark, None)]);
        state.select(Some("dusk".into()));
        assert!(state.set_themes(Vec::new()));
        assert!(state.active().is_none());
        assert_eq!(state.selected().map(|id| id.as_ref()), Some("dusk"));
    }

    #[test]
    fn selecting_none_returns_to_the_builtin_choice() {
        let mut state = CustomState::default();
        state.set_themes(vec![theme("dusk", Scheme::Dark, None)]);
        state.select(Some("dusk".into()));
        assert!(state.select(None));
        assert!(state.active().is_none());
        assert!(state.selected().is_none());
    }

    #[test]
    fn a_list_update_reports_a_change_only_for_the_active_theme() {
        let mut state = CustomState::default();
        let dusk = theme("dusk", Scheme::Dark, None);
        let other = theme("other", Scheme::Light, None);
        state.set_themes(vec![dusk.clone(), other.clone()]);
        state.select(Some("dusk".into()));
        // The same active theme: no resync.
        assert!(!state.set_themes(vec![dusk.clone()]));
        // Another theme changed: no resync.
        let mut changed = other.clone();
        changed.colors.border = color(0x999999);
        assert!(!state.set_themes(vec![dusk.clone(), changed]));
        // The active theme's colors changed: resync, and the tokens follow.
        let mut recolored = dusk.clone();
        recolored.colors.background = color(0x0a0b0c);
        assert!(state.set_themes(vec![recolored.clone()]));
        assert_eq!(
            state.active().unwrap().tokens.surface0,
            recolored.colors.background
        );
        // The scheme changed: resync.
        let mut flipped = recolored.clone();
        flipped.scheme = Scheme::Light;
        assert!(state.set_themes(vec![flipped]));
        assert_eq!(state.active().unwrap().theme.scheme, Scheme::Light);
    }

    #[test]
    fn the_cycle_leaves_a_plugin_theme_for_the_next_builtin_theme() {
        let mut state = CustomState::default();
        state.set_themes(vec![theme("dusk", Scheme::Dark, None)]);
        state.select(Some("dusk".into()));
        assert_eq!(
            cycle_builtin(ThemeChoice::Light, &mut state),
            ThemeChoice::ClaudeDark
        );
        assert!(state.active().is_none());
        assert!(state.selected().is_none());
        assert_eq!(
            cycle_builtin(ThemeChoice::System, &mut state),
            ThemeChoice::Light
        );
        assert_eq!(
            next_builtin_choice(ThemeChoice::Light),
            ThemeChoice::ClaudeDark
        );
        assert_eq!(
            next_builtin_choice(ThemeChoice::ClaudeDark),
            ThemeChoice::System
        );
        assert_eq!(next_builtin_choice(ThemeChoice::System), ThemeChoice::Light);
    }
}
