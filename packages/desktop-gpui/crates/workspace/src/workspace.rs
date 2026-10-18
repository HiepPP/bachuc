//! The `Workspace` view: renders the split tree, one Electron tab bar per group, and the dividers.
//!
//! Render never touches a pane view; it only places the active item's view. Every change happens
//! in an event handler or a public method, and the one layout read that a frame cannot answer
//! itself, the root size, comes from a canvas and re-renders on the next frame.

use crate::actions::{CloseActiveTab, NewTab, OpenTabMenu, SplitDown, SplitRight};
use crate::model::{
    DIVIDER, DIVIDER_HIGHLIGHT, DIVIDER_HIT, Group, Layout, Node, NodeId, Rect, Split, SplitAxis,
    child_rects, divider_inset,
};
use crate::tab_layout::{
    self, SPLIT_ACTIONS_MARGIN, SPLIT_ACTIONS_PADDING, TAB_CHIP_GAP, TAB_CHIP_PADDING,
    TAB_CONTENT_GAP, TAB_DROP_INDICATOR_WIDTH, TAB_ICON_WIDTH, TAB_MODIFIED_DOT, TAB_ROW_PADDING,
    TOOLBAR_BUTTON, TabLayout, label_allowance,
};
use crate::tab_menu::{self, TabExtras, TabKind, TabMenuAction, TabMenuEntry, TabMenuItem};
use crate::{KEY_CONTEXT, PaneEvent, PaneItem, TabMenuEvent};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    CONTROL_XS, FONT_BASE, ICON_SM, ICON_XS, RADIUS_MD, RADIUS_SM, SPACE_0_5, SPACE_1,
    TAB_BAR_HEIGHT,
};
use bachuc_theme::{Tokens, tokens, ui_font_size};
use gpui_kit::component::menu::{DropdownMenu as _, PopupMenu, PopupMenuItem};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Selectable, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::{Cell, RefCell};
use std::collections::{HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

type MenuBuilder = Rc<dyn Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu>;

/// The close button overlay and its scrim (`tabTrailingOverlay`, `TrailingActionScrim`).
const CLOSE_OVERLAY_WIDTH: f32 = 48.;
const CLOSE_SCRIM_SOLID: f32 = 0.55;
const CLOSE_BUTTON: f32 = 18.;
const SPLIT_MENU_WIDTH: f32 = 220.;
const TAB_MENU_WIDTH: f32 = 220.;
/// The pointer rests this long on a tab before its title shows (`delayDuration={400}`).
const TAB_TOOLTIP_DELAY: Duration = Duration::from_millis(400);

#[derive(Clone)]
struct TabDrag {
    key: String,
    title: SharedString,
    icon: IconName,
    logo: Option<Arc<Image>>,
}

/// The open tab context menu and what keeps it alive.
struct OpenTabMenuState {
    menu: Entity<PopupMenu>,
    position: Point<Pixels>,
    _dismiss: Subscription,
}

#[derive(Clone, Copy)]
struct DividerDrag {
    split: NodeId,
    index: usize,
}

/// What follows the pointer during a drag: a tab chip, or nothing for a divider.
enum DragPreview {
    Tab {
        title: SharedString,
        icon: IconName,
        logo: Option<Arc<Image>>,
    },
    Divider,
}

impl Render for DragPreview {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let DragPreview::Tab {
            title,
            icon: name,
            logo,
        } = self
        else {
            return div().into_any_element();
        };
        let t = tokens(cx);
        h_flex()
            .h(px(CONTROL_XS))
            .px(px(TAB_CHIP_PADDING))
            .gap(px(TAB_CONTENT_GAP))
            .rounded(px(RADIUS_MD))
            .bg(t.surface2)
            .opacity(0.85)
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_color(t.foreground)
            .child(tab_glyph(*name, logo.as_ref(), t.foreground))
            .child(title.clone())
            .into_any_element()
    }
}

/// The tab's provider logo when it has one, else its icon.
fn tab_glyph(name: IconName, logo: Option<&Arc<Image>>, color: Hsla) -> AnyElement {
    match logo {
        Some(logo) => img(logo.clone())
            .flex_none()
            .size(px(TAB_ICON_WIDTH))
            .into_any_element(),
        None => icon(name)
            .with_size(px(TAB_ICON_WIDTH))
            .text_color(color)
            .into_any_element(),
    }
}

/// An icon button that can anchor a gpui-kit dropdown menu and fills while the menu is open.
#[derive(IntoElement)]
struct ChromeButton {
    base: Stateful<Div>,
    open: bool,
    fill: Hsla,
}

impl Selectable for ChromeButton {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl Styled for ChromeButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for ChromeButton {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl gpui_kit::component::menu::DropdownMenu for ChromeButton {}

impl RenderOnce for ChromeButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let fill = self.fill;
        self.base.when(self.open, |button| button.bg(fill))
    }
}

/// A title on one line, because the text system shapes one line at a time.
fn one_line(title: &SharedString) -> SharedString {
    if title.contains('\n') {
        title.replace('\n', " ").into()
    } else {
        title.clone()
    }
}

