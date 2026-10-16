//! Electron design tokens for the `light` and `claude` dark themes (ADR-0043).
//! Every value is copied from `packages/app/src/styles/theme.ts`; the test in this file reads
//! that file and compares. A token change there needs the same change here.

use gpui_kit::{Hsla, Rgba};

#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Tokens {
    pub surface0: Hsla,
    pub surface1: Hsla,
    pub surface2: Hsla,
    pub surface3: Hsla,
    pub surface4: Hsla,
    pub surface_sidebar: Hsla,
    pub surface_sidebar_hover: Hsla,
    pub surface_sidebar_selected: Hsla,
    pub surface_workspace: Hsla,
    pub surface_diff_empty: Hsla,
    pub interaction_highlight: Hsla,

    pub foreground: Hsla,
    pub foreground_muted: Hsla,
    pub foreground_extra_muted: Hsla,

    pub border: Hsla,
    pub border_accent: Hsla,

    pub accent: Hsla,
    pub accent_bright: Hsla,
    pub accent_foreground: Hsla,
    pub destructive: Hsla,
    pub destructive_foreground: Hsla,
    pub ring: Hsla,
    pub popover: Hsla,

    pub status_success: Hsla,
    pub status_danger: Hsla,
    pub status_warning: Hsla,
    pub status_merged: Hsla,
    pub tint_success: Hsla,
    pub tint_danger: Hsla,
    pub tint_warning: Hsla,
    /// `theme.ts` has no merged tint. This is `status_merged` at the alpha of the other tints
    /// (the `statusTints` rule), so a merged badge follows the same recipe.
    pub tint_merged: Hsla,
    pub dot_success: Hsla,
    pub dot_danger: Hsla,
    pub dot_warning: Hsla,
    pub dot_running: Hsla,

    pub diff_addition: Hsla,
    pub diff_deletion: Hsla,

    pub terminal_background: Hsla,
    pub terminal_foreground: Hsla,
    pub terminal_cursor: Hsla,
    pub terminal_selection: Hsla,
    /// black, red, green, yellow, blue, magenta, cyan, white, then the eight bright colors in the
    /// same order.
    pub ansi: [Hsla; 16],
}

/// `0xRRGGBB`.
fn hex(rgb: u32) -> Hsla {
    hexa(rgb, 255)
}

/// `0xRRGGBB` with an alpha in 0 to 255, as the two hex digits `theme.ts` appends.
fn hexa(rgb: u32, alpha: u8) -> Hsla {
    Rgba {
        r: ((rgb >> 16) & 0xff) as f32 / 255.,
        g: ((rgb >> 8) & 0xff) as f32 / 255.,
        b: (rgb & 0xff) as f32 / 255.,
        a: alpha as f32 / 255.,
    }
    .into()
}

/// `rgba(r, g, b, alpha)` as `theme.ts` writes it.
fn channels(r: u8, g: u8, b: u8, alpha: f32) -> Hsla {
    Rgba {
        r: r as f32 / 255.,
        g: g as f32 / 255.,
        b: b as f32 / 255.,
        a: alpha,
    }
    .into()
}

impl Tokens {
    /// `lightSemanticColors`.
    pub fn light() -> Self {
        let surface0 = hex(0xffffff);
        let surface1 = hex(0xfafafa);
        let surface3 = hex(0xe4e4e7);
        let foreground = hex(0x1a1a1e);
        // statusTints: the status color at 0x1f (12%).
        let tint = |rgb| hexa(rgb, 0x1f);
        Tokens {
            surface0,
            surface1,
            surface2: hex(0xf4f4f5),
            surface3,
            surface4: hex(0xd4d4d8),
            surface_sidebar: hex(0xf4f4f5),
            surface_sidebar_hover: surface1,
            surface_sidebar_selected: surface3,
            surface_workspace: surface0,
            surface_diff_empty: hex(0xf6f6f6),
            interaction_highlight: channels(0, 0, 0, 0.06),

            foreground,
            foreground_muted: hex(0x71717a),
            foreground_extra_muted: hex(0xa1a1aa),

            border: hex(0xe4e4e7),
            border_accent: hex(0xececf1),

            accent: hex(0x20744a),
            accent_bright: hex(0x239956),
            accent_foreground: hex(0xffffff),
            destructive: hex(0xb04138),
            destructive_foreground: surface0,
            ring: hex(0x18181b),
            popover: surface0,

            status_success: hex(0x3e704a),
            status_danger: hex(0x9d433b),
            status_warning: hex(0x7b5d39),
            status_merged: hex(0x7347af),
            tint_success: tint(0x3e704a),
            tint_danger: tint(0x9d433b),
            tint_warning: tint(0x7b5d39),
            tint_merged: tint(0x7347af),
            dot_success: hex(0x299f51),
            dot_danger: hex(0xf12e2f),
            dot_warning: hex(0xb37824),
            dot_running: hex(0x268ae0),

            diff_addition: hex(0x15803d),
            diff_deletion: hex(0xb91c1c),

            terminal_background: surface0,
            terminal_foreground: foreground,
            terminal_cursor: foreground,
            terminal_selection: channels(0, 0, 0, 0.15),
            ansi: [
                hex(0x1a1a1e), // black: terminalBlack
                hex(0xdc2626),
                hex(0x16a34a),
                hex(0xca8a04),
                hex(0x2563eb),
                hex(0x9333ea),
                hex(0x0891b2),
                hex(0xffffff),
                hex(0x3f3f46), // bright black: terminalBrightBlack
                hex(0xef4444),
                hex(0x22c55e),
                hex(0xf59e0b),
                hex(0x3b82f6),
                hex(0xa855f7),
                hex(0x06b6d4),
                hex(0xfafafa),
            ],
        }
    }

