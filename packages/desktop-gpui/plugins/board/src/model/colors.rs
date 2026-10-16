//! Hue colors: `client/repo-color.ts`, `client/project-header.ts`, `shared/effort.ts`, and
//! `isDarkSurface` of `shared/orb.ts`. Electron writes `hsl(h, s%, l%)`; these return `Hsla`.

use bachuc_theme::Tokens;
use gpui_kit::{Hsla, hsla};

/// `hsl(hue, saturation%, lightness%)`.
fn hsl(hue: u16, saturation: f32, lightness: f32) -> Hsla {
    hsl_a(hue, saturation, lightness, 1.)
}

fn hsl_a(hue: u16, saturation: f32, lightness: f32, alpha: f32) -> Hsla {
    hsla(
        f32::from(hue) / 360.,
        saturation / 100.,
        lightness / 100.,
        alpha,
    )
}

/// Infers light or dark from a surface's relative luminance (WCAG).
pub(crate) fn is_dark_surface(color: Hsla) -> bool {
    let rgb = color.to_rgb();
    let linear = |c: f32| {
        if c <= 0.03928 {
            c / 12.92
        } else {
            ((c + 0.055) / 1.055).powf(2.4)
        }
    };
    0.2126 * linear(rgb.r) + 0.7152 * linear(rgb.g) + 0.0722 * linear(rgb.b) < 0.5
}

/// Card tint and border in the project's saved hue; neutral until a hue is assigned.
pub(crate) fn repo_surface(hue: Option<u16>, t: &Tokens) -> (Hsla, Hsla) {
    let Some(hue) = hue else {
        return (t.surface1, t.border);
    };
    if is_dark_surface(t.surface1) {
        (hsl(hue, 14., 14.), hsl(hue, 22., 29.))
    } else {
        (hsl(hue, 38., 97.), hsl(hue, 30., 84.))
    }
}

/// Project mark fill, as in the sidebar.
pub(crate) fn repo_mark(hue: Option<u16>, t: &Tokens) -> Hsla {
    hue.map_or(t.foreground_muted, |hue| hsl(hue, 42., 58.))
}

/// The repo name inside the card. Light lightness 31 keeps 4.5:1 on the tint, hovered and
/// pressed, for every hue.
pub(crate) fn repo_label(hue: Option<u16>, t: &Tokens) -> Hsla {
    hue.map_or(t.foreground_muted, |hue| {
        hsl(
            hue,
            45.,
            if is_dark_surface(t.surface1) {
                75.
            } else {
                31.
            },
        )
    })
}

/// The line between a parent card and its subagents.
pub(crate) fn repo_divider(hue: Option<u16>, t: &Tokens) -> Hsla {
    match hue {
        None => t.border,
        Some(hue) if is_dark_surface(t.surface1) => repo_surface(Some(hue), t).1,
        Some(hue) => hsl(hue, 30., 88.),
    }
}

#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct ChipColors {
    pub background: Hsla,
    pub border: Hsla,
    pub mark_background: Hsla,
    pub mark_text: Hsla,
}

/// The project chip: the Board hue once assigned, else a neutral tint from the theme.
#[allow(dead_code)] // TASK-193's header chip draws with it.
pub(crate) fn chip_colors(hue: Option<u16>, t: &Tokens) -> ChipColors {
    match hue {
        None => ChipColors {
            background: t.surface2,
            border: t.border,
            mark_background: t.foreground_muted,
            mark_text: t.surface0,
        },
        Some(hue) => ChipColors {
            background: hsl_a(hue, 60., 72., 0.2),
            border: hsl_a(hue, 60., 72., 0.8),
            mark_background: hsl(hue, 42., 58.),
            mark_text: hsla(0., 0., 1., 1.),
        },
    }
}

/// The first character of the trimmed name, uppercased; `?` when empty.
pub(crate) fn project_initial(name: &str) -> String {
    name.trim()
        .chars()
        .next()
        .map_or_else(|| "?".into(), |first| first.to_uppercase().collect())
}

