//! The terminal pane: it owns an emulator, feeds it the daemon's frames, sizes it to the pane,
//! and turns keys, the mouse, and the clipboard into terminal input. Cmd+F opens a find bar, and
//! a Cmd-click opens a URL or a file path.

use crate::emulator::{Emulator, Palette};
use crate::find::{self, Search};
use crate::keys::{key_bytes, paste_bytes};
use crate::links::{self, LinkTarget};
use crate::size::{PADDING, cell_at, grid_size};
use alacritty_terminal::term::TermMode;
use bachuc_client::{Client, MatchMode, TerminalFrame, TerminalHandle};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    BORDER_WIDTH_2, FONT_SM, ICON_SM, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_1_5, SPACE_2, SPACE_3,
};
use bachuc_theme::{ShadowSize, Tokens, shadow, tokens, ui_font_size};
use futures::StreamExt;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::Cell;
use std::ops::Range;
use std::rc::Rc;

/// The Electron terminal font list (`terminal-font.ts`), in order; the first installed one wins.
const FONT_FAMILIES: [&str; 13] = [
    "JetBrains Mono",
    "JetBrainsMono Nerd Font",
    "JetBrainsMono NF",
    "MesloLGM Nerd Font",
    "MesloLGM NF",
    "Hack Nerd Font",
    "FiraCode Nerd Font",
    "Symbols Nerd Font",
    "SF Mono",
    "Menlo",
    "Monaco",
    "Consolas",
    "Liberation Mono",
];
const FALLBACK_FAMILY: &str = "Menlo";
const INITIAL_ROWS: usize = 24;
const INITIAL_COLS: usize = 80;
/// Frames fed to the emulator before the view draws again, so a flood of output costs one draw.
const FRAMES_PER_BATCH: usize = 256;
/// Suggestions asked for a file link. The client sends at most 100.
const LOOKUP_LIMIT: u32 = 100;
/// `FIND_WIDGET_WIDTH` and `COLLAPSED_FIELD_HEIGHT` of `pane-find`, as the agent pane's bar.
const FIND_WIDTH: f32 = 340.;
const FIELD_HEIGHT: f32 = 28.;
/// The find field's focus outline: 2 px wide, 1 px outside the field (`control-geometry.ts`).
const RING_WIDTH: f32 = BORDER_WIDTH_2;
const RING_OFFSET: f32 = 1.;
/// Electron marks matches with the theme's yellow and the current match with its blue. GPUI
/// spans have no border, so they fill the cells, faded to keep the text readable.
const MATCH_ALPHA: f32 = 0.35;
const CURRENT_MATCH_ALPHA: f32 = 0.6;

/// What a terminal view tells its owner.
#[derive(Clone, Debug, PartialEq)]
pub enum TerminalEvent {
    /// The daemon terminal is open; this is its id.
    Opened { id: String },
    /// The terminal process ended or the terminal was killed.
    Exited,
    /// A Cmd-click on a file path the daemon found, as `TimelineEvent::OpenFile`: `path` is
    /// relative to `cwd`, or absolute for a file outside it.
    OpenFile {
        cwd: String,
        path: String,
        line: Option<u32>,
    },
}

impl EventEmitter<TerminalEvent> for TerminalView {}

enum Source {
    Create(String),
    Attach(String),
}

/// The open find bar. Its search lives in the view, so the query outlives the bar.
struct Finder {
    input: Entity<InputState>,
    _subscriptions: Vec<Subscription>,
}

/// The link under the pointer while Cmd is held: its screen cell and its byte range in each row
/// it covers, as a wrapped link covers several.
#[derive(Clone, PartialEq)]
struct Hover {
    row: usize,
    col: usize,
    spans: Vec<(usize, Range<usize>)>,
}

#[derive(Clone, Copy)]
struct Metrics {
    cell_width: f32,
    line_height: f32,
}

/// The font, size, and cell size the pane draws with.
#[derive(Clone, PartialEq)]
struct Typeface {
    family: &'static str,
    size: f32,
}

impl Typeface {
    /// The first installed family of the Electron list, at the code font size.
    fn current(cx: &App) -> Self {
        let installed = cx.text_system().all_font_names();
        let family = FONT_FAMILIES
            .into_iter()
            .find(|name| installed.iter().any(|font| font == name))
            .unwrap_or(FALLBACK_FAMILY);
        Self {
            family,
            size: f32::from(bachuc_theme::code_font_size(
                bachuc_theme::metrics::FONT_CODE,
                cx,
            )),
        }
    }

