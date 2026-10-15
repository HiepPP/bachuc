//! Daemon terminal rendered with alacritty_terminal: output frames feed the
//! emulator, and each screen line becomes one StyledText with color runs.

use alacritty_terminal::event::{Event, EventListener};
use alacritty_terminal::grid::Dimensions;
use alacritty_terminal::index::{Column, Line, Point};
use alacritty_terminal::term::cell::Flags;
use alacritty_terminal::term::{Config, Term};
use alacritty_terminal::vte::ansi::{Color, NamedColor, Processor, Rgb};
use gpui_kit::*;
use std::cell::RefCell;
use std::rc::Rc;

pub const COLS: usize = 60;
pub const ROWS: usize = 24;

struct Size;
impl Dimensions for Size {
    fn total_lines(&self) -> usize {
        ROWS
    }
    fn screen_lines(&self) -> usize {
        ROWS
    }
    fn columns(&self) -> usize {
        COLS
    }
}

/// Collects replies the emulator wants to write back to the PTY (cursor
/// position reports and similar).
#[derive(Clone, Default)]
pub struct Replies(Rc<RefCell<Vec<u8>>>);

impl EventListener for Replies {
    fn send_event(&self, event: Event) {
        if let Event::PtyWrite(text) = event {
            self.0.borrow_mut().extend_from_slice(text.as_bytes());
        }
    }
}

pub struct TerminalState {
    term: Term<Replies>,
    parser: Processor,
    replies: Replies,
    pub slot: Option<u8>,
    pub id: Option<String>,
}

impl TerminalState {
    pub fn new() -> Self {
        let replies = Replies::default();
        Self {
            term: Term::new(Config::default(), &Size, replies.clone()),
            parser: Processor::new(),
            replies,
            slot: None,
            id: None,
        }
    }

    pub fn feed(&mut self, bytes: &[u8]) {
        self.parser.advance(&mut self.term, bytes);
    }

    pub fn take_replies(&mut self) -> Vec<u8> {
        std::mem::take(&mut *self.replies.0.borrow_mut())
    }

    pub fn render_lines(&self) -> Vec<AnyElement> {
        let content = self.term.renderable_content();
        let cursor = content.cursor.point;
        let mut lines = Vec::with_capacity(ROWS);
        for row in 0..ROWS as i32 {
            let mut text = String::with_capacity(COLS);
            let mut runs: Vec<(std::ops::Range<usize>, HighlightStyle)> = Vec::new();
            for col in 0..COLS {
                let point = Point::new(Line(row), Column(col));
                let cell = &self.term.grid()[point];
                if cell.flags.contains(Flags::WIDE_CHAR_SPACER) {
                    continue;
                }
                let start = text.len();
                text.push(if cell.c == '\0' { ' ' } else { cell.c });
                let mut style = HighlightStyle::default();
                if let Some(color) = to_hsla(cell.fg, true) {
                    style.color = Some(color);
                }
                if let Some(color) = to_hsla(cell.bg, false) {
                    style.background_color = Some(color);
                }
                if point == cursor {
                    style.background_color = Some(rgb(0xd0d0d0).into());
                    style.color = Some(rgb(0x000000).into());
                }
                if style != HighlightStyle::default() {
                    runs.push((start..text.len(), style));
                }
            }
            lines.push(
                div()
                    .h(px(16.))
                    .whitespace_nowrap()
                    .child(StyledText::new(text).with_highlights(runs))
                    .into_any_element(),
            );
        }
        lines
    }
}

/// Maps a cell color to Hsla; `None` means the default foreground or background.
fn to_hsla(color: Color, foreground: bool) -> Option<Hsla> {
    let rgb_value = match color {
        Color::Spec(Rgb { r, g, b }) => (r, g, b),
        Color::Named(NamedColor::Foreground) if foreground => return None,
        Color::Named(NamedColor::Background) if !foreground => return None,
        Color::Named(name) if (name as usize) < 16 => ANSI[name as usize],
        Color::Named(_) => return None,
        Color::Indexed(index) => indexed(index),
    };
    let (r, g, b) = rgb_value;
    Some(rgb(((r as u32) << 16) | ((g as u32) << 8) | b as u32).into())
}

const ANSI: [(u8, u8, u8); 16] = [
    (0, 0, 0),
    (205, 49, 49),
    (13, 188, 121),
    (229, 229, 16),
    (36, 114, 200),
    (188, 63, 188),
    (17, 168, 205),
    (229, 229, 229),
    (102, 102, 102),
    (241, 76, 76),
    (35, 209, 139),
    (245, 245, 67),
    (59, 142, 234),
    (214, 112, 214),
    (41, 184, 219),
    (255, 255, 255),
];

fn indexed(index: u8) -> (u8, u8, u8) {
    match index {
        0..=15 => ANSI[index as usize],
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

/// Converts a key press into the bytes a terminal expects.
pub fn key_bytes(keystroke: &Keystroke) -> Option<Vec<u8>> {
    let key = keystroke.key.as_str();
    if keystroke.modifiers.control && key.len() == 1 {
        let c = key.as_bytes()[0].to_ascii_lowercase();
        if c.is_ascii_lowercase() {
            return Some(vec![c - b'a' + 1]);
        }
    }
    let fixed: &[u8] = match key {
        "enter" => b"\r",
        "backspace" => b"\x7f",
        "tab" => b"\t",
        "escape" => b"\x1b",
        "up" => b"\x1b[A",
        "down" => b"\x1b[B",
        "right" => b"\x1b[C",
        "left" => b"\x1b[D",
        _ => return keystroke.key_char.as_ref().map(|s| s.as_bytes().to_vec()),
    };
    Some(fixed.to_vec())
}