/// Effort tag color: the hue rises with reasoning depth; `None` for an unknown level.
pub(crate) fn effort_color(effort: &str, surface: Hsla) -> Option<Hsla> {
    let level: String = effort
        .to_lowercase()
        .chars()
        .filter(|c| !c.is_whitespace() && *c != '_' && *c != '-')
        .collect();
    let has = |words: &[&str]| words.iter().any(|word| level.contains(word));
    let (hue, saturation, light, dark) = if has(&["max", "ultra"]) {
        (330, 70., 42., 77.)
    } else if has(&["extra", "xhigh"]) {
        (15, 85., 38., 72.)
    } else if has(&["high"]) {
        (35, 90., 30., 55.)
    } else if has(&["medium"]) {
        (220, 75., 45., 76.)
    } else if has(&["low", "minimal"]) {
        (185, 70., 28., 52.)
    } else {
        return None;
    };
    let lightness = if is_dark_surface(surface) {
        dark
    } else {
        light
    };
    Some(hsl(hue, saturation, lightness))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::Rgba;

    fn hex(rgb: u32) -> Hsla {
        Rgba {
            r: ((rgb >> 16) & 0xff) as f32 / 255.,
            g: ((rgb >> 8) & 0xff) as f32 / 255.,
            b: (rgb & 0xff) as f32 / 255.,
            a: 1.,
        }
        .into()
    }

    fn luminance(color: Hsla) -> f32 {
        let rgb = color.to_rgb();
        let linear = |c: f32| {
            if c <= 0.03928 {
                c / 12.92
            } else {
                ((c + 0.055) / 1.055).powf(2.4)
            }
        };
        0.2126 * linear(rgb.r) + 0.7152 * linear(rgb.g) + 0.0722 * linear(rgb.b)
    }

    fn contrast(a: Hsla, b: Hsla) -> f32 {
        let (x, y) = (luminance(a), luminance(b));
        (x.max(y) + 0.05) / (x.min(y) + 0.05)
    }

    fn mix(a: Hsla, b: Hsla, t: f32) -> Hsla {
        let (a, b) = (a.to_rgb(), b.to_rgb());
        Rgba {
            r: a.r * (1. - t) + b.r * t,
            g: a.g * (1. - t) + b.g * t,
            b: a.b * (1. - t) + b.b * t,
            a: 1.,
        }
        .into()
    }

    fn theme(surface1: u32, surface2: u32, border: u32) -> Tokens {
        Tokens {
            surface1: hex(surface1),
            surface2: hex(surface2),
            border: hex(border),
            ..Tokens::light()
        }
    }

    #[test]
    fn reads_dark_and_light_surfaces() {
        assert!(is_dark_surface(hex(0x141416)));
        assert!(!is_dark_surface(hex(0xffffff)));
        assert!(!is_dark_surface(hex(0xf6f6f7)));
        assert!(is_dark_surface(hex(0x1c1c1f)));
        assert!(!is_dark_surface(hex(0xfafafa)));
        assert!(!is_dark_surface(Tokens::light().surface1));
        assert!(is_dark_surface(Tokens::claude_dark().surface1));
    }

    #[test]
    fn the_repo_label_keeps_4_5_to_1_on_tint_hover_and_press_for_every_hue() {
        // Paseo light, zinc dark, and the dark theme with the lightest surfaces.
        for t in [
            theme(0xfafafa, 0xf4f4f5, 0xe4e4e7),
            theme(0x1f1f22, 0x27272a, 0x27272a),
            theme(0x2f333d, 0x383c48, 0x353a47),
        ] {
            for hue in 0..360 {
                let text = repo_label(Some(hue), &t);
                let tint = repo_surface(Some(hue), &t).0;
                for surface in [tint, mix(tint, t.surface2, 0.55), t.surface2] {
                    let ratio = contrast(text, surface);
                    assert!(ratio >= 4.5, "hue {hue}: {ratio}");
                }
            }
        }
    }

    #[test]
    fn light_and_dark_repo_label_lightness() {
        let light = theme(0xfafafa, 0xf4f4f5, 0xe4e4e7);
        let dark = theme(0x1f1f22, 0x27272a, 0x27272a);
        assert_eq!(
            repo_label(Some(200), &light),
            hsla(200. / 360., 0.45, 0.31, 1.)
        );
        assert_eq!(
            repo_label(Some(200), &dark),
            hsla(200. / 360., 0.45, 0.75, 1.)
        );
    }

    #[test]
    fn projects_without_a_hue_stay_neutral() {
        let t = Tokens::light();
        assert_eq!(repo_label(None, &t), t.foreground_muted);
        assert_eq!(repo_mark(None, &t), t.foreground_muted);
        assert_eq!(repo_divider(None, &t), t.border);
        assert_eq!(repo_surface(None, &t), (t.surface1, t.border));
        assert_eq!(
            repo_surface(Some(200), &t).0,
            hsla(200. / 360., 0.38, 0.97, 1.)
        );
        assert_eq!(repo_mark(Some(200), &t), hsla(200. / 360., 0.42, 0.58, 1.));
    }

    #[test]
    fn chip_colors_use_the_theme_until_a_hue_exists() {
        let t = Tokens::light();
        assert_eq!(
            chip_colors(None, &t),
            ChipColors {
                background: t.surface2,
                border: t.border,
                mark_background: t.foreground_muted,
                mark_text: t.surface0,
            }
        );
        assert_eq!(
            chip_colors(Some(200), &t),
            ChipColors {
                background: hsla(200. / 360., 0.6, 0.72, 0.2),
                border: hsla(200. / 360., 0.6, 0.72, 0.8),
                mark_background: hsla(200. / 360., 0.42, 0.58, 1.),
                mark_text: hsla(0., 0., 1., 1.),
            }
        );
    }

    #[test]
    fn the_mark_shows_the_first_character_uppercased() {
        assert_eq!(project_initial(" hiep-paseo-plugin "), "H");
        assert_eq!(project_initial("élan"), "É");
        assert_eq!(project_initial("  "), "?");
    }

    #[test]
    fn every_effort_tag_reaches_4_5_to_1_on_every_theme_surface() {
        let surfaces: [&[u32]; 5] = [
            &[0xfafafa, 0xf4f4f5],
            &[0x1e2120, 0x272a29, 0x1f1f22, 0x27272a, 0x1c1e27, 0x252731],
            &[0x262523, 0x2f2d2b],
            &[0x2f333d, 0x383c48],
            &[0x0a0a0a, 0x111111],
        ];
        for theme in surfaces {
            for level in [
                "low",
                "medium",
                "high",
                "xhigh",
                "Extra High",
                "max",
                "ultracode",
            ] {
                let color = effort_color(level, hex(*theme.last().unwrap())).unwrap();
                for &surface in theme {
                    let ratio = contrast(color, hex(surface));
                    assert!(ratio >= 4.5, "{level} on {surface:06x}: {ratio}");
                }
            }
        }
    }

    #[test]
    fn effort_keeps_hues_by_depth_and_leaves_unknown_levels() {
        assert_eq!(effort_color("high", hex(0xf4f4f5)).unwrap().h, 35. / 360.);
        assert_eq!(
            effort_color("Extra High", hex(0x272a29)).unwrap().h,
            15. / 360.
        );
        assert_eq!(effort_color("off", hex(0xf4f4f5)), None);
    }
}