    fn measure(&self, cx: &App) -> Metrics {
        let system = cx.text_system();
        let font = Font {
            family: self.family.into(),
            features: FontFeatures::default(),
            fallbacks: None,
            weight: FontWeight::NORMAL,
            style: FontStyle::Normal,
        };
        let id = system.resolve_font(&font);
        let size = px(self.size);
        let cell_width = system
            .advance(id, size, 'm')
            .map_or(self.size * 0.6, |advance| f32::from(advance.width));
        let line_height =
            f32::from(system.ascent(id, size)) + f32::from(system.descent(id, size)).abs();
        Metrics {
            cell_width,
            line_height: line_height.ceil().max(self.size),
        }
    }
}

pub struct TerminalView {
    client: Entity<Client>,
    name: String,
    /// The folder the terminal opened in, for file links. Empty for an attached terminal.
    cwd: String,
    emulator: Emulator,
    terminal_id: Option<String>,
    exited: bool,
    /// Rows and columns of the pane, once it has been laid out.
    size: Option<(u16, u16)>,
    /// Whether the daemon has been told to follow this pane's size. The first size claims it;
    /// later sizes only resize. Every snapshot starts over, because a reconnect resubscribes.
    claimed: bool,
    focus: FocusHandle,
    typeface: Typeface,
    metrics: Metrics,
    palette: Palette,
    _theme: Subscription,
    /// The pane's bounds from the last paint, for mouse positions.
    bounds: Rc<Cell<Bounds<Pixels>>>,
    /// The size the last paint asked for, so a steady pane asks once.
    painted_size: Rc<Cell<(u16, u16)>>,
    dragging: bool,
    wheel_remainder: f32,
    reader: Option<Task<()>>,
    finder: Option<Finder>,
    search: Search,
    hover: Option<Hover>,
    /// The last pointer position, so a Cmd press can underline the link under it.
    pointer: Option<Point<Pixels>>,
    /// The daemon lookup of the last clicked file link.
    link_lookup: Option<Task<()>>,
}

impl TerminalView {
    /// Opens a new terminal in `cwd`, a folder the daemon knows as a workspace.
    pub fn new(
        client: Entity<Client>,
        cwd: String,
        window: &mut Window,
        cx: &mut Context<TerminalView>,
    ) -> TerminalView {
        let name = cwd
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .filter(|name| !name.is_empty())
            .unwrap_or("terminal")
            .to_string();
        Self::start(client, Source::Create(cwd), name, window, cx)
    }

    /// Shows an existing daemon terminal. Its history arrives as the first frame.
    pub fn attach(
        client: Entity<Client>,
        terminal_id: String,
        window: &mut Window,
        cx: &mut Context<TerminalView>,
    ) -> TerminalView {
        let name = terminal_id.chars().take(8).collect();
        Self::start(client, Source::Attach(terminal_id), name, window, cx)
    }

    fn start(
        client: Entity<Client>,
        source: Source,
        name: String,
        window: &mut Window,
        cx: &mut Context<TerminalView>,
    ) -> TerminalView {
        let cwd = match &source {
            Source::Create(cwd) => cwd.clone(),
            Source::Attach(_) => String::new(),
        };
        let opening = client.update(cx, |client, cx| match &source {
            Source::Create(cwd) => client.create_terminal(cwd, cx),
            Source::Attach(id) => client.attach_terminal(id, cx),
        });
        let reader = cx.spawn(async move |this, cx| {
            let handle = match opening.await {
                Ok(handle) => handle,
                Err(error) => {
                    let message = format!("\x1b[31mcould not open the terminal: {error}\x1b[0m");
                    let _ = this.update(cx, |this, cx| this.show_notice(&message, cx));
                    return;
                }
            };
            let TerminalHandle { id, mut frames } = handle;
            if this.update(cx, |this, cx| this.opened(id, cx)).is_err() {
                return;
            }
            while let Some(first) = frames.next().await {
                let mut batch = vec![first];
                while batch.len() < FRAMES_PER_BATCH {
                    match frames.try_recv() {
                        Ok(frame) => batch.push(frame),
                        Err(_) => break,
                    }
                }
                if this.update(cx, |this, cx| this.feed(batch, cx)).is_err() {
                    return;
                }
            }
            let _ = this.update(cx, |this, cx| {
                this.show_notice("\x1b[2m[process exited]\x1b[0m", cx);
                cx.emit(TerminalEvent::Exited);
            });
        });

        let focus = cx.focus_handle();
        focus.focus(window, cx);
        let typeface = Typeface::current(cx);
        let palette = Palette::from_tokens(&bachuc_theme::tokens(cx));
        let mut emulator = Emulator::new(INITIAL_ROWS, INITIAL_COLS);
        emulator.set_palette(palette);
        TerminalView {
            client,
            name,
            cwd,
            emulator,
            terminal_id: None,
            exited: false,
            size: None,
            claimed: false,
            focus,
            metrics: typeface.measure(cx),
            typeface,
            palette,
            // `bachuc_theme::set_theme` and `set_font_scale` write the gpui-kit theme.
            _theme: cx.observe_global::<Theme>(|this, cx| this.restyle(cx)),
            bounds: Rc::new(Cell::new(Bounds::default())),
            painted_size: Rc::new(Cell::new((0, 0))),
            dragging: false,
            wheel_remainder: 0.0,
            reader: Some(reader),
            finder: None,
            search: Search::default(),
            hover: None,
            pointer: None,
            link_lookup: None,
        }
    }