/// One context menu row. A hint or a tooltip needs a custom row; the rest are plain items.
fn menu_item(item: &TabMenuItem, muted: Hsla) -> PopupMenuItem {
    let glyph = icon(item.icon).with_size(px(ICON_SM)).text_color(muted);
    let base = if item.hint.is_some() || item.tooltip.is_some() {
        let (key, label, hint, tooltip) = (item.key, item.label, item.hint.clone(), item.tooltip);
        PopupMenuItem::element(move |_, _| {
            h_flex()
                .id(key)
                .w_full()
                .gap(px(SPACE_1))
                .justify_between()
                .child(label)
                .when_some(hint.clone(), |row, hint| {
                    row.child(div().text_color(muted).child(hint))
                })
                .when_some(tooltip, |row, tip| {
                    row.tooltip(move |window, cx| Tooltip::new(tip).build(window, cx))
                })
        })
    } else {
        PopupMenuItem::new(item.label)
    };
    base.icon(glyph).disabled(item.disabled)
}

pub struct Workspace {
    layout: Layout<PaneItem>,
    focus_handle: FocusHandle,
    /// The root bounds of the last frame, written by a canvas.
    bounds: Rc<Cell<Bounds<Pixels>>>,
    hovered_tab: Option<String>,
    hovered_close: Option<String>,
    hovered_divider: Option<(NodeId, usize)>,
    dragging_divider: Option<(NodeId, usize)>,
    drop_target: Option<(NodeId, usize)>,
    label_widths: HashMap<SharedString, f32>,
    label_font_size: Pixels,
    tab_layouts: HashMap<NodeId, TabLayout>,
    scroll: HashMap<NodeId, ScrollHandle>,
    new_tab_menu: Option<MenuBuilder>,
    extras: TabExtras,
    tab_menu: Option<OpenTabMenuState>,
    /// The bounds of each tab chip in the last frame, for the keyboard menu key.
    tab_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
}

impl EventEmitter<PaneEvent> for Workspace {}
impl EventEmitter<TabMenuEvent> for Workspace {}

impl Focusable for Workspace {
    fn focus_handle(&self, _: &App) -> FocusHandle {
        self.focus_handle.clone()
    }
}

impl Workspace {
    pub fn new(_window: &mut Window, cx: &mut Context<Workspace>) -> Workspace {
        Workspace {
            layout: Layout::new(),
            focus_handle: cx.focus_handle(),
            bounds: Rc::default(),
            hovered_tab: None,
            hovered_close: None,
            hovered_divider: None,
            dragging_divider: None,
            drop_target: None,
            label_widths: HashMap::new(),
            label_font_size: px(0.),
            tab_layouts: HashMap::new(),
            scroll: HashMap::new(),
            new_tab_menu: None,
            extras: TabExtras::default(),
            tab_menu: None,
            tab_bounds: Rc::default(),
        }
    }

    /// Opens `item` as the last tab of the focused group and activates it. An open key is
    /// replaced in place and activated.
    pub fn open(&mut self, item: PaneItem, _window: &mut Window, cx: &mut Context<Workspace>) {
        let before = self.active();
        self.layout.open(item);
        self.changed(before, cx);
    }

    /// Replaces the open item with the same key (title, icon, modified dot, or view) without
    /// activating it. Returns false when the key is not open.
    pub fn update(&mut self, item: PaneItem, cx: &mut Context<Workspace>) -> bool {
        let updated = self.layout.update(item);
        if updated {
            cx.notify();
        }
        updated
    }

    /// Shows `logo` in place of the icon of the open tab `key`; `None` brings the icon back. An
    /// unknown key changes nothing.
    pub fn set_item_logo(
        &mut self,
        key: &str,
        logo: Option<Arc<Image>>,
        cx: &mut Context<Workspace>,
    ) {
        let open = self.layout.item(key).is_some();
        if self.extras.set_logo(open, key, logo) {
            cx.notify();
        }
    }

    /// Tells the workspace what the open tab `key` shows, which decides its context menu. An
    /// unknown key changes nothing.
    pub fn set_item_kind(&mut self, key: &str, kind: TabKind, cx: &mut Context<Workspace>) {
        let open = self.layout.item(key).is_some();
        if self.extras.set_kind(open, key, kind) {
            cx.notify();
        }
    }

    /// Shows the focused group alone, or restores the layout when one group is maximized. With
    /// one group there is nothing to maximize.
    pub fn toggle_maximize(&mut self, _window: &mut Window, cx: &mut Context<Workspace>) {
        if self.layout.toggle_maximize() {
            cx.notify();
        }
    }

    /// Closes the active tab of the focused group.
    pub fn close_active(&mut self, window: &mut Window, cx: &mut Context<Workspace>) {
        if let Some(key) = self.active() {
            self.close(&key, window, cx);
        }
    }

