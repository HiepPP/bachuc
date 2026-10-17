//! The command center: a modal palette over the window, as `command-center.tsx` draws it.
//!
//! The view knows no app internals. The app passes `CommandItem`s, shows the view above its
//! content, and handles `CommandEvent`: `Run` carries the picked id, `Dismissed` means Escape or a
//! click outside. The app removes the view after either event; the view never closes itself.
//!
//! The view also lists the Command Center items of the active plugins, read from the plugin host
//! global, before the app's items. It runs a plugin row itself and then emits `Run` with an id
//! that starts with `plugin:`, which the app ignores. The app calls `set_plugin_context` to tell
//! the view which workspace and agent are open.

use crate::chord::display_combos;
use crate::plugin_rows::{self, PluginRow};
use crate::search::{Layout, Row, SearchIndex, step};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{CommandInput, PluginHost, PluginHostEvent};
use bachuc_theme::metrics::{
    BORDER_WIDTH_1, FONT_BASE, FONT_SM, ICON_MD, RADIUS_LG, RADIUS_MD, SPACE_0_5, SPACE_1, SPACE_2,
    SPACE_3, SPACE_4, SPACE_6,
};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::h_flex;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// The placeholder of the search field, as in the Electron app.
const PLACEHOLDER: &str = "Search commands, files, workspaces, and agents...";
/// Panel size and the share of the window it may take, from `command-center.tsx`.
const PANEL_WIDTH: f32 = 640.;
const PANEL_HEIGHT: f32 = 560.;
const PANEL_MAX_WIDTH: f32 = 0.92;
const PANEL_MAX_HEIGHT: f32 = 0.8;
/// The Electron input has no line height, so the browser uses `normal`: about 1.2 times the font
/// size, 17 px at FONT_BASE.
const INPUT_LINE_HEIGHT: f32 = 1.2;
/// How far the list draws beyond the visible rows.
const OVERDRAW: f32 = 400.;

/// One row of the palette. The app builds the list and runs the picked id.
#[derive(Clone, Debug, PartialEq)]
pub struct CommandItem {
    /// Returned in `CommandEvent::Run`.
    pub id: String,
    /// The row text. The search matches it.
    pub title: SharedString,
    /// The section the row sits in. Rows with the same group share a section; the search also
    /// matches it, below the title. An empty group has no section title.
    pub group: SharedString,
    pub icon: Option<IconName>,
    /// An Electron combo string such as `Cmd+Shift+P`, shown as a hint on the row.
    pub shortcut: Option<String>,
}

/// What the palette tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandEvent {
    /// A row was picked, by Enter or a click.
    Run { id: String },
    /// Escape, or a click outside the panel.
    Dismissed,
}