    /// Takes the colors, font, and size of the theme on screen. A new cell size changes the grid
    /// at the next paint, which resizes the emulator and the PTY.
    fn restyle(&mut self, cx: &mut Context<TerminalView>) {
        let palette = Palette::from_tokens(&bachuc_theme::tokens(cx));
        let typeface = Typeface::current(cx);
        if palette == self.palette && typeface == self.typeface {
            return;
        }
        self.palette = palette;
        self.emulator.set_palette(palette);
        if typeface != self.typeface {
            self.metrics = typeface.measure(cx);
            self.typeface = typeface;
        }
        cx.notify();
    }

    /// The daemon id of the terminal, once it is open.
    pub fn terminal_id(&self) -> Option<&str> {
        self.terminal_id.as_deref()
    }

    /// Names the pane for assistive technology, for example with the tab title.
    pub fn set_name(&mut self, name: impl Into<String>, cx: &mut Context<TerminalView>) {
        self.name = name.into();
        cx.notify();
    }

    /// Kills the daemon terminal and stops reading from it.
    pub fn close(&mut self, cx: &mut Context<TerminalView>) {
        if let Some(id) = self.terminal_id.take() {
            self.client
                .update(cx, |client, cx| client.kill_terminal(&id, cx));
        }
        self.reader = None;
        self.exited = true;
        cx.notify();
    }

    fn opened(&mut self, id: String, cx: &mut Context<TerminalView>) {
        self.terminal_id = Some(id.clone());
        cx.emit(TerminalEvent::Opened { id });
    }

    /// Prints a line of our own text in the pane, such as an error or the exit notice.
    fn show_notice(&mut self, text: &str, cx: &mut Context<TerminalView>) {
        self.exited = true;
        self.emulator.feed(format!("\r\n{text}\r\n").as_bytes());
        cx.notify();
    }

    fn feed(&mut self, frames: Vec<TerminalFrame>, cx: &mut Context<TerminalView>) {
        for frame in frames {
            match frame {
                TerminalFrame::Output(bytes) => self.emulator.feed(&bytes),
                TerminalFrame::Snapshot(snapshot) => {
                    match self.emulator.apply_snapshot(&snapshot) {
                        Ok(()) => {
                            self.dragging = false;
                            self.claimed = false;
                            self.sync_size(cx);
                        }
                        Err(error) => log::warn!("terminal snapshot ignored: {error}"),
                    }
                }
            }
        }
        if self.finder.is_some() {
            self.search.stale = true;
        }
        let replies = self.emulator.take_replies();
        if !replies.is_empty() {
            self.send(&replies, cx);
        }
        cx.notify();
    }

    fn send(&mut self, bytes: &[u8], cx: &mut Context<TerminalView>) {
        if let Some(id) = &self.terminal_id {
            self.client
                .update(cx, |client, _| client.terminal_input(id, bytes));
        }
    }

    /// Called when the pane is laid out at a new size.
    fn set_size(&mut self, rows: u16, cols: u16, cx: &mut Context<TerminalView>) {
        if self.size != Some((rows, cols)) {
            self.size = Some((rows, cols));
            self.sync_size(cx);
        }
    }

