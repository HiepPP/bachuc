//! The terminal emulator: `alacritty_terminal` plus a cache of drawn lines. It knows nothing
//! about the daemon or the window, so tests drive it with plain bytes.

use crate::snapshot::Snapshot;
use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::{Dimensions, Scroll};
use alacritty_terminal::index::{Column, Line, Point, Side};
use alacritty_terminal::selection::{Selection, SelectionType};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term, TermDamage, TermMode};
use alacritty_terminal::vte::ansi::{Color, CursorShape, NamedColor, Processor, Rgb};
use bachuc_theme::Tokens;
use gpui_kit::{
    FontStyle, FontWeight, HighlightStyle, Hsla, Rgba, SharedString, StrikethroughStyle,
    UnderlineStyle, px, rgb,
};
use serde_json::Value;
use std::cell::{Cell, RefCell};
use std::ops::Range;
use std::rc::Rc;

/// Largest grid a snapshot may ask for. A bad value must not allocate a huge grid.
const MAX_DIMENSION: usize = 1000;

/// The terminal colors: the 16 ANSI colors plus the default background, foreground, cursor, and
/// selection. The view takes them from the theme tokens.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Palette {
    pub ansi: [Hsla; 16],
    pub background: Hsla,
    pub foreground: Hsla,
    pub cursor: Hsla,
    pub selection: Hsla,
}

impl Palette {
    pub fn from_tokens(tokens: &Tokens) -> Self {
        Self {
            ansi: tokens.ansi,
            background: tokens.terminal_background,
            foreground: tokens.terminal_foreground,
            cursor: tokens.terminal_cursor,
            selection: tokens.terminal_selection,
        }
    }
}

impl Default for Palette {
    fn default() -> Self {
        Self::from_tokens(&Tokens::claude_dark())
    }
}

fn channels(color: Hsla) -> (u8, u8, u8) {
    let rgba = Rgba::from(color);
    let byte = |value: f32| (value.clamp(0., 1.) * 255.).round() as u8;
    (byte(rgba.r), byte(rgba.g), byte(rgba.b))
}

struct Size {
    rows: usize,
    cols: usize,
}

impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        self.rows
    }
    fn screen_lines(&self) -> usize {
        self.rows
    }
    fn columns(&self) -> usize {
        self.cols
    }
}

/// Collects what the emulator writes back to the PTY: cursor position reports, color queries,
/// and similar.
#[derive(Clone, Default)]
struct Replies {
    bytes: Rc<RefCell<Vec<u8>>>,
    palette: Rc<Cell<Palette>>,
}

impl EventListener for Replies {
    fn send_event(&self, event: Event) {
        match event {
            Event::PtyWrite(text) => self.bytes.borrow_mut().extend_from_slice(text.as_bytes()),
            Event::ColorRequest(index, format) => {
                let (r, g, b) = palette_color(&self.palette.get(), index);
                let text = format(Rgb { r, g, b });
                self.bytes.borrow_mut().extend_from_slice(text.as_bytes());
            }
            _ => {}
        }
    }
}

/// One drawn screen line: its text and the styled spans over it.
#[derive(Clone, Debug)]
pub struct RenderedLine {
    pub text: SharedString,
    pub runs: Vec<(Range<usize>, HighlightStyle)>,
}

pub struct Emulator {
    term: Term<Replies>,
    parser: Processor,
    replies: Replies,
    lines: Vec<Option<RenderedLine>>,
}

impl Emulator {
    pub fn new(rows: usize, cols: usize) -> Self {
        let replies = Replies::default();
        let (rows, cols) = (rows.max(1), cols.max(1));
        Self {
            term: Term::new(Config::default(), &Size { rows, cols }, replies.clone()),
            parser: Processor::new(),
            replies,
            lines: vec![None; rows],
        }
    }

    pub fn rows(&self) -> usize {
        self.term.screen_lines()
    }

    pub fn cols(&self) -> usize {
        self.term.columns()
    }

    /// Lines kept above the screen.
    pub fn history_size(&self) -> usize {
        self.term.history_size()
    }