pub struct CommandCenter {
    /// The rows that the app passed.
    app_items: Vec<CommandItem>,
    /// The plugin rows. They are the first `plugin_rows.len()` entries of `items`, and carry the
    /// hidden keywords and the handlers.
    plugin_rows: Vec<PluginRow>,
    /// The plugin rows, then `app_items`: what the layout indexes into.
    items: Vec<CommandItem>,
    workspace_id: Option<String>,
    agent_id: Option<String>,
    index: SearchIndex,
    layout: Layout,
    query: Entity<InputState>,
    /// The query the layout was built for.
    applied: String,
    /// The highlighted position in `layout.order`.
    active: Option<usize>,
    list: ListState,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<CommandEvent> for CommandCenter {}

/// The rows of the active plugins for this context. Before `set_plugin_context`, both ids are
/// None, so only `Global` items show.
fn plugin_rows_of(workspace_id: Option<&str>, agent_id: Option<&str>, cx: &App) -> Vec<PluginRow> {
    let Some(host) = bachuc_plugin_host::host(cx) else {
        return Vec::new();
    };
    plugin_rows::plugin_rows(
        host.read(cx).index().command_items(),
        workspace_id,
        agent_id,
        |item| item.visible.as_ref().is_none_or(|visible| visible(cx)),
    )
}

impl CommandCenter {
    pub fn new(
        items: Vec<CommandItem>,
        window: &mut Window,
        cx: &mut Context<CommandCenter>,
    ) -> CommandCenter {
        let query = cx.new(|cx| InputState::new(window, cx).placeholder(PLACEHOLDER));
        let rows = plugin_rows_of(None, None, cx);
        let all = plugin_rows::combine(&rows, &items);
        let index = plugin_rows::index_for(&rows, &all);
        let layout = index.layout("");
        let list = ListState::new(layout.rows.len(), ListAlignment::Top, px(OVERDRAW));
        let weak = cx.weak_entity();
        let mut subscriptions = vec![
            cx.subscribe_in(
                &query,
                window,
                |this: &mut CommandCenter, _, event: &InputEvent, window, cx| match event {
                    InputEvent::Change => this.apply_query(cx),
                    InputEvent::PressEnter { .. } => this.run_active(window, cx),
                    _ => {}
                },
            ),
            // The input takes the arrow keys and Escape for itself, so the palette reads them
            // before dispatch. Only while the input has focus.
            cx.intercept_keystrokes(move |event, window, cx| {
                let modifiers = &event.keystroke.modifiers;
                if modifiers.platform || modifiers.control || modifiers.alt || modifiers.shift {
                    return;
                }
                let key = event.keystroke.key.clone();
                let handled = weak
                    .update(cx, |this, cx| this.key_down(&key, window, cx))
                    .unwrap_or(false);
                if handled {
                    cx.stop_propagation();
                }
            }),
        ];
        // A plugin may become active or change what it shows while the palette is open.
        if let Some(host) = bachuc_plugin_host::host(cx) {
            subscriptions.push(cx.subscribe(
                &host,
                |this: &mut CommandCenter, _: Entity<PluginHost>, event: &PluginHostEvent, cx| {
                    if matches!(
                        event,
                        PluginHostEvent::ContributionsChanged
                            | PluginHostEvent::PluginStateChanged { .. }
                    ) {
                        this.refresh_plugin_rows(cx);
                    }
                },
            ));
        }
        let mut center = CommandCenter {
            app_items: items,
            plugin_rows: rows,
            items: all,
            workspace_id: None,
            agent_id: None,
            index,
            layout,
            query,
            applied: String::new(),
            active: None,
            list,
            _subscriptions: subscriptions,
        };
        center.active = center.first();
        center.query.update(cx, |input, cx| input.focus(window, cx));
        center
    }

    /// Replaces the rows, for example when agents come and go while the palette is open. The
    /// query stays and the highlight returns to the first row.
    pub fn set_items(&mut self, items: Vec<CommandItem>, cx: &mut Context<Self>) {
        self.app_items = items;
        self.rebuild_items();
        self.relayout(cx);
    }

    /// Tells the view which workspace and agent are open, which gates the plugin items. Call it
    /// after `new`; before that, only `Global` items show.
    pub fn set_plugin_context(
        &mut self,
        workspace_id: Option<String>,
        agent_id: Option<String>,
        cx: &mut Context<CommandCenter>,
    ) {
        if self.workspace_id == workspace_id && self.agent_id == agent_id {
            return;
        }
        self.workspace_id = workspace_id;
        self.agent_id = agent_id;
        self.refresh_plugin_rows(cx);
    }

    /// Reads the plugin rows again. The query and the selected row stay; the layout resets only
    /// when a row changed what it draws or matches.
    fn refresh_plugin_rows(&mut self, cx: &mut Context<Self>) {
        let rows = plugin_rows_of(self.workspace_id.as_deref(), self.agent_id.as_deref(), cx);
        let changed = !plugin_rows::same_rows(&self.plugin_rows, &rows);
        // A handler may change without a visible change, so the rows are always replaced.
        self.plugin_rows = rows;
        if !changed {
            return;
        }
        self.rebuild_items();
        let selected = self
            .active
            .and_then(|position| self.layout.order.get(position))
            .and_then(|item| self.items.get(*item))
            .map(|item| item.id.clone());
        self.applied = self.query.read(cx).value().to_string();
        let (layout, active) = plugin_rows::layout_keeping(
            &self.index,
            &self.items,
            &self.applied,
            selected.as_deref(),
        );
        self.layout = layout;
        self.list.reset(self.layout.rows.len());
        self.active = active;
        cx.notify();
    }

    fn rebuild_items(&mut self) {
        self.items = plugin_rows::combine(&self.plugin_rows, &self.app_items);
        self.index = plugin_rows::index_for(&self.plugin_rows, &self.items);
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.query.update(cx, |input, cx| input.focus(window, cx));
    }

    /// The text in the search field.
    pub fn query(&self, cx: &App) -> String {
        self.query.read(cx).value().to_string()
    }

    fn first(&self) -> Option<usize> {
        (!self.layout.order.is_empty()).then_some(0)
    }