    /// Resizes the emulator to the pane and tells the daemon.
    fn sync_size(&mut self, cx: &mut Context<TerminalView>) {
        let Some((rows, cols)) = self.size else {
            return;
        };
        self.emulator.resize(rows.into(), cols.into());
        if let Some(id) = &self.terminal_id {
            let claim = !self.claimed;
            self.claimed = true;
            self.client.update(cx, |client, _| {
                client.terminal_resize(id, rows, cols, claim)
            });
        }
        cx.notify();
    }

    fn key_down(&mut self, event: &KeyDownEvent, cx: &mut Context<TerminalView>) {
        let keystroke = &event.keystroke;
        let modifiers = keystroke.modifiers;
        if modifiers.platform && !modifiers.control && !modifiers.alt {
            match keystroke.key.as_str() {
                "c" => self.copy(cx),
                "v" => self.paste(cx),
                // Other Cmd shortcuts belong to the app.
                _ => return,
            }
            cx.stop_propagation();
            return;
        }
        let mode = self.emulator.mode();
        if modifiers.shift && !mode.contains(TermMode::ALT_SCREEN) {
            let page = self.emulator.rows().saturating_sub(1).max(1) as i32;
            let scroll = match keystroke.key.as_str() {
                "pageup" => Some(page),
                "pagedown" => Some(-page),
                _ => None,
            };
            if let Some(lines) = scroll {
                self.emulator.scroll(lines);
                cx.stop_propagation();
                cx.notify();
                return;
            }
        }
        if self.exited {
            return;
        }
        if let Some(bytes) = key_bytes(keystroke, mode.contains(TermMode::APP_CURSOR)) {
            // Typing shows the prompt again and ends a selection.
            self.emulator.scroll_to_bottom();
            self.emulator.clear_selection();
            self.send(&bytes, cx);
            cx.stop_propagation();
            cx.notify();
        }
    }

    fn copy(&mut self, cx: &mut Context<TerminalView>) {
        if let Some(text) = self.emulator.selection_text() {
            cx.write_to_clipboard(ClipboardItem::new_string(text));
        }
    }

    fn paste(&mut self, cx: &mut Context<TerminalView>) {
        if self.exited {
            return;
        }
        let Some(text) = cx.read_from_clipboard().and_then(|item| item.text()) else {
            return;
        };
        let bracketed = self.emulator.mode().contains(TermMode::BRACKETED_PASTE);
        self.emulator.scroll_to_bottom();
        self.send(&paste_bytes(&text, bracketed), cx);
        cx.notify();
    }

    fn cell_under(&self, position: Point<Pixels>) -> (usize, usize, bool) {
        let origin = self.bounds.get().origin;
        cell_at(
            f32::from(position.x - origin.x),
            f32::from(position.y - origin.y),
            self.metrics.cell_width,
            self.metrics.line_height,
            self.emulator.rows(),
            self.emulator.cols(),
        )
    }

    fn mouse_down(&mut self, event: &MouseDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        self.focus.focus(window, cx);
        let (row, col, right) = self.cell_under(event.position);
        if event.modifiers.platform
            && let Some(link) = self.link_under(row, col)
        {
            self.open_link(link.target, cx);
            return;
        }
        self.emulator
            .start_selection(row, col, right, event.click_count);
        self.dragging = true;
        cx.notify();
    }

    fn mouse_moved(&mut self, event: &MouseMoveEvent, cx: &mut Context<Self>) {
        let position = event.position;
        if !self.dragging {
            self.pointer = Some(position);
            self.update_hover(position, event.modifiers.platform, cx);
            return;
        }
        // Dragging above or below the pane scrolls the history under the selection.
        let bounds = self.bounds.get();
        if position.y < bounds.top() {
            self.emulator.scroll(1);
        } else if position.y > bounds.bottom() {
            self.emulator.scroll(-1);
        }
        let (row, col, right) = self.cell_under(position);
        self.emulator.update_selection(row, col, right);
        cx.notify();
    }

    fn mouse_released(&mut self, cx: &mut Context<Self>) {
        if self.dragging {
            self.dragging = false;
            self.emulator.finish_selection();
            cx.notify();
        }
    }