    /// How many lines the view is scrolled up from the bottom. 0 means at the bottom.
    pub fn display_offset(&self) -> usize {
        self.term.grid().display_offset()
    }

    pub fn mode(&self) -> TermMode {
        *self.term.mode()
    }

    /// Throws the screen and its history away and starts again at `rows` x `cols`.
    pub fn reset(&mut self, rows: usize, cols: usize) {
        let palette = self.palette();
        *self = Self::new(rows, cols);
        self.set_palette(palette);
    }

    pub fn palette(&self) -> Palette {
        self.replies.palette.get()
    }

    /// Recolors the screen. Cells keep their ANSI indexes, so old output changes color too.
    pub fn set_palette(&mut self, palette: Palette) {
        if self.palette() != palette {
            self.replies.palette.set(palette);
            self.invalidate_all();
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    /// Bytes the emulator wants to send to the PTY since the last call.
    pub fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut *self.replies.bytes.borrow_mut())
    }

    /// Replaces everything with the daemon's snapshot: the emulator takes the snapshot's size,
    /// then the history and the screen are replayed as text. The replay's replies are dropped,
    /// because the shell is not waiting for them.
    pub fn apply_snapshot(&mut self, value: &Value) -> Result<(), serde_json::Error> {
        let snapshot = Snapshot::from_value(value)?;
        self.reset(
            snapshot.rows.clamp(1, MAX_DIMENSION),
            snapshot.cols.clamp(1, MAX_DIMENSION),
        );
        self.feed(snapshot.to_ansi().as_bytes());
        self.take_replies();
        Ok(())
    }

    pub fn resize(&mut self, rows: usize, cols: usize) {
        let (rows, cols) = (rows.max(1), cols.max(1));
        if rows == self.rows() && cols == self.cols() {
            return;
        }
        self.term.resize(Size { rows, cols });
        self.invalidate_all();
    }

    /// Scrolls the view by `lines`; positive goes back into the history.
    pub fn scroll(&mut self, lines: i32) {
        self.term.scroll_display(Scroll::Delta(lines));
    }

    pub fn scroll_to_bottom(&mut self) {
        self.term.scroll_display(Scroll::Bottom);
    }

    fn point(&self, row: usize, col: usize) -> Point {
        let offset = self.display_offset() as i32;
        Point::new(Line(row as i32 - offset), Column(col))
    }

    /// Starts a selection at a screen cell. One click selects by cell, two by word, three by
    /// line. `right` tells whether the pointer is in the right half of the cell.
    pub fn start_selection(&mut self, row: usize, col: usize, right: bool, clicks: usize) {
        let kind = match clicks {
            2 => SelectionType::Semantic,
            3.. => SelectionType::Lines,
            _ => SelectionType::Simple,
        };
        let point = self.point(row, col);
        self.term.selection = Some(Selection::new(kind, point, side(right)));
        self.invalidate_all();
    }

    pub fn update_selection(&mut self, row: usize, col: usize, right: bool) {
        let point = self.point(row, col);
        if let Some(selection) = self.term.selection.as_mut() {
            selection.update(point, side(right));
            self.invalidate_all();
        }
    }

    /// Drops a selection that covers nothing, as a plain click leaves.
    pub fn finish_selection(&mut self) {
        if self
            .term
            .selection
            .as_ref()
            .is_some_and(|selection| selection.is_empty())
        {
            self.clear_selection();
        }
    }

    pub fn clear_selection(&mut self) {
        if self.term.selection.take().is_some() {
            self.invalidate_all();
        }
    }

    pub fn has_selection(&self) -> bool {
        self.term.selection.is_some()
    }

    pub fn selection_text(&self) -> Option<String> {
        self.term
            .selection_to_string()
            .filter(|text| !text.is_empty())
    }

