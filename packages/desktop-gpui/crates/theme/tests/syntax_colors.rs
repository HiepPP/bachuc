//! Compares every syntax theme with the sources in `packages/highlight/src`: `colors.ts` holds
//! the GitHub palettes, and `themes.ts` holds the role palettes, how they expand to the 20 roles,
//! and which palette each theme uses in the light and the dark app theme.

use std::collections::HashMap;

use bachuc_theme::{SYNTAX_ROLES, Scheme, SyntaxThemeId, syntax_colors_for};

const COLORS_TS: &str = include_str!("../../../../highlight/src/colors.ts");
const THEMES_TS: &str = include_str!("../../../../highlight/src/themes.ts");

type Palette = HashMap<String, [u8; 3]>;

fn hex(value: &str) -> [u8; 3] {
    let value = value
        .trim_matches(|c| c == '"' || c == ',' || c == ' ')
        .trim_start_matches('#');
    let number = u32::from_str_radix(value, 16).expect("a hex color");
    [(number >> 16) as u8, (number >> 8) as u8, number as u8]
}

/// The `key: "value"` lines between `start` and the next line that begins with `}`.
fn block(source: &str, start: &str) -> Palette {
    let body = source
        .split(start)
        .nth(1)
        .unwrap_or_else(|| panic!("{start} not found"));
    body.lines()
        .skip(1)
        .take_while(|line| !line.starts_with('}'))
        .filter_map(|line| {
            let (key, value) = line.trim().split_once(':')?;
            let value = value.split("//").next()?.trim();
            value
                .contains('#')
                .then(|| (key.trim().to_string(), hex(value)))
        })
        .collect()
}

/// `expandRolePalette`: highlight role -> role palette key.
fn expansion() -> Vec<(String, String)> {
    let body = THEMES_TS
        .split("function expandRolePalette")
        .nth(1)
        .expect("expandRolePalette");
    body.lines()
        .skip(2)
        .take_while(|line| !line.starts_with("  };"))
        .filter_map(|line| {
            let (role, source) = line.trim().split_once(':')?;
            let key = source.trim().trim_start_matches("r.").trim_end_matches(',');
            Some((role.trim().to_string(), key.to_string()))
        })
        .collect()
}

/// `resolveSyntaxColors`: theme id -> (light palette name, dark palette name). GitHub names the
/// full maps of `colors.ts`.
fn selection(id: &str) -> (String, String) {
    if id == "github" {
        return ("lightHighlightColors".into(), "darkHighlightColors".into());
    }
    let line = THEMES_TS
        .lines()
        .skip_while(|line| !line.contains(&format!("case \"{id}\":")))
        .nth(1)
        .unwrap_or_else(|| panic!("case {id}"))
        .trim();
    let call = line
        .split_once("expandRolePalette(")
        .expect("expandRolePalette call")
        .1
        .trim_end_matches(");");
    match call.split_once(" ? ") {
        Some((_, branches)) => {
            let (dark, light) = branches.split_once(" : ").expect("dark : light");
            (light.to_string(), dark.to_string())
        }
        None => (call.to_string(), call.to_string()),
    }
}

fn expected(id: &str, scheme: Scheme) -> Palette {
    let (light, dark) = selection(id);
    let name = if scheme == Scheme::Dark { dark } else { light };
    if id == "github" {
        return block(COLORS_TS, &format!("export const {name}:"));
    }
    let roles = block(THEMES_TS, &format!("const {name}:"));
    expansion()
        .into_iter()
        .map(|(role, key)| (role, roles[&key]))
        .collect()
}

fn assert_close(actual: [f32; 3], want: [u8; 3], what: &str) {
    for (got, want) in actual.iter().zip(want) {
        assert!(
            (got * 255. - want as f32).abs() <= 1.,
            "{what}: got {actual:?}, want {want:?}"
        );
    }
}

#[test]
fn every_theme_matches_the_highlight_package_in_light_and_dark() {
    for id in SyntaxThemeId::ALL {
        for scheme in [Scheme::Light, Scheme::Dark] {
            let want = expected(id.id(), scheme);
            assert_eq!(want.len(), 20, "{} {scheme:?} has 20 roles", id.id());
            let colors = syntax_colors_for(id, scheme);
            for (role, color) in colors.entries() {
                let rgba = color.to_rgb();
                assert_close(
                    [rgba.r, rgba.g, rgba.b],
                    want[role],
                    &format!("{} {scheme:?} {role}", id.id()),
                );
            }
        }
    }
}

#[test]
fn the_roles_are_the_twenty_of_colors_ts() {
    let github = block(COLORS_TS, "export const darkHighlightColors:");
    let mut want: Vec<_> = github.keys().cloned().collect();
    let mut got: Vec<_> = SYNTAX_ROLES.iter().map(|role| role.to_string()).collect();
    want.sort();
    got.sort();
    assert_eq!(got, want);
}

#[test]
fn the_ids_and_labels_match_themes_ts() {
    let ids: Vec<_> = SyntaxThemeId::ALL.iter().map(|id| id.id()).collect();
    for line in THEMES_TS
        .lines()
        .filter(|line| line.trim_start().starts_with("{ id: \""))
    {
        let id = line.split('"').nth(1).unwrap();
        let label = line.split('"').nth(3).unwrap();
        let theme = SyntaxThemeId::from_id(id).expect(id);
        assert_eq!(theme.label(), label);
        assert!(ids.contains(&id));
    }
    assert_eq!(ids.len(), 8);
}

#[test]
fn an_unknown_id_falls_back_to_one() {
    assert_eq!(SyntaxThemeId::from_id("monokai"), None);
    assert_eq!(SyntaxThemeId::from_id(""), None);
    assert_eq!(
        SyntaxThemeId::from_id("monokai").unwrap_or_default(),
        SyntaxThemeId::One
    );
    assert_eq!(SyntaxThemeId::default(), SyntaxThemeId::One);
}

#[test]
fn every_id_round_trips() {
    for id in SyntaxThemeId::ALL {
        assert_eq!(SyntaxThemeId::from_id(id.id()), Some(id));
    }
}