    fn wheel(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let line_height = self.metrics.line_height;
        let delta = event.delta.pixel_delta(px(line_height)).y;
        self.wheel_remainder += f32::from(delta) / line_height;
        let lines = self.wheel_remainder.trunc();
        if lines == 0.0 {
            return;
        }
        self.wheel_remainder -= lines;
        let mode = self.emulator.mode();
        if mode.contains(TermMode::ALT_SCREEN)
            && mode.contains(TermMode::ALTERNATE_SCROLL)
            && !mode.intersects(TermMode::MOUSE_MODE)
        {
            // Full-screen apps such as less have no history; they take the wheel as arrows.
            let arrow: &[u8] = match (lines > 0.0, mode.contains(TermMode::APP_CURSOR)) {
                (true, true) => b"\x1bOA",
                (true, false) => b"\x1b[A",
                (false, true) => b"\x1bOB",
                (false, false) => b"\x1b[B",
            };
            let bytes = arrow.repeat(lines.abs() as usize);
            self.send(&bytes, cx);
        } else {
            self.emulator.scroll(lines as i32);
        }
        cx.stop_propagation();
        cx.notify();
    }
}

/// The find field's outline, drawn as a border on a box that reaches past the field. GPUI has no
/// outline, and the box takes no pointer events because it has no handler.
fn focus_ring(color: Hsla) -> Div {
    let reach = RING_WIDTH + RING_OFFSET;
    div()
        .absolute()
        .top(px(-reach))
        .right(px(-reach))
        .bottom(px(-reach))
        .left(px(-reach))
        .border(px(RING_WIDTH))
        .border_color(color)
        .rounded(px(RADIUS_MD + reach))
}

type BarAction = fn(&mut TerminalView, &mut Window, &mut Context<TerminalView>);

