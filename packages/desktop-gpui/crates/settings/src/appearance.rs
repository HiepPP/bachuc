//! The Appearance settings: theme, the interface and code font families and sizes, the content
//! size, the syntax theme, the whole-UI zoom, and the tab close button side, stored in `appearance.json` next to `hosts.json` and applied
//! through `bachuc_theme`.

use anyhow::{Context as _, Result};
use bachuc_theme::{
    DEFAULT_CONTENT_FONT_SIZE, MAX_CONTENT_FONT_SIZE, MIN_CONTENT_FONT_SIZE, SyntaxThemeId,
    TabClosePosition, ThemeChoice, metrics, set_content_font_size, set_custom_theme,
    set_font_families, set_font_scale, set_syntax_theme, set_tab_close_position, set_theme,
    set_zoom,
};
use gpui_kit::{App, SharedString};
use serde_json::{Map, Value, json};
use std::path::Path;

const FILE: &str = "appearance.json";

/// `MIN_UI_BASE_FONT_SIZE` and `MAX_UI_BASE_FONT_SIZE` of the Electron app.
pub const INTERFACE_RANGE: (f32, f32) = (10., 21.);
/// `MIN_CODE_FONT_SIZE` and `MAX_CODE_FONT_SIZE` of the Electron app.
pub const CODE_RANGE: (f32, f32) = (9., 22.);
/// `MIN_CONTENT_FONT_SIZE` and `MAX_CONTENT_FONT_SIZE` of the Electron app.
pub const CONTENT_RANGE: (f32, f32) = (MIN_CONTENT_FONT_SIZE, MAX_CONTENT_FONT_SIZE);

/// The whole-UI zoom levels, as factors: the Electron and Chrome zoom steps from 50% to 200%.
pub const ZOOM_STEPS: [f32; 11] = [0.5, 0.67, 0.75, 0.8, 0.9, 1., 1.1, 1.25, 1.5, 1.75, 2.];

/// A zoom shortcut: one step in, one step out, or back to 100%.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ZoomDirection {
    In,
    Out,
    Reset,
}

/// The [`ZOOM_STEPS`] entry nearest to `factor`. A factor that is not finite gives 1.0, and
/// one out of range gives the end step.
pub fn nearest_zoom_step(factor: f32) -> f32 {
    if !factor.is_finite() {
        return 1.;
    }
    ZOOM_STEPS
        .into_iter()
        .min_by(|a, b| (a - factor).abs().total_cmp(&(b - factor).abs()))
        .unwrap_or(1.)
}

/// The zoom factor after one shortcut from `current`. In and Out move one step from the step
/// nearest to `current` and stop at 50% and 200%.
pub fn zoom_step(current: f32, direction: ZoomDirection) -> f32 {
    let index = ZOOM_STEPS
        .iter()
        .position(|step| *step == nearest_zoom_step(current))
        .unwrap_or(5);
    match direction {
        ZoomDirection::Reset => 1.,
        ZoomDirection::In => ZOOM_STEPS[(index + 1).min(ZOOM_STEPS.len() - 1)],
        ZoomDirection::Out => ZOOM_STEPS[index.saturating_sub(1)],
    }
}

/// The first entry of a font family picker: no family chosen.
pub const DEFAULT_FAMILY: &str = "Default";

#[derive(Clone, Debug, PartialEq)]
pub struct Appearance {
    pub theme: ThemeChoice,
    /// Pixels; 14 is the default.
    pub interface_size: f32,
    /// Pixels; 12 is the default.
    pub code_size: f32,
    /// Pixels; 15 is the default.
    pub content_size: f32,
    /// The stored interface family; `None` is the default. The name may not be installed (see
    /// [`family_choice`]); it stays in the file so a font that comes back is used again.
    pub ui_font: Option<String>,
    pub code_font: Option<String>,
    pub syntax_theme: SyntaxThemeId,
    /// The selected plugin theme, `<plugin_id>/theme/<id>` (`pluginThemeId` in Electron). The id
    /// stays in the file while its plugin is off; the stored `theme` applies then.
    pub plugin_theme: Option<String>,
    /// The whole-UI zoom factor, one of [`ZOOM_STEPS`]; 1.0 is 100%.
    pub zoom: f32,
    /// The side of the tab close button; a file without it reads as `Left`.
    pub tab_close_position: TabClosePosition,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            interface_size: metrics::FONT_BASE,
            code_size: metrics::FONT_CODE,
            content_size: DEFAULT_CONTENT_FONT_SIZE,
            ui_font: None,
            code_font: None,
            syntax_theme: SyntaxThemeId::default(),
            plugin_theme: None,
            zoom: 1.,
            tab_close_position: TabClosePosition::Left,
        }
    }
}

