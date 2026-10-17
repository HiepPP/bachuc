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
//!
//! The view searches files in the Files scope (`open_files_scope`, Cmd+P) and, as Electron does,
//! in the full list for a non-empty query. The command crate has no daemon client, so the view
//! emits `CommandFileEvent::FileQuery` after a typing pause, the app answers with
//! `set_file_rows` (or `set_file_results`) or `set_file_error`, and a pick emits
//! `CommandFileEvent::OpenFile`. The app removes the view after `OpenFile` too. The full list
//! shows the files as a last Files section; the Files scope shows only them. Backspace in an
//! empty field, or the scope chip, goes back to the full list.

use crate::chord::display_combos;
use crate::plugin_rows::{self, PluginRow};
use crate::search::{Layout, Row, SearchIndex, step};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{CommandInput, PluginHost, PluginHostEvent};
use bachuc_theme::metrics::{
    BORDER_WIDTH_1, FONT_BASE, FONT_SM, ICON_MD, ICON_SM, ICON_XS, RADIUS_LG, RADIUS_MD, SPACE_0_5,
    SPACE_1, SPACE_1_5, SPACE_2, SPACE_3, SPACE_4, SPACE_6,
};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{Sizable as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::sync::Arc;
use std::time::Duration;

/// The placeholder of the search field, as in the Electron app.
const PLACEHOLDER: &str = "Search commands, files, workspaces, and agents...";
/// The placeholder in the Files scope, `shell.commandCenter.filePlaceholder`.
const FILE_PLACEHOLDER: &str = "Search files...";
/// The section title and scope chip label of the Files scope.
const FILES: &str = "Files";
/// The typing pause before a file search, `FILE_SEARCH_DEBOUNCE_MS` in
/// `workspace-file-search.ts`. An empty query searches at once.
const FILE_SEARCH_DEBOUNCE: Duration = Duration::from_millis(100);
/// The status slot right of the input, `fileSearchStatus`.
const SPIN: Duration = Duration::from_millis(1000);
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

/// What the palette asks of its owner in the Files scope. A separate type from `CommandEvent`,
/// which the app matches exhaustively.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum CommandFileEvent {
    /// Search the workspace files for `query`: files only, at most 100. The app answers with
    /// `CommandCenter::set_file_rows` or `CommandCenter::set_file_error`.
    FileQuery { query: String },
    /// A file row was picked: open `path` in a file tab, then remove the view.
    OpenFile { path: String },
}

/// One file of a file search answer: a workspace-relative path and its file icon, which the app
/// takes from `bachuc_explorer::file_icon`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct FileResult {
    pub path: String,
    pub icon: Arc<Image>,
}

/// What the palette lists.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
enum Scope {
    /// Actions, workspaces, agents, and plugin rows.
    #[default]
    All,
    /// Only the files of the app's last answer.
    Files,
}

/// Whether the view searches files: in the Files scope, or for a non-empty query in the full
/// list (`command-center.tsx`, `useWorkspaceFileSearch` `enabled`).
fn searches_files(scope: Scope, query: &str) -> bool {
    scope == Scope::Files || !query.trim().is_empty()
}

/// The scope after a key press in the search field, or None when the key does not change it.
/// Backspace in an empty field leaves the Files scope, as in Electron.
fn scope_after_key(scope: Scope, key: &str, query: &str) -> Option<Scope> {
    (scope == Scope::Files && key == "backspace" && query.trim().is_empty()).then_some(Scope::All)
}

/// How long to wait after a change before the file search.
fn file_query_delay(query: &str) -> Duration {
    if query.trim().is_empty() {
        Duration::ZERO
    } else {
        FILE_SEARCH_DEBOUNCE
    }
}

/// One file row: the icon, the name in front, the folder after it in the muted color.
#[derive(Clone, Debug, PartialEq, Eq)]
struct FileRow {
    path: String,
    name: SharedString,
    directory: SharedString,
    /// None draws the generic file icon (`set_file_results` passes no icons).
    icon: Option<Arc<Image>>,
}

/// `describeWorkspaceFilePath`: backslashes become slashes, then the last segment is the name.
fn file_row(path: &str) -> FileRow {
    let normalized = path.replace('\\', "/");
    let (directory, name) = match normalized.rfind('/') {
        Some(separator) => (
            normalized[..separator].to_string(),
            normalized[separator + 1..].to_string(),
        ),
        None => (String::new(), normalized.clone()),
    };
    FileRow {
        path: normalized,
        name: name.into(),
        directory: directory.into(),
        icon: None,
    }
}