    /// Closes the tab with `key`. The next tab, else the previous one, becomes active, and a
    /// split group left empty goes away with its divider.
    pub fn close(&mut self, key: &str, _window: &mut Window, cx: &mut Context<Workspace>) {
        let Some(closed) = self.layout.close(key) else {
            return;
        };
        self.extras.forget(key);
        self.tab_bounds.borrow_mut().remove(key);
        cx.emit(PaneEvent::Closed {
            key: closed.item.key,
        });
        if let Some(key) = closed.activated {
            cx.emit(PaneEvent::Activated { key });
        }
        self.reveal_active();
        cx.notify();
    }

    /// Adds an empty group to the right of or below the focused group, focuses it, and emits
    /// `PaneEvent::NewTab` so the app opens a tab there, as Electron's split opens a new tab.
    pub fn split(&mut self, axis: SplitAxis, _window: &mut Window, cx: &mut Context<Workspace>) {
        self.layout.split_focused(axis);
        cx.emit(PaneEvent::NewTab);
        cx.notify();
    }

    /// Activates the tab at the 0-based `index` of the focused group.
    pub fn activate_index(
        &mut self,
        index: usize,
        _window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let before = self.active();
        if self.layout.activate_index(index) {
            self.changed(before, cx);
        }
    }

    /// The active key of the focused group.
    pub fn active_key(&self) -> Option<&str> {
        self.layout.active_key()
    }

    /// The groups and their tabs, for reading.
    pub fn layout(&self) -> &Layout<PaneItem> {
        &self.layout
    }

    /// Makes the new-tab button open a menu built by `builder`, as Electron's new-tab menu does.
    /// Without it the button emits `PaneEvent::NewTab`. The button's group has the focus while
    /// the menu is open, so `Workspace::open` from a menu item puts the tab there.
    pub fn set_new_tab_menu(
        &mut self,
        builder: impl Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
        cx: &mut Context<Workspace>,
    ) {
        self.new_tab_menu = Some(Rc::new(builder));
        cx.notify();
    }

    fn active(&self) -> Option<String> {
        self.layout.active_key().map(str::to_owned)
    }

    /// Emits `Activated` when the focused group's active key moved away from `before`.
    fn changed(&mut self, before: Option<String>, cx: &mut Context<Workspace>) {
        let after = self.active();
        if after != before
            && let Some(key) = after
        {
            cx.emit(PaneEvent::Activated { key });
        }
        self.reveal_active();
        cx.notify();
    }

    /// Scrolls the focused group's tab bar to its active tab.
    fn reveal_active(&self) {
        let group = self
            .layout
            .group(self.layout.focused())
            .expect("the focused group exists");
        if let Some(scroll) = self.scroll.get(&group.id)
            && !group.items.is_empty()
        {
            scroll.scroll_to_item(group.active);
        }
    }

    fn focus_group(&mut self, group: NodeId, cx: &mut Context<Workspace>) {
        let before = self.active();
        if self.layout.focus(group) {
            self.changed(before, cx);
        }
    }

    fn activate_tab(&mut self, group: NodeId, index: usize, cx: &mut Context<Workspace>) {
        let before = self.active();
        if self.layout.activate(group, index) {
            self.changed(before, cx);
        }
    }

    fn request_new_tab(&mut self, group: NodeId, cx: &mut Context<Workspace>) {
        self.focus_group(group, cx);
        cx.emit(PaneEvent::NewTab);
    }

    /// Opens the context menu of tab `key` at `position`. A second call replaces the open menu.
    fn open_tab_menu(
        &mut self,
        key: &str,
        position: Point<Pixels>,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        let Some((group, index)) = self.layout.find(key) else {
            return;
        };
        let count = self
            .layout
            .group(group)
            .map_or(0, |group| group.items.len());
        let entries = tab_menu::menu_entries(&self.extras.kind(key), index, count);
        let workspace = cx.entity().downgrade();
        let key = key.to_owned();
        let muted = tokens(cx).foreground_muted;
        let menu = PopupMenu::build(window, cx, move |menu, _, _| {
            let mut menu = menu.min_w(px(TAB_MENU_WIDTH));
            for entry in &entries {
                menu = match entry {
                    TabMenuEntry::Separator { .. } => menu.separator(),
                    TabMenuEntry::Item(item) => {
                        let workspace = workspace.clone();
                        let key = key.clone();
                        let action = item.action;
                        menu.item(menu_item(item, muted).on_click(move |_, window, cx| {
                            workspace
                                .update(cx, |this, cx| {
                                    this.run_tab_menu_action(action, &key, window, cx)
                                })
                                .ok();
                        }))
                    }
                };
            }
            menu
        });
        let dismiss = cx.subscribe_in(&menu, window, |this, _, _: &DismissEvent, window, cx| {
            this.tab_menu = None;
            this.focus_handle.focus(window, cx);
            cx.notify();
        });
        menu.focus_handle(cx).focus(window, cx);
        self.tab_menu = Some(OpenTabMenuState {
            menu,
            position,
            _dismiss: dismiss,
        });
        cx.notify();
    }

    /// Opens the menu of the focused group's active tab below the tab, for the keyboard menu key.
    fn open_active_tab_menu(&mut self, window: &mut Window, cx: &mut Context<Workspace>) {
        let Some(key) = self.active() else {
            return;
        };
        let Some(bounds) = self.tab_bounds.borrow().get(&key).copied() else {
            return;
        };
        self.open_tab_menu(&key, bounds.bottom_left(), window, cx);
    }