/// `appearance` with a built-in theme picked: the plugin theme selection is cleared.
pub fn with_builtin_theme(appearance: &Appearance, theme: ThemeChoice) -> Appearance {
    Appearance {
        theme,
        plugin_theme: None,
        ..appearance.clone()
    }
}

/// `appearance` with a plugin theme picked. The built-in choice stays for when the plugin goes.
pub fn with_plugin_theme(appearance: &Appearance, id: &str) -> Appearance {
    Appearance {
        plugin_theme: Some(id.to_string()),
        ..appearance.clone()
    }
}

/// The stored family when it names an installed one, else `None` (the default). This is the rule
/// `bachuc_theme::set_font_families` applies.
pub fn family_choice(stored: Option<&str>, installed: &[SharedString]) -> Option<SharedString> {
    let name = stored?.trim();
    installed.iter().find(|font| font.as_ref() == name).cloned()
}

/// The text on a family picker: the family when it is installed, else [`DEFAULT_FAMILY`].
pub fn family_label(stored: Option<&str>, installed: &[SharedString]) -> SharedString {
    family_choice(stored, installed).unwrap_or_else(|| DEFAULT_FAMILY.into())
}

/// Whole pixels within [`CONTENT_RANGE`], as `bachuc_theme::set_content_font_size` reads them.
pub fn clamp_content_size(size: f32) -> f32 {
    if size.is_finite() {
        size.floor().clamp(CONTENT_RANGE.0, CONTENT_RANGE.1)
    } else {
        DEFAULT_CONTENT_FONT_SIZE
    }
}

pub(crate) fn theme_key(theme: ThemeChoice) -> &'static str {
    match theme {
        ThemeChoice::Light => "light",
        ThemeChoice::ClaudeDark => "claude-dark",
        ThemeChoice::System => "system",
    }
}

fn tab_close_key(position: TabClosePosition) -> &'static str {
    match position {
        TabClosePosition::Left => "left",
        TabClosePosition::Right => "right",
    }
}

fn tab_close_from_key(key: &str) -> Option<TabClosePosition> {
    Some(match key {
        "left" => TabClosePosition::Left,
        "right" => TabClosePosition::Right,
        _ => return None,
    })
}

/// The label on the tab close button control.
pub fn tab_close_label(position: TabClosePosition) -> &'static str {
    match position {
        TabClosePosition::Left => "Left",
        TabClosePosition::Right => "Right",
    }
}

fn theme_from_key(key: &str) -> Option<ThemeChoice> {
    Some(match key {
        "light" => ThemeChoice::Light,
        "claude-dark" => ThemeChoice::ClaudeDark,
        "system" => ThemeChoice::System,
        _ => return None,
    })
}

/// The label on the theme control.
pub fn theme_label(theme: ThemeChoice) -> &'static str {
    match theme {
        ThemeChoice::Light => "Light",
        ThemeChoice::ClaudeDark => "Claude dark",
        ThemeChoice::System => "System",
    }
}

/// Reads a size field: digits only, clamped to the range. Empty or non-numeric text gives None,
/// and the field falls back to the committed value.
pub fn parse_size(text: &str, range: (f32, f32)) -> Option<f32> {
    let digits = text.trim();
    if digits.is_empty() || !digits.chars().all(|c| c.is_ascii_digit()) {
        return None;
    }
    let value: f32 = digits.parse().ok()?;
    Some(value.clamp(range.0, range.1))
}