impl TerminalView {
    /// Opens the find bar, or focuses it and selects its text when it is open. A new bar shows
    /// the last query and searches it again.
    fn open_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let opening = self.finder.is_none();
        if opening {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Find in pane"));
            let weak = cx.weak_entity();
            let input_focus = input.focus_handle(cx);
            let subscriptions =
                vec![
                    // The focus ring shows while the field has focus.
                    cx.on_focus_in(&input_focus, window, |_, _, cx| cx.notify()),
                    cx.on_focus_out(&input_focus, window, |_, _, _, cx| cx.notify()),
                    cx.subscribe_in(&input, window, |this, input, event: &InputEvent, _, cx| {
                        match event {
                            InputEvent::Change => {
                                let query = input.read(cx).value().to_string();
                                this.set_query(query, cx);
                            }
                            InputEvent::PressEnter { shift, .. } => this.step_find(!*shift, cx),
                            _ => {}
                        }
                    }),
                    // The field keeps Escape for itself, so the pane reads it first while the field
                    // has focus.
                    cx.intercept_keystrokes(move |event, window, cx| {
                        if event.keystroke.key != "escape" {
                            return;
                        }
                        let closed = weak
                            .update(cx, |this, cx| {
                                let focused = this.finder.as_ref().is_some_and(|finder| {
                                    finder.input.focus_handle(cx).is_focused(window)
                                });
                                if focused {
                                    this.close_find(window, cx);
                                }
                                focused
                            })
                            .unwrap_or(false);
                        if closed {
                            cx.stop_propagation();
                        }
                    }),
                ];
            // `set_value` sends no change event, so the kept query is searched below.
            let query = self.search.query.clone();
            input.update(cx, |input, cx| input.set_value(query, window, cx));
            self.finder = Some(Finder {
                input,
                _subscriptions: subscriptions,
            });
        }
        if let Some(finder) = &self.finder {
            finder.input.update(cx, |input, cx| {
                input.focus(window, cx);
                input.select_all(window, cx);
            });
        }
        if opening && !self.search.query.is_empty() {
            self.find_first(cx);
        }
        cx.notify();
    }

    fn close_find(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.finder.take().is_some() {
            self.search.close();
            self.focus.focus(window, cx);
            cx.notify();
        }
    }

    /// Cmd+F, from the pane or from the find field.
    fn find_key(&mut self, event: &KeyDownEvent, window: &mut Window, cx: &mut Context<Self>) {
        let keystroke = &event.keystroke;
        let modifiers = keystroke.modifiers;
        if keystroke.key == "f"
            && modifiers.platform
            && !modifiers.control
            && !modifiers.alt
            && !modifiers.shift
        {
            self.open_find(window, cx);
            cx.stop_propagation();
        }
    }

    /// A new query searches again and shows the first match from the top of the screen down.
    fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        if self.finder.is_none() || self.search.query == query {
            return;
        }
        self.search.query = query;
        self.find_first(cx);
    }

    /// Searches the query and shows the first match from the top of the screen down.
    fn find_first(&mut self, cx: &mut Context<Self>) {
        self.search(cx);
        let top = find::top_line(self.emulator.history_size(), self.emulator.display_offset());
        self.search.current = find::first_from(&self.search.matches, top);
        self.reveal_current(cx);
    }

    fn step_find(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self.finder.is_none() {
            return;
        }
        if self.search.stale {
            self.search(cx);
        }
        let top = find::top_line(self.emulator.history_size(), self.emulator.display_offset());
        let search = &mut self.search;
        let count = search.matches.len();
        search.current = match search.current {
            Some(current) => (count > 0).then(|| find::step(current, count, forward)),
            None => find::first_from(&search.matches, top),
        };
        self.reveal_current(cx);
    }

    /// Finds the query in the history and the screen. It runs on a query change, on opening the
    /// bar, and on a step after new output, never per output frame.
    fn search(&mut self, cx: &mut Context<Self>) {
        let lines = if self.search.query.is_empty() {
            Vec::new()
        } else {
            find::buffer_lines(&mut self.emulator)
        };
        self.search.run(&lines);
        cx.notify();
    }

    /// Scrolls the history so the current match is on screen.
    fn reveal_current(&mut self, cx: &mut Context<Self>) {
        let Some(line) = self
            .search
            .current
            .and_then(|current| self.search.matches.get(current))
            .map(|found| found.line)
        else {
            cx.notify();
            return;
        };
        let offset = self.emulator.display_offset();
        let target = find::reveal_offset(
            line,
            self.emulator.history_size(),
            self.emulator.rows(),
            offset,
        );
        self.emulator.scroll(target as i32 - offset as i32);
        cx.notify();
    }

    /// The link drawn in a screen cell, joined across the rows it wraps over.
    fn link_under(&mut self, row: usize, col: usize) -> Option<links::RowLink> {
        let rows: Vec<(&str, bool)> = self
            .emulator
            .lines()
            .map(|line| (line.text.as_ref(), line.wrapped))
            .collect();
        links::link_in_rows(&rows, row, col)
    }

    /// Opens a URL in the browser. A file link asks the daemon for the file and opens it only
    /// when the path exists exactly or has a single match (Q-112).
    fn open_link(&mut self, target: LinkTarget, cx: &mut Context<Self>) {
        match target {
            LinkTarget::Url(url) => cx.open_url(&url),
            LinkTarget::File { path, line } => {
                let Some(lookup) = links::lookup(&self.cwd, &path) else {
                    return;
                };
                // The daemon matches by path suffix; `resolve` still checks the suffix.
                let found = self.client.update(cx, |client, cx| {
                    client.directory_suggestions_matching(
                        &lookup.cwd,
                        &lookup.query,
                        true,
                        false,
                        LOOKUP_LIMIT,
                        MatchMode::Suffix,
                        cx,
                    )
                });
                let cwd = self.cwd.clone();
                self.link_lookup = Some(cx.spawn(async move |this, cx| {
                    let Ok(found) = found.await else {
                        return;
                    };
                    let Some(path) = links::resolve(&lookup, &found) else {
                        return;
                    };
                    let _ = this.update(cx, |_, cx| {
                        cx.emit(TerminalEvent::OpenFile { cwd, path, line })
                    });
                }));
            }
        }
    }

    /// Underlines the link under the pointer while Cmd is held.
    fn update_hover(&mut self, position: Point<Pixels>, platform: bool, cx: &mut Context<Self>) {
        if !platform && self.hover.is_none() {
            return;
        }
        let hover = if platform && self.bounds.get().contains(&position) {
            let (row, col, _) = self.cell_under(position);
            self.link_under(row, col).map(|link| Hover {
                row,
                col,
                spans: link.spans,
            })
        } else {
            None
        };
        let moved = |a: &Option<Hover>, b: &Option<Hover>| match (a, b) {
            (Some(a), Some(b)) => a.spans != b.spans,
            (a, b) => a.is_some() != b.is_some(),
        };
        if moved(&self.hover, &hover) {
            cx.notify();
        }
        self.hover = hover;
    }

    fn bar_button(
        id: &'static str,
        label: &'static str,
        icon_name: IconName,
        enabled: bool,
        run: BarAction,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let fill = t.interaction_highlight;
        let weak = cx.weak_entity();
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .p(px(SPACE_1))
            .rounded(px(RADIUS_MD))
            .cursor_pointer()
            .hover(move |button| button.bg(fill))
            .when(!enabled, |button| button.opacity(0.5))
            .on_click(cx.listener(move |this, _, window, cx| run(this, window, cx)))
            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                weak.update(cx, |this, cx| run(this, window, cx)).ok();
            })
            .child(
                icon(icon_name)
                    .with_size(px(ICON_SM))
                    .text_color(t.foreground_muted),
            )
    }

    /// The bar over the top right of the pane, as `PaneFind`.
    fn render_find_bar(&self, window: &Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        let finder = self.finder.as_ref()?;
        let search = &self.search;
        let t = tokens(cx);
        let focused = finder.input.focus_handle(cx).is_focused(window);
        let count = search.matches.len();
        let text = find::status(&search.query, count, search.current, search.limited);
        let size = ui_font_size(FONT_SM, cx);
        let line_height = px((f32::from(size) * 4. / 3.).round());
        let can_step = count > 0;
        let previous: BarAction = |this, _, cx| this.step_find(false, cx);
        let next: BarAction = |this, _, cx| this.step_find(true, cx);
        let close: BarAction = |this, window, cx| this.close_find(window, cx);
        Some(
            div()
                .absolute()
                .top(px(SPACE_2))
                .right(px(SPACE_3))
                .occlude()
                .child(
                    h_flex()
                        .id("terminal-find")
                        .role(Role::Search)
                        .aria_label("Find")
                        .w(px(FIND_WIDTH))
                        .max_w_full()
                        .p(px(SPACE_1_5))
                        .gap(px(SPACE_1))
                        .items_center()
                        .text_size(size)
                        .line_height(line_height)
                        .bg(t.surface1)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(RADIUS_LG))
                        .shadow(vec![shadow(ShadowSize::Md, cx)])
                        .child(
                            h_flex()
                                .flex_1()
                                .min_w_0()
                                .h(px(FIELD_HEIGHT))
                                .gap(px(SPACE_2))
                                .items_center()
                                .px(px(SPACE_2))
                                .relative()
                                .bg(t.surface2)
                                .rounded(px(RADIUS_MD))
                                .when(focused, |field| field.child(focus_ring(t.accent)))
                                .child(
                                    div().flex_1().min_w_0().child(
                                        Input::new(&finder.input)
                                            .appearance(false)
                                            .bordered(false)
                                            .focus_bordered(false)
                                            .px_0()
                                            .py_0()
                                            .text_size(size)
                                            .aria_label("Find in pane"),
                                    ),
                                )
                                .child(
                                    div()
                                        .id("terminal-find-status")
                                        .role(Role::Status)
                                        .aria_label(if text.is_empty() {
                                            "Find matches".to_string()
                                        } else {
                                            text.clone()
                                        })
                                        .flex_none()
                                        .text_color(t.foreground_muted)
                                        .child(text),
                                ),
                        )
                        .child(Self::bar_button(
                            "terminal-find-previous",
                            "Previous match",
                            IconName::ArrowUp,
                            can_step,
                            previous,
                            &t,
                            cx,
                        ))
                        .child(Self::bar_button(
                            "terminal-find-next",
                            "Next match",
                            IconName::ArrowDown,
                            can_step,
                            next,
                            &t,
                            cx,
                        ))
                        .child(Self::bar_button(
                            "terminal-find-close",
                            "Close Find",
                            IconName::X,
                            true,
                            close,
                            &t,
                            cx,
                        )),
                )
                .into_any_element(),
        )
    }
}