    fn run_tab_menu_action(
        &mut self,
        action: TabMenuAction,
        key: &str,
        window: &mut Window,
        cx: &mut Context<Workspace>,
    ) {
        if let Some(event) = action.event(key) {
            cx.emit(event);
            return;
        }
        let Some((group, index)) = self.layout.find(key) else {
            return;
        };
        let keys: Vec<String> = self
            .layout
            .group(group)
            .map(|group| group.items.iter().map(|item| item.key.clone()).collect())
            .unwrap_or_default();
        for target in tab_menu::close_targets(action, &keys, index) {
            self.close(&target, window, cx);
        }
    }

    fn track_drop(
        &mut self,
        group: NodeId,
        event: &DragMoveEvent<TabDrag>,
        cx: &mut Context<Workspace>,
    ) {
        let position = event.event.position;
        let target = if event.bounds.contains(&position) {
            let scroll = self
                .scroll
                .get(&group)
                .map_or(0., |scroll| f32::from(scroll.offset().x));
            let x = f32::from(position.x - event.bounds.left()) - scroll;
            let widths = self
                .tab_layouts
                .get(&group)
                .map_or(&[][..], |layout| &layout.widths[..]);
            Some((group, tab_layout::drop_slot(widths, x)))
        } else if self.drop_target.is_some_and(|(target, _)| target == group) {
            None
        } else {
            return;
        };
        if self.drop_target != target {
            self.drop_target = target;
            cx.notify();
        }
    }

    fn drop_tab(&mut self, group: NodeId, drag: &TabDrag, cx: &mut Context<Workspace>) {
        let slot = match self.drop_target.take() {
            Some((target, slot)) if target == group => slot,
            _ => self
                .layout
                .group(group)
                .map_or(0, |group| group.items.len()),
        };
        let before = self.active();
        self.layout.move_item(&drag.key, group, slot);
        self.changed(before, cx);
    }

    fn drag_divider(&mut self, event: &DragMoveEvent<DividerDrag>, cx: &mut Context<Workspace>) {
        let DividerDrag { split, index } = *event.drag(cx);
        self.dragging_divider = Some((split, index));
        let size = event.bounds.size;
        let root = Rect {
            x: 0.,
            y: 0.,
            w: f32::from(size.width),
            h: f32::from(size.height),
        };
        let Some(split_ref) = self.layout.split(split) else {
            return;
        };
        let Some(rect) = self.layout.rects(root).get(&split).copied() else {
            return;
        };
        let children = child_rects(rect, split_ref.axis, &split_ref.ratios);
        let dividers = split_ref.ratios.len().saturating_sub(1) as f32 * DIVIDER;
        let offset = event.event.position - event.bounds.origin;
        let (pointer, start, length) = match split_ref.axis {
            SplitAxis::Right => (f32::from(offset.x), children[index].x, rect.w),
            SplitAxis::Down => (f32::from(offset.y), children[index].y, rect.h),
        };
        let available = length - dividers;
        if available <= 0. {
            return;
        }
        self.layout
            .resize(split, index, (pointer - start) / available);
        cx.notify();
    }

    /// Measures every tab label once per title and font size, then lays out each tab bar.
    fn layout_tabs(&mut self, window: &Window, font: Pixels) {
        if font != self.label_font_size {
            self.label_widths.clear();
            self.label_font_size = font;
        }
        let size = self.bounds.get().size;
        let rects = self.layout.visible_rects(Rect {
            x: 0.,
            y: 0.,
            w: f32::from(size.width),
            h: f32::from(size.height),
        });
        let groups: Vec<(NodeId, Vec<(SharedString, bool)>)> = self
            .layout
            .groups()
            .into_iter()
            .map(|group| {
                let labels = group
                    .items
                    .iter()
                    .map(|item| (item.title.clone(), item.modified))
                    .collect();
                (group.id, labels)
            })
            .collect();
        let text_style = window.text_style();
        let mut layouts = HashMap::new();
        let mut titles = HashSet::new();
        for (id, labels) in groups {
            let widths: Vec<f32> = labels
                .iter()
                .map(|(title, modified)| {
                    let width = *self.label_widths.entry(title.clone()).or_insert_with(|| {
                        let line = one_line(title);
                        let run = text_style.to_run(line.len());
                        let shaped = window.text_system().shape_line(line, font, &[run], None);
                        f32::from(shaped.width).ceil()
                    });
                    titles.insert(title.clone());
                    label_allowance(width, *modified)
                })
                .collect();
            // The first frame has no size yet: lay tabs out at their natural width until the
            // canvas reports one.
            let row = rects.get(&id).map_or(0., |rect| rect.w);
            let row = if row > 0. { row } else { f32::MAX / 4. };
            layouts.insert(id, tab_layout::compute(row, &widths));
            self.scroll.entry(id).or_default();
        }
        if self.label_widths.len() > titles.len() * 2 {
            self.label_widths.retain(|title, _| titles.contains(title));
        }
        self.scroll.retain(|id, _| layouts.contains_key(id));
        self.tab_layouts = layouts;
    }