    fn apply_query(&mut self, cx: &mut Context<Self>) {
        let value = self.query.read(cx).value().to_string();
        if value == self.applied {
            return;
        }
        self.relayout(cx);
    }

    fn relayout(&mut self, cx: &mut Context<Self>) {
        self.applied = self.query.read(cx).value().to_string();
        self.layout = self.index.layout(&self.applied);
        self.list.reset(self.layout.rows.len());
        // Changing the query re-ranks everything, so an old selection means nothing.
        self.active = self.first();
        cx.notify();
    }

    fn key_down(&mut self, key: &str, window: &Window, cx: &mut Context<Self>) -> bool {
        if !self.query.read(cx).focus_handle(cx).is_focused(window) {
            return false;
        }
        match key {
            "down" | "up" => {
                let delta = if key == "down" { 1 } else { -1 };
                self.active = step(self.active, self.layout.order.len(), delta);
                if let Some(row) = self.active.map(|position| self.layout.row_of[position]) {
                    self.list.scroll_to_reveal_item(row);
                }
                cx.notify();
                true
            }
            "escape" => {
                cx.emit(CommandEvent::Dismissed);
                true
            }
            _ => false,
        }
    }

    fn run_active(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if let Some(position) = self.active {
            self.run(position, window, cx);
        }
    }

    fn run(&mut self, position: usize, window: &mut Window, cx: &mut Context<Self>) {
        let Some(&index) = self.layout.order.get(position) else {
            return;
        };
        let Some(item) = self.items.get(index) else {
            return;
        };
        let event = CommandEvent::Run {
            id: item.id.clone(),
        };
        // The plugin rows are the first entries of `items`.
        let Some(row) = self.plugin_rows.get(index) else {
            cx.emit(event);
            return;
        };
        let handler = row.on_select.clone();
        let input = CommandInput {
            workspace_id: self.workspace_id.clone(),
            agent_id: self.agent_id.clone(),
        };
        plugin_rows::run_row(
            cx,
            event,
            |cx| handler(input, window, cx),
            |cx, event| cx.emit(event),
        );
    }

    fn render_row(&mut self, row: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        match self.layout.rows.get(row) {
            Some(Row::Section {
                title,
                divider,
                height,
            }) => v_section(*height)
                .when(*divider, |this| {
                    this.child(
                        div()
                            .h(px(1.))
                            .my(px(SPACE_2))
                            .bg(colors.border)
                            .flex_shrink_0(),
                    )
                })
                .when_some(title.clone(), |this, title| {
                    this.child(
                        div()
                            .px(px(SPACE_4))
                            .pb(px(SPACE_2))
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.foreground_muted)
                            .child(title),
                    )
                })
                .into_any_element(),
            Some(Row::Item { item }) => self.render_item(row, *item, cx),
            None => div().into_any_element(),
        }
    }

    fn render_item(&self, row: usize, item: usize, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let entry = &self.items[item];
        let position = self.layout.row_of.iter().position(|r| *r == row);
        let active = position.is_some() && position == self.active;
        let hints = entry
            .shortcut
            .as_deref()
            .map(display_combos)
            .unwrap_or_default();
        div()
            .id(("command-item", item))
            .role(Role::Button)
            .aria_label(entry.title.clone())
            .w_full()
            .h(px(crate::search::ROW_HEIGHT))
            .px(px(SPACE_4))
            .flex()
            .flex_row()
            .items_center()
            .justify_between()
            .gap(px(SPACE_3))
            .cursor_pointer()
            .when(active, |this| this.bg(colors.surface1))
            .hover(|style| style.bg(colors.surface1))
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some(position) = position {
                    this.run(position, window, cx);
                }
            }))
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .gap(px(SPACE_3))
                    // The slot stays without an icon, so every title starts on the same rail.
                    .child(
                        div()
                            .w(px(ICON_MD))
                            .h(px(20.))
                            .flex()
                            .items_center()
                            .justify_center()
                            .flex_shrink_0()
                            .children(
                                entry
                                    .icon
                                    .map(|name| icon(name).text_color(colors.foreground_muted)),
                            ),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .line_height(px(18.))
                            .text_color(colors.foreground)
                            .truncate()
                            .child(entry.title.clone()),
                    ),
            )
            .when(!hints.is_empty(), |this| {
                this.child(h_flex().flex_shrink_0().gap(px(SPACE_1)).children(
                    hints.into_iter().map(|text| {
                        div()
                            .px(px(SPACE_1))
                            .py(px(SPACE_0_5))
                            .rounded(px(RADIUS_MD))
                            .bg(colors.surface3.opacity(0.5))
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.foreground_muted)
                            .child(text)
                    }),
                ))
            })
            .into_any_element()
    }
}

