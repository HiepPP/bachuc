//! The terminal pane: it owns an emulator, feeds it the daemon's frames, sizes it to the pane,
//! and turns keys, the mouse, and the clipboard into terminal input.

use crate::emulator::{Emulator, Palette};
use crate::keys::{key_bytes, paste_bytes};
use crate::size::{PADDING, cell_at, grid_size};
use alacritty_terminal::term::TermMode;
use bachuc_client::{Client, TerminalFrame, TerminalHandle};
use futures::StreamExt;
use gpui_kit::component::theme::Theme;
use gpui_kit::*;
use std::cell::Cell;
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

/// What a terminal view tells its owner.
#[derive(Clone, Debug, PartialEq)]
pub enum TerminalEvent {
    /// The daemon terminal is open; this is its id.
    Opened { id: String },
    /// The terminal process ended or the terminal was killed.
    Exited,
}

impl EventEmitter<TerminalEvent> for TerminalView {}

enum Source {
    Create(String),
    Attach(String),
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
        self.emulator
            .start_selection(row, col, right, event.click_count);
        self.dragging = true;
        cx.notify();
    }

    fn mouse_moved(&mut self, position: Point<Pixels>, cx: &mut Context<Self>) {
        if !self.dragging {
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

impl Render for TerminalView {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Metrics {
            cell_width,
            line_height,
        } = self.metrics;
        let lines: Vec<AnyElement> = self
            .emulator
            .lines()
            .map(|line| {
                div()
                    .h(px(line_height))
                    .w_full()
                    .flex_none()
                    .whitespace_nowrap()
                    .child(StyledText::new(line.text.clone()).with_highlights(line.runs.clone()))
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
                            view.update(cx, |this, cx| this.mouse_moved(event.position, cx));
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

        div()
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
            )
    }
}
