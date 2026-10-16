//! The Electron provider logos. `codegen/generate-provider-logos.mjs` writes
//! `assets/providers/*.svg`; each file is a template with `{FILL}` and `{OPACITY}` placeholders.

use crate::IconName;
use gpui_kit::{Hsla, Image, ImageFormat, Rgba};
use std::collections::HashMap;
use std::sync::{Arc, Mutex, OnceLock};

/// Sorted by provider id. `kiro` has no entry: Electron draws a lucide icon for it.
static LOGOS: &[(&str, &str)] = &[
    ("claude", include_str!("../assets/providers/claude.svg")),
    ("codex", include_str!("../assets/providers/codex.svg")),
    ("copilot", include_str!("../assets/providers/copilot.svg")),
    ("glm", include_str!("../assets/providers/glm.svg")),
    ("minimax", include_str!("../assets/providers/minimax.svg")),
    ("omp", include_str!("../assets/providers/omp.svg")),
    ("opencode", include_str!("../assets/providers/opencode.svg")),
    ("pi", include_str!("../assets/providers/pi.svg")),
];

type CacheKey = (&'static str, [u32; 4]);

/// The SVG template of a provider logo, with `{FILL}` and `{OPACITY}` placeholders, or `None` when
/// Electron draws no filled logo for the provider.
pub fn provider_logo_svg(provider: &str) -> Option<&'static str> {
    LOGOS
        .binary_search_by(|(id, _)| (*id).cmp(provider))
        .ok()
        .map(|found| LOGOS[found].1)
}

/// The provider logo filled with `fill`, or `None` when the provider has no logo. GPUI draws an
/// SVG image at twice the layout size. The image is cached per provider and color, so equal calls
/// return the same `Arc`.
pub fn provider_logo(provider: &str, fill: Hsla) -> Option<Arc<Image>> {
    static CACHE: OnceLock<Mutex<HashMap<CacheKey, Arc<Image>>>> = OnceLock::new();
    let found = LOGOS.binary_search_by(|(id, _)| (*id).cmp(provider)).ok()?;
    let (id, template) = LOGOS[found];
    let key = (id, [fill.h, fill.s, fill.l, fill.a].map(f32::to_bits));
    let mut cache = CACHE
        .get_or_init(Default::default)
        .lock()
        .unwrap_or_else(|poisoned| poisoned.into_inner());
    Some(
        cache
            .entry(key)
            .or_insert_with(|| {
                Arc::new(Image::from_bytes(
                    ImageFormat::Svg,
                    filled(template, fill).into_bytes(),
                ))
            })
            .clone(),
    )
}

/// The icon to draw when `provider_logo` gives `None`: `kiro` draws `PackagePlus` in Electron, and
/// every other id draws `Bot`.
pub fn provider_fallback_icon(provider: &str) -> IconName {
    match provider {
        "kiro" => IconName::PackagePlus,
        _ => IconName::Bot,
    }
}

fn filled(template: &str, color: Hsla) -> String {
    let rgba = Rgba::from(color);
    let channel = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
    template
        .replace(
            "{FILL}",
            &format!(
                "#{:02x}{:02x}{:02x}",
                channel(rgba.r),
                channel(rgba.g),
                channel(rgba.b)
            ),
        )
        .replace("{OPACITY}", &format!("{:.3}", rgba.a.clamp(0., 1.)))
}

#[cfg(test)]
mod tests {
    use super::*;
    use gpui_kit::hsla;

    /// The ids of `packages/protocol/src/provider-icon-names.ts` plus `glm`.
    const ELECTRON_IDS: [&str; 9] = [
        "claude", "codex", "copilot", "glm", "kiro", "minimax", "omp", "opencode", "pi",
    ];

    fn white() -> Hsla {
        hsla(0., 0., 1., 1.)
    }

    #[test]
    fn every_electron_provider_id_has_a_logo_or_a_fallback_icon() {
        for id in ELECTRON_IDS {
            if id == "kiro" {
                assert!(provider_logo_svg(id).is_none(), "{id}");
                assert!(provider_logo(id, white()).is_none(), "{id}");
                continue;
            }
            let svg = provider_logo_svg(id).unwrap_or_else(|| panic!("{id}: no logo"));
            assert!(svg.starts_with("<svg"), "{id}: {svg}");
            assert!(svg.contains("<path "), "{id}");
            assert!(svg.contains("fill=\"{FILL}\""), "{id}");
            assert!(svg.contains("{OPACITY}"), "{id}");
            assert!(svg.trim_end().ends_with("</svg>"), "{id}");
            assert!(provider_logo(id, white()).is_some(), "{id}");
        }
    }

    #[test]
    fn each_logo_has_its_own_glyph() {
        let mut seen = std::collections::HashSet::new();
        for (id, svg) in LOGOS {
            assert!(seen.insert(*svg), "{id} repeats another logo");
        }
        assert_eq!(LOGOS.len(), ELECTRON_IDS.len() - 1);
    }

    #[test]
    fn the_table_is_sorted_for_binary_search() {
        assert!(LOGOS.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn the_fill_is_written_into_the_svg_text() {
        let text = filled(provider_logo_svg("claude").unwrap(), hsla(0., 1., 0.5, 0.5));
        assert!(text.contains("fill=\"#ff0000\""), "{text}");
        assert!(text.contains("fill-opacity=\"0.500\""), "{text}");
        assert!(!text.contains('{'), "a placeholder is left: {text}");
    }

    #[test]
    fn opencode_keeps_its_dimmed_rectangle() {
        assert!(
            provider_logo_svg("opencode")
                .unwrap()
                .contains("opacity=\"0.4\"")
        );
    }

    #[test]
    fn kiro_and_unknown_ids_have_no_logo_and_a_fallback_icon() {
        assert_eq!(provider_fallback_icon("kiro"), IconName::PackagePlus);
        for id in ["not-a-provider", "", "Claude"] {
            assert!(provider_logo_svg(id).is_none(), "{id:?}");
            assert!(provider_logo(id, white()).is_none(), "{id:?}");
            assert_eq!(provider_fallback_icon(id), IconName::Bot, "{id:?}");
        }
    }

    #[test]
    fn the_same_provider_and_color_return_the_cached_image() {
        let first = provider_logo("codex", hsla(0.3, 0.4, 0.5, 1.)).unwrap();
        let second = provider_logo("codex", hsla(0.3, 0.4, 0.5, 1.)).unwrap();
        assert!(Arc::ptr_eq(&first, &second));

        let other_color = provider_logo("codex", hsla(0.3, 0.4, 0.6, 1.)).unwrap();
        assert!(!Arc::ptr_eq(&first, &other_color));
        let other_provider = provider_logo("pi", hsla(0.3, 0.4, 0.5, 1.)).unwrap();
        assert!(!Arc::ptr_eq(&first, &other_provider));
    }
}
