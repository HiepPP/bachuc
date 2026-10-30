//! Turns a Pele SVG into a PNG: replaces its CSS `var()` colors with theme colors, then rasterizes
//! it with resvg (ADR-0067). usvg does not read custom properties, so the `var()` step is ours.

use std::sync::{Arc, LazyLock};

use bachuc_theme::Tokens;
use resvg::tiny_skia::{Pixmap, Transform};
use resvg::usvg::{self, fontdb};

/// Pele's theme properties as CSS text, one string per `--pele-*` property.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PeleTheme {
    pub bg: String,
    pub fg: String,
    pub muted: String,
    pub line: String,
    pub surface: String,
    pub surface_alt: String,
    pub border: String,
    pub accent: String,
    pub font: String,
    pub font_mono: String,
    pub radius: String,
    pub series: [String; 8],
    /// PNG pixels per SVG pixel. It holds the 2x snapshot scale and the content zoom.
    pub scale: f32,
}

fn css_color(color: gpui_kit::Hsla) -> String {
    let rgba = gpui_kit::Rgba::from(color);
    let channel = |value: f32| (value * 255.0).round() as u8;
    let alpha = (rgba.a * 1000.0).round() / 1000.0;
    let (r, g, b) = (channel(rgba.r), channel(rgba.g), channel(rgba.b));
    if alpha >= 1.0 {
        format!("#{r:02x}{g:02x}{b:02x}")
    } else {
        format!("rgba({r},{g},{b},{alpha})")
    }
}

/// Pele lays text out at 16px. The diagram scales so that 16px shows at the content font size.
const PELE_FONT_SIZE: f32 = 16.0;

pub(crate) fn pele_theme(tokens: &Tokens, content_size: f32) -> PeleTheme {
    let ansi = |index: usize| css_color(tokens.ansi[index]);
    PeleTheme {
        bg: css_color(tokens.surface0),
        fg: css_color(tokens.foreground),
        muted: css_color(tokens.foreground_muted),
        line: css_color(tokens.foreground_muted),
        surface: css_color(tokens.surface1),
        surface_alt: css_color(tokens.surface2),
        border: css_color(tokens.border),
        accent: css_color(tokens.accent),
        // Pele has no canvas on QuickJS, so it measures text with Helvetica widths. A wider font,
        // such as the UI font, would overflow the node and label boxes.
        font: "Helvetica Neue, Helvetica, Arial, sans-serif".into(),
        font_mono: "Menlo, monospace".into(),
        radius: "6px".into(),
        series: [4, 2, 3, 1, 5, 6, 12, 10].map(ansi),
        scale: crate::SCALE * content_size / PELE_FONT_SIZE,
    }
}

impl PeleTheme {
    fn property(&self, name: &str) -> Option<&str> {
        let value = match name {
            "--pele-bg" => &self.bg,
            "--pele-fg" => &self.fg,
            "--pele-muted" => &self.muted,
            "--pele-line" => &self.line,
            "--pele-surface" => &self.surface,
            "--pele-surface-alt" => &self.surface_alt,
            "--pele-border" => &self.border,
            "--pele-accent" => &self.accent,
            "--pele-font" => &self.font,
            "--pele-font-mono" => &self.font_mono,
            "--pele-radius" => &self.radius,
            _ => {
                let number: usize = name.strip_prefix("--pele-series-")?.parse().ok()?;
                self.series.get(number.checked_sub(1)?)?
            }
        };
        Some(value)
    }
}

/// Private values (`--_x:...`) that Pele sets in the style of its root element.
fn private_definitions(svg: &str) -> Vec<(&str, &str)> {
    let mut found = Vec::new();
    let mut rest = svg;
    while let Some(at) = rest.find("--_") {
        rest = &rest[at..];
        let name_end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(rest.len());
        let (name, after) = rest.split_at(name_end);
        if let Some(value) = after.strip_prefix(':') {
            let value_end = value.find([';', '"', '\'']).unwrap_or(value.len());
            found.push((name, &value[..value_end]));
        }
        rest = after;
    }
    found
}

const MAX_DEPTH: usize = 8;