    fn render_node(
        &self,
        node: &Node<PaneItem>,
        t: &Tokens,
        font: Pixels,
        cx: &mut Context<Workspace>,
    ) -> AnyElement {
        match node {
            Node::Group(group) => self.render_group(group, t, font, cx),
            Node::Split(split) => self.render_split(split, t, font, cx),
        }
    }

    fn render_split(
        &self,
        split: &Split<PaneItem>,
        t: &Tokens,
        font: Pixels,
        cx: &mut Context<Workspace>,
    ) -> AnyElement {
        let row = split.axis == SplitAxis::Right;
        let mut element = div()
            .flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .when(row, |element| element.flex_row())
            .when(!row, |element| element.flex_col());
        for (index, (child, ratio)) in split.children.iter().zip(&split.ratios).enumerate() {
            if index > 0 {
                element =
                    element.child(self.render_divider(split.id, index - 1, split.axis, t, cx));
            }
            let mut slot = div()
                .flex_basis(px(0.))
                .min_w_0()
                .min_h_0()
                .overflow_hidden();
            // Grow by the ratio from a zero basis, so each child gets exactly its share of the
            // length left after the dividers, the same rects `model::child_rects` computes.
            slot.style().flex_grow = Some(*ratio);
            slot.style().flex_shrink = Some(1.);
            element = element.child(slot.child(self.render_node(child, t, font, cx)));
        }
        element.into_any_element()
    }

