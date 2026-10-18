//! `bachuc-plugin-claude-look`: the native client of the claude-look plugin
//! (`hiep-plugins/plugins/claude-look/index.client.tsx`, ADR-0045). It ports the palette of
//! `client/theme.ts`, the 1.5 reply line height of `client/web.ts`, and the 768 px chat column of
//! `client/width.ts`.

use bachuc_plugin_host::{
    Contributions, NativePlugin, PluginContext, PluginThemeColors, ThemeAppearance,
    ThemeContribution,
};
use gpui_kit::{App, Hsla, Rgba};

const PLUGIN_ID: &str = "claude-look";

/// Claude desktop's thread width (`client/width.ts`), reply line height, and regular-weight
/// header title (`client/web.ts`).
pub(crate) const READING_LOOK: bachuc_theme::ReadingLook = bachuc_theme::ReadingLook {
    content_width: 768.,
    response_line_height: 1.5,
    header_title_weight: 400.,
};

/// The eight colors of `claudeLight` in `client/theme.ts`, as `0xRRGGBB`.
pub(crate) struct Palette {
    pub(crate) background: u32,
    pub(crate) foreground: u32,
    pub(crate) raised: u32,
    pub(crate) control: u32,
    pub(crate) border: u32,
    pub(crate) accent: u32,
    pub(crate) muted_foreground: u32,
    pub(crate) ring: u32,
}

pub(crate) const CLAUDE_LIGHT: Palette = Palette {
    background: 0xFCFCFB,
    foreground: 0x131313,
    raised: 0xFFFFFF,
    control: 0xF9F9F7,
    border: 0xE7E6E1,
    accent: 0x256ABF,
    muted_foreground: 0x7B7974,
    ring: 0xA5A49A,
};

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(ClaudeLook)
}

struct ClaudeLook;

impl NativePlugin for ClaudeLook {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, _ctx: PluginContext, contributions: &mut Contributions, _cx: &mut App) {
        contributions.add_theme(claude_light());
        contributions.set_reading_look(READING_LOOK);
    }
}

fn hex(rgb: u32) -> Hsla {
    Rgba {
        r: ((rgb >> 16) & 0xff) as f32 / 255.,
        g: ((rgb >> 8) & 0xff) as f32 / 255.,
        b: (rgb & 0xff) as f32 / 255.,
        a: 1.,
    }
    .into()
}

pub(crate) fn claude_light() -> ThemeContribution {
    let palette = &CLAUDE_LIGHT;
    ThemeContribution {
        id: "claude-light",
        name: "Claude Light".into(),
        appearance: ThemeAppearance::Light,
        colors: PluginThemeColors {
            background: hex(palette.background),
            foreground: hex(palette.foreground),
            raised: hex(palette.raised),
            control: hex(palette.control),
            border: hex(palette.border),
            muted_foreground: hex(palette.muted_foreground),
            ring: hex(palette.ring),
            accent: Some(hex(palette.accent)),
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_plugin_host::ContributionIndex;

    /// The `0xRRGGBB` value of `key: "#RRGGBB"` in `client/theme.ts`.
    fn theme_ts_color(source: &str, key: &str) -> u32 {
        let line = source
            .lines()
            .find(|line| line.trim_start().starts_with(&format!("{key}:")))
            .unwrap_or_else(|| panic!("theme.ts has no {key}"));
        let hex = line
            .split('"')
            .nth(1)
            .and_then(|value| value.strip_prefix('#'))
            .unwrap_or_else(|| panic!("{key} is not a hex string"));
        u32::from_str_radix(hex, 16).unwrap()
    }

    #[test]
    fn palette_matches_theme_ts() {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../hiep-plugins/plugins/claude-look/client/theme.ts"
        );
        let source = std::fs::read_to_string(path).unwrap();
        let ts = |key| theme_ts_color(&source, key);
        let palette = &CLAUDE_LIGHT;
        assert_eq!(palette.background, ts("background"));
        assert_eq!(palette.foreground, ts("foreground"));
        assert_eq!(palette.raised, ts("raised"));
        assert_eq!(palette.control, ts("control"));
        assert_eq!(palette.border, ts("border"));
        assert_eq!(palette.accent, ts("accent"));
        assert_eq!(palette.muted_foreground, ts("mutedForeground"));
        assert_eq!(palette.ring, ts("ring"));
    }

    #[test]
    fn reading_look_matches_the_web_client() {
        let root = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../../hiep-plugins/plugins/claude-look"
        );
        let width = std::fs::read_to_string(format!("{root}/client/width.ts")).unwrap();
        let web = std::fs::read_to_string(format!("{root}/client/web.ts")).unwrap();
        assert!(width.contains("768"), "width.ts no longer narrows to 768");
        assert!(web.contains("line-height:1.5"), "web.ts no longer sets 1.5");
        assert!(
            web.contains("font-weight:400"),
            "web.ts no longer sets the title weight"
        );
        assert_eq!(READING_LOOK.header_title_weight, 400.);
        assert_eq!(READING_LOOK.content_width, 768.);
        assert_eq!(READING_LOOK.response_line_height, 1.5);
    }

    #[test]
    fn contributes_one_light_theme() {
        let mut contributions = Contributions::default();
        contributions.add_theme(claude_light());
        let index = ContributionIndex::build([(PLUGIN_ID, &contributions)]);
        let [theme] = index.themes() else {
            panic!("expected one theme");
        };
        assert_eq!(theme.plugin_id, PLUGIN_ID);
        assert_eq!(theme.value.id, "claude-light");
        assert_eq!(theme.value.name.as_ref(), "Claude Light");
        assert_eq!(theme.value.appearance, ThemeAppearance::Light);
        let colors = theme.value.colors;
        assert_eq!(colors.background, hex(0xFCFCFB));
        assert_eq!(colors.accent, Some(hex(0x256ABF)));
        assert_eq!(colors.ring, hex(0xA5A49A));
    }
}