    /// The text of a screen row without trailing spaces.
    pub fn row_text(&self, row: usize) -> String {
        let line = Line(row as i32 - self.display_offset() as i32);
        let mut text = String::new();
        for col in 0..self.cols() {
            let cell = &self.term.grid()[Point::new(line, Column(col))];
            if !cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                text.push(if cell.c == '\0' { ' ' } else { cell.c });
            }
        }
        text.truncate(text.trim_end().len());
        text
    }

    fn invalidate_all(&mut self) {
        self.lines.clear();
    }

    /// The screen lines for drawing. Only lines the emulator reports as changed are built again.
    pub fn lines(&mut self) -> impl Iterator<Item = &RenderedLine> {
        let rows = self.rows();
        if self.lines.len() != rows {
            self.lines = vec![None; rows];
        }
        match self.term.damage() {
            TermDamage::Full => self.lines.iter_mut().for_each(|line| *line = None),
            TermDamage::Partial(damaged) => {
                for bounds in damaged {
                    if let Some(line) = self.lines.get_mut(bounds.line) {
                        *line = None;
                    }
                }
            }
        }
        self.term.reset_damage();

        if self.lines.iter().any(Option::is_none) {
            let selection = self
                .term
                .selection
                .as_ref()
                .and_then(|selection| selection.to_range(&self.term));
            let cursor = self.term.renderable_content().cursor;
            let offset = self.display_offset();
            let palette = self.palette();
            let cursor_row = cursor.point.line.0 + offset as i32;
            for row in 0..rows {
                if self.lines[row].is_none() {
                    let cursor_here = (cursor_row == row as i32
                        && cursor.shape != CursorShape::Hidden)
                        .then_some((cursor.point.column.0, cursor.shape));
                    let line =
                        build_line(&self.term, &palette, row, offset, selection, cursor_here);
                    self.lines[row] = Some(line);
                }
            }
        }
        self.lines.iter().flatten()
    }
}

fn side(right: bool) -> Side {
    if right { Side::Right } else { Side::Left }
}

fn build_line(
    term: &Term<Replies>,
    palette: &Palette,
    row: usize,
    offset: usize,
    selection: Option<alacritty_terminal::selection::SelectionRange>,
    cursor: Option<(usize, CursorShape)>,
) -> RenderedLine {
    let line = Line(row as i32 - offset as i32);
    let cols = term.columns();
    let mut text = String::with_capacity(cols);
    let mut runs: Vec<(Range<usize>, HighlightStyle)> = Vec::new();
    for col in 0..cols {
        let point = Point::new(line, Column(col));
        let cell = &term.grid()[point];
        if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
            continue;
        }
        let start = text.len();
        if cell.flags.contains(Flags::HIDDEN) || cell.c == '\0' {
            text.push(' ');
        } else {
            text.push(cell.c);
            for extra in cell.zerowidth().into_iter().flatten() {
                text.push(*extra);
            }
        }

        let mut foreground = resolve(palette, cell.fg, true);
        let mut background = resolve(palette, cell.bg, false);
        if cell.flags.contains(Flags::INVERSE) {
            let (fg, bg) = (background, foreground);
            foreground = Some(fg.unwrap_or(palette.background));
            background = Some(bg.unwrap_or(palette.foreground));
        }
        let mut style = HighlightStyle {
            color: foreground,
            background_color: background,
            ..Default::default()
        };
        if cell.flags.contains(Flags::BOLD) {
            style.font_weight = Some(FontWeight::BOLD);
        }
        if cell.flags.contains(Flags::ITALIC) {
            style.font_style = Some(FontStyle::Italic);
        }
        if cell.flags.contains(Flags::DIM) {
            style.fade_out = Some(0.4);
        }
        if cell.flags.intersects(Flags::ALL_UNDERLINES) {
            style.underline = Some(underline());
        }
        if cell.flags.contains(Flags::STRIKEOUT) {
            style.strikethrough = Some(StrikethroughStyle {
                thickness: px(1.),
                color: None,
            });
        }
        if selection.is_some_and(|range| range.contains(point)) {
            style.background_color = Some(palette.selection);
        }
        if let Some((cursor_col, shape)) = cursor
            && cursor_col == col
        {
            if shape == CursorShape::Underline {
                style.underline = Some(underline());
            } else {
                style.background_color = Some(palette.cursor);
                style.color = Some(palette.background);
            }
        }

        if style == HighlightStyle::default() {
            continue;
        }
        match runs.last_mut() {
            Some((range, last)) if range.end == start && *last == style => {
                range.end = text.len();
            }
            _ => runs.push((start..text.len(), style)),
        }
    }
    RenderedLine {
        text: text.into(),
        runs,
    }
}