    fn render_divider(
        &self,
        split: NodeId,
        index: usize,
        axis: SplitAxis,
        t: &Tokens,
        cx: &mut Context<Workspace>,
    ) -> AnyElement {
        let row = axis == SplitAxis::Right;
        let me = Some((split, index));
        let highlighted = self.dragging_divider == me
            || (self.dragging_divider.is_none() && self.hovered_divider == me);
        let inset = divider_inset(DIVIDER_HIT);
        let band = div()
            .id(SharedString::from(format!("divider:{split}:{index}")))
            .role(Role::Splitter)
            .aria_label(if row {
                "Resize panes left and right"
            } else {
                "Resize panes up and down"
            })
            .aria_orientation(if row {
                Orientation::Vertical
            } else {
                Orientation::Horizontal
            })
            .absolute()
            .when(row, |band| {
                band.top_0()
                    .h_full()
                    .left(px(-inset))
                    .w(px(DIVIDER_HIT))
                    .cursor_col_resize()
            })
            .when(!row, |band| {
                band.left_0()
                    .w_full()
                    .top(px(-inset))
                    .h(px(DIVIDER_HIT))
                    .cursor_row_resize()
            })
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                let next = if *hovered { me } else { None };
                if this.hovered_divider != next && (*hovered || this.hovered_divider == me) {
                    this.hovered_divider = next;
                    cx.notify();
                }
            }))
            .on_drag(DividerDrag { split, index }, |_, _, _, cx| {
                cx.new(|_| DragPreview::Divider)
            })
            .when(highlighted, |band| {
                let start = inset - divider_inset(DIVIDER_HIGHLIGHT);
                band.child(
                    div()
                        .absolute()
                        .bg(t.accent)
                        .when(row, |line| {
                            line.top_0()
                                .h_full()
                                .left(px(start))
                                .w(px(DIVIDER_HIGHLIGHT))
                        })
                        .when(!row, |line| {
                            line.left_0()
                                .w_full()
                                .top(px(start))
                                .h(px(DIVIDER_HIGHLIGHT))
                        }),
                )
            });
        // The band straddles the line, so it would sit under the next pane's hitboxes; defer it
        // to paint above both panes.
        div()
            .flex_none()
            .relative()
            .bg(t.border)
            .when(row, |line| line.w(px(DIVIDER)).h_full())
            .when(!row, |line| line.h(px(DIVIDER)).w_full())
            .child(deferred(band))
            .into_any_element()
    }

    fn render_group(
        &self,
        group: &Group<PaneItem>,
        t: &Tokens,
        font: Pixels,
        cx: &mut Context<Workspace>,
    ) -> AnyElement {
        let id = group.id;
        let focused = id == self.layout.focused();
        let layout = self.tab_layouts.get(&id).cloned().unwrap_or(TabLayout {
            widths: Vec::new(),
            scrolls: false,
        });
        let scroll = self.scroll.get(&id).cloned().unwrap_or_default();
        let drop_slot = self
            .drop_target
            .filter(|(target, _)| *target == id)
            .map(|(_, slot)| slot);
        let count = group.items.len();
        let tabs: Vec<AnyElement> = group
            .items
            .iter()
            .enumerate()
            .map(|(index, item)| {
                let width = layout.widths.get(index).copied().unwrap_or(0.);
                let indicator = match drop_slot {
                    Some(slot) if slot == index => Some(true),
                    Some(slot) if slot == count && index + 1 == count => Some(false),
                    _ => None,
                };
                let active = group.active == index;
                self.render_tab(
                    id, index, item, active, focused, width, indicator, t, font, cx,
                )
            })
            .collect();
        let body = group.active_item().map(|item| item.view.clone());

        let tab_row = h_flex()
            .id(("tabs", id as usize))
            .flex_1()
            .min_w_0()
            .h_full()
            .px(px(TAB_ROW_PADDING))
            .overflow_x_scroll()
            .track_scroll(&scroll)
            .children(tabs)
            .when(!layout.scrolls, |row| {
                row.child(
                    div()
                        .flex_none()
                        .px(px(SPACE_1))
                        .child(self.render_new_tab_button(id, CONTROL_XS, t, cx)),
                )
            });
        let toolbar = h_flex()
            .flex_none()
            .gap(px(1.))
            .px(px(SPLIT_ACTIONS_PADDING))
            .mr(px(SPLIT_ACTIONS_MARGIN))
            .when(layout.scrolls, |toolbar| {
                toolbar.child(self.render_new_tab_button(id, TOOLBAR_BUTTON, t, cx))
            })
            // One group has nothing to maximize; a maximized group keeps the button to restore.
            .when(
                self.layout.maximized().is_some() || self.layout.groups().len() > 1,
                |toolbar| toolbar.child(self.render_maximize_button(id, t, cx)),
            )
            .child(self.render_split_menu(id, t));
        let tab_bar = h_flex()
            .id(("tab-bar", id as usize))
            .role(Role::TabList)
            .aria_label("Tabs")
            .flex_none()
            .w_full()
            .h(px(TAB_BAR_HEIGHT))
            .border_b_1()
            .border_color(t.border)
            .bg(t.surface0)
            .on_drag_move::<TabDrag>(
                cx.listener(move |this, event, _, cx| this.track_drop(id, event, cx)),
            )
            .on_drop::<TabDrag>(cx.listener(move |this, drag, _, cx| this.drop_tab(id, drag, cx)))
            .child(tab_row)
            .child(toolbar);

        v_flex()
            .size_full()
            .min_w_0()
            .min_h_0()
            .capture_any_mouse_down(cx.listener(move |this, _, _, cx| this.focus_group(id, cx)))
            .child(tab_bar)
            .child(
                div()
                    .flex_1()
                    .min_h_0()
                    .w_full()
                    .overflow_hidden()
                    .bg(t.surface_workspace)
                    .children(body),
            )
            .into_any_element()
    }

    /// One tab chip in its slot. `indicator` draws the drop pill before (`true`) or after
    /// (`false`) the chip.
    #[allow(clippy::too_many_arguments)]
    fn render_tab(
        &self,
        group: NodeId,
        index: usize,
        item: &PaneItem,
        active: bool,
        focused: bool,
        width: f32,
        indicator: Option<bool>,
        t: &Tokens,
        font: Pixels,
        cx: &mut Context<Workspace>,
    ) -> AnyElement {
        let key = item.key.clone();
        let title = one_line(&item.title);
        let close_hovered = self.hovered_close.as_deref() == Some(key.as_str());
        let hovered = self.hovered_tab.as_deref() == Some(key.as_str()) || close_hovered;
        let active_focused = active && focused;
        let highlighted = active_focused || hovered;
        // An active tab in an unfocused group keeps the hover fill and the muted label, so only
        // one chip in the window reads as the live one.
        let fill = if active_focused {
            Some(t.surface2)
        } else if active || hovered {
            Some(t.surface1)
        } else {
            None
        };
        let backdrop = fill.unwrap_or(t.surface0);
        let label_color = if highlighted {
            t.foreground
        } else {
            t.foreground_muted
        };

        let close_key = key.clone();
        let close_button = div()
            .id(SharedString::from(format!("tab-close:{key}")))
            .role(Role::Button)
            .aria_label(SharedString::from(format!("Close {title}")))
            .flex()
            .items_center()
            .justify_center()
            .size(px(CLOSE_BUTTON))
            .rounded(px(RADIUS_SM))
            .child(
                icon(IconName::X)
                    .with_size(px(ICON_XS))
                    .text_color(if close_hovered {
                        t.foreground
                    } else {
                        t.foreground_muted
                    }),
            )
            // Hidden, the button stays in the accessibility tree but takes no pointer events,
            // so the chip under it keeps its clicks.
            .when(hovered, |button| {
                let hover_key = key.clone();
                button
                    .cursor_pointer()
                    .on_mouse_down(MouseButton::Left, |_, _, cx| cx.stop_propagation())
                    .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                        let next = hovered.then(|| hover_key.clone());
                        if this.hovered_close != next {
                            this.hovered_close = next;
                            cx.notify();
                        }
                    }))
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.hovered_close = None;
                        this.close(&close_key, window, cx);
                    }))
            });
        let close_overlay = h_flex()
            .absolute()
            .top_0()
            .bottom_0()
            .right_0()
            .w(px(CLOSE_OVERLAY_WIDTH))
            .justify_end()
            .pr(px(SPACE_1))
            .rounded_r(px(RADIUS_MD))
            .bg(linear_gradient(
                90.,
                linear_color_stop(backdrop.opacity(0.), 0.),
                linear_color_stop(backdrop, CLOSE_SCRIM_SOLID),
            ))
            .when(!hovered, |overlay| overlay.opacity(0.))
            .child(close_button);

        let hover_key = key.clone();
        let middle_key = key.clone();
        let menu_key = key.clone();
        let tooltip_title = title.clone();
        let logo = self.extras.logo(&key).cloned();
        let bounds_key = key.clone();
        let tab_bounds = self.tab_bounds.clone();
        let measure = canvas(
            move |bounds, _, _| {
                let mut known = tab_bounds.borrow_mut();
                if known.get(&bounds_key) != Some(&bounds) {
                    known.insert(bounds_key, bounds);
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        let chip = h_flex()
            .id(SharedString::from(format!("tab:{key}")))
            .role(Role::Tab)
            .aria_label(title.clone())
            .aria_selected(active)
            .relative()
            .w(px(width))
            .h(px(CONTROL_XS))
            .px(px(TAB_CHIP_PADDING))
            .gap(px(TAB_CONTENT_GAP))
            .rounded(px(RADIUS_MD))
            .overflow_hidden()
            .when_some(fill, |chip, fill| chip.bg(fill))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(move |this, _, _, cx| this.activate_tab(group, index, cx)),
            )
            // An AX press does what a left mouse-down does, with the group focus that the
            // group's capture mouse-down gives a click. GPUI's default replays a mouse click
            // at the node centre, which the UI checks saw do nothing.
            .on_a11y_action(AccessibleAction::Click, {
                let view = cx.weak_entity();
                move |_, _, cx| {
                    view.update(cx, |this, cx| {
                        this.focus_group(group, cx);
                        this.activate_tab(group, index, cx);
                    })
                    .ok();
                }
            })
            .on_mouse_down(
                MouseButton::Middle,
                cx.listener(move |this, _, window, cx| this.close(&middle_key, window, cx)),
            )
            .on_mouse_down(
                MouseButton::Right,
                cx.listener(move |this, event: &MouseDownEvent, window, cx| {
                    this.open_tab_menu(&menu_key, event.position, window, cx)
                }),
            )
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                let next = hovered.then(|| hover_key.clone());
                if this.hovered_tab != next
                    && (*hovered || this.hovered_tab.as_deref() == Some(hover_key.as_str()))
                {
                    this.hovered_tab = next;
                    cx.notify();
                }
            }))
            .on_drag(
                TabDrag {
                    key: key.clone(),
                    title: title.clone(),
                    icon: item.icon,
                    logo: logo.clone(),
                },
                |drag, _, _, cx| {
                    cx.new(|_| DragPreview::Tab {
                        title: drag.title.clone(),
                        icon: drag.icon,
                        logo: drag.logo.clone(),
                    })
                },
            )
            .tooltip(move |window, cx| Tooltip::new(tooltip_title.clone()).build(window, cx))
            .tooltip_show_delay(TAB_TOOLTIP_DELAY)
            .child(tab_glyph(item.icon, logo.as_ref(), label_color))
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(font)
                    .text_color(label_color)
                    .child(title),
            )
            .when(item.modified, |chip| {
                chip.child(
                    div()
                        .id(SharedString::from(format!("tab-modified:{key}")))
                        .role(Role::Image)
                        .aria_label("Modified")
                        .flex_none()
                        .size(px(TAB_MODIFIED_DOT))
                        .rounded_full()
                        .bg(t.foreground_muted),
                )
            })
            .child(close_overlay)
            .child(measure);

        let pill_offset = -(TAB_CHIP_GAP / 2. + TAB_DROP_INDICATOR_WIDTH / 2.);
        div()
            .flex_none()
            .relative()
            .mx(px(TAB_CHIP_GAP / 2.))
            .child(chip)
            .when_some(indicator, |slot, before| {
                let pill = div()
                    .absolute()
                    .top(px(SPACE_0_5))
                    .bottom(px(SPACE_0_5))
                    .w(px(TAB_DROP_INDICATOR_WIDTH))
                    .rounded_full()
                    .bg(t.accent);
                slot.child(if before {
                    pill.left(px(pill_offset))
                } else {
                    pill.right(px(pill_offset))
                })
            })
            .into_any_element()
    }

    fn icon_button(
        &self,
        id: ElementId,
        label: &'static str,
        name: IconName,
        size: f32,
        t: &Tokens,
    ) -> Stateful<Div> {
        let fill = t.interaction_highlight;
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .flex()
            .flex_none()
            .items_center()
            .justify_center()
            .size(px(size))
            .rounded(px(RADIUS_MD))
            .cursor_pointer()
            .hover(move |button| button.bg(fill))
            .child(
                icon(name)
                    .with_size(px(ICON_SM))
                    .text_color(t.foreground_extra_muted),
            )
    }

    fn render_new_tab_button(
        &self,
        group: NodeId,
        size: f32,
        t: &Tokens,
        cx: &mut Context<Workspace>,
    ) -> AnyElement {
        let button = self
            .icon_button(
                ("new-tab", group as usize).into(),
                "New tab",
                IconName::Plus,
                size,
                t,
            )
            .tooltip(|window, cx| {
                Tooltip::new("New tab")
                    .action(&NewTab, Some(KEY_CONTEXT))
                    .build(window, cx)
            });
        match self.new_tab_menu.clone() {
            Some(builder) => ChromeButton {
                base: button,
                open: false,
                fill: t.interaction_highlight,
            }
            .dropdown_menu(move |menu, window, cx| builder(menu, window, cx))
            .into_any_element(),
            None => button
                .on_click(cx.listener(move |this, _, _, cx| this.request_new_tab(group, cx)))
                .into_any_element(),
        }
    }

    fn render_maximize_button(
        &self,
        group: NodeId,
        t: &Tokens,
        cx: &mut Context<Workspace>,
    ) -> AnyElement {
        let maximized = self.layout.maximized().is_some();
        let (label, name) = if maximized {
            ("Restore pane", IconName::Minimize)
        } else {
            ("Maximize pane", IconName::Maximize)
        };
        let fill = t.interaction_highlight;
        self.icon_button(
            ("maximize-pane", group as usize).into(),
            label,
            name,
            TOOLBAR_BUTTON,
            t,
        )
        .when(maximized, |button| button.bg(fill))
        .tooltip(move |window, cx| Tooltip::new(label).build(window, cx))
        .on_click(cx.listener(|this, _, window, cx| this.toggle_maximize(window, cx)))
        .into_any_element()
    }

    fn render_split_menu(&self, group: NodeId, t: &Tokens) -> AnyElement {
        let focus = self.focus_handle.clone();
        let muted = t.foreground_muted;
        let button = self
            .icon_button(
                ("split-menu", group as usize).into(),
                "More actions",
                IconName::Ellipsis,
                TOOLBAR_BUTTON,
                t,
            )
            .tooltip(|window, cx| Tooltip::new("More actions").build(window, cx));
        ChromeButton {
            base: button,
            open: false,
            fill: t.interaction_highlight,
        }
        .dropdown_menu_with_anchor(Anchor::TopRight, move |menu, _, _| {
            menu.action_context(focus.clone())
                .min_w(px(SPLIT_MENU_WIDTH))
                .menu_with_icon(
                    "Split right",
                    icon(IconName::Columns2)
                        .with_size(px(ICON_SM))
                        .text_color(muted),
                    Box::new(SplitRight),
                )
                .menu_with_icon(
                    "Split down",
                    icon(IconName::Rows2)
                        .with_size(px(ICON_SM))
                        .text_color(muted),
                    Box::new(SplitDown),
                )
        })
        .into_any_element()
    }
}

