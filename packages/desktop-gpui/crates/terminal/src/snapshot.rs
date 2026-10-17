//! Snapshot restore. The daemon sends the screen as a JSON cell grid (`TerminalStateSchema`);
//! this module turns it into SGR text that the emulator can replay. It is a port of
//! `renderTerminalSnapshotToAnsi` in `packages/protocol/src/terminal-snapshot.ts`.

use serde::Deserialize;
use serde_json::Value;

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Cell {
    pub char: String,
    pub fg: Option<i64>,
    pub bg: Option<i64>,
    pub fg_mode: Option<u32>,
    pub bg_mode: Option<u32>,
    pub bold: bool,
    pub italic: bool,
    pub underline: bool,
    pub dim: bool,
    pub inverse: bool,
    pub strikethrough: bool,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(default)]
pub struct Cursor {
    pub row: usize,
    pub col: usize,
    pub hidden: bool,
    pub style: Option<String>,
    pub blink: Option<bool>,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq)]
#[serde(rename_all = "camelCase", default)]
pub struct Snapshot {
    pub rows: usize,
    pub cols: usize,
    pub grid: Vec<Vec<Cell>>,
    pub scrollback: Vec<Vec<Cell>>,
    pub cursor: Cursor,
    pub grid_wrapped: Option<Vec<bool>>,
    pub scrollback_wrapped: Option<Vec<bool>>,
}

impl Snapshot {
    pub fn from_value(value: &Value) -> Result<Self, serde_json::Error> {
        Snapshot::deserialize(value)
    }

    /// The text that redraws this screen and its history on an empty emulator of the same size:
    /// scrollback rows first, then the grid, then the cursor.
    pub fn to_ansi(&self) -> String {
        let rows: Vec<&Vec<Cell>> = self.scrollback.iter().chain(self.grid.iter()).collect();
        let wrapped: Vec<bool> = self
            .scrollback_wrapped
            .iter()
            .flatten()
            .chain(self.grid_wrapped.iter().flatten())
            .copied()
            .collect();
        // Soft-wrapped rows replay as one unbroken run so the emulator marks them as wrapped.
        // Without the flags, autowrap is off and each row ends with a hard newline.
        let has_wrap_info = wrapped.len() == rows.len();
        let mut out = String::new();
        if !has_wrap_info {
            out.push_str("\x1b[?7l");
        }
        for (index, row) in rows.iter().enumerate() {
            let continues = has_wrap_info && wrapped[index];
            render_row(&mut out, row, continues.then_some(self.cols));
            if index + 1 < rows.len() && !continues {
                out.push_str("\r\n");
            }
        }
        out.push_str("\x1b[0m");
        if let Some(code) = cursor_style_code(&self.cursor) {
            out.push_str(&format!("\x1b[{code} q"));
        }
        out.push_str(&format!(
            "\x1b[{};{}H",
            self.cursor.row + 1,
            self.cursor.col + 1
        ));
        out.push_str(if self.cursor.hidden {
            "\x1b[?25l"
        } else {
            "\x1b[?25h"
        });
        if !has_wrap_info {
            out.push_str("\x1b[?7h");
        }
        out
    }
}

fn cursor_style_code(cursor: &Cursor) -> Option<u8> {
    let steady = cursor.blink == Some(false);
    let base = match cursor.style.as_deref()? {
        "block" => 1,
        "underline" => 3,
        "bar" => 5,
        _ => return None,
    };
    Some(base + steady as u8)
}

#[derive(Clone, Copy, PartialEq)]
struct Style {
    fg: Option<i64>,
    bg: Option<i64>,
    fg_mode: Option<u32>,
    bg_mode: Option<u32>,
    bold: bool,
    italic: bool,
    underline: bool,
    dim: bool,
    inverse: bool,
    strikethrough: bool,
}

const DEFAULT_STYLE: Style = Style {
    fg: None,
    bg: None,
    fg_mode: None,
    bg_mode: None,
    bold: false,
    italic: false,
    underline: false,
    dim: false,
    inverse: false,
    strikethrough: false,
};

impl Style {
    fn of(cell: &Cell) -> Self {
        Style {
            fg: cell.fg,
            bg: cell.bg,
            fg_mode: cell.fg_mode,
            bg_mode: cell.bg_mode,
            bold: cell.bold,
            italic: cell.italic,
            underline: cell.underline,
            dim: cell.dim,
            inverse: cell.inverse,
            strikethrough: cell.strikethrough,
        }
    }