/// Reads `appearance.json` in `dir`. A missing, corrupt, or partial file gives the defaults for
/// what it lacks; the file is never rewritten here.
pub fn load_from(dir: &Path) -> Appearance {
    let mut appearance = Appearance::default();
    let path = dir.join(FILE);
    let Ok(text) = std::fs::read_to_string(&path) else {
        return appearance;
    };
    let Ok(Value::Object(stored)) = serde_json::from_str::<Value>(&text) else {
        log::warn!(
            "{} is not a JSON object; using the defaults",
            path.display()
        );
        return appearance;
    };
    if let Some(theme) = stored
        .get("theme")
        .and_then(Value::as_str)
        .and_then(theme_from_key)
    {
        appearance.theme = theme;
    }
    let size = |key: &str, range: (f32, f32), fallback: f32| {
        stored
            .get(key)
            .and_then(Value::as_f64)
            .map_or(fallback, |value| (value as f32).clamp(range.0, range.1))
    };
    appearance.interface_size = size("interfaceSize", INTERFACE_RANGE, appearance.interface_size);
    appearance.code_size = size("codeSize", CODE_RANGE, appearance.code_size);
    appearance.content_size = stored
        .get("contentSize")
        .and_then(Value::as_f64)
        .map_or(appearance.content_size, |value| {
            clamp_content_size(value as f32)
        });
    let family = |key: &str| {
        stored
            .get(key)
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(str::to_string)
    };
    appearance.ui_font = family("uiFont");
    appearance.code_font = family("codeFont");
    if let Some(id) = stored
        .get("syntaxTheme")
        .and_then(Value::as_str)
        .and_then(SyntaxThemeId::from_id)
    {
        appearance.syntax_theme = id;
    }
    appearance.plugin_theme = family("pluginThemeId");
    if let Some(zoom) = stored.get("zoom").and_then(Value::as_f64) {
        appearance.zoom = nearest_zoom_step(zoom as f32);
    }
    if let Some(position) = stored
        .get("tabClosePosition")
        .and_then(Value::as_str)
        .and_then(tab_close_from_key)
    {
        appearance.tab_close_position = position;
    }
    appearance
}

/// Writes `appearance.json` in `dir` through a temporary file and a rename.
pub fn save_to(dir: &Path, appearance: &Appearance) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let mut value = Map::new();
    value.insert("theme".into(), json!(theme_key(appearance.theme)));
    value.insert("interfaceSize".into(), json!(appearance.interface_size));
    value.insert("codeSize".into(), json!(appearance.code_size));
    value.insert("contentSize".into(), json!(appearance.content_size));
    if let Some(name) = &appearance.ui_font {
        value.insert("uiFont".into(), json!(name));
    }
    if let Some(name) = &appearance.code_font {
        value.insert("codeFont".into(), json!(name));
    }
    value.insert("syntaxTheme".into(), json!(appearance.syntax_theme.id()));
    if let Some(id) = &appearance.plugin_theme {
        value.insert("pluginThemeId".into(), json!(id));
    }
    // Rounded, so 1.1 is stored as 1.1 and not as the f64 of the f32.
    let zoom = (f64::from(appearance.zoom) * 100.).round() / 100.;
    value.insert("zoom".into(), json!(zoom));
    value.insert(
        "tabClosePosition".into(),
        json!(tab_close_key(appearance.tab_close_position)),
    );
    let value = Value::Object(value);
    let temp = dir.join(format!("{FILE}.tmp"));
    std::fs::write(
        &temp,
        format!("{}\n", serde_json::to_string_pretty(&value)?),
    )
    .with_context(|| format!("write {}", temp.display()))?;
    std::fs::rename(&temp, dir.join(FILE)).with_context(|| format!("replace {FILE}"))?;
    Ok(())
}