impl Render for Workspace {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // A drag that ended outside every drop target leaves no listener to clear its marks.
        if !cx.has_active_drag() {
            self.drop_target = None;
            self.dragging_divider = None;
        }
        let t = tokens(cx);
        let font = ui_font_size(FONT_BASE, cx);
        self.layout_tabs(window, font);

        let bounds = self.bounds.clone();
        let entity = cx.entity_id();
        let measure = canvas(
            move |frame, _, cx| {
                if bounds.get() != frame {
                    bounds.set(frame);
                    // Re-render with the new size on the next frame, never inside this one.
                    cx.defer(move |cx| cx.notify(entity));
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();

        // A maximized group fills the root alone; the other groups and the dividers do not render.
        let tree = match self.layout.maximized().and_then(|id| self.layout.group(id)) {
            Some(group) => self.render_group(group, &t, font, cx),
            None => self.render_node(self.layout.root(), &t, font, cx),
        };
        let tab_menu = self.tab_menu.as_ref().map(|open| {
            deferred(
                anchored()
                    .position(open.position)
                    .snap_to_window_with_margin(px(8.))
                    .child(open.menu.clone()),
            )
            .with_priority(gpui_kit::base::POPUP_PRIORITY)
        });
        div()
            .id("workspace")
            .role(Role::Group)
            .aria_label("Workspace tabs")
            .key_context(KEY_CONTEXT)
            .track_focus(&self.focus_handle)
            .relative()
            .size_full()
            .bg(t.surface_workspace)
            .on_action(cx.listener(|this, _: &SplitRight, window, cx| {
                this.split(SplitAxis::Right, window, cx)
            }))
            .on_action(cx.listener(|this, _: &SplitDown, window, cx| {
                this.split(SplitAxis::Down, window, cx)
            }))
            .on_action(
                cx.listener(|this, _: &CloseActiveTab, window, cx| this.close_active(window, cx)),
            )
            .on_action(cx.listener(|this, _: &NewTab, _, cx| {
                let group = this.layout.focused();
                this.request_new_tab(group, cx)
            }))
            .on_action(cx.listener(|this, _: &OpenTabMenu, window, cx| {
                this.open_active_tab_menu(window, cx)
            }))
            .on_drag_move::<DividerDrag>(
                cx.listener(|this, event, _, cx| this.drag_divider(event, cx)),
            )
            .child(measure)
            .child(tree)
            .children(tab_menu)
    }
}