impl Render for TerminalView {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Metrics {
            cell_width,
            line_height,
        } = self.metrics;
        // Find marks and the link underline are found in each drawn row, so they follow output
        // that arrived after the search.
        let top = find::top_line(self.emulator.history_size(), self.emulator.display_offset());
        let open = self.finder.is_some();
        let query = Some(self.search.query.clone()).filter(|query| open && !query.is_empty());
        let current = find::current_place(&self.search.matches, self.search.current);
        let hover = self.hover.clone();
        let mark = |color: Hsla, alpha: f32| HighlightStyle {
            background_color: Some(color.opacity(alpha)),
            ..Default::default()
        };
        let other_mark = mark(self.palette.ansi[3], MATCH_ALPHA);
        let current_mark = mark(self.palette.ansi[4], CURRENT_MATCH_ALPHA);
        let underline = HighlightStyle {
            underline: Some(UnderlineStyle {
                thickness: px(1.),
                color: None,
                wavy: false,
            }),
            ..Default::default()
        };
        let drawn: Vec<&crate::emulator::RenderedLine> = self.emulator.lines().collect();
        // The hovered link is found again in the drawn rows, so it follows new output.
        let hover_spans = hover
            .and_then(|hover| {
                let rows: Vec<(&str, bool)> = drawn
                    .iter()
                    .map(|line| (line.text.as_ref(), line.wrapped))
                    .collect();
                links::link_in_rows(&rows, hover.row, hover.col)
            })
            .map_or_else(Vec::new, |link| link.spans);
        let lines: Vec<AnyElement> = drawn
            .into_iter()
            .enumerate()
            .map(|(row, line)| {
                let mut overlays = Vec::new();
                if let Some(query) = &query {
                    for (place, range) in find::match_ranges(&line.text, query)
                        .into_iter()
                        .enumerate()
                    {
                        let style = if current == Some((top + row, place)) {
                            current_mark
                        } else {
                            other_mark
                        };
                        overlays.push((range, style));
                    }
                }
                if let Some((_, range)) = hover_spans.iter().find(|(span_row, _)| *span_row == row)
                {
                    overlays.push((range.clone(), underline));
                }
                let runs = if overlays.is_empty() {
                    line.runs.clone()
                } else {
                    find::paint(&line.runs, &overlays)
                };
                div()
                    .h(px(line_height))
                    .w_full()
                    .flex_none()
                    .whitespace_nowrap()
                    .child(StyledText::new(line.text.clone()).with_highlights(runs))
                    .into_any_element()
            })
            .collect();