/// Adds the Files section after the sections of `layout`: the files in the daemon's order. The
/// daemon already matched them, so nothing is filtered or ranked here. File `n` takes the item
/// index `first + n`, so it never collides with an index into the items.
fn with_files(mut layout: Layout, first: usize, count: usize) -> Layout {
    if count == 0 {
        return layout;
    }
    let divider = !layout.rows.is_empty();
    layout.rows.push(Row::Section {
        title: Some(FILES.to_string()),
        divider,
        height: if divider {
            crate::search::DIVIDED_SECTION_HEIGHT
        } else {
            crate::search::TITLED_SECTION_HEIGHT
        },
    });
    for file in 0..count {
        layout.row_of.push(layout.rows.len());
        layout.order.push(first + file);
        layout.rows.push(Row::Item { item: first + file });
    }
    layout
}

/// The file that a layout item index stands for, or None for an item. The Files scope lists only
/// files from index 0; the full list puts them after its `item_count` items.
fn file_of(scope: Scope, item_count: usize, item: usize) -> Option<usize> {
    match scope {
        Scope::Files => Some(item),
        Scope::All => item.checked_sub(item_count),
    }
}

/// What the highlight points at, so a new layout can find the same row again.
#[derive(Clone, Debug, PartialEq, Eq)]
enum Selected {
    Item(String),
    File(String),
}