/// Applies the theme, the font scale, the families, the content size, the syntax theme, the
/// zoom, and the tab close button side to the running app. A family that is not installed selects the default.
pub fn apply(appearance: &Appearance, cx: &mut App) {
    set_theme(appearance.theme, cx);
    set_font_scale(
        appearance.interface_size / metrics::FONT_BASE,
        appearance.code_size / metrics::FONT_CODE,
        cx,
    );
    set_font_families(
        appearance.ui_font.clone().map(SharedString::from),
        appearance.code_font.clone().map(SharedString::from),
        cx,
    );
    set_content_font_size(appearance.content_size, cx);
    set_syntax_theme(appearance.syntax_theme, cx);
    set_custom_theme(appearance.plugin_theme.clone().map(SharedString::from), cx);
    set_zoom(appearance.zoom, cx);
    set_tab_close_position(appearance.tab_close_position, cx);
}

/// Reads `appearance.json` from `~/Library/Application Support/BachucGpui` and applies it. Call
/// it once at start, after `bachuc_theme::init`.
pub fn load_appearance(cx: &mut App) {
    apply(&load_from(&bachuc_command::data_dir()), cx);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn temp_dir(name: &str) -> std::path::PathBuf {
        let dir =
            std::env::temp_dir().join(format!("bachuc-settings-{name}-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        dir
    }

    #[test]
    fn missing_file_gives_the_defaults() {
        let dir = temp_dir("appearance-missing");
        let appearance = load_from(&dir);
        assert_eq!(appearance, Appearance::default());
        assert_eq!(appearance.interface_size, 14.);
        assert_eq!(appearance.code_size, 12.);
        assert!(!dir.join(FILE).exists(), "load never writes");
    }

    #[test]
    fn saved_appearance_survives_a_reload() {
        let dir = temp_dir("appearance-saved");
        let saved = Appearance {
            theme: ThemeChoice::ClaudeDark,
            interface_size: 16.,
            code_size: 13.,
            ..Appearance::default()
        };
        save_to(&dir, &saved).unwrap();
        assert_eq!(load_from(&dir), saved);
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(text.contains("\"claude-dark\""), "{text}");
        assert!(!dir.join("appearance.json.tmp").exists());
    }

    #[test]
    fn the_four_new_values_survive_a_reload() {
        let dir = temp_dir("appearance-new-values");
        let saved = Appearance {
            ui_font: Some("Helvetica Neue".into()),
            code_font: Some("Menlo".into()),
            content_size: 18.,
            syntax_theme: SyntaxThemeId::TokyoNight,
            ..Appearance::default()
        };
        save_to(&dir, &saved).unwrap();
        assert_eq!(load_from(&dir), saved);
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(text.contains("\"tokyo-night\""), "{text}");
        for theme in SyntaxThemeId::ALL {
            let saved = Appearance {
                syntax_theme: theme,
                ..Appearance::default()
            };
            save_to(&dir, &saved).unwrap();
            assert_eq!(load_from(&dir).syntax_theme, theme);
        }
    }

    #[test]
    fn an_unset_family_stays_out_of_the_file() {
        let dir = temp_dir("appearance-unset-family");
        save_to(&dir, &Appearance::default()).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(
            !text.contains("uiFont") && !text.contains("codeFont"),
            "{text}"
        );
        assert_eq!(load_from(&dir), Appearance::default());
    }

    #[test]
    fn an_old_file_without_the_new_keys_loads_the_defaults() {
        let dir = temp_dir("appearance-old-file");
        std::fs::write(
            dir.join(FILE),
            r#"{"theme":"light","interfaceSize":16,"codeSize":13}"#,
        )
        .unwrap();
        let appearance = load_from(&dir);
        assert_eq!(appearance.theme, ThemeChoice::Light);
        assert_eq!(appearance.interface_size, 16.);
        assert_eq!(appearance.code_size, 13.);
        assert_eq!(appearance.content_size, 15.);
        assert_eq!(appearance.ui_font, None);
        assert_eq!(appearance.code_font, None);
        assert_eq!(appearance.syntax_theme, SyntaxThemeId::One);
    }

    #[test]
    fn saving_writes_only_appearance_json() {
        let dir = temp_dir("appearance-write-path");
        save_to(&dir, &Appearance::default()).unwrap();
        save_to(
            &dir,
            &Appearance {
                content_size: 12.,
                ..Appearance::default()
            },
        )
        .unwrap();
        let mut names: Vec<_> = std::fs::read_dir(&dir)
            .unwrap()
            .map(|entry| entry.unwrap().file_name().to_string_lossy().into_owned())
            .collect();
        names.sort();
        assert_eq!(names, [FILE]);
    }

    #[test]
    fn content_size_is_floored_and_clamped() {
        assert_eq!(clamp_content_size(15.), 15.);
        assert_eq!(clamp_content_size(15.9), 15.);
        assert_eq!(clamp_content_size(2.), 10.);
        assert_eq!(clamp_content_size(400.), 21.);
        assert_eq!(clamp_content_size(f32::NAN), 15.);
        assert_eq!(parse_size("400", CONTENT_RANGE), Some(21.));
        assert_eq!(parse_size("3", CONTENT_RANGE), Some(10.));
        let dir = temp_dir("appearance-content-clamp");
        std::fs::write(dir.join(FILE), r#"{"contentSize":99}"#).unwrap();
        assert_eq!(load_from(&dir).content_size, 21.);
        std::fs::write(dir.join(FILE), r#"{"contentSize":1}"#).unwrap();
        assert_eq!(load_from(&dir).content_size, 10.);
        std::fs::write(dir.join(FILE), r#"{"contentSize":"big"}"#).unwrap();
        assert_eq!(load_from(&dir).content_size, 15.);
    }

    #[test]
    fn a_stored_font_that_is_not_installed_shows_the_default() {
        let installed: Vec<SharedString> = vec!["Helvetica".into(), "Menlo".into()];
        assert_eq!(
            family_choice(Some("Menlo"), &installed),
            Some("Menlo".into())
        );
        assert_eq!(
            family_choice(Some(" Menlo "), &installed),
            Some("Menlo".into())
        );
        assert_eq!(family_choice(Some("Gone Sans"), &installed), None);
        assert_eq!(family_choice(None, &installed), None);
        assert_eq!(family_label(Some("Menlo"), &installed), "Menlo");
        assert_eq!(family_label(Some("Gone Sans"), &installed), DEFAULT_FAMILY);
        assert_eq!(family_label(None, &installed), DEFAULT_FAMILY);
        let dir = temp_dir("appearance-missing-font");
        std::fs::write(dir.join(FILE), r#"{"uiFont":"Gone Sans","codeFont":""}"#).unwrap();
        let appearance = load_from(&dir);
        assert_eq!(appearance.ui_font.as_deref(), Some("Gone Sans"));
        assert_eq!(appearance.code_font, None, "an empty name is unset");
    }

    #[test]
    fn an_unknown_syntax_id_gives_the_default() {
        let dir = temp_dir("appearance-syntax-unknown");
        std::fs::write(dir.join(FILE), r#"{"syntaxTheme":"neon"}"#).unwrap();
        assert_eq!(load_from(&dir).syntax_theme, SyntaxThemeId::One);
        std::fs::write(dir.join(FILE), r#"{"syntaxTheme":4}"#).unwrap();
        assert_eq!(load_from(&dir).syntax_theme, SyntaxThemeId::One);
    }

    #[test]
    fn every_theme_round_trips() {
        let dir = temp_dir("appearance-themes");
        for theme in [
            ThemeChoice::Light,
            ThemeChoice::ClaudeDark,
            ThemeChoice::System,
        ] {
            let saved = Appearance {
                theme,
                ..Appearance::default()
            };
            save_to(&dir, &saved).unwrap();
            assert_eq!(load_from(&dir).theme, theme);
        }
    }

    #[test]
    fn a_file_without_a_plugin_theme_loads_as_before() {
        let dir = temp_dir("appearance-no-plugin-theme");
        std::fs::write(dir.join(FILE), r#"{"theme":"light","interfaceSize":16}"#).unwrap();
        assert_eq!(load_from(&dir).plugin_theme, None);
        save_to(&dir, &load_from(&dir)).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!text.contains("pluginThemeId"), "{text}");
    }

    #[test]
    fn the_plugin_theme_id_loads_and_survives_a_reload() {
        let dir = temp_dir("appearance-plugin-theme");
        std::fs::write(
            dir.join(FILE),
            r#"{"theme":"claude-dark","pluginThemeId":"claude-look/theme/claude-light"}"#,
        )
        .unwrap();
        let loaded = load_from(&dir);
        assert_eq!(loaded.theme, ThemeChoice::ClaudeDark);
        assert_eq!(
            loaded.plugin_theme.as_deref(),
            Some("claude-look/theme/claude-light")
        );
        save_to(&dir, &loaded).unwrap();
        assert_eq!(load_from(&dir), loaded);
        std::fs::write(dir.join(FILE), r#"{"pluginThemeId":4}"#).unwrap();
        assert_eq!(load_from(&dir).plugin_theme, None);
    }

    #[test]
    fn a_plugin_theme_whose_plugin_is_off_stays_in_the_file_on_another_save() {
        let dir = temp_dir("appearance-plugin-theme-kept");
        std::fs::write(
            dir.join(FILE),
            r#"{"pluginThemeId":"claude-look/theme/claude-light"}"#,
        )
        .unwrap();
        let mut next = load_from(&dir);
        next.interface_size = 17.;
        save_to(&dir, &next).unwrap();
        let reloaded = load_from(&dir);
        assert_eq!(reloaded.interface_size, 17.);
        assert_eq!(
            reloaded.plugin_theme.as_deref(),
            Some("claude-look/theme/claude-light")
        );
        assert_eq!(
            reloaded.theme,
            ThemeChoice::System,
            "the stored choice applies"
        );
    }

    #[test]
    fn picking_a_built_in_theme_clears_the_plugin_theme() {
        let dir = temp_dir("appearance-plugin-theme-clear");
        let picked = with_plugin_theme(&Appearance::default(), "claude-look/theme/claude-light");
        assert_eq!(
            picked.theme,
            ThemeChoice::System,
            "the built-in choice stays"
        );
        save_to(&dir, &picked).unwrap();
        assert!(
            std::fs::read_to_string(dir.join(FILE))
                .unwrap()
                .contains("pluginThemeId")
        );
        let light = with_builtin_theme(&load_from(&dir), ThemeChoice::Light);
        assert_eq!(light.theme, ThemeChoice::Light);
        assert_eq!(light.plugin_theme, None);
        save_to(&dir, &light).unwrap();
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(!text.contains("pluginThemeId"), "{text}");
        assert_eq!(load_from(&dir), light);
    }

    #[test]
    fn corrupt_and_out_of_range_values_fall_back() {
        let dir = temp_dir("appearance-corrupt");
        std::fs::write(dir.join(FILE), "not json").unwrap();
        assert_eq!(load_from(&dir), Appearance::default());
        assert_eq!(
            std::fs::read_to_string(dir.join(FILE)).unwrap(),
            "not json",
            "a corrupt file stays for the user"
        );
        std::fs::write(
            dir.join(FILE),
            r#"{"theme":"nope","interfaceSize":99,"codeSize":"big"}"#,
        )
        .unwrap();
        let appearance = load_from(&dir);
        assert_eq!(appearance.theme, ThemeChoice::System);
        assert_eq!(appearance.interface_size, INTERFACE_RANGE.1);
        assert_eq!(appearance.code_size, 12.);
    }

    #[test]
    fn zoom_steps_in_and_out_and_stops_at_50_and_200_percent() {
        assert_eq!(zoom_step(1., ZoomDirection::In), 1.1);
        assert_eq!(zoom_step(1., ZoomDirection::Out), 0.9);
        assert_eq!(zoom_step(0.9, ZoomDirection::Out), 0.8);
        assert_eq!(zoom_step(1.5, ZoomDirection::Reset), 1.);
        assert_eq!(zoom_step(0.5, ZoomDirection::Out), 0.5);
        assert_eq!(zoom_step(0.67, ZoomDirection::Out), 0.5);
        assert_eq!(zoom_step(2., ZoomDirection::In), 2.);
        assert_eq!(zoom_step(1.75, ZoomDirection::In), 2.);
        let mut level = 1.;
        for _ in 0..20 {
            level = zoom_step(level, ZoomDirection::In);
        }
        assert_eq!(level, 2.);
        for _ in 0..20 {
            level = zoom_step(level, ZoomDirection::Out);
        }
        assert_eq!(level, 0.5);
        assert_eq!(zoom_step(f32::NAN, ZoomDirection::In), 1.1);
    }

    #[test]
    fn a_stored_zoom_reads_as_the_nearest_step_or_100_percent() {
        assert_eq!(nearest_zoom_step(1.5), 1.5);
        assert_eq!(nearest_zoom_step(1.3), 1.25);
        assert_eq!(nearest_zoom_step(0.66), 0.67);
        assert_eq!(nearest_zoom_step(9.), 2.);
        assert_eq!(nearest_zoom_step(0.01), 0.5);
        assert_eq!(nearest_zoom_step(-4.), 0.5);
        assert_eq!(nearest_zoom_step(f32::NAN), 1.);
        assert_eq!(nearest_zoom_step(f32::INFINITY), 1.);
        let dir = temp_dir("appearance-zoom");
        for (text, want) in [
            (r#"{"zoom":1.5}"#, 1.5),
            (r#"{"zoom":1.3}"#, 1.25),
            (r#"{"zoom":40}"#, 2.),
            (r#"{"zoom":0}"#, 0.5),
            (r#"{"zoom":"NaN"}"#, 1.),
            (r#"{"zoom":null}"#, 1.),
            (r#"{"theme":"light"}"#, 1.),
        ] {
            std::fs::write(dir.join(FILE), text).unwrap();
            assert_eq!(load_from(&dir).zoom, want, "{text}");
        }
        let saved = Appearance {
            zoom: 1.5,
            ..Appearance::default()
        };
        save_to(&dir, &saved).unwrap();
        assert_eq!(load_from(&dir), saved);
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(text.contains("\"zoom\": 1.5"), "{text}");
        for step in ZOOM_STEPS {
            let saved = Appearance {
                zoom: step,
                ..Appearance::default()
            };
            save_to(&dir, &saved).unwrap();
            assert_eq!(load_from(&dir).zoom, step);
        }
    }

    #[test]
    fn the_tab_close_button_is_left_unless_the_file_says_right() {
        let dir = temp_dir("appearance-tab-close");
        // A file from before the setting.
        std::fs::write(
            dir.join(FILE),
            r#"{"theme":"light","interfaceSize":16,"codeSize":13}"#,
        )
        .unwrap();
        assert_eq!(load_from(&dir).tab_close_position, TabClosePosition::Left);
        assert_eq!(
            Appearance::default().tab_close_position,
            TabClosePosition::Left
        );
        for (text, want) in [
            (r#"{"tabClosePosition":"right"}"#, TabClosePosition::Right),
            (r#"{"tabClosePosition":"left"}"#, TabClosePosition::Left),
            (r#"{"tabClosePosition":"top"}"#, TabClosePosition::Left),
            (r#"{"tabClosePosition":2}"#, TabClosePosition::Left),
        ] {
            std::fs::write(dir.join(FILE), text).unwrap();
            assert_eq!(load_from(&dir).tab_close_position, want, "{text}");
        }
        let saved = Appearance {
            tab_close_position: TabClosePosition::Right,
            ..Appearance::default()
        };
        save_to(&dir, &saved).unwrap();
        assert_eq!(load_from(&dir), saved);
        assert_eq!(tab_close_label(TabClosePosition::Right), "Right");
    }

    #[test]
    fn size_text_is_digits_clamped() {
        assert_eq!(parse_size("16", INTERFACE_RANGE), Some(16.));
        assert_eq!(parse_size("2", INTERFACE_RANGE), Some(10.));
        assert_eq!(parse_size("400", CODE_RANGE), Some(22.));
        assert_eq!(parse_size("", CODE_RANGE), None);
        assert_eq!(parse_size("1x", CODE_RANGE), None);
        assert_eq!(parse_size("-3", CODE_RANGE), None);
    }
}