fn resolve_text(
    text: &str,
    theme: &PeleTheme,
    definitions: &[(&str, &str)],
    depth: usize,
) -> String {
    let mut text = text.to_string();
    // The last `var(` holds no other `var(`, so each pass resolves an innermost call.
    while let Some(start) = text.rfind("var(") {
        let Some(length) = text[start..].find(')') else {
            break;
        };
        let end = start + length;
        let arguments = &text[start + 4..end];
        let (name, fallback) = match arguments.split_once(',') {
            Some((name, fallback)) => (name.trim(), Some(fallback.trim())),
            None => (arguments.trim(), None),
        };
        let value = if depth >= MAX_DEPTH {
            theme.fg.clone()
        } else if let Some(value) = theme.property(name) {
            value.to_string()
        } else if let Some((_, value)) = definitions.iter().find(|(key, _)| *key == name) {
            resolve_text(value, theme, definitions, depth + 1)
        } else if let Some(fallback) = fallback {
            resolve_text(fallback, theme, definitions, depth + 1)
        } else {
            theme.fg.clone()
        };
        text.replace_range(start..=end, &value);
    }
    text
}

/// Replaces every `var()` in a Pele SVG with a theme value, a private value, or the fallback.
/// A name with no value and no fallback becomes the theme foreground.
pub(crate) fn resolve_vars(svg: &str, theme: &PeleTheme) -> String {
    let definitions = private_definitions(svg);
    with_radius_attributes(&without_private_definitions(&resolve_text(
        svg,
        theme,
        &definitions,
        0,
    )))
}

/// Turns Pele's `style="rx:6px"` into `rx` and `ry` attributes. usvg reads the corner radius only
/// from attributes, so the node boxes lose their round corners otherwise.
fn with_radius_attributes(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = rest.find("style=\"rx:") {
        out.push_str(&rest[..at]);
        let value = &rest[at + "style=\"rx:".len()..];
        let Some(end) = value.find('"') else {
            out.push_str(&rest[at..]);
            return out;
        };
        let radius = value[..end].trim().trim_end_matches("px");
        out.push_str(&format!("rx=\"{radius}\" ry=\"{radius}\""));
        rest = &value[end + 1..];
    }
    out.push_str(rest);
    out
}

/// Drops the `--_x:value;` declarations. usvg stops reading a `style` attribute at the first one,
/// so the `font-family` that follows them would be lost.
fn without_private_definitions(svg: &str) -> String {
    let mut out = String::with_capacity(svg.len());
    let mut rest = svg;
    while let Some(at) = rest.find("--_") {
        out.push_str(&rest[..at]);
        rest = &rest[at..];
        let name_end = rest
            .find(|c: char| !(c.is_ascii_alphanumeric() || c == '-' || c == '_'))
            .unwrap_or(rest.len());
        if rest[name_end..].starts_with(':') {
            let value_end = rest.find([';', '"', '\'']).unwrap_or(rest.len());
            rest = rest[value_end..]
                .strip_prefix(';')
                .unwrap_or(&rest[value_end..]);
        } else {
            out.push_str(&rest[..name_end]);
            rest = &rest[name_end..];
        }
    }
    out.push_str(rest);
    out
}

static FONTS: LazyLock<Arc<fontdb::Database>> = LazyLock::new(|| {
    let mut db = fontdb::Database::new();
    db.load_system_fonts();
    db.set_sans_serif_family("Helvetica Neue");
    Arc::new(db)
});

/// Loads the system fonts now. The engine thread calls this once, so the first render is not slow.
pub(crate) fn load_fonts() {
    LazyLock::force(&FONTS);
}

/// Rasterizes an SVG with no `var()` left to a PNG at `scale` times the SVG size.
pub(crate) fn rasterize(svg: &str, scale: f32) -> Result<Vec<u8>, String> {
    render_tree(&parse(svg)?, scale)
}

/// Rasterizes an SVG with no `var()` left to a PNG `width` pixels wide.
pub(crate) fn rasterize_to_width(svg: &str, width: u32) -> Result<Vec<u8>, String> {
    let tree = parse(svg)?;
    let scale = width as f32 / tree.size().width();
    render_tree(&tree, scale)
}