fn underline() -> UnderlineStyle {
    UnderlineStyle {
        thickness: px(1.),
        color: None,
        wavy: false,
    }
}

fn from_bytes((r, g, b): (u8, u8, u8)) -> Hsla {
    rgb(((r as u32) << 16) | ((g as u32) << 8) | b as u32).into()
}

/// A cell color. `None` means the default foreground or background.
fn resolve(palette: &Palette, color: Color, foreground: bool) -> Option<Hsla> {
    match color {
        Color::Spec(Rgb { r, g, b }) => Some(from_bytes((r, g, b))),
        Color::Named(NamedColor::Foreground) if foreground => None,
        Color::Named(NamedColor::Background) if !foreground => None,
        Color::Named(name) if (name as usize) < 16 => Some(palette.ansi[name as usize]),
        Color::Named(_) => None,
        Color::Indexed(index @ 0..=15) => Some(palette.ansi[index as usize]),
        Color::Indexed(index) => Some(from_bytes(indexed(index))),
    }
}

/// The RGB value for a color query (OSC 4, 10, 11).
fn palette_color(palette: &Palette, index: usize) -> (u8, u8, u8) {
    match index {
        0..=15 => channels(palette.ansi[index]),
        16..=255 => indexed(index as u8),
        257 => channels(palette.background),
        _ => channels(palette.foreground),
    }
}