    fn sgr(&self) -> String {
        let mut codes = vec!["0".to_string()];
        for (on, code) in [
            (self.bold, "1"),
            (self.dim, "2"),
            (self.italic, "3"),
            (self.underline, "4"),
            (self.inverse, "7"),
            (self.strikethrough, "9"),
        ] {
            if on {
                codes.push(code.into());
            }
        }
        if let (Some(value), Some(mode)) = (self.fg, self.fg_mode) {
            codes.extend(color_codes(mode, value, false));
        }
        if let (Some(value), Some(mode)) = (self.bg, self.bg_mode) {
            codes.extend(color_codes(mode, value, true));
        }
        format!("\x1b[{}m", codes.join(";"))
    }
}

fn color_codes(mode: u32, value: i64, background: bool) -> Vec<String> {
    match mode {
        1 if value >= 8 => vec![((if background { 100 } else { 90 }) + value - 8).to_string()],
        1 => vec![((if background { 40 } else { 30 }) + value).to_string()],
        2 => vec![
            (if background { "48" } else { "38" }).into(),
            "5".into(),
            value.to_string(),
        ],
        3 => vec![
            (if background { "48" } else { "38" }).into(),
            "2".into(),
            ((value >> 16) & 0xff).to_string(),
            ((value >> 8) & 0xff).to_string(),
            (value & 0xff).to_string(),
        ],
        _ => Vec::new(),
    }
}

/// Number of cells up to the last one that shows anything.
fn content_length(row: &[Cell]) -> usize {
    row.iter()
        .rposition(|cell| {
            (cell.char != " " && !cell.char.is_empty()) || Style::of(cell) != DEFAULT_STYLE
        })
        .map_or(0, |index| index + 1)
}

fn render_row(out: &mut String, row: &[Cell], pad_to: Option<usize>) {
    let content = content_length(row);
    let length = pad_to.map_or(content, |cols| content.max(cols));
    let blank = Cell::default();
    let mut previous = DEFAULT_STYLE;
    for index in 0..length {
        let cell = row.get(index).unwrap_or(&blank);
        let style = Style::of(cell);
        if style != previous {
            out.push_str(&style.sgr());
            previous = style;
        }
        out.push_str(if cell.char.is_empty() {
            " "
        } else {
            &cell.char
        });
    }
    if previous != DEFAULT_STYLE {
        out.push_str("\x1b[0m");
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn cell(c: &str) -> Value {
        json!({"char": c})
    }

    #[test]
    fn rows_become_text_with_sgr_color_codes() {
        let snapshot = Snapshot::from_value(&json!({
            "rows": 2, "cols": 4,
            "grid": [
                [{"char": "h", "fg": 1, "fgMode": 1, "bold": true}, cell("i"), cell(" "), cell(" ")],
                [{"char": "x", "bg": 200, "bgMode": 2}, {"char": "y", "fg": 16711680, "fgMode": 3}, cell(" "), cell(" ")],
            ],
            "scrollback": [[cell("o"), cell("l"), cell("d"), cell(" ")]],
            "cursor": {"row": 1, "col": 2, "hidden": false, "style": "bar", "blink": false},
            "gridWrapped": [false, false],
            "scrollbackWrapped": [false],
        }))
        .unwrap();
        assert_eq!(
            snapshot.to_ansi(),
            "old\r\n\x1b[0;1;31mh\x1b[0mi\r\n\x1b[0;48;5;200mx\x1b[0;38;2;255;0;0my\x1b[0m\
             \x1b[0m\x1b[6 q\x1b[2;3H\x1b[?25h"
        );
    }

    #[test]
    fn soft_wrapped_rows_fill_the_width_and_skip_the_newline() {
        let snapshot = Snapshot::from_value(&json!({
            "rows": 2, "cols": 3,
            "grid": [[cell("a"), cell("b"), cell("c")], [cell("d"), cell(" "), cell(" ")]],
            "scrollback": [],
            "cursor": {"row": 1, "col": 1, "hidden": true},
            "gridWrapped": [true, false],
        }))
        .unwrap();
        assert_eq!(snapshot.to_ansi(), "abcd\x1b[0m\x1b[2;2H\x1b[?25l");
    }

    #[test]
    fn missing_wrap_flags_turn_autowrap_off_around_the_replay() {
        let snapshot = Snapshot::from_value(&json!({
            "rows": 1, "cols": 2,
            "grid": [[cell("a"), cell("b")]],
            "scrollback": [],
            "cursor": {"row": 0, "col": 2},
        }))
        .unwrap();
        assert_eq!(
            snapshot.to_ansi(),
            "\x1b[?7lab\x1b[0m\x1b[1;3H\x1b[?25h\x1b[?7h"
        );
    }

    #[test]
    fn an_invalid_value_is_an_error() {
        assert!(Snapshot::from_value(&json!({"rows": "x"})).is_err());
    }
}