    /// `claudeDarkColors`, built by `buildDarkSemanticColors`.
    pub fn claude_dark() -> Self {
        let surface0 = hex(0x1f1f1e);
        let surface1 = hex(0x262523);
        let surface2 = hex(0x2f2d2b);
        let foreground = hex(0xfafafa);
        // statusTints: the status color at 0x29 (16%).
        let tint = |rgb| hexa(rgb, 0x29);
        Tokens {
            surface0,
            surface1,
            surface2,
            surface3: hex(0x4a4745),
            surface4: hex(0x605d5b),
            surface_sidebar: hex(0x1a1918),
            surface_sidebar_hover: surface1,
            surface_sidebar_selected: surface2,
            surface_workspace: surface1,
            surface_diff_empty: hex(0x2a2826),
            interaction_highlight: channels(255, 255, 255, 0.08),

            foreground,
            foreground_muted: hex(0xada9a5),
            foreground_extra_muted: hex(0x78746f),

            border: hex(0x2c2a27),
            border_accent: hex(0x36332f),

            accent: hex(0xd97757),
            accent_bright: hex(0xe89a7f),
            accent_foreground: hex(0xffffff),
            destructive: hex(0xcf513e),
            destructive_foreground: hex(0xffffff),
            ring: hex(0xd4d4d8),
            popover: surface2,

            status_success: hex(0x6cb17b),
            status_danger: hex(0xd8847b),
            status_warning: hex(0xc09664),
            status_merged: hex(0xa890d5),
            tint_success: tint(0x6cb17b),
            tint_danger: tint(0xd8847b),
            tint_warning: tint(0xc09664),
            tint_merged: tint(0xa890d5),
            dot_success: hex(0x35c264),
            dot_danger: hex(0xf7796d),
            dot_warning: hex(0xdb932e),
            dot_running: hex(0x5caaf6),

            diff_addition: hex(0x4ade80),
            diff_deletion: hex(0xef4444),

            terminal_background: surface0,
            terminal_foreground: foreground,
            terminal_cursor: foreground,
            terminal_selection: channels(255, 255, 255, 0.2),
            ansi: [
                hex(0x1a1918), // black: terminalBlack
                hex(0xe07070),
                hex(0x5dba80),
                hex(0xd4a44a),
                hex(0x6a9de0),
                hex(0xb07ad0),
                hex(0x4aabb8),
                hex(0xd4d4d8),
                hex(0x4a4745), // bright black: terminalBrightBlack
                hex(0xe89090),
                hex(0x7ecf9a),
                hex(0xe0be6e),
                hex(0x8ab4e8),
                hex(0xc49ae0),
                hex(0x6ec2cc),
                hex(0xf0f0f2),
            ],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;

    /// The `"key": "value"` and `key: "value"` pairs of the object that starts at the line
    /// containing `marker` in `theme.ts`, up to its closing line.
    fn block(source: &str, marker: &str) -> HashMap<String, String> {
        let mut found = HashMap::new();
        let mut lines = source.lines();
        assert!(
            lines.any(|line| line.contains(marker)),
            "theme.ts has no `{marker}`"
        );
        for line in lines {
            let line = line.trim();
            if line.starts_with('}') {
                break;
            }
            let Some((key, rest)) = line.split_once(':') else {
                continue;
            };
            let Some(value) = rest.trim().strip_prefix('"') else {
                continue;
            };
            let value = value.split('"').next().unwrap_or_default();
            found.insert(key.trim().trim_matches('"').to_string(), value.to_string());
        }
        found
    }

    fn theme_ts() -> String {
        let path = concat!(
            env!("CARGO_MANIFEST_DIR"),
            "/../../../app/src/styles/theme.ts"
        );
        std::fs::read_to_string(path).unwrap_or_else(|error| panic!("{path}: {error}"))
    }

    /// `#rrggbb` or `#rrggbbaa` to channels in 0 to 255 and alpha in 0 to 1.
    fn parse(value: &str) -> ([f32; 3], f32) {
        let digits = value.strip_prefix('#').unwrap_or_else(|| panic!("{value}"));
        let byte = |at: usize| u8::from_str_radix(&digits[at..at + 2], 16).unwrap() as f32;
        let alpha = if digits.len() == 8 {
            byte(6) / 255.
        } else {
            1.
        };
        ([byte(0), byte(2), byte(4)], alpha)
    }

    /// 1 unit per RGB channel and 0.01 alpha.
    fn assert_token(actual: Hsla, expected: &str, what: &str) {
        let (want, want_alpha) = parse(expected);
        let rgba = actual.to_rgb();
        let got = [rgba.r, rgba.g, rgba.b].map(|channel| channel * 255.);
        for (got, want) in got.iter().zip(want) {
            assert!(
                (got - want).abs() <= 1.,
                "{what}: got {got:?}, want {expected}"
            );
        }
        assert!(
            (rgba.a - want_alpha).abs() <= 0.01,
            "{what}: alpha {}, want {expected}",
            rgba.a
        );
    }

    fn assert_rgba(actual: Hsla, r: f32, g: f32, b: f32, alpha: f32, what: &str) {
        let rgba = actual.to_rgb();
        for (got, want) in [rgba.r, rgba.g, rgba.b].iter().zip([r, g, b]) {
            assert!((got * 255. - want).abs() <= 1., "{what}: rgb {rgba:?}");
        }
        assert!((rgba.a - alpha).abs() <= 0.01, "{what}: alpha {}", rgba.a);
    }

    fn ansi_names() -> [&'static str; 14] {
        [
            "red",
            "green",
            "yellow",
            "blue",
            "magenta",
            "cyan",
            "white",
            "brightRed",
            "brightGreen",
            "brightYellow",
            "brightBlue",
            "brightMagenta",
            "brightCyan",
            "brightWhite",
        ]
    }

    /// Compares each token with the value in `theme.ts` for one theme. `semantic` holds the
    /// colors passed to the builder; the rest are the builder's own rules.
    #[allow(clippy::too_many_arguments)]
    fn check_theme(
        tokens: Tokens,
        semantic: HashMap<String, String>,
        diff: HashMap<String, String>,
        status: HashMap<String, String>,
        dots: HashMap<String, String>,
        ansi: HashMap<String, String>,
        dark: bool,
        tint_alpha: &str,
    ) {
        let get = |key: &str| semantic[key].as_str();
        let surface = |index: usize| semantic[&format!("surface{index}")].clone();
        for (index, token) in [
            tokens.surface0,
            tokens.surface1,
            tokens.surface2,
            tokens.surface3,
            tokens.surface4,
        ]
        .into_iter()
        .enumerate()
        {
            assert_token(token, &surface(index), &format!("surface{index}"));
        }
        assert_token(
            tokens.surface_sidebar,
            get("surfaceSidebar"),
            "surfaceSidebar",
        );
        assert_token(
            tokens.surface_diff_empty,
            get("surfaceDiffEmpty"),
            "surfaceDiffEmpty",
        );
        assert_token(
            tokens.surface_sidebar_hover,
            &surface(1),
            "surfaceSidebarHover",
        );
        let (selected, workspace, popover) = if dark {
            (surface(2), surface(1), surface(2))
        } else {
            (surface(3), surface(0), surface(0))
        };
        assert_token(
            tokens.surface_sidebar_selected,
            &selected,
            "surfaceSidebarSelected",
        );
        assert_token(tokens.surface_workspace, &workspace, "surfaceWorkspace");
        assert_token(tokens.popover, &popover, "popover");

        let (highlight, selection) = if dark {
            ((255., 0.08), (255., 0.2))
        } else {
            ((0., 0.06), (0., 0.15))
        };
        assert_rgba(
            tokens.interaction_highlight,
            highlight.0,
            highlight.0,
            highlight.0,
            highlight.1,
            "interactionHighlight",
        );
        assert_rgba(
            tokens.terminal_selection,
            selection.0,
            selection.0,
            selection.0,
            selection.1,
            "terminal selectionBackground",
        );

        let foreground = semantic.get("foreground").map_or("#fafafa", String::as_str);
        assert_token(tokens.foreground, foreground, "foreground");
        assert_token(
            tokens.terminal_foreground,
            foreground,
            "terminal foreground",
        );
        assert_token(tokens.terminal_cursor, foreground, "terminal cursor");
        assert_token(
            tokens.terminal_background,
            &surface(0),
            "terminal background",
        );
        assert_token(
            tokens.foreground_muted,
            get("foregroundMuted"),
            "foregroundMuted",
        );
        assert_token(
            tokens.foreground_extra_muted,
            get("foregroundExtraMuted"),
            "foregroundExtraMuted",
        );
        assert_token(tokens.border, get("border"), "border");
        assert_token(tokens.border_accent, get("borderAccent"), "borderAccent");
        assert_token(tokens.accent, get("accent"), "accent");
        assert_token(tokens.accent_bright, get("accentBright"), "accentBright");
        let accent_foreground = semantic
            .get("accentForeground")
            .cloned()
            .unwrap_or_else(|| if dark { "#ffffff".into() } else { surface(0) });
        assert_token(
            tokens.accent_foreground,
            &accent_foreground,
            "accentForeground",
        );
        assert_token(tokens.destructive, get("destructive"), "destructive");
        let destructive_foreground = if dark {
            "#ffffff".to_string()
        } else {
            surface(0)
        };
        assert_token(
            tokens.destructive_foreground,
            &destructive_foreground,
            "destructiveForeground",
        );
        let ring = semantic
            .get("ring")
            .map_or(if dark { "#d4d4d8" } else { "#18181b" }, String::as_str);
        assert_token(tokens.ring, ring, "ring");

        assert_token(tokens.diff_addition, &diff["diffAddition"], "diffAddition");
        assert_token(tokens.diff_deletion, &diff["diffDeletion"], "diffDeletion");
        for (token, key) in [
            (tokens.status_success, "statusSuccess"),
            (tokens.status_danger, "statusDanger"),
            (tokens.status_warning, "statusWarning"),
            (tokens.status_merged, "statusMerged"),
        ] {
            assert_token(token, &status[key], key);
        }
        for (token, key) in [
            (tokens.tint_success, "statusSuccess"),
            (tokens.tint_danger, "statusDanger"),
            (tokens.tint_warning, "statusWarning"),
            (tokens.tint_merged, "statusMerged"),
        ] {
            assert_token(
                token,
                &format!("{}{tint_alpha}", status[key]),
                &format!("{key} tint"),
            );
        }
        for (token, key) in [
            (tokens.dot_success, "statusDotSuccess"),
            (tokens.dot_danger, "statusDotDanger"),
            (tokens.dot_warning, "statusDotWarning"),
            (tokens.dot_running, "statusDotRunning"),
        ] {
            assert_token(token, &dots[key], key);
        }

        assert_token(tokens.ansi[0], get("terminalBlack"), "terminalBlack");
        assert_token(
            tokens.ansi[8],
            get("terminalBrightBlack"),
            "terminalBrightBlack",
        );
        let mut index = 1;
        for name in ansi_names() {
            if index == 8 {
                index += 1;
            }
            assert_token(tokens.ansi[index], &ansi[name], name);
            index += 1;
        }
    }

    #[test]
    fn light_tokens_match_theme_ts() {
        let source = theme_ts();
        check_theme(
            Tokens::light(),
            block(
                &source,
                "const lightSemanticColors = buildLightSemanticColors({",
            ),
            block(&source, "const lightDiffColors = {"),
            block(&source, "const lightStatusColors = {"),
            block(&source, "const lightStatusDotColors = {"),
            block(&source, "const lightTerminalAnsi = {"),
            false,
            "1f",
        );
    }

    #[test]
    fn claude_dark_tokens_match_theme_ts() {
        let source = theme_ts();
        check_theme(
            Tokens::claude_dark(),
            block(
                &source,
                "const claudeDarkColors = buildDarkSemanticColors({",
            ),
            block(&source, "const darkDiffColors = {"),
            block(&source, "const darkStatusColors = {"),
            block(&source, "const darkStatusDotColors = {"),
            block(&source, "const darkTerminalAnsi = {"),
            true,
            "29",
        );
    }

    #[test]
    fn tint_alphas_are_the_theme_ts_rule() {
        let source = theme_ts();
        assert!(source.contains("statusTints(lightStatusColors, \"1f\")"));
        assert!(source.contains("statusTints(darkStatusColors, \"29\")"));
    }
}