/// The layout of a scope and the position to highlight: the row of `selected` when the layout
/// still has it, else the first row.
fn layout_of(
    scope: Scope,
    index: &SearchIndex,
    items: &[CommandItem],
    query: &str,
    files: &[FileRow],
    selected: Option<&Selected>,
) -> (Layout, Option<usize>) {
    let (base, active, first) = match scope {
        Scope::All => {
            let id = match selected {
                Some(Selected::Item(id)) => Some(id.as_str()),
                _ => None,
            };
            let (layout, active) = plugin_rows::layout_keeping(index, items, query, id);
            (layout, active, items.len())
        }
        Scope::Files => (Layout::default(), None, 0),
    };
    let first_file = base.order.len();
    let layout = with_files(base, first, files.len());
    let file = match selected {
        Some(Selected::File(path)) => files
            .iter()
            .position(|file| &file.path == path)
            .map(|file| first_file + file),
        _ => None,
    };
    let active = file.or(active).or((!layout.order.is_empty()).then_some(0));
    (layout, active)
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
    scope: Scope,
    /// The rows of the last file answer. `file_of` maps a layout item to one of them.
    files: Vec<FileRow>,
    /// The message of the last failed file search, drawn as one line above the list.
    file_error: Option<SharedString>,
    /// A file search is waiting for its answer.
    file_loading: bool,
    /// The pending `FileQuery`; replacing it drops the older one.
    file_query: Option<Task<()>>,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<CommandEvent> for CommandCenter {}
impl EventEmitter<CommandFileEvent> for CommandCenter {}

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
            scope: Scope::All,
            files: Vec::new(),
            file_error: None,
            file_loading: false,
            file_query: None,
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
        self.relayout(None, cx);
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
        // Read before the rebuild: the layout indexes into the old items.
        let selected = self.selected();
        self.rebuild_items();
        if self.scope == Scope::Files {
            // The plugin rows are hidden in the Files scope; the next relayout picks them up.
            return;
        }
        self.relayout(selected, cx);
    }

    /// The highlighted row, by item id or file path.
    fn selected(&self) -> Option<Selected> {
        let item = *self.layout.order.get(self.active?)?;
        match file_of(self.scope, self.items.len(), item) {
            Some(file) => self
                .files
                .get(file)
                .map(|file| Selected::File(file.path.clone())),
            None => self
                .items
                .get(item)
                .map(|item| Selected::Item(item.id.clone())),
        }
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

    /// Lists only files, and asks the app for the files of the current query at once. Call it
    /// right after `new` for Cmd+P.
    pub fn open_files_scope(&mut self, window: &mut Window, cx: &mut Context<CommandCenter>) {
        if self.scope != Scope::Files {
            self.scope = Scope::Files;
            self.files.clear();
            self.query.update(cx, |input, cx| {
                input.set_placeholder(FILE_PLACEHOLDER, window, cx)
            });
            self.relayout(None, cx);
        }
        self.schedule_file_query(cx);
        self.focus(window, cx);
    }

    /// The answer to the last `FileQuery`: workspace-relative file paths in the daemon's order,
    /// drawn with the generic file icon. `set_file_rows` also passes the icons.
    pub fn set_file_results(&mut self, paths: Vec<String>, cx: &mut Context<CommandCenter>) {
        let files = paths.iter().map(|path| file_row(path)).collect();
        self.show_files(files, None, cx);
    }

    /// The answer to the last `FileQuery`: the files in the daemon's order, each with its icon.
    /// It clears the error line.
    pub fn set_file_rows(&mut self, rows: Vec<FileResult>, cx: &mut Context<CommandCenter>) {
        let files = rows
            .into_iter()
            .map(|row| FileRow {
                icon: Some(row.icon),
                ..file_row(&row.path)
            })
            .collect();
        self.show_files(files, None, cx);
    }

    /// A failed answer to the last `FileQuery`: Some shows "Error: <message>" above the list and
    /// clears the file rows, as Electron does. None only hides the line.
    pub fn set_file_error(&mut self, message: Option<String>, cx: &mut Context<CommandCenter>) {
        match message {
            Some(message) => self.show_files(Vec::new(), Some(message.into()), cx),
            None => {
                self.file_error = None;
                cx.notify();
            }
        }
    }

    /// Shows a file answer. The highlighted row stays when the new layout still has it.
    fn show_files(
        &mut self,
        files: Vec<FileRow>,
        error: Option<SharedString>,
        cx: &mut Context<Self>,
    ) {
        if !searches_files(self.scope, &self.applied) {
            // A late answer after the full-list query was cleared.
            return;
        }
        let selected = self.selected();
        self.files = files;
        self.file_error = error;
        self.file_loading = false;
        self.relayout(selected, cx);
    }

    /// Drops the file rows, the error, and the pending query.
    fn clear_file_search(&mut self) {
        self.files.clear();
        self.file_error = None;
        self.file_loading = false;
        self.file_query = None;
    }

    /// Back to the full list. Its query is the same, so the file answer for it stays; an empty
    /// query shows no files.
    fn leave_files_scope(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.scope = Scope::All;
        let query = self.query.read(cx).value().to_string();
        if !searches_files(self.scope, &query) {
            self.clear_file_search();
        }
        self.query.update(cx, |input, cx| {
            input.set_placeholder(PLACEHOLDER, window, cx)
        });
        self.relayout(None, cx);
    }

    /// Emits `FileQuery` for the current text after `file_query_delay`. A newer change drops
    /// the pending one. The emit always runs on a later tick, so an owner that subscribes right
    /// after `open_files_scope` still gets the first query.
    fn schedule_file_query(&mut self, cx: &mut Context<Self>) {
        let query = self.query.read(cx).value().to_string();
        let delay = file_query_delay(&query);
        self.file_loading = true;
        self.file_error = None;
        self.file_query = Some(cx.spawn(async move |this, cx| {
            if !delay.is_zero() {
                cx.background_executor().timer(delay).await;
            }
            let _ = this.update(cx, |this: &mut CommandCenter, cx| {
                this.file_query = None;
                cx.emit(CommandFileEvent::FileQuery { query });
            });
        }));
        cx.notify();
    }

    fn first(&self) -> Option<usize> {
        (!self.layout.order.is_empty()).then_some(0)
    }

    fn apply_query(&mut self, cx: &mut Context<Self>) {
        let value = self.query.read(cx).value().to_string();
        if value == self.applied {
            return;
        }
        if self.scope == Scope::Files {
            // The daemon matches files; the list stays until the answer comes.
            self.applied = value;
            self.active = self.first();
            self.schedule_file_query(cx);
            return;
        }
        // The full list keeps the last files until the answer for the new query comes.
        if searches_files(self.scope, &value) {
            self.schedule_file_query(cx);
        } else {
            self.clear_file_search();
        }
        // Changing the query re-ranks everything, so an old selection means nothing.
        self.relayout(None, cx);
    }

    /// Lays out the current query and highlights `selected`, or the first row when it is gone.
    fn relayout(&mut self, selected: Option<Selected>, cx: &mut Context<Self>) {
        self.applied = self.query.read(cx).value().to_string();
        let (layout, active) = layout_of(
            self.scope,
            &self.index,
            &self.items,
            &self.applied,
            &self.files,
            selected.as_ref(),
        );
        self.layout = layout;
        self.list.reset(self.layout.rows.len());
        self.active = active;
        cx.notify();
    }

    fn key_down(&mut self, key: &str, window: &mut Window, cx: &mut Context<Self>) -> bool {
        if !self.query.read(cx).focus_handle(cx).is_focused(window) {
            return false;
        }
        let query = self.query.read(cx).value().to_string();
        if let Some(Scope::All) = scope_after_key(self.scope, key, &query) {
            self.leave_files_scope(window, cx);
            return true;
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
        if let Some(file) = file_of(self.scope, self.items.len(), index) {
            if let Some(file) = self.files.get(file) {
                cx.emit(CommandFileEvent::OpenFile {
                    path: file.path.clone(),
                });
            }
            return;
        }
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
            Some(Row::Item { item }) => match file_of(self.scope, self.items.len(), *item) {
                Some(file) => self.render_file(row, file, cx),
                None => self.render_item(row, *item, cx),
            },
            None => div().into_any_element(),
        }
    }

    /// A file row, as `ResultContent` draws a `file` result: an icon slot, then the name and
    /// the folder on one 20 px line.
    fn render_file(&self, row: usize, item: usize, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let Some(file) = self.files.get(item) else {
            return div().into_any_element();
        };
        let position = self.layout.row_of.iter().position(|r| *r == row);
        let active = position.is_some() && position == self.active;
        div()
            .id(("command-file", item))
            .role(Role::Button)
            .aria_label(SharedString::from(file.path.clone()))
            .w_full()
            .h(px(crate::search::ROW_HEIGHT))
            .px(px(SPACE_4))
            .flex()
            .flex_row()
            .items_center()
            .gap(px(SPACE_3))
            .cursor_pointer()
            .when(active, |this| this.bg(colors.surface1))
            .hover(|style| style.bg(colors.surface1))
            .on_click(cx.listener(move |this, _, window, cx| {
                if let Some(position) = position {
                    this.run(position, window, cx);
                }
            }))
            .on_a11y_action(AccessibleAction::Click, {
                let view = cx.weak_entity();
                move |_, window, cx| {
                    if let Some(position) = position {
                        view.update(cx, |this, cx| this.run(position, window, cx))
                            .ok();
                    }
                }
            })
            .child(
                div()
                    .w(px(ICON_MD))
                    .h(px(20.))
                    .flex()
                    .items_center()
                    .justify_center()
                    .flex_shrink_0()
                    .map(|slot| match &file.icon {
                        // `MaterialFileIcon` at size 16.
                        Some(image) => slot.child(img(image.clone()).size(px(ICON_MD))),
                        None => slot.child(
                            icon(IconName::File)
                                .with_size(px(ICON_MD))
                                .text_color(colors.foreground_muted),
                        ),
                    }),
            )
            .child(
                h_flex()
                    .flex_1()
                    .min_w_0()
                    .overflow_hidden()
                    .gap(px(SPACE_1))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .line_height(px(20.))
                    .child(
                        div()
                            .flex_shrink_0()
                            .max_w_full()
                            .truncate()
                            .text_color(colors.foreground)
                            .child(file.name.clone()),
                    )
                    .when(!file.directory.is_empty(), |this| {
                        this.child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_color(colors.foreground_muted)
                                .child(file.directory.clone()),
                        )
                    }),
            )
            .into_any_element()
    }

    /// The Files chip left of the input; a press leaves the scope. It fills the input's line
    /// box, so the header keeps its height.
    fn render_scope_chip(&self, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        h_flex()
            .id("command-center-files-scope")
            .role(Role::Button)
            .aria_label(FILES)
            .flex_shrink_0()
            .h_full()
            .items_center()
            .gap(px(SPACE_1_5))
            .mr(px(SPACE_1))
            .px(px(SPACE_2))
            .rounded(px(RADIUS_MD))
            .border_1()
            .border_color(colors.border_accent)
            .bg(colors.surface2)
            .cursor_pointer()
            .on_click(cx.listener(|this, _, window, cx| this.leave_files_scope(window, cx)))
            .on_a11y_action(AccessibleAction::Click, {
                let view = cx.weak_entity();
                move |_, window, cx| {
                    view.update(cx, |this, cx| this.leave_files_scope(window, cx))
                        .ok();
                }
            })
            .child(
                icon(IconName::Folder)
                    .with_size(px(ICON_SM))
                    .text_color(colors.foreground),
            )
            .child(
                div()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .line_height(px(17.))
                    .font_weight(FontWeight::MEDIUM)
                    .text_color(colors.foreground)
                    .child(FILES),
            )
            .child(
                icon(IconName::X)
                    .with_size(px(ICON_XS))
                    .text_color(colors.foreground),
            )
            .into_any_element()
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
        let files = self.scope == Scope::Files;
        let body =
            if self.layout.rows.is_empty() && (self.file_loading || self.file_error.is_some()) {
                // As Electron: no "No matches" while a file search runs or after it failed.
                div().flex_1().into_any_element()
            } else if self.layout.rows.is_empty() {
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
                            .flex()
                            .flex_row()
                            .items_center()
                            .gap(px(SPACE_2))
                            .when(files, |this| this.child(self.render_scope_chip(cx)))
                            .child(
                                div().flex_1().min_w_0().child(
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
                                        .aria_label(if files {
                                            "Search files"
                                        } else {
                                            "Search commands"
                                        }),
                                ),
                            )
                            // The slot stays in both scopes, as `FileSearchLoadingIndicator`.
                            .child(
                                div()
                                    .id("command-center-file-search-status")
                                    .w(px(ICON_SM))
                                    .h(px(ICON_SM))
                                    .flex_shrink_0()
                                    .flex()
                                    .items_center()
                                    .justify_center()
                                    .when(self.file_loading, |this| {
                                        this.role(Role::ProgressIndicator)
                                            .aria_label("Searching files...")
                                            .child(
                                                icon(IconName::LoaderCircle)
                                                    .with_size(px(ICON_SM))
                                                    .text_color(colors.foreground_muted)
                                                    .with_animation(
                                                        "command-center-file-spin",
                                                        Animation::new(SPIN).repeat(),
                                                        |icon, delta| {
                                                            icon.transform(Transformation::rotate(
                                                                percentage(delta),
                                                            ))
                                                        },
                                                    ),
                                            )
                                    }),
                            ),
                    )
                    .when_some(self.file_error.clone(), |this, message| {
                        this.child(
                            div()
                                .id("command-center-file-search-error")
                                .role(Role::Alert)
                                .aria_label(SharedString::from(format!("Error: {message}")))
                                .flex_shrink_0()
                                .px(px(SPACE_4))
                                .py(px(SPACE_3))
                                .border_b_1()
                                .border_color(colors.border)
                                .text_size(ui_font_size(FONT_BASE, cx))
                                .line_height(px(20.))
                                .text_color(colors.status_danger)
                                .child(format!("Error: {message}")),
                        )
                    })
                    .child(body),
            )
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CommandItem, Duration, FONT_BASE, Layout, Row, Scope, Selected, file_of, file_query_delay,
        file_row, header_height, input_height, input_line_height, layout_of, scope_after_key,
        searches_files, with_files,
    };
    use crate::search::SearchIndex;

    fn item(id: &str, title: &str, group: &str) -> CommandItem {
        CommandItem {
            id: id.into(),
            title: title.to_string().into(),
            group: group.to_string().into(),
            icon: None,
            shortcut: None,
        }
    }

    fn titles(layout: &Layout) -> Vec<(Option<String>, bool)> {
        layout
            .rows
            .iter()
            .filter_map(|row| match row {
                Row::Section { title, divider, .. } => Some((title.clone(), *divider)),
                Row::Item { .. } => None,
            })
            .collect()
    }

    #[test]
    fn scope_backspace_in_an_empty_field_leaves_the_files_scope() {
        assert_eq!(
            scope_after_key(Scope::Files, "backspace", ""),
            Some(Scope::All)
        );
        assert_eq!(
            scope_after_key(Scope::Files, "backspace", "  "),
            Some(Scope::All)
        );
        assert_eq!(scope_after_key(Scope::Files, "backspace", "src"), None);
        assert_eq!(scope_after_key(Scope::Files, "a", ""), None);
        assert_eq!(scope_after_key(Scope::All, "backspace", ""), None);
    }

    #[test]
    fn scope_file_query_waits_100_ms_unless_empty() {
        assert_eq!(file_query_delay(""), Duration::ZERO);
        assert_eq!(file_query_delay("   "), Duration::ZERO);
        assert_eq!(file_query_delay("main"), Duration::from_millis(100));
    }

    #[test]
    fn scope_file_rows_split_name_and_folder() {
        let row = file_row("src\\app\\main.rs");
        assert_eq!(row.path, "src/app/main.rs");
        assert_eq!(row.name.as_ref(), "main.rs");
        assert_eq!(row.directory.as_ref(), "src/app");
        let top = file_row("README.md");
        assert_eq!(top.name.as_ref(), "README.md");
        assert_eq!(top.directory.as_ref(), "");
    }

    #[test]
    fn scope_files_layout_keeps_the_daemon_order_under_one_section() {
        assert!(with_files(Layout::default(), 0, 0).rows.is_empty());
        let layout = with_files(Layout::default(), 0, 3);
        assert!(matches!(
            &layout.rows[0],
            Row::Section { title: Some(title), divider: false, .. } if title == "Files"
        ));
        assert_eq!(layout.order, [0, 1, 2]);
        assert_eq!(layout.row_of, [1, 2, 3]);
        assert_eq!(file_of(Scope::Files, 5, 2), Some(2));
    }

    #[test]
    fn files_search_in_the_files_scope_or_for_a_typed_query() {
        assert!(searches_files(Scope::Files, ""));
        assert!(searches_files(Scope::All, "main"));
        assert!(!searches_files(Scope::All, ""));
        assert!(!searches_files(Scope::All, "  "));
    }

    #[test]
    fn full_list_puts_the_files_section_after_the_other_sections() {
        let items = [
            item("action:main", "Open main window", "Actions"),
            item("agent:1", "Fix main loop", "Agents"),
        ];
        let index = SearchIndex::new(items.iter().map(|i| (i.title.as_ref(), i.group.as_ref())));
        let files = [file_row("src/main.rs"), file_row("main.go")];
        let (layout, active) = layout_of(Scope::All, &index, &items, "main", &files, None);
        let sections = titles(&layout);
        assert_eq!(sections.last(), Some(&(Some("Files".to_string()), true)));
        assert_eq!(sections.len(), 3);
        // Keyboard order walks the items, then the files in the daemon's order.
        assert_eq!(layout.order.len(), 4);
        assert_eq!(&layout.order[2..], [2, 3]);
        assert_eq!(file_of(Scope::All, items.len(), layout.order[2]), Some(0));
        assert_eq!(file_of(Scope::All, items.len(), layout.order[0]), None);
        assert_eq!(active, Some(0));
        // Every position in `order` points at its own item row.
        for (position, row) in layout.row_of.iter().enumerate() {
            assert_eq!(
                layout.rows[*row],
                Row::Item {
                    item: layout.order[position]
                }
            );
        }
    }

    #[test]
    fn full_list_files_alone_have_a_titled_section_without_a_divider() {
        let items = [item("action:a", "Alpha", "Actions")];
        let index = SearchIndex::new(items.iter().map(|i| (i.title.as_ref(), i.group.as_ref())));
        let files = [file_row("src/main.rs")];
        let (layout, active) = layout_of(Scope::All, &index, &items, "main", &files, None);
        assert_eq!(titles(&layout), [(Some("Files".to_string()), false)]);
        assert_eq!(layout.order, [1]);
        assert_eq!(active, Some(0));
        let (empty, active) = layout_of(Scope::All, &index, &items, "", &[], None);
        assert_eq!(titles(&empty), [(Some("Actions".to_string()), false)]);
        assert_eq!(active, Some(0));
    }

    #[test]
    fn a_file_answer_keeps_the_highlighted_row() {
        let items = [
            item("action:main", "Open main window", "Actions"),
            item("action:m2", "Main menu", "Actions"),
        ];
        let index = SearchIndex::new(items.iter().map(|i| (i.title.as_ref(), i.group.as_ref())));
        let files = [file_row("a/main.rs"), file_row("src/main.rs")];
        let file = Selected::File("src/main.rs".into());
        let (layout, active) = layout_of(Scope::All, &index, &items, "main", &files, Some(&file));
        assert_eq!(
            file_of(Scope::All, 2, layout.order[active.unwrap()]),
            Some(1)
        );
        let kept = Selected::Item("action:m2".into());
        let (layout, active) = layout_of(Scope::All, &index, &items, "main", &files, Some(&kept));
        assert_eq!(items[layout.order[active.unwrap()]].id, "action:m2");
        let gone = Selected::File("gone.rs".into());
        let (_, active) = layout_of(Scope::Files, &index, &items, "main", &files, Some(&gone));
        assert_eq!(active, Some(0));
        let (_, active) = layout_of(Scope::Files, &index, &items, "main", &[], None);
        assert_eq!(active, None);
    }

    #[test]
    fn header_matches_the_electron_50_px_header() {
        // Electron: 12 + (4 + 17 + 4) + 12 + 1 border, measured at y 48 to y 98.
        assert_eq!(input_line_height(FONT_BASE), 17.);
        assert_eq!(input_height(FONT_BASE), 25.);
        assert_eq!(header_height(FONT_BASE), 50.);
    }
}