fn parse(svg: &str) -> Result<usvg::Tree, String> {
    let options = usvg::Options {
        fontdb: FONTS.clone(),
        ..Default::default()
    };
    usvg::Tree::from_str(svg, &options).map_err(|error| error.to_string())
}

fn render_tree(tree: &usvg::Tree, scale: f32) -> Result<Vec<u8>, String> {
    let width = (tree.size().width() * scale).ceil() as u32;
    let height = (tree.size().height() * scale).ceil() as u32;
    let mut pixmap =
        Pixmap::new(width, height).ok_or_else(|| "the diagram has no size".to_string())?;
    resvg::render(
        tree,
        Transform::from_scale(scale, scale),
        &mut pixmap.as_mut(),
    );
    pixmap.encode_png().map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn theme() -> PeleTheme {
        let mut theme = pele_theme(&bachuc_theme::Tokens::claude_dark(), 16.0);
        theme.fg = "#e6e6e6".into();
        theme.line = "#808080".into();
        theme
    }

    #[test]
    fn private_vars_resolve_through_the_theme() {
        let svg = r#"<svg style="--_fg:var(--pele-fg,#1f1f1f);--_l:var(--pele-line,#1f1f1f);color:var(--_fg)" fill="var(--_fg)"><path stroke="var(--_l)"/></svg>"#;
        let out = resolve_vars(svg, &theme());
        assert!(!out.contains("var("), "{out}");
        assert!(out.contains(r##"fill="#e6e6e6""##), "{out}");
        assert!(out.contains(r##"stroke="#808080""##), "{out}");
    }

    #[test]
    fn unknown_names_use_their_fallback_or_the_fg() {
        let out = resolve_vars(
            r#"<svg><path fill="var(--pele-unknown,#123456)"/><path fill="var(--_nope)"/></svg>"#,
            &theme(),
        );
        assert!(out.contains(r##"fill="#123456""##), "{out}");
        assert!(out.contains(r##"fill="#e6e6e6""##), "{out}");
        assert!(!out.contains("var("), "{out}");
    }

    #[test]
    fn rasterize_returns_a_png_at_twice_the_size() {
        let png = rasterize(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="10" height="5"><rect width="10" height="5" fill="#ff0000"/></svg>"##,
            2.0,
        )
        .expect("png");
        let read = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
        assert_eq!((read(16), read(20)), (20, 10));
    }

    #[test]
    fn rasterize_to_width_scales_the_svg_to_that_width() {
        let png = rasterize_to_width(
            r##"<svg xmlns="http://www.w3.org/2000/svg" width="200" height="50"><rect width="200" height="50" fill="#ff0000"/></svg>"##,
            50,
        )
        .expect("png");
        let read = |at: usize| u32::from_be_bytes(png[at..at + 4].try_into().unwrap());
        assert_eq!((read(16), read(20)), (50, 13));
    }

    #[test]
    fn colors_with_alpha_resolve_to_rgba_that_usvg_reads() {
        let mut theme = theme();
        theme.surface = "rgba(255,0,0,0.5)".into();
        let out = resolve_vars(
            r#"<svg xmlns="http://www.w3.org/2000/svg" width="4" height="4" style="--_s:var(--pele-surface,#f3f3f3)"><rect width="4" height="4" fill="var(--_s)"/></svg>"#,
            &theme,
        );
        assert!(out.contains("rgba(255,0,0,0.5)"), "{out}");
        assert!(rasterize(&out, 2.0).is_ok());
    }

    #[test]
    fn node_radius_becomes_attributes_and_the_scale_follows_the_content_size() {
        let out = resolve_vars(
            r#"<svg style="--_r:var(--pele-radius,4px);color:red"><rect width="4" height="4" style="rx:var(--_r)"/></svg>"#,
            &theme(),
        );
        assert!(out.contains(r#"rx="6" ry="6""#), "{out}");
        assert!(!out.contains("style=\"rx"), "{out}");
        assert_eq!(pele_theme(&bachuc_theme::Tokens::light(), 20.0).scale, 2.5);
    }
}
