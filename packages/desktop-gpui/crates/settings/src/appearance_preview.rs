//! The live preview under the Syntax section (`appearance-preview.tsx`): a line of content text
//! and a unified diff of a small TypeScript change, in the code font with the syntax colors on
//! screen. It reads the applied theme state, so a new font, size, or syntax theme shows at once.

use bachuc_theme::metrics::{FONT_CODE, RADIUS_LG, SPACE_2, SPACE_3};
use bachuc_theme::{
    SyntaxColors, code_font_family, code_font_size, content_font_size, syntax_colors, tokens,
};
use gpui_kit::component::v_flex;
use gpui_kit::*;

/// `diff-pane.tsx` tints: `removeLineContainer` and `addLineContainer`.
const REMOVED_TINT: (u8, u8, u8, f32) = (248, 81, 73, 0.1);
const ADDED_TINT: (u8, u8, u8, f32) = (46, 160, 67, 0.15);

type Pick = fn(&SyntaxColors) -> Hsla;
type Token = (&'static str, Option<Pick>);

#[derive(Clone, Copy, PartialEq)]
enum Kind {
    Context,
    Removed,
    Added,
}

const KEYWORD: Option<Pick> = Some(|c| c.keyword);
const COMMENT: Option<Pick> = Some(|c| c.comment);
const STRING: Option<Pick> = Some(|c| c.string);
const NUMBER: Option<Pick> = Some(|c| c.number);
const FUNCTION: Option<Pick> = Some(|c| c.function);
const TYPE: Option<Pick> = Some(|c| c.r#type);
const VARIABLE: Option<Pick> = Some(|c| c.variable);
const OPERATOR: Option<Pick> = Some(|c| c.operator);
const PUNCT: Option<Pick> = Some(|c| c.punctuation);

/// `PREVIEW_BEFORE` and `PREVIEW_AFTER` of `preview-snippet.ts`, interleaved as the preview
/// shows them: a changed line is a removed row, then an added row.
const ROWS: [(Kind, &[Token]); 7] = [
    (Kind::Context, &[("// Format a price for display", COMMENT)]),
    (
        Kind::Removed,
        &[
            ("export", KEYWORD),
            (" ", None),
            ("function", KEYWORD),
            (" ", None),
            ("formatPrice", FUNCTION),
            ("(", PUNCT),
            ("cents", VARIABLE),
            (":", PUNCT),
            (" ", None),
            ("number", TYPE),
            (")", PUNCT),
            (" ", None),
            ("{", PUNCT),
        ],
    ),
    (
        Kind::Added,
        &[
            ("export", KEYWORD),
            (" ", None),
            ("function", KEYWORD),
            (" ", None),
            ("formatPrice", FUNCTION),
            ("(", PUNCT),
            ("cents", VARIABLE),
            (":", PUNCT),
            (" ", None),
            ("number", TYPE),
            (")", PUNCT),
            (":", PUNCT),
            (" ", None),
            ("string", TYPE),
            (" ", None),
            ("{", PUNCT),
        ],
    ),
    (
        Kind::Context,
        &[
            ("  ", None),
            ("const", KEYWORD),
            (" ", None),
            ("amount", VARIABLE),
            (" ", None),
            ("=", OPERATOR),
            (" ", None),
            ("cents", VARIABLE),
            (" ", None),
            ("/", OPERATOR),
            (" ", None),
            ("100", NUMBER),
            (";", PUNCT),
        ],
    ),
    (
        Kind::Removed,
        &[
            ("  ", None),
            ("return", KEYWORD),
            (" ", None),
            ("\"$\"", STRING),
            (" ", None),
            ("+", OPERATOR),
            (" ", None),
            ("amount", VARIABLE),
            (";", PUNCT),
        ],
    ),
    (
        Kind::Added,
        &[
            ("  ", None),
            ("return", KEYWORD),
            (" ", None),
            ("`$", STRING),
            ("${", PUNCT),
            ("amount", VARIABLE),
            (".", PUNCT),
            ("toFixed", FUNCTION),
            ("(", PUNCT),
            ("2", NUMBER),
            (")", PUNCT),
            ("}", PUNCT),
            ("`", STRING),
            (";", PUNCT),
        ],
    ),
    (Kind::Context, &[("}", PUNCT)]),
];

fn tint((r, g, b, a): (u8, u8, u8, f32)) -> Hsla {
    Rgba {
        r: r as f32 / 255.,
        g: g as f32 / 255.,
        b: b as f32 / 255.,
        a,
    }
    .into()
}

/// One row: the marker (`  `, `- `, or `+ `) and the tokens, each token in its role color.
fn code_row(kind: Kind, tokens_of_line: &[Token], colors: &SyntaxColors, cx: &App) -> Div {
    let ui = tokens(cx);
    let (marker, marker_color, background) = match kind {
        Kind::Context => ("  ", ui.foreground_muted, None),
        Kind::Removed => ("- ", ui.diff_deletion, Some(tint(REMOVED_TINT))),
        Kind::Added => ("+ ", ui.diff_addition, Some(tint(ADDED_TINT))),
    };
    let mut text = String::from(marker);
    let mut runs = vec![(0..marker.len(), color_style(marker_color))];
    for (piece, pick) in tokens_of_line {
        let start = text.len();
        text.push_str(piece);
        if let Some(pick) = pick {
            runs.push((start..text.len(), color_style(pick(colors))));
        }
    }
    let mut row = div().px(px(SPACE_3)).whitespace_nowrap();
    if let Some(background) = background {
        row = row.bg(background);
    }
    row.child(StyledText::new(SharedString::from(text)).with_highlights(runs))
}

fn color_style(color: Hsla) -> HighlightStyle {
    HighlightStyle {
        color: Some(color),
        ..Default::default()
    }
}

/// The preview card. `label` is the accessible name.
pub fn preview(label: &'static str, sample: &'static str, cx: &App) -> Stateful<Div> {
    let ui = tokens(cx);
    let colors = syntax_colors(cx);
    let content = content_font_size(cx);
    let code = code_font_size(FONT_CODE, cx);
    v_flex()
        .id("appearance-preview")
        .role(Role::Image)
        .aria_label(label)
        .w_full()
        .py(px(SPACE_2))
        .bg(ui.surface1)
        .rounded(px(RADIUS_LG))
        .border_1()
        .border_color(ui.border)
        .overflow_hidden()
        .text_color(ui.foreground)
        .child(
            div()
                .px(px(SPACE_3))
                .pb(px(SPACE_2))
                .text_size(content)
                .line_height((content * 1.4).round())
                .child(sample),
        )
        .child(
            v_flex()
                .font_family(code_font_family(cx))
                .text_size(code)
                .line_height((code * 1.5).round())
                .children(
                    ROWS.iter()
                        .map(|(kind, line)| code_row(*kind, line, &colors, cx)),
                ),
        )
}