/// The 6x6x6 color cube and the gray ramp of the 256-color palette.
fn indexed(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => (0, 0, 0),
        16..=231 => {
            let i = index - 16;
            let level = |v: u8| if v == 0 { 0 } else { 55 + v * 40 };
            (level(i / 36), level((i / 6) % 6), level(i % 6))
        }
        _ => {
            let v = 8 + (index - 232) * 10;
            (v, v, v)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn screen(emulator: &Emulator) -> Vec<String> {
        (0..emulator.rows())
            .map(|row| emulator.row_text(row))
            .collect()
    }

    #[test]
    fn output_draws_on_the_screen() {
        let mut emulator = Emulator::new(3, 20);
        emulator.feed(b"hello\r\nworld");
        assert_eq!(screen(&emulator), ["hello", "world", ""]);
    }

    #[test]
    fn color_spans_merge_into_one_run_per_style() {
        let mut emulator = Emulator::new(2, 20);
        // The last escape moves the cursor off the line, so it adds no run.
        emulator.feed(b"\x1b[31mred\x1b[0m plain \x1b[32mgreen\x1b[0m\x1b[2;1H");
        let line = emulator.lines().next().unwrap().clone();
        let spans: Vec<_> = line.runs.iter().map(|(range, _)| range.clone()).collect();
        assert_eq!(spans, [0..3, 10..15]);
        assert_eq!(&line.text[..15], "red plain green");
    }

    #[test]
    fn a_repeated_read_reuses_unchanged_lines_and_rebuilds_changed_ones() {
        let mut emulator = Emulator::new(3, 10);
        emulator.feed(b"one\r\ntwo");
        let first: Vec<_> = emulator.lines().cloned().collect();
        // Lines 0 and 2 are unchanged, so they come from the cache with the same text.
        let again: Vec<_> = emulator.lines().cloned().collect();
        assert_eq!(first[0].text, again[0].text);
        emulator.feed(b"\rTWO");
        let changed: Vec<_> = emulator.lines().cloned().collect();
        assert!(changed[1].text.starts_with("TWO"));
        assert!(changed[0].text.starts_with("one"));
    }

    #[test]
    fn scrollback_keeps_10000_lines() {
        let mut emulator = Emulator::new(5, 20);
        let text: String = (1..=20000).map(|n| format!("{n}\r\n")).collect();
        emulator.feed(text.as_bytes());
        assert_eq!(emulator.history_size(), 10000);
        emulator.scroll(10000);
        assert_eq!(emulator.display_offset(), 10000);
        // The screen shows 19997..20000 and an empty line; the history above it starts at 9997.
        assert_eq!(emulator.row_text(0), "9997");
    }

    #[test]
    fn new_output_moves_a_view_at_the_bottom_but_not_a_scrolled_view() {
        let mut emulator = Emulator::new(4, 20);
        let numbers = |from: usize, to: usize| -> String {
            (from..=to).map(|n| format!("{n}\r\n")).collect()
        };
        emulator.feed(numbers(1, 30).as_bytes());
        assert_eq!(emulator.display_offset(), 0);
        assert_eq!(emulator.row_text(2), "30");

        emulator.scroll(5);
        let top = emulator.row_text(0);
        emulator.feed(numbers(31, 40).as_bytes());
        assert_eq!(emulator.row_text(0), top, "a scrolled view must stay put");
        assert!(emulator.display_offset() > 5);

        emulator.scroll_to_bottom();
        emulator.feed(numbers(41, 45).as_bytes());
        assert_eq!(emulator.display_offset(), 0);
        assert_eq!(emulator.row_text(2), "45");
    }

    #[test]
    fn a_snapshot_resets_the_screen_and_brings_its_history() {
        let mut emulator = Emulator::new(10, 40);
        emulator.feed(b"stale output");
        let cell = |c: &str| json!({"char": c});
        emulator
            .apply_snapshot(&json!({
                "rows": 2, "cols": 6,
                "grid": [
                    [cell("n"), cell("o"), cell("w"), cell(" "), cell(" "), cell(" ")],
                    [cell("$"), cell(" "), cell(" "), cell(" "), cell(" "), cell(" ")],
                ],
                "scrollback": [[cell("o"), cell("l"), cell("d"), cell(" "), cell(" "), cell(" ")]],
                "cursor": {"row": 1, "col": 2},
                "gridWrapped": [false, false],
                "scrollbackWrapped": [false],
            }))
            .unwrap();
        assert_eq!((emulator.rows(), emulator.cols()), (2, 6));
        assert_eq!(screen(&emulator), ["now", "$"]);
        assert_eq!(emulator.history_size(), 1);
        emulator.scroll(1);
        assert_eq!(emulator.row_text(0), "old");
        // The snapshot is replaced, not merged, by the next one.
        emulator.apply_snapshot(&json!({"rows": 2, "cols": 6, "grid": [[], []], "scrollback": [], "cursor": {"row": 0, "col": 0}})).unwrap();
        assert_eq!(emulator.history_size(), 0);
        assert_eq!(screen(&emulator), ["", ""]);
    }

    #[test]
    fn an_invalid_snapshot_is_an_error_and_keeps_the_screen() {
        let mut emulator = Emulator::new(2, 10);
        emulator.feed(b"keep");
        assert!(emulator.apply_snapshot(&json!({"rows": "x"})).is_err());
        assert_eq!(emulator.row_text(0), "keep");
    }

    #[test]
    fn resizing_changes_the_grid() {
        let mut emulator = Emulator::new(4, 20);
        emulator.feed(b"abc");
        emulator.resize(6, 50);
        assert_eq!((emulator.rows(), emulator.cols()), (6, 50));
        assert_eq!(emulator.row_text(0), "abc");
        emulator.resize(0, 0);
        assert_eq!((emulator.rows(), emulator.cols()), (1, 1));
    }

    #[test]
    fn dragging_selects_cells_and_copies_their_text() {
        let mut emulator = Emulator::new(3, 20);
        emulator.feed(b"hello world\r\nsecond line");
        emulator.start_selection(0, 0, false, 1);
        emulator.update_selection(0, 4, true);
        emulator.finish_selection();
        assert_eq!(emulator.selection_text().as_deref(), Some("hello"));

        emulator.start_selection(0, 6, false, 1);
        emulator.update_selection(1, 5, true);
        assert_eq!(emulator.selection_text().as_deref(), Some("world\nsecond"));
    }

    #[test]
    fn a_click_without_a_drag_selects_nothing() {
        let mut emulator = Emulator::new(2, 10);
        emulator.feed(b"abc");
        emulator.start_selection(0, 1, false, 1);
        emulator.finish_selection();
        assert!(!emulator.has_selection());
        assert_eq!(emulator.selection_text(), None);
    }

    #[test]
    fn double_click_selects_a_word_and_triple_click_a_line() {
        let mut emulator = Emulator::new(2, 20);
        emulator.feed(b"one two three");
        emulator.start_selection(0, 5, false, 2);
        assert_eq!(emulator.selection_text().as_deref(), Some("two"));
        emulator.start_selection(0, 5, false, 3);
        assert_eq!(
            emulator.selection_text().as_deref(),
            Some("one two three\n")
        );
    }

    #[test]
    fn selected_cells_get_the_selection_background() {
        let mut emulator = Emulator::new(2, 10);
        emulator.feed(b"abcdef\x1b[2;1H");
        emulator.start_selection(0, 1, false, 1);
        emulator.update_selection(0, 2, true);
        let line = emulator.lines().next().unwrap().clone();
        assert_eq!(line.runs.len(), 1);
        assert_eq!(line.runs[0].0, 1..3);
    }

    #[test]
    fn each_ansi_index_maps_to_its_token() {
        for tokens in [Tokens::light(), Tokens::claude_dark()] {
            let palette = Palette::from_tokens(&tokens);
            for index in 0..16u8 {
                let expected = tokens.ansi[index as usize];
                assert_eq!(
                    resolve(&palette, Color::Indexed(index), true),
                    Some(expected)
                );
                let named = [
                    NamedColor::Black,
                    NamedColor::Red,
                    NamedColor::Green,
                    NamedColor::Yellow,
                    NamedColor::Blue,
                    NamedColor::Magenta,
                    NamedColor::Cyan,
                    NamedColor::White,
                    NamedColor::BrightBlack,
                    NamedColor::BrightRed,
                    NamedColor::BrightGreen,
                    NamedColor::BrightYellow,
                    NamedColor::BrightBlue,
                    NamedColor::BrightMagenta,
                    NamedColor::BrightCyan,
                    NamedColor::BrightWhite,
                ][index as usize];
                assert_eq!(resolve(&palette, Color::Named(named), true), Some(expected));
                assert_eq!(palette_color(&palette, index as usize), channels(expected));
            }
            assert_eq!(
                resolve(&palette, Color::Named(NamedColor::Foreground), true),
                None
            );
        }
    }

    #[test]
    fn a_new_palette_recolors_and_survives_a_reset() {
        let mut emulator = Emulator::new(2, 10);
        emulator.feed(b"\x1b[31mred");
        let before = emulator.lines().next().unwrap().runs[0].1.color;
        let light = Palette::from_tokens(&Tokens::light());
        emulator.set_palette(light);
        let after = emulator.lines().next().unwrap().runs[0].1.color;
        assert_ne!(before, after);
        assert_eq!(after, Some(light.ansi[1]));
        emulator.reset(2, 10);
        assert_eq!(emulator.palette(), light);
    }

    #[test]
    fn the_app_modes_are_readable() {
        let mut emulator = Emulator::new(2, 10);
        assert!(!emulator.mode().contains(TermMode::BRACKETED_PASTE));
        emulator.feed(b"\x1b[?2004h\x1b[?1h");
        assert!(emulator.mode().contains(TermMode::BRACKETED_PASTE));
        assert!(emulator.mode().contains(TermMode::APP_CURSOR));
    }

    #[test]
    fn a_cursor_position_query_gets_a_reply() {
        let mut emulator = Emulator::new(5, 20);
        emulator.feed(b"ab\x1b[6n");
        assert_eq!(emulator.take_replies(), b"\x1b[1;3R");
        assert!(emulator.take_replies().is_empty());
    }
}