        let view = cx.entity();
        let bounds = self.bounds.clone();
        let painted_size = self.painted_size.clone();
        let measure = canvas(
            {
                let view = view.clone();
                move |pane: Bounds<Pixels>, window: &mut Window, cx: &mut App| {
                    bounds.set(pane);
                    let (width, height) = (f32::from(pane.size.width), f32::from(pane.size.height));
                    // A pane that has not been laid out yet is not a size to claim.
                    if width < cell_width + 2.0 * PADDING || height < line_height + 2.0 * PADDING {
                        return;
                    }
                    let size = grid_size(width, height, cell_width, line_height);
                    if painted_size.replace(size) != size {
                        window.defer(cx, move |_, cx| {
                            view.update(cx, |this, cx| this.set_size(size.0, size.1, cx));
                        });
                    }
                }
            },
            move |_, _, window, _| {
                // The drag and the release can end outside the pane.
                window.on_mouse_event({
                    let view = view.clone();
                    move |event: &MouseMoveEvent, phase, _, cx| {
                        if phase.bubble() {
                            view.update(cx, |this, cx| this.mouse_moved(event, cx));
                        }
                    }
                });
                window.on_mouse_event({
                    let view = view.clone();
                    move |_: &MouseUpEvent, phase, _, cx| {
                        if phase.bubble() {
                            view.update(cx, |this, cx| this.mouse_released(cx));
                        }
                    }
                });
            },
        )
        .absolute()
        .size_full();

        let terminal = div()
            .id("terminal")
            .role(Role::Terminal)
            .aria_label(format!("Terminal {}", self.name))
            .track_focus(&self.focus)
            .key_context("Terminal")
            .relative()
            .size_full()
            .overflow_hidden()
            .bg(self.palette.background)
            .text_color(self.palette.foreground)
            .font_family(self.typeface.family)
            .text_size(px(self.typeface.size))
            .line_height(px(line_height))
            .when(self.hover.is_some(), |pane| pane.cursor_pointer())
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, _, cx| this.key_down(event, cx)))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, window, cx| {
                    this.mouse_down(event, window, cx)
                }),
            )
            .on_scroll_wheel(
                cx.listener(|this, event: &ScrollWheelEvent, _, cx| this.wheel(event, cx)),
            )
            .child(measure)
            .child(
                div()
                    .flex()
                    .flex_col()
                    .p(px(PADDING))
                    .size_full()
                    .children(lines),
            );

        // The bar sits beside the pane, not in it, so keys typed in its field never reach the
        // terminal.
        div()
            .relative()
            .size_full()
            .on_key_down(cx.listener(|this, event: &KeyDownEvent, window, cx| {
                this.find_key(event, window, cx)
            }))
            .on_modifiers_changed(cx.listener(|this, event: &ModifiersChangedEvent, _, cx| {
                if let Some(pointer) = this.pointer {
                    this.update_hover(pointer, event.modifiers.platform, cx);
                }
            }))
            .child(terminal)
            .children(self.render_find_bar(window, cx))
    }
}
