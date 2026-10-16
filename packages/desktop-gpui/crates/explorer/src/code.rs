//! What the file viewer and the diff viewer share: the code font, line text with colored runs,
//! and the gutter width.

use crate::syntax::{Run, expand_tabs};
use bachuc_theme::metrics::FONT_CODE;
use bachuc_theme::{code_font_size, mono_font_family};
use gpui_kit::*;

/// Lines longer than this many bytes show their start and an ellipsis, because shaping one huge
/// line (a minified bundle) would freeze the window.
pub const MAX_LINE_BYTES: usize = 20_000;

/// The line text a row draws: carriage return removed, long lines cut, tabs expanded, and the runs
/// moved with the text.
pub fn display_line(line: &str, runs: &[Run]) -> (String, Vec<Run>) {
    let line = line.strip_suffix('\r').unwrap_or(line);
    let (line, cut) = if line.len() > MAX_LINE_BYTES {
        let mut end = MAX_LINE_BYTES;
        while !line.is_char_boundary(end) {
            end -= 1;
        }
        (&line[..end], true)
    } else {
        (line, false)
    };
    // A run may reach past the text: over the carriage return, or beyond the cut.
    let runs: Vec<Run> = runs
        .iter()
        .filter(|(range, _)| range.start < line.len())
        .map(|(range, style)| (range.start..range.end.min(line.len()), *style))
        .collect();
    let (mut text, runs) = expand_tabs(line, &runs);
    if cut {
        text.push('…');
    }
    (text, runs)
}

/// A line of code as styled text. The parent sets the font; the element does not wrap.
pub fn code_text(line: &str, runs: &[Run]) -> StyledText {
    let (text, runs) = display_line(line, runs);
    StyledText::new(SharedString::from(text)).with_highlights(runs)
}

/// The size and family of code, with the Appearance setting applied.
pub fn code_font(cx: &App) -> (Pixels, &'static str) {
    (code_font_size(FONT_CODE, cx), mono_font_family(cx))
}

/// The width of one digit in the code font, as the gutter computes it (`ceil(size * 0.62)`).
pub fn digit_width(size: f32) -> f32 {
    (size * 0.62).ceil()
}

/// `lineNumberGutterWidth`: room for the widest number, at least two digits, plus 12 px.
pub fn gutter_width(max_number: u32, size: f32) -> f32 {
    let digits = max_number.to_string().len().max(2);
    digits as f32 * digit_width(size) + 12.
}