/// A section row: content sits at the bottom of a fixed height, as in Electron.
/// One line of search text. GPUI rounds a line height to whole pixels, as the browser does.
fn input_line_height(font_size: f32) -> f32 {
    (font_size * INPUT_LINE_HEIGHT).round()
}

/// The search input: one line with SPACE_1 above and below, as `styles.input`.
fn input_height(font_size: f32) -> f32 {
    input_line_height(font_size) + SPACE_1 * 2.
}

/// The header: SPACE_3 above and below the input and a bottom border, as `styles.header`.
fn header_height(font_size: f32) -> f32 {
    SPACE_3 * 2. + input_height(font_size) + BORDER_WIDTH_1
}

fn v_section(height: f32) -> Div {
    div()
        .w_full()
        .h(px(height))
        .flex()
        .flex_col()
        .justify_end()
        .flex_shrink_0()
}

impl Render for CommandCenter {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let input_font = ui_font_size(FONT_BASE, cx).as_f32();
        let body = if self.layout.rows.is_empty() {
            div()
                .id("command-center-empty")
                .role(Role::Status)
                .aria_label("No matches")
                .px(px(SPACE_4))
                .py(px(SPACE_6))
                .text_center()
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(colors.foreground_muted)
                .child("No matches")
                .into_any_element()
        } else {
            div()
                .id("command-center-results")
                .role(Role::Group)
                .aria_label("Commands")
                .flex_1()
                .min_h_0()
                .child(list(self.list.clone(), cx.processor(Self::render_row)).size_full())
                .into_any_element()
        };
        div()
            .id("command-center")
            .absolute()
            .top_0()
            .left_0()
            .size_full()
            .flex()
            .flex_col()
            .items_center()
            .pt(px(SPACE_6 * 2.))
            .child(
                div()
                    .id("command-center-backdrop")
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .bg(black().opacity(0.5))
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(|_, _, _, cx| cx.emit(CommandEvent::Dismissed)),
                    ),
            )
            .child(
                div()
                    .id("command-center-panel")
                    .role(Role::Dialog)
                    .aria_label("Command center")
                    .occlude()
                    .flex()
                    .flex_col()
                    .w(px(PANEL_WIDTH))
                    .h(px(PANEL_HEIGHT))
                    .max_w(relative(PANEL_MAX_WIDTH))
                    .max_h(relative(PANEL_MAX_HEIGHT))
                    .overflow_hidden()
                    .border_1()
                    .border_color(colors.border)
                    .rounded(px(RADIUS_LG))
                    .bg(colors.surface0)
                    .text_color(colors.foreground)
                    .shadow(vec![shadow(ShadowSize::Lg, cx)])
                    .child(
                        div()
                            .flex_shrink_0()
                            .h(px(header_height(input_font)))
                            .px(px(SPACE_4))
                            .py(px(SPACE_3))
                            .border_b_1()
                            .border_color(colors.border)
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .child(
                                Input::new(&self.query)
                                    .appearance(false)
                                    .bordered(false)
                                    .focus_bordered(false)
                                    // Replace the gpui-kit Medium padding, height, and text_sm
                                    // with the Electron input: SPACE_1 above and below one
                                    // line of FONT_BASE, no side inset.
                                    .px_0()
                                    .py(px(SPACE_1))
                                    // `Input::h` sets only a multi-line height and shadows
                                    // `Styled::h`; without this the frame keeps the Medium
                                    // `h_8` (2 rem, 28 px) and the header is 53 px.
                                    .map(|input| Styled::h(input, px(input_height(input_font))))
                                    .line_height(px(input_line_height(input_font)))
                                    .text_size(ui_font_size(FONT_BASE, cx))
                                    .accessibility_id("command-center-input")
                                    .aria_label("Search commands"),
                            ),
                    )
                    .child(body),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{FONT_BASE, header_height, input_height, input_line_height};

    #[test]
    fn header_matches_the_electron_50_px_header() {
        // Electron: 12 + (4 + 17 + 4) + 12 + 1 border, measured at y 48 to y 98.
        assert_eq!(input_line_height(FONT_BASE), 17.);
        assert_eq!(input_height(FONT_BASE), 25.);
        assert_eq!(header_height(FONT_BASE), 50.);
    }
}
