//! The Appearance settings: theme and the interface and code font sizes, stored in
//! `appearance.json` next to `hosts.json` and applied through `bachuc_theme`.

use anyhow::{Context as _, Result};
use bachuc_theme::{ThemeChoice, metrics, set_font_scale, set_theme};
use gpui_kit::App;
use serde_json::{Value, json};
use std::path::Path;

const FILE: &str = "appearance.json";

/// `MIN_UI_BASE_FONT_SIZE` and `MAX_UI_BASE_FONT_SIZE` of the Electron app.
pub const INTERFACE_RANGE: (f32, f32) = (10., 21.);
/// `MIN_CODE_FONT_SIZE` and `MAX_CODE_FONT_SIZE` of the Electron app.
pub const CODE_RANGE: (f32, f32) = (9., 22.);

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Appearance {
    pub theme: ThemeChoice,
    /// Pixels; 14 is the default.
    pub interface_size: f32,
    /// Pixels; 12 is the default.
    pub code_size: f32,
}

impl Default for Appearance {
    fn default() -> Self {
        Self {
            theme: ThemeChoice::System,
            interface_size: metrics::FONT_BASE,
            code_size: metrics::FONT_CODE,
        }
    }
}

pub(crate) fn theme_key(theme: ThemeChoice) -> &'static str {
    match theme {
        ThemeChoice::Light => "light",
        ThemeChoice::ClaudeDark => "claude-dark",
        ThemeChoice::System => "system",
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
    appearance
}

/// Writes `appearance.json` in `dir` through a temporary file and a rename.
pub fn save_to(dir: &Path, appearance: &Appearance) -> Result<()> {
    std::fs::create_dir_all(dir).with_context(|| format!("create {}", dir.display()))?;
    let value = json!({
        "theme": theme_key(appearance.theme),
        "interfaceSize": appearance.interface_size,
        "codeSize": appearance.code_size,
    });
    let temp = dir.join(format!("{FILE}.tmp"));
    std::fs::write(
        &temp,
        format!("{}\n", serde_json::to_string_pretty(&value)?),
    )
    .with_context(|| format!("write {}", temp.display()))?;
    std::fs::rename(&temp, dir.join(FILE)).with_context(|| format!("replace {FILE}"))?;
    Ok(())
}

/// Applies the theme and the font scale to the running app.
pub fn apply(appearance: &Appearance, cx: &mut App) {
    set_theme(appearance.theme, cx);
    set_font_scale(
        appearance.interface_size / metrics::FONT_BASE,
        appearance.code_size / metrics::FONT_CODE,
        cx,
    );
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
        };
        save_to(&dir, &saved).unwrap();
        assert_eq!(load_from(&dir), saved);
        let text = std::fs::read_to_string(dir.join(FILE)).unwrap();
        assert!(text.contains("\"claude-dark\""), "{text}");
        assert!(!dir.join("appearance.json.tmp").exists());
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
    fn size_text_is_digits_clamped() {
        assert_eq!(parse_size("16", INTERFACE_RANGE), Some(16.));
        assert_eq!(parse_size("2", INTERFACE_RANGE), Some(10.));
        assert_eq!(parse_size("400", CODE_RANGE), Some(22.));
        assert_eq!(parse_size("", CODE_RANGE), None);
        assert_eq!(parse_size("1x", CODE_RANGE), None);
        assert_eq!(parse_size("-3", CODE_RANGE), None);
    }
}
