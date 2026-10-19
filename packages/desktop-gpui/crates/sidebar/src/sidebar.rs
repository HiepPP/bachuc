//! The sidebar view. Sizes and states follow `sidebar-workspace-list.tsx` (lines 2541-2760),
//! `left-sidebar.tsx`, `sidebar-header-row.tsx`, and the row and menu files next to them.
//!
//! Hover, pressed, and the kebab use GPUI hover styles on a row group, so moving the pointer
//! redraws nothing in the view. The list rebuilds only when the tree changes, and a status change
//! that keeps every row's height redraws without a new measure.

use crate::metrics::{ADD_PROJECT_MIN, LABEL_LINE, NAV_ROW_MIN};
use crate::model::{
    Facts, Group, ProjectRow, Row, RowShape, StatusBucket, WorkspaceRow,
    archive_prompt_for_checkout, changed_range, clamp_width, format_diff_count, identity_color,
    project_initial, workspace_label,
};
use crate::plugin::{
    NavPress, NavRow, SwipeGesture, SwipeStep, added_sidebar_project, drag_frame, filtered_rows,
    is_horizontal, nav_press, nav_rows, pick_title, run_in_order, slide_frame, slide_out_ms,
    submenu_row_label, submenu_trailing, visible_projects, wheel_deltas,
};
use crate::prefs::SidebarPrefs;
use crate::ring::{FRAME as RING_FRAME, Knockout, RunningRing};
use bachuc_client::{Client, ClientEvent, DirectoryEvent, ProjectNode};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{
    PluginHost, PluginHostEvent, ProjectMenuItem, SidebarProject, SwipeDirection, ViewFactory,
};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_MD, ICON_SM, ICON_XS, RADIUS_BASE, RADIUS_LG, RADIUS_MD,
    SIDEBAR_WIDTH, SPACE_0_5, SPACE_1, SPACE_1_5, SPACE_2, SPACE_3, SPACE_4, SPACE_6,
    WEIGHT_MEDIUM,
};
use bachuc_theme::{Tokens, tokens, ui_font_size, zoomed};
use gpui_kit::component::button::ButtonVariant;
use gpui_kit::component::dialog::DialogButtonProps;
use gpui_kit::component::input::{Input, InputState};
use gpui_kit::component::menu::{DropdownMenu, PopupMenu, PopupMenuItem};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Selectable, Side, Sizable as _, WindowExt as _, h_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::borrow::Cow;
use std::cell::Cell;
use std::collections::{HashMap, HashSet};
use std::path::PathBuf;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// How far the list draws beyond the visible rows.
const OVERDRAW: f32 = 400.;
/// `workspaceRow` and `projectRow` margins under a row.
const WORKSPACE_GAP: f32 = SPACE_0_5;
const PROJECT_GAP: f32 = SPACE_1;
/// `projectBlockExpanded` padding under a project block, and the `pinnedSection` margin.
const PROJECT_BLOCK_GAP: f32 = SPACE_3;
const PINNED_BLOCK_GAP: f32 = SPACE_1;
/// Row sizes: `minHeight` of project, workspace, and pinned header rows, and the title line.
const ROW_HEIGHT: f32 = 36.;
const LINE_HEIGHT: f32 = 20.;
/// The workspace kebab: 14 px icon, padding 2, so 18 px, pulled 7 px past the content rail.
const WORKSPACE_KEBAB: f32 = 18.;
/// The project kebab: 24 px, pulled 6 px past the row padding.
const PROJECT_KEBAB: f32 = 24.;
/// `projectTrailingActions`: the "+" and the kebab, 2 px apart; the "+" glyph is 15 px.
const PROJECT_ACTION_GAP: f32 = 2.;
const PROJECT_PLUS_GLYPH: f32 = 15.;
/// `TrailingActionScrim`: 48 px wide, solid after 55%.
const SCRIM_WIDTH: f32 = 48.;
const SCRIM_SOLID: f32 = 0.55;
/// The resize handle straddles the border, 10 px wide (`POINTER_HANDLE_WIDTH`).
const HANDLE_WIDTH: f32 = 10.;
/// A workspace that did not leave the tree after this long shows its row again.
const ARCHIVE_TIMEOUT: Duration = Duration::from_secs(10);
/// `STATUS_BADGE_SIZE` and `STATUS_BADGE_OFFSET` of the corner mark on a project icon.
const BADGE: f32 = 12.;
const BADGE_OFFSET: f32 = -4.;
/// `getStatusRingOffset(BADGE_OFFSET, BADGE)`: the ring grows around the badge center.
const RING_OFFSET: f32 = BADGE_OFFSET - (RING_FRAME - BADGE) / 2.;
/// `STATUS_INDICATOR_FILLED_DOT_SIZE`.
const DOT: f32 = 6.;

/// What the sidebar asks the app to do.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarEvent {
    OpenWorkspace { workspace_id: String },
    OpenSettings,
    OpenCommandCenter,
}

impl EventEmitter<SidebarEvent> for Sidebar {}

/// What the plugin parts of the sidebar ask the app to do. A second type keeps the match on
/// [`SidebarEvent`] exhaustive in the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarPluginEvent {
    /// A nav row without an action asks for its plugin surface.
    OpenSurface {
        plugin_id: String,
        surface_id: String,
    },
    /// The Add project button added a project and every filter took notice.
    ProjectAdded { project_id: String, cwd: String },
}

impl EventEmitter<SidebarPluginEvent> for Sidebar {}

/// What the built-in nav rows and the project "+" ask the app to do. A third type keeps the
/// match on [`SidebarEvent`] exhaustive in the app.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum SidebarNavEvent {
    /// The History row asks for the sessions screen.
    OpenHistory,
    /// The New workspace row (`None`) or a project "+" (`Some`) asks for the new workspace
    /// screen.
    NewWorkspace { project_id: Option<String> },
}

impl EventEmitter<SidebarNavEvent> for Sidebar {}

/// A click and an AX press run the same handler. GPUI's default AX press replays a mouse click
/// at the node centre, which the 2026-10-17 UI check saw do nothing on the sidebar rows.
trait OnPress: StatefulInteractiveElement + Sized {
    fn on_press(
        self,
        cx: &Context<Sidebar>,
        handler: impl Fn(&mut Sidebar, &mut Window, &mut Context<Sidebar>) + 'static,
    ) -> Self {
        let handler = Rc::new(handler);
        let click = handler.clone();
        let view = cx.weak_entity();
        self.on_click(cx.listener(move |this, _, window, cx| click(this, window, cx)))
            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                view.update(cx, |this, cx| handler(this, window, cx)).ok();
            })
    }
}

impl<E: StatefulInteractiveElement> OnPress for E {}

/// A plugin section view, built once per contribution.
struct SectionView {
    plugin_id: &'static str,
    id: &'static str,
    view: AnyView,
}

/// A filter slide in flight (`slide.tsx`), or a snap back when `target` is `None`. A slide
/// starts at `from`, where the finger left the list, and the rows change at the end of its out
/// phase.
struct Slide {
    started: Instant,
    target: Option<SwipeDirection>,
    from: f32,
    generation: u64,
}

#[derive(Clone, Copy)]
struct ResizeDrag;

/// What follows the pointer while the handle is dragged: nothing.
struct NoPreview;

impl Render for NoPreview {
    fn render(&mut self, _: &mut Window, _: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

/// A small icon button that anchors a gpui-kit dropdown menu and stays visible while the menu is
/// open. It paints only a fill on hover; the glyph comes from its child.
#[derive(IntoElement)]
struct KebabButton {
    base: Stateful<Div>,
    open: bool,
    group: SharedString,
    fill: Hsla,
}

impl Selectable for KebabButton {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl Styled for KebabButton {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for KebabButton {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl DropdownMenu for KebabButton {}

impl RenderOnce for KebabButton {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let fill = self.fill;
        let group = self.group;
        self.base
            .when(self.open, |button| button.bg(fill))
            .when(!self.open, |button| {
                button
                    .invisible()
                    .group_hover(group, |style| style.visible())
            })
    }
}

pub struct Sidebar {
    client: Entity<Client>,
    selected: Option<String>,
    prefs: SidebarPrefs,
    /// Where `sidebar.json` lives. None keeps the prefs in memory only.
    data_dir: Option<PathBuf>,
    /// Whether the first project list already dropped the prefs of projects that went.
    pruned: bool,
    rows: Vec<Row>,
    shapes: Vec<RowShape>,
    list: ListState,
    width: f32,
    rings: HashMap<String, Entity<RunningRing>>,
    archiving: HashSet<String>,
    host: Option<Entity<PluginHost>>,
    sections: Vec<SectionView>,
    /// (plugin id, surface id) of the open plugin page, for the active nav row.
    active_surface: Option<(String, String)>,
    swipe: SwipeGesture,
    wheel_epoch: Instant,
    /// The web-sign pixel offset of the list while a finger drags it.
    drag: Option<f32>,
    /// A released swipe that waits for its filter change: the direction and the drag fraction.
    /// A swipe the filter does not answer before the next frame snaps back.
    awaiting: Option<(SwipeDirection, f32)>,
    /// The way a swipe last found no Space; a drag that way rubber-bands.
    swipe_edge: Option<SwipeDirection>,
    _swipe_timeout: Option<Task<()>>,
    slide: Option<Slide>,
    slide_generation: u64,
    adding: bool,
    _subscriptions: Vec<Subscription>,
    _host_subscription: Option<Subscription>,
}

impl Sidebar {
    /// A sidebar that keeps its collapse state in memory only.
    pub fn new(client: Entity<Client>, window: &mut Window, cx: &mut Context<Sidebar>) -> Sidebar {
        Self::with_data_dir(client, None, window, cx)
    }

    /// A sidebar that loads its prefs from `sidebar.json` in `data_dir` and saves them there on
    /// each toggle.
    pub fn with_data_dir(
        client: Entity<Client>,
        data_dir: Option<PathBuf>,
        window: &mut Window,
        cx: &mut Context<Sidebar>,
    ) -> Sidebar {
        let prefs = data_dir
            .as_deref()
            .map(SidebarPrefs::load_from)
            .unwrap_or_default();
        let subscriptions = vec![
            cx.subscribe(&client, |this, _, event: &ClientEvent, cx| match event {
                ClientEvent::Connected
                | ClientEvent::Disconnected { .. }
                | ClientEvent::AgentsChanged => this.refresh(cx),
                _ => {}
            }),
            cx.subscribe(&client, |this, _, event: &DirectoryEvent, cx| match event {
                DirectoryEvent::ProjectsChanged => this.refresh(cx),
                // The daemon pushes the diff counts with each workspace update.
                DirectoryEvent::CheckoutChanged { .. } => {}
            }),
            // set_theme and set_font_scale write the gpui-kit theme global.
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
        ];
        let mut sidebar = Sidebar {
            client,
            selected: None,
            prefs,
            data_dir,
            pruned: false,
            rows: Vec::new(),
            shapes: Vec::new(),
            list: ListState::new(0, ListAlignment::Top, px(OVERDRAW)),
            width: SIDEBAR_WIDTH,
            rings: HashMap::new(),
            archiving: HashSet::new(),
            host: None,
            sections: Vec::new(),
            active_surface: None,
            swipe: SwipeGesture::default(),
            wheel_epoch: Instant::now(),
            drag: None,
            awaiting: None,
            swipe_edge: None,
            _swipe_timeout: None,
            slide: None,
            slide_generation: 0,
            adding: false,
            _subscriptions: subscriptions,
            _host_subscription: None,
        };
        sidebar.attach_host(window, cx);
        sidebar.refresh(cx);
        sidebar
    }

    /// Marks the plugin page the app shows (plugin id, surface id), or none. The nav row of that
    /// surface draws as hovered.
    pub fn set_active_surface(
        &mut self,
        surface: Option<(String, String)>,
        cx: &mut Context<Sidebar>,
    ) {
        if self.active_surface != surface {
            self.active_surface = surface;
            cx.notify();
        }
    }

    /// Follows the plugin host once the app has created it. Reads contributions only from the
    /// host index.
    fn attach_host(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.host.is_some() {
            return;
        }
        let Some(host) = bachuc_plugin_host::host(cx) else {
            return;
        };
        self._host_subscription = Some(cx.subscribe_in(
            &host,
            window,
            |this, _, event: &PluginHostEvent, window, cx| match event {
                PluginHostEvent::ContributionsChanged => {
                    this.sync_sections(window, cx);
                    this.refresh(cx);
                    cx.notify();
                }
                // A filter, a menu item, or a disabled state may change.
                PluginHostEvent::PluginStateChanged { .. } => {
                    this.refresh(cx);
                    cx.notify();
                }
                PluginHostEvent::SidebarFilterChanged {
                    direction: Some(direction),
                } => this.start_slide(*direction, cx),
                PluginHostEvent::SidebarFilterChanged { direction: None } => {
                    this.finish_slide(cx);
                    this.refresh(cx);
                    cx.notify();
                }
                _ => {}
            },
        ));
        self.host = Some(host);
        self.sync_sections(window, cx);
        self.refresh(cx);
    }

    /// Builds the view of each new section and drops the views of sections that went.
    fn sync_sections(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        let Some(host) = &self.host else {
            return;
        };
        let wanted: Vec<(&'static str, &'static str, ViewFactory<()>)> = host
            .read(cx)
            .index()
            .sidebar_sections()
            .iter()
            .map(|section| {
                (
                    section.plugin_id,
                    section.value.id,
                    section.value.view.clone(),
                )
            })
            .collect();
        let mut old = std::mem::take(&mut self.sections);
        self.sections = wanted
            .into_iter()
            .map(|(plugin_id, id, factory)| {
                match old
                    .iter()
                    .position(|section| section.plugin_id == plugin_id && section.id == id)
                {
                    Some(at) => old.swap_remove(at),
                    None => SectionView {
                        plugin_id,
                        id,
                        view: factory((), window, cx),
                    },
                }
            })
            .collect();
    }

    /// Applies a pending filter change at once and drops the slide.
    fn finish_slide(&mut self, cx: &mut Context<Self>) {
        if let Some(slide) = self.slide.take() {
            self.slide_generation += 1;
            if slide.target.is_some() {
                self.refresh(cx);
            }
        }
    }

    /// Slides the list out, swaps the rows, and slides the new list in. A second change during
    /// a slide applies the first one at once. The swipe that asked for this change starts the
    /// slide where the finger left the list.
    fn start_slide(&mut self, direction: SwipeDirection, cx: &mut Context<Self>) {
        self.finish_slide(cx);
        let from = match self.awaiting.take() {
            Some((asked, from)) if asked == direction => from,
            _ => 0.,
        };
        self.swipe_edge = None;
        self.slide_generation += 1;
        let generation = self.slide_generation;
        self.slide = Some(Slide {
            started: Instant::now(),
            target: Some(direction),
            from,
            generation,
        });
        let apply = Duration::from_secs_f32(slide_out_ms(direction, from) / 1000.);
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(apply).await;
            this.update(cx, |this, cx| {
                if this.slide.as_ref().map(|slide| slide.generation) == Some(generation) {
                    this.refresh(cx);
                }
            })
            .ok();
        })
        .detach();
        cx.notify();
    }

    /// Marks the workspace the app shows, or none.
    pub fn set_selected_workspace(
        &mut self,
        workspace_id: Option<String>,
        cx: &mut Context<Sidebar>,
    ) {
        if self.selected != workspace_id {
            self.selected = workspace_id;
            cx.notify();
        }
    }

    /// Builds the rows again from the client and redraws only when something changed.
    fn refresh(&mut self, cx: &mut Context<Self>) {
        self.prune_stale_prefs(cx);
        let client = self.client.read(cx);
        let rows = if client.projects().is_empty() && !client.is_connected() {
            // Nothing to say before the first answer.
            Vec::new()
        } else {
            let facts = Facts::read(client, client.projects(), client.agents());
            let filters = self
                .host
                .as_ref()
                .map_or(&[][..], |host| host.read(cx).index().project_filters());
            // A hidden project drops its workspaces, pinned ones included. The selection and the
            // open tab are the app's, so a hidden selected workspace stays selected.
            let visible: Cow<[ProjectNode]> = if filters.is_empty() {
                Cow::Borrowed(client.projects())
            } else {
                let app: &App = cx;
                Cow::Owned(visible_projects(
                    client.projects(),
                    &facts.project_keys,
                    |project| {
                        filters
                            .iter()
                            .all(|filter| (filter.value.is_visible)(project, app))
                    },
                ))
            };
            filtered_rows(
                client.projects(),
                &visible,
                client.agents(),
                &facts,
                &self.prefs.view,
            )
        };
        if rows == self.rows {
            return;
        }
        let shapes: Vec<RowShape> = rows.iter().map(Row::shape).collect();
        if let Some((range, count)) = changed_range(&self.shapes, &shapes) {
            self.list.splice(range, count);
        }
        let gone: Vec<String> = self
            .archiving
            .iter()
            .filter(|id| {
                !rows
                    .iter()
                    .any(|row| matches!(row, Row::Workspace(w) if &w.id == *id))
            })
            .cloned()
            .collect();
        for id in gone {
            self.archiving.remove(&id);
        }
        let live: HashSet<String> = rows.iter().filter_map(ring_key).collect();
        self.rings.retain(|key, _| live.contains(key));
        self.shapes = shapes;
        self.rows = rows;
        cx.notify();
    }

    /// The first project list drops the stored ids of projects that no longer exist. An empty
    /// list proves nothing, because it is also the state before the first answer.
    fn prune_stale_prefs(&mut self, cx: &mut Context<Self>) {
        if self.pruned {
            return;
        }
        let projects = self.client.read(cx).projects();
        if projects.is_empty() {
            return;
        }
        let known: HashSet<&str> = projects.iter().map(|project| project.id.as_str()).collect();
        let changed = self.prefs.retain_projects(&known);
        self.pruned = true;
        if changed {
            self.save_prefs();
        }
    }

    /// Called on a toggle only. A failed save logs a warning and the sidebar keeps working.
    fn save_prefs(&self) {
        let Some(dir) = &self.data_dir else {
            return;
        };
        if let Err(error) = self.prefs.save_to(dir) {
            log::warn!("cannot save sidebar.json: {error}");
        }
    }

    fn toggle_project(&mut self, id: &str, cx: &mut Context<Self>) {
        let collapsed = &mut self.prefs.view.collapsed_projects;
        if !collapsed.remove(id) {
            collapsed.insert(id.to_string());
        }
        self.save_prefs();
        self.refresh(cx);
    }

    fn toggle_pinned(&mut self, cx: &mut Context<Self>) {
        self.prefs.view.pinned_collapsed = !self.prefs.view.pinned_collapsed;
        self.save_prefs();
        self.refresh(cx);
    }

    fn toggle_group(&mut self, group: Group, cx: &mut Context<Self>) {
        let expanded = &mut self.prefs.view.expanded_groups;
        if !expanded.remove(&group) {
            expanded.insert(group);
        }
        self.save_prefs();
        self.refresh(cx);
    }

    fn open_workspace(&mut self, id: &str, cx: &mut Context<Self>) {
        self.selected = Some(id.to_string());
        cx.emit(SidebarEvent::OpenWorkspace {
            workspace_id: id.to_string(),
        });
        cx.notify();
    }

    fn pin(&mut self, id: &str, pinned: bool, cx: &mut Context<Self>) {
        self.client
            .update(cx, |client, cx| client.pin_workspace(id, pinned, cx));
    }

    fn mark_unread(&mut self, id: &str, cx: &mut Context<Self>) {
        self.client
            .update(cx, |client, cx| client.mark_workspace_unread(id, cx));
    }

    fn rename(&mut self, id: String, current: String, window: &mut Window, cx: &mut Context<Self>) {
        let input = cx.new(|cx| InputState::new(window, cx).default_value(current));
        let weak = cx.weak_entity();
        // The builder runs again whenever the dialog layer is rebuilt; focus once.
        let focus_once = Cell::new(true);
        window.open_dialog(cx, move |dialog, window, cx| {
            if focus_once.replace(false) {
                input.update(cx, |input, cx| input.focus(window, cx));
            }
            let weak = weak.clone();
            let id = id.clone();
            let field = input.clone();
            dialog
                .title("Rename workspace")
                .w(zoomed(420., cx))
                .child(Input::new(&input).aria_label("Workspace name"))
                .button_props(
                    DialogButtonProps::default()
                        .ok_text("Rename")
                        .show_cancel(true)
                        .on_ok(move |_, _, cx| {
                            let title = field.read(cx).value().trim().to_string();
                            if !title.is_empty() {
                                weak.update(cx, |this, cx| {
                                    this.client.update(cx, |client, cx| {
                                        client.rename_workspace(&id, &title, cx)
                                    });
                                })
                                .ok();
                            }
                            true
                        }),
                )
        });
    }

    fn copy_path(&mut self, path: String, window: &mut Window, cx: &mut Context<Self>) {
        cx.write_to_clipboard(ClipboardItem::new_string(path));
        window.push_notification("Path copied", cx);
    }

    /// Asks for a confirm when the workspace diff stat has a changed line or the checkout is
    /// dirty; a clean workspace goes at once. Both come with the workspace, so the menu sends no
    /// request.
    fn archive(&mut self, id: String, name: String, window: &mut Window, cx: &mut Context<Self>) {
        if self.archiving.contains(&id) {
            return;
        }
        let client = self.client.read(cx);
        let stat = client
            .workspace_diff_stat(&id)
            .map(|stat| (stat.additions, stat.deletions));
        let is_dirty = client.workspace_is_dirty(&id);
        match archive_prompt_for_checkout(&name, stat, is_dirty) {
            None => self.start_archive(id, cx),
            Some(prompt) => {
                let weak = cx.weak_entity();
                window.open_alert_dialog(cx, move |dialog, _, _| {
                    let weak = weak.clone();
                    let id = id.clone();
                    dialog
                        .title(prompt.title.clone())
                        .description(prompt.message.clone())
                        .confirm()
                        .ok_text("Archive")
                        .ok_variant(ButtonVariant::Danger)
                        .cancel_text("Cancel")
                        .on_ok(move |_, _, cx| {
                            let id = id.clone();
                            weak.update(cx, |this, cx| this.start_archive(id, cx)).ok();
                            true
                        })
                });
            }
        }
    }

    fn start_archive(&mut self, id: String, cx: &mut Context<Self>) {
        self.client
            .update(cx, |client, cx| client.archive_workspace(&id, cx));
        self.archiving.insert(id.clone());
        cx.notify();
        cx.spawn(async move |this, cx| {
            cx.background_executor().timer(ARCHIVE_TIMEOUT).await;
            this.update(cx, |this, cx| {
                if this.archiving.remove(&id) {
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn ring(&mut self, key: String, knockout: Knockout, cx: &mut Context<Self>) -> AnyElement {
        self.rings
            .entry(key)
            .or_insert_with(|| cx.new(|cx| RunningRing::new(knockout, cx)))
            .clone()
            .into_any_element()
    }

    // ---- plugins ----

    /// Runs the press on a plugin nav row. The row is read again from the index, so a press on
    /// a row that just went does nothing.
    fn press_nav(
        &mut self,
        plugin_id: &'static str,
        id: &'static str,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(host) = &self.host else {
            return;
        };
        let items = host.read(cx).index().sidebar_items();
        let rows = nav_rows(items, self.selected.as_deref(), None);
        let Some((row, item)) = rows
            .iter()
            .zip(items)
            .find(|(row, _)| row.plugin_id == plugin_id && row.id == id)
        else {
            return;
        };
        let press = nav_press(row);
        let surface_id = row.surface.to_string();
        let handler = item
            .value
            .action
            .as_ref()
            .map(|action| action.on_press.clone());
        match (press, handler) {
            (NavPress::Action, Some(handler)) => handler(self.selected.clone(), window, cx),
            (NavPress::OpenSurface, _) => cx.emit(SidebarPluginEvent::OpenSurface {
                plugin_id: plugin_id.to_string(),
                surface_id,
            }),
            _ => {}
        }
    }

    /// Feeds a wheel event over the project list to the swipe gesture: the list follows the
    /// finger, and a release calls every filter's `on_swipe` or snaps back. A horizontal scroll
    /// never scrolls the list.
    fn on_wheel(&mut self, event: &ScrollWheelEvent, cx: &mut Context<Self>) {
        let Some(host) = &self.host else {
            return;
        };
        let filters = host.read(cx).index().project_filters();
        if !filters.iter().any(|filter| filter.value.on_swipe.is_some()) {
            return;
        }
        let (dx, dy) = wheel_deltas(event.delta);
        if is_horizontal(dx, dy) || self.drag.is_some() {
            cx.stop_propagation();
        }
        // A pressed button means a drag is in flight, so the swipe must not switch under it.
        if event.modifiers.control || cx.has_active_drag() {
            return;
        }
        let now = self.swipe_now();
        let step = self.swipe.push(dx, dy, event.touch_phase, now, self.width);
        self.apply_swipe(step, cx);
        self.schedule_swipe_timeout(cx);
    }

    fn swipe_now(&self) -> f64 {
        self.wheel_epoch.elapsed().as_secs_f64() * 1000.
    }

    /// Ends a gesture whose events stopped without a release. Each event replaces the timer.
    fn schedule_swipe_timeout(&mut self, cx: &mut Context<Self>) {
        let Some(timeout) = self.swipe.timeout_ms() else {
            self._swipe_timeout = None;
            return;
        };
        self._swipe_timeout = Some(cx.spawn(async move |this, cx| {
            cx.background_executor()
                .timer(Duration::from_secs_f64(timeout / 1000.))
                .await;
            this.update(cx, |this, cx| {
                let step = this.swipe.tick(this.swipe_now(), this.width);
                this.apply_swipe(step, cx);
            })
            .ok();
        }));
    }

    fn apply_swipe(&mut self, step: SwipeStep, cx: &mut Context<Self>) {
        match step {
            SwipeStep::Ignore => {}
            SwipeStep::Drag { offset } => {
                if self.drag.is_none() {
                    // A slide still running shows its end at once under the finger.
                    self.finish_slide(cx);
                    self.awaiting = None;
                }
                if self.drag != Some(offset) {
                    self.drag = Some(offset);
                    cx.notify();
                }
            }
            SwipeStep::Commit { direction, offset } => {
                let from = self.drag_fraction(offset);
                self.drag = None;
                self.awaiting = Some((direction, from));
                cx.notify();
                let handlers: Vec<_> = self.host.as_ref().map_or_else(Vec::new, |host| {
                    host.read(cx)
                        .index()
                        .project_filters()
                        .iter()
                        .filter_map(|filter| filter.value.on_swipe.clone())
                        .collect()
                });
                for handler in handlers {
                    handler(direction, cx);
                }
            }
            SwipeStep::SnapBack { offset } => {
                let from = self.drag_fraction(offset);
                self.drag = None;
                self.snap_back(from, cx);
            }
        }
    }

    /// Where the list is drawn for a drag `offset`, as a fraction of the width.
    fn drag_fraction(&self, offset: f32) -> f32 {
        drag_frame(offset, self.width, self.blocked(offset)).offset
    }

    fn blocked(&self, offset: f32) -> bool {
        let toward = if offset > 0. {
            SwipeDirection::Next
        } else {
            SwipeDirection::Previous
        };
        self.swipe_edge == Some(toward)
    }

    fn snap_back(&mut self, from: f32, cx: &mut Context<Self>) {
        self.slide_generation += 1;
        self.slide = Some(Slide {
            started: Instant::now(),
            target: None,
            from,
            generation: self.slide_generation,
        });
        cx.notify();
    }

    /// Picks a folder, adds it as a project, and tells every filter about it, in index order.
    /// A filter error shows its text and stops the rest; the project stays added.
    fn add_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.adding {
            return;
        }
        self.adding = true;
        cx.notify();
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Add project".into()),
        });
        cx.spawn_in(window, async move |this, cx| {
            let folder = match picked.await {
                Ok(Ok(Some(paths))) => paths.into_iter().next(),
                _ => None,
            };
            let outcome = match folder {
                Some(folder) => {
                    let cwd = folder.to_string_lossy().into_owned();
                    add_project_and_notify(&this, &cwd, cx).await
                }
                None => Ok(None),
            };
            let _ = this.update_in(cx, |this, window, cx| {
                this.adding = false;
                match outcome {
                    Ok(Some(added)) => cx.emit(SidebarPluginEvent::ProjectAdded {
                        project_id: added.0,
                        cwd: added.1,
                    }),
                    Ok(None) => {}
                    Err(text) => window.push_notification(SharedString::from(text), cx),
                }
                cx.notify();
            });
        })
        .detach();
    }

    /// The heading over the project list: the first plugin filter title, else "Workspaces".
    fn section_title(&self, cx: &App) -> SharedString {
        let filters = self
            .host
            .as_ref()
            .map_or(&[][..], |host| host.read(cx).index().project_filters());
        pick_title(
            filters
                .iter()
                .map(|filter| filter.value.title.as_ref().and_then(|title| title(cx))),
        )
        .unwrap_or_else(|| "Workspaces".into())
    }

    // ---- rows ----

    fn render_row(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Self>) -> AnyElement {
        let Some(row) = self.rows.get(ix).cloned() else {
            return div().into_any_element();
        };
        let t = tokens(cx);
        let last = ix + 1 == self.rows.len();
        let body = match &row {
            Row::PinnedHeader { collapsed, .. } => self.render_pinned_header(*collapsed, &t, cx),
            Row::SectionTitle => {
                let title = self.section_title(cx);
                render_section_title(title, &t, cx)
            }
            Row::Project(project) => self.render_project(ix, project, &t, cx),
            Row::Workspace(workspace) => self.render_workspace(ix, workspace, &t, cx),
            Row::ShowMore {
                group, expanded, ..
            } => self.render_show_more(ix, group.clone(), *expanded, &t, cx),
            Row::Empty => render_empty(&t, cx),
        };
        let below = match &row {
            Row::Project(_) => PROJECT_GAP,
            Row::Workspace(WorkspaceRow {
                block_end, group, ..
            })
            | Row::ShowMore {
                block_end, group, ..
            } => WORKSPACE_GAP + block_gap(*block_end, group),
            Row::PinnedHeader {
                block_end: true, ..
            } => PINNED_BLOCK_GAP,
            _ => 0.,
        };
        // The list container pads 8 px at the sides, and 16 px under the last row.
        div()
            .w_full()
            .px(zoomed(SPACE_2, cx))
            .pb(zoomed(below + if last { SPACE_4 } else { 0. }, cx))
            .child(body)
            .into_any_element()
    }

    fn render_pinned_header(
        &mut self,
        collapsed: bool,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let muted = t.foreground_muted;
        let chevron = if collapsed {
            IconName::ChevronRight
        } else {
            IconName::ChevronDown
        };
        div()
            .id("sidebar-pinned-header")
            .role(Role::Button)
            .aria_label("Pinned")
            .aria_expanded(!collapsed)
            .group("pinned-header")
            .flex()
            .flex_row()
            .items_center()
            .self_start()
            .min_h(zoomed(ROW_HEIGHT, cx))
            .px(zoomed(SPACE_2, cx))
            .py(zoomed(SPACE_1, cx))
            .gap(zoomed(SPACE_1, cx))
            .text_size(ui_font_size(FONT_SM, cx))
            .text_color(muted)
            .cursor_pointer()
            .on_press(cx, |this, _, cx| this.toggle_pinned(cx))
            .child("Pinned")
            .child(
                div()
                    .invisible()
                    .group_hover("pinned-header", |style| style.visible())
                    .child(icon(chevron).with_size(zoomed(ICON_XS, cx))),
            )
            .into_any_element()
    }

    fn render_project(
        &mut self,
        ix: usize,
        project: &ProjectRow,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let group = SharedString::from(format!("project-{ix}"));
        let id = project.id.clone();
        let chevron = if project.collapsed {
            IconName::ChevronRight
        } else {
            IconName::ChevronDown
        };
        let badge =
            (project.collapsed && project.bucket != StatusBucket::Done).then_some(project.bucket);
        let ring = (badge == Some(StatusBucket::Running))
            .then(|| self.ring(format!("p:{}", project.id), Knockout::Sidebar, cx));
        let leading = div()
            .relative()
            .w(zoomed(ICON_MD, cx))
            .h(zoomed(LINE_HEIGHT, cx))
            .flex_shrink_0()
            .flex()
            .items_center()
            .justify_center()
            .child(
                div()
                    .group_hover(group.clone(), |style| style.invisible())
                    .child(project_icon(
                        &project.color_key,
                        &project.name,
                        badge,
                        ring,
                        &group,
                        t,
                        cx,
                    )),
            )
            .child(
                div()
                    .absolute()
                    .top_0()
                    .left_0()
                    .size_full()
                    .flex()
                    .items_center()
                    .justify_center()
                    .invisible()
                    .group_hover(group.clone(), |style| style.visible())
                    // `#9ca3af` in Electron; the closest token.
                    .text_color(t.foreground_extra_muted)
                    .child(icon(chevron).with_size(zoomed(ICON_SM, cx))),
            );
        let toggle_id = id.clone();
        let click = div()
            .id(("project", ix))
            .role(Role::Button)
            .aria_label(project.name.clone())
            .aria_expanded(!project.collapsed)
            .w_full()
            .min_h(zoomed(ROW_HEIGHT, cx))
            .p(zoomed(SPACE_2, cx))
            // The "+" and the kebab sit on the right rail: 24 px each pulled 6 px out, then the
            // row gap.
            .pr(zoomed(
                SPACE_2 + PROJECT_KEBAB * 2. + PROJECT_ACTION_GAP - 6. + SPACE_2,
                cx,
            ))
            .rounded(zoomed(RADIUS_LG, cx))
            .flex()
            .flex_row()
            .items_center()
            .gap(zoomed(SPACE_2, cx))
            .cursor_pointer()
            .hover(|style| style.bg(t.surface_sidebar_hover))
            .active(|style| style.bg(t.surface2))
            .on_press(cx, move |this, _, cx| this.toggle_project(&toggle_id, cx))
            .child(leading)
            .child(
                div()
                    .flex_1()
                    .min_w_0()
                    .truncate()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .line_height(zoomed(LINE_HEIGHT, cx))
                    .text_color(t.foreground_muted)
                    .child(project.name.clone()),
            );
        let root = project.root.clone();
        let weak = cx.weak_entity();
        let plugin_project = SidebarProject {
            view_key: project.color_key.clone(),
            name: project.name.clone(),
            project_ids: vec![project.id.clone()],
        };
        let kebab = kebab(
            ("project-kebab", ix),
            "Project actions",
            PROJECT_KEBAB,
            RADIUS_MD,
            group.clone(),
            t,
            cx,
            move |menu, window, cx| {
                let menu = menu
                    .min_w(zoomed(220., cx))
                    .check_side(Side::Right)
                    .item(menu_item(
                        "Copy path",
                        IconName::Copy,
                        weak.clone(),
                        root.clone(),
                        cx,
                        |this, path, window, cx| this.copy_path(path, window, cx),
                    ));
                // Plugin items follow "Copy path"; they are read each time the menu opens.
                let items = plugin_menu_items(&plugin_project, cx);
                add_plugin_items(menu, items, window, cx)
            },
        );
        let plus =
            new_workspace_button(ix, &project.name, project.id.clone(), group.clone(), t, cx);
        div()
            .relative()
            .group(group)
            .w_full()
            .child(click)
            .child(
                div()
                    .absolute()
                    .top(zoomed((ROW_HEIGHT - PROJECT_KEBAB) / 2., cx))
                    .right(zoomed(SPACE_2 - 6., cx))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(zoomed(PROJECT_ACTION_GAP, cx))
                    .child(plus)
                    .child(kebab),
            )
            .into_any_element()
    }

    fn render_workspace(
        &mut self,
        ix: usize,
        row: &WorkspaceRow,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let group = SharedString::from(format!("workspace-{ix}"));
        let selected = self.selected.as_deref() == Some(row.id.as_str());
        let archiving = self.archiving.contains(&row.id);
        let pinned_section = row.group == Group::Pinned;
        let leading = if pinned_section {
            // A pinned row has no project row above it, so the project icon leads.
            let badge = (row.bucket != StatusBucket::Done).then_some(row.bucket);
            let ring = (badge == Some(StatusBucket::Running))
                .then(|| self.ring(format!("w:{}:icon", row.id), Knockout::Sidebar, cx));
            div()
                .w(zoomed(ICON_MD, cx))
                .h(zoomed(LINE_HEIGHT, cx))
                .flex_shrink_0()
                .flex()
                .items_center()
                .justify_center()
                .child(project_icon(
                    &row.color_key,
                    &row.project_name,
                    badge,
                    ring,
                    &group,
                    t,
                    cx,
                ))
                .into_any_element()
        } else {
            let ring = (row.bucket == StatusBucket::Running)
                .then(|| self.ring(format!("w:{}:dot", row.id), Knockout::None, cx));
            workspace_status(row, ring, t, cx)
        };
        let click_id = row.id.clone();
        let rest = if selected {
            t.surface_sidebar_selected
        } else {
            t.surface_sidebar_hover
        };
        let title = div()
            .flex_1()
            .min_w_0()
            .truncate()
            .text_size(ui_font_size(FONT_BASE, cx))
            .line_height(zoomed(LINE_HEIGHT, cx))
            .text_color(t.foreground.opacity(0.76))
            .group_hover(group.clone(), |style| style.text_color(t.foreground))
            .child(row.name.clone());
        // `DiffStat` in the trailing slot: 4 px apart, small type, the status colors.
        // A `0/0` stat shows `+0 -0`, as Electron does; no stat shows nothing.
        let diff = row.diff.map(|(added, removed)| {
            h_flex()
                .flex_shrink_0()
                .h(zoomed(LINE_HEIGHT, cx))
                .gap(zoomed(SPACE_1, cx))
                .text_size(ui_font_size(FONT_SM, cx))
                .child(
                    div()
                        .text_color(t.status_success)
                        .child(format!("+{}", format_diff_count(added))),
                )
                .child(
                    div()
                        .text_color(t.status_danger)
                        .child(format!("-{}", format_diff_count(removed))),
                )
        });
        let click = div()
            .id(("workspace", ix))
            .role(Role::Button)
            .aria_label(workspace_label(row, pinned_section))
            .aria_selected(selected)
            .w_full()
            .min_h(zoomed(ROW_HEIGHT, cx))
            .py(zoomed(SPACE_2, cx))
            .pl(zoomed(SPACE_2, cx))
            .pr(zoomed(SPACE_3, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .flex()
            .flex_row()
            .items_center()
            .gap(zoomed(SPACE_2, cx))
            .cursor_pointer()
            .when(selected, |this| this.bg(t.surface_sidebar_selected))
            .when(!selected, |this| {
                this.hover(|style| style.bg(t.surface_sidebar_hover))
            })
            .active(|style| style.bg(t.surface2))
            .when(!archiving, |this| {
                this.on_press(cx, move |this, _, cx| this.open_workspace(&click_id, cx))
            })
            .child(leading)
            .child(title)
            .children(diff);

        let weak = cx.weak_entity();
        let menu_row = row.clone();
        let kebab = kebab(
            ("workspace-kebab", ix),
            "Workspace actions",
            WORKSPACE_KEBAB,
            RADIUS_BASE,
            group.clone(),
            t,
            cx,
            move |menu, _, cx| workspace_menu(menu, &weak, &menu_row, cx),
        );
        // The scrim fades the title tail into the row color under the kebab.
        let scrim = div()
            .absolute()
            .top(zoomed(SPACE_2, cx))
            .right(zoomed(SPACE_3, cx))
            .w(zoomed(SCRIM_WIDTH, cx))
            .h(zoomed(LINE_HEIGHT, cx))
            .bg(linear_gradient(
                90.,
                linear_color_stop(rest.opacity(0.), 0.),
                linear_color_stop(rest, SCRIM_SOLID),
            ))
            .invisible()
            .group_hover(group.clone(), |style| style.visible());
        div()
            .relative()
            .group(group)
            .w_full()
            .child(click)
            .when(!archiving, |this| {
                this.child(scrim).child(
                    div()
                        .absolute()
                        .top(zoomed(SPACE_2, cx))
                        .right(zoomed(SPACE_3 - 7., cx))
                        .child(kebab),
                )
            })
            .when(archiving, |this| {
                this.child(
                    div()
                        .id(("workspace-archiving", ix))
                        .role(Role::Status)
                        .aria_label("Archiving")
                        .absolute()
                        .top_0()
                        .left_0()
                        .size_full()
                        .rounded(zoomed(RADIUS_LG, cx))
                        .bg(t.surface0.opacity(0.8))
                        .flex()
                        .items_center()
                        .justify_center()
                        .text_size(ui_font_size(FONT_SM, cx))
                        .text_color(t.foreground_muted)
                        .child("Archiving..."),
                )
            })
            .into_any_element()
    }

    fn render_show_more(
        &mut self,
        ix: usize,
        group: Group,
        expanded: bool,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let label = if expanded { "Show less" } else { "Show more" };
        let chevron = if expanded {
            IconName::ChevronUp
        } else {
            IconName::ChevronDown
        };
        div()
            .id(("show-more", ix))
            .role(Role::Button)
            .aria_label(label)
            .w_full()
            .min_h(zoomed(ROW_HEIGHT, cx))
            .py(zoomed(SPACE_2, cx))
            .pl(zoomed(SPACE_2, cx))
            .pr(zoomed(SPACE_3, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .flex()
            .flex_row()
            .items_center()
            .gap(zoomed(SPACE_2, cx))
            .cursor_pointer()
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_color(t.foreground_muted)
            .hover(|style| style.bg(t.surface_sidebar_hover).text_color(t.foreground))
            .active(|style| style.bg(t.surface2).text_color(t.foreground))
            .on_press(cx, move |this, _, cx| this.toggle_group(group.clone(), cx))
            .child(
                div()
                    .w(zoomed(ICON_MD, cx))
                    .h(zoomed(ICON_MD, cx))
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(icon(chevron).with_size(zoomed(ICON_SM, cx))),
            )
            .child(div().min_w_0().truncate().child(label))
            .into_any_element()
    }

    // ---- chrome ----

    fn render_nav(&self, t: &Tokens, cx: &mut Context<Self>) -> impl IntoElement {
        // `sidebarHeaderGroup` holds the rows in Electron's default order
        // (`BUILTIN_SIDEBAR_NAV_IDS`): New workspace, History, Search, then the plugin rows.
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .gap(zoomed(SPACE_0_5, cx))
            .pt(zoomed(SPACE_2, cx))
            .pb(zoomed(SPACE_1_5, cx))
            .border_b_1()
            .border_color(t.border)
            .child(builtin_nav_row(
                "sidebar-global-new-workspace",
                "New workspace",
                IconName::Plus,
                t,
                cx,
                |_, _, cx| cx.emit(SidebarNavEvent::NewWorkspace { project_id: None }),
            ))
            .child(builtin_nav_row(
                "sidebar-sessions",
                "History",
                IconName::History,
                t,
                cx,
                |_, _, cx| cx.emit(SidebarNavEvent::OpenHistory),
            ))
            .child(builtin_nav_row(
                "sidebar-search",
                "Search",
                IconName::Search,
                t,
                cx,
                |_, _, cx| cx.emit(SidebarEvent::OpenCommandCenter),
            ))
            .children(self.render_plugin_nav(t, cx))
    }

    /// The rows of the active plugins after Search, in index order (`PluginSidebarItemRow`).
    fn render_plugin_nav(&self, t: &Tokens, cx: &mut Context<Self>) -> Vec<AnyElement> {
        let Some(host) = &self.host else {
            return Vec::new();
        };
        let active = self
            .active_surface
            .as_ref()
            .map(|(plugin, surface)| (plugin.as_str(), surface.as_str()));
        nav_rows(
            host.read(cx).index().sidebar_items(),
            self.selected.as_deref(),
            active,
        )
        .into_iter()
        .enumerate()
        .map(|(ix, row)| self.render_nav_row(ix, row, t, cx))
        .collect()
    }

    fn render_nav_row(
        &self,
        ix: usize,
        row: NavRow,
        t: &Tokens,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let hover = t.surface_sidebar_hover;
        let foreground = t.foreground;
        let (plugin_id, id) = (row.plugin_id, row.id);
        let enabled = !row.disabled;
        div()
            .px(zoomed(SPACE_2, cx))
            .child(
                div()
                    .id(("sidebar-plugin-nav", ix))
                    .role(Role::Button)
                    .aria_label(row.label.clone())
                    .when(row.disabled, |this| {
                        this.aria_description("Disabled: no workspace is open")
                    })
                    .min_h(zoomed(NAV_ROW_MIN, cx))
                    .px(zoomed(SPACE_2, cx))
                    .py(zoomed(SPACE_1, cx))
                    .rounded(zoomed(RADIUS_LG, cx))
                    .flex()
                    .flex_row()
                    .items_center()
                    .gap(zoomed(SPACE_2, cx))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .line_height(zoomed(LABEL_LINE, cx))
                    .text_color(if row.active {
                        foreground
                    } else {
                        t.foreground_muted
                    })
                    .when(row.active, |this| this.bg(hover))
                    .when(row.disabled, |this| this.opacity(0.5))
                    .when(enabled, |this| {
                        this.cursor_pointer()
                            .hover(move |style| style.bg(hover).text_color(foreground))
                            .on_press(cx, move |this, window, cx| {
                                this.press_nav(plugin_id, id, window, cx)
                            })
                    })
                    .child(icon(row.icon).with_size(zoomed(ICON_SM, cx)))
                    .child(div().min_w_0().truncate().child(row.label)),
            )
            .into_any_element()
    }

    /// The sections of the active plugins, between the project list and the footer. They add no
    /// padding or border of their own.
    fn render_sections(&self) -> impl IntoElement {
        div()
            .flex()
            .flex_col()
            .flex_shrink_0()
            .children(self.sections.iter().map(|section| section.view.clone()))
    }

    /// `FooterAddProjectButton`.
    fn render_add_project(&self, t: &Tokens, cx: &mut Context<Self>) -> impl IntoElement {
        let hover = t.surface_sidebar_hover;
        let foreground = t.foreground;
        div()
            .id("sidebar-add-project")
            .role(Role::Button)
            .aria_label("Add project")
            .min_w_0()
            .min_h(zoomed(ADD_PROJECT_MIN, cx))
            .flex_1()
            .px(zoomed(SPACE_2, cx))
            .py(zoomed(SPACE_1_5, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .flex()
            .flex_row()
            .items_center()
            .gap(zoomed(SPACE_2, cx))
            .text_size(ui_font_size(FONT_BASE, cx))
            .line_height(zoomed(LABEL_LINE, cx))
            .text_color(t.foreground_muted)
            .when(self.adding, |this| this.opacity(0.5))
            .when(!self.adding, |this| {
                this.cursor_pointer()
                    .hover(move |style| style.bg(hover).text_color(foreground))
                    .on_press(cx, |this, window, cx| this.add_project(window, cx))
            })
            .child(icon(IconName::FolderPlus).with_size(zoomed(ICON_SM, cx)))
            .child(div().min_w_0().truncate().child("Add project"))
    }

    fn render_footer(&self, t: &Tokens, cx: &mut Context<Self>) -> impl IntoElement {
        h_flex()
            .flex_shrink_0()
            .justify_between()
            .items_center()
            .gap(zoomed(SPACE_2, cx))
            .px(zoomed(SPACE_2, cx))
            .py(zoomed(SPACE_3, cx))
            .border_t_1()
            .border_color(t.border)
            .child(self.render_add_project(t, cx))
            .child(
                div()
                    .id("sidebar-settings")
                    .role(Role::Button)
                    .aria_label("Settings")
                    .flex_shrink_0()
                    .w(zoomed(28., cx))
                    .h(zoomed(28., cx))
                    .flex()
                    .items_center()
                    .justify_center()
                    .cursor_pointer()
                    .text_color(t.foreground_muted)
                    .hover(|style| style.text_color(t.foreground))
                    .on_press(cx, |_, _, cx| cx.emit(SidebarEvent::OpenSettings))
                    .child(icon(IconName::Settings).with_size(zoomed(ICON_MD, cx))),
            )
    }

    fn render_resize_handle(&self, t: &Tokens, cx: &App) -> impl IntoElement {
        div()
            .id("sidebar-resize-handle")
            .role(Role::Splitter)
            .aria_label("Resize sidebar")
            .aria_numeric_value(f64::from(self.width))
            .group("sidebar-resize")
            .absolute()
            .top_0()
            .bottom_0()
            .right(zoomed(-HANDLE_WIDTH / 2., cx))
            .w(zoomed(HANDLE_WIDTH, cx))
            .cursor_col_resize()
            .on_drag(ResizeDrag, |_, _, _, cx| cx.new(|_| NoPreview))
            .child(
                // `highlight`: 1 px, foreground at 25%, 5 px in from the handle's left edge.
                div()
                    .absolute()
                    .top_0()
                    .bottom_0()
                    .left(zoomed(HANDLE_WIDTH / 2., cx))
                    .w(px(1.))
                    .bg(t.foreground.opacity(0.25))
                    .invisible()
                    .group_hover("sidebar-resize", |style| style.visible()),
            )
    }
}

fn block_gap(block_end: bool, group: &Group) -> f32 {
    match (block_end, group) {
        (false, _) => 0.,
        (true, Group::Pinned) => PINNED_BLOCK_GAP,
        (true, Group::Project(_)) => PROJECT_BLOCK_GAP,
    }
}

/// The ring entity key of a row that shows one, so the view drops rings that stop running.
fn ring_key(row: &Row) -> Option<String> {
    match row {
        Row::Workspace(w) if w.bucket == StatusBucket::Running => {
            Some(if w.group == Group::Pinned {
                format!("w:{}:icon", w.id)
            } else {
                format!("w:{}:dot", w.id)
            })
        }
        Row::Project(p) if p.collapsed && p.bucket == StatusBucket::Running => {
            Some(format!("p:{}", p.id))
        }
        _ => None,
    }
}

/// A built-in nav row (`SidebarHeaderRow`, compact variant).
fn builtin_nav_row(
    id: &'static str,
    label: &'static str,
    glyph: IconName,
    t: &Tokens,
    cx: &mut Context<Sidebar>,
    handler: impl Fn(&mut Sidebar, &mut Window, &mut Context<Sidebar>) + 'static,
) -> impl IntoElement {
    div().px(zoomed(SPACE_2, cx)).child(
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .min_h(zoomed(NAV_ROW_MIN, cx))
            .px(zoomed(SPACE_2, cx))
            .py(zoomed(SPACE_1, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .flex()
            .flex_row()
            .items_center()
            .gap(zoomed(SPACE_2, cx))
            .cursor_pointer()
            .text_size(ui_font_size(FONT_BASE, cx))
            .line_height(zoomed(LABEL_LINE, cx))
            .text_color(t.foreground_muted)
            .hover(|style| style.bg(t.surface_sidebar_hover).text_color(t.foreground))
            .on_press(cx, handler)
            .child(icon(glyph).with_size(zoomed(ICON_SM, cx)))
            .child(label),
    )
}

fn render_section_title(title: SharedString, t: &Tokens, cx: &mut Context<Sidebar>) -> AnyElement {
    // `workspacesSectionHeader`: the title with the display menu, which is out of scope. A
    // plugin filter title replaces "Workspaces".
    div()
        .id("sidebar-workspaces-title")
        .role(Role::Heading)
        .aria_label(title.clone())
        .flex()
        .flex_row()
        .items_center()
        .min_h(zoomed(SECTION_TITLE_HEIGHT, cx))
        .pl(zoomed(SPACE_2, cx))
        .pr(zoomed(SPACE_1, cx))
        .py(zoomed(SPACE_1, cx))
        .text_size(ui_font_size(FONT_SM, cx))
        .text_color(t.foreground_muted)
        .child(div().min_w_0().truncate().child(title))
        .into_any_element()
}

/// The height of the "Workspaces" heading row: the 24 px display button plus its padding.
const SECTION_TITLE_HEIGHT: f32 = 32.;

/// `SidebarProjectEmptyState` without its two buttons (open project and import are out of scope).
fn render_empty(t: &Tokens, cx: &mut Context<Sidebar>) -> AnyElement {
    div()
        .id("sidebar-empty-state")
        .role(Role::Status)
        .aria_label("No projects yet")
        .mx(zoomed(SPACE_2, cx))
        .mt(zoomed(SPACE_4, cx))
        .pt(zoomed(SPACE_6, cx))
        .pb(zoomed(SPACE_4, cx))
        .px(zoomed(SPACE_4, cx))
        .rounded(zoomed(RADIUS_LG, cx))
        .bg(t.surface0)
        .flex()
        .flex_col()
        .items_center()
        .gap(zoomed(SPACE_3, cx))
        .text_size(ui_font_size(FONT_SM, cx))
        .text_center()
        .child(
            div()
                .font_weight(FontWeight(WEIGHT_MEDIUM))
                .text_color(t.foreground)
                .child("No projects yet"),
        )
        .child(
            div()
                .text_color(t.foreground_muted)
                .child("Add a project to get started"),
        )
        .into_any_element()
}

/// A disc of the alert color with a bang, which Electron draws as a filled `CircleAlert`.
fn alert_disc(side: f32, t: &Tokens, cx: &App) -> Div {
    div()
        .w(zoomed(side, cx))
        .h(zoomed(side, cx))
        .flex_shrink_0()
        .rounded_full()
        .bg(t.dot_warning)
        .flex()
        .items_center()
        .justify_center()
        .text_color(t.surface0)
        .text_size(zoomed(side * 0.75, cx))
        .font_weight(FontWeight::BOLD)
        .line_height(zoomed(side, cx))
        .child("!")
}

fn dot(color: Hsla, cx: &App) -> Div {
    div()
        .w(zoomed(DOT, cx))
        .h(zoomed(DOT, cx))
        .flex_shrink_0()
        .rounded_full()
        .bg(color)
}

/// The mark in the leading slot of a workspace row: `WorkspaceStatusIndicator`.
fn workspace_status(
    row: &WorkspaceRow,
    ring: Option<AnyElement>,
    t: &Tokens,
    cx: &mut Context<Sidebar>,
) -> AnyElement {
    let slot = div()
        .relative()
        .w(zoomed(ICON_MD, cx))
        .h(zoomed(LINE_HEIGHT, cx))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center();
    match row.bucket {
        StatusBucket::Running => slot.children(ring).into_any_element(),
        StatusBucket::NeedsInput => slot.child(alert_disc(12., t, cx)).into_any_element(),
        StatusBucket::Attention => slot.child(dot(t.dot_success, cx)).into_any_element(),
        // An idle row keeps a muted dot so the rail has an edge to read against.
        StatusBucket::Done => slot
            .child(dot(t.foreground_extra_muted.opacity(0.3), cx))
            .into_any_element(),
        StatusBucket::Failed => slot
            .child(
                icon(IconName::Monitor)
                    .with_size(zoomed(ICON_SM, cx))
                    .text_color(t.foreground_muted),
            )
            .child(
                div()
                    .absolute()
                    .right_0()
                    .bottom_0()
                    .w(zoomed(DOT + 2., cx))
                    .h(zoomed(DOT + 2., cx))
                    .rounded_full()
                    .border_1()
                    .border_color(t.surface0)
                    .bg(t.dot_danger),
            )
            .into_any_element(),
    }
}

/// `ProjectStatusIndicator`: the lettered project box, with the status mark on its corner.
fn project_icon(
    key: &str,
    name: &str,
    badge: Option<StatusBucket>,
    ring: Option<AnyElement>,
    group: &SharedString,
    t: &Tokens,
    cx: &mut Context<Sidebar>,
) -> AnyElement {
    // `key` is the project view key: the daemon `projectKey`, or `[serverId, projectId]`
    // without one. The project id stands in only when the client cannot give a key.
    let color: Rgba = rgb(identity_color(key));
    // `projectIconRadius`: a quarter of the box.
    let radius = (ICON_MD * 0.25).round();
    let mut icon_box = div()
        .relative()
        .w(zoomed(ICON_MD, cx))
        .h(zoomed(ICON_MD, cx))
        .child(
            div()
                .size_full()
                .rounded(zoomed(radius, cx))
                .bg(color)
                .flex()
                .items_center()
                .justify_center()
                .text_color(white())
                .text_size(zoomed(9., cx))
                .child(project_initial(name)),
        );
    if let Some(bucket) = badge {
        icon_box = if bucket == StatusBucket::Running {
            icon_box.child(
                div()
                    .absolute()
                    .right(zoomed(RING_OFFSET, cx))
                    .bottom(zoomed(RING_OFFSET, cx))
                    .children(ring),
            )
        } else {
            let hover = t.surface_sidebar_hover;
            icon_box.child(
                // The shell is a knockout: it takes the color of the row behind it.
                div()
                    .absolute()
                    .right(zoomed(BADGE_OFFSET, cx))
                    .bottom(zoomed(BADGE_OFFSET, cx))
                    .w(zoomed(BADGE, cx))
                    .h(zoomed(BADGE, cx))
                    .rounded_full()
                    .overflow_hidden()
                    .bg(t.surface_sidebar)
                    .group_hover(group.clone(), move |style| style.bg(hover))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(match bucket {
                        StatusBucket::NeedsInput => alert_disc(BADGE, t, cx),
                        StatusBucket::Failed => dot(t.dot_danger, cx),
                        _ => dot(t.dot_success, cx),
                    }),
            )
        };
    }
    icon_box.into_any_element()
}

/// One menu row: a muted icon, a label, and a handler that gets the view and a string.
fn menu_item(
    label: &'static str,
    name: IconName,
    weak: WeakEntity<Sidebar>,
    value: String,
    cx: &App,
    run: impl Fn(&mut Sidebar, String, &mut Window, &mut Context<Sidebar>) + 'static,
) -> PopupMenuItem {
    PopupMenuItem::new(label)
        .icon(icon(name).with_size(zoomed(ICON_SM, cx)))
        .on_click(move |_, window, cx| {
            let value = value.clone();
            weak.update(cx, |this, cx| run(this, value, window, cx))
                .ok();
        })
}

/// The items every project menu contribution adds for `project`, in index order.
fn plugin_menu_items(project: &SidebarProject, cx: &App) -> Vec<ProjectMenuItem> {
    let Some(host) = bachuc_plugin_host::host(cx) else {
        return Vec::new();
    };
    host.read(cx)
        .index()
        .project_menus()
        .iter()
        .flat_map(|menu| (menu.value.items)(project, cx))
        .collect()
}

/// Adds plugin items: title only, no icon. `disabled` greys an item, `checked` ends it with a
/// check, and an item with children opens a submenu that ends with its checked child's title.
fn add_plugin_items(
    mut menu: PopupMenu,
    items: Vec<ProjectMenuItem>,
    window: &mut Window,
    cx: &mut Context<PopupMenu>,
) -> PopupMenu {
    for item in items {
        if item.items.is_empty() || item.disabled {
            let on_select = item.on_select.clone();
            menu = menu.item(
                PopupMenuItem::new(item.title.clone())
                    .disabled(item.disabled)
                    .checked(item.checked)
                    .on_click(move |_, window, cx| {
                        if let Some(on_select) = &on_select {
                            on_select((), window, cx);
                        }
                    }),
            );
        } else {
            let label = submenu_row_label(&item.title, submenu_trailing(&item.items));
            let children = item.items.clone();
            menu = menu.submenu(label, window, cx, move |sub, window, cx| {
                add_plugin_items(sub.check_side(Side::Right), children.clone(), window, cx)
            });
        }
    }
    menu
}

/// Adds the project and runs every filter's `on_project_added` in index order. Returns the
/// project id and the folder, or the text of the first error.
async fn add_project_and_notify(
    this: &WeakEntity<Sidebar>,
    cwd: &str,
    cx: &mut AsyncWindowContext,
) -> Result<Option<(String, String)>, String> {
    let task = this
        .update(cx, |this, cx| {
            this.client
                .update(cx, |client, cx| client.add_project(cwd, cx))
        })
        .map_err(|error| error.to_string())?;
    let added = task.await.map_err(|error| error.to_string())?;
    let project = added_sidebar_project(&added);
    let filters = this
        .update(cx, |this, cx| {
            this.host
                .as_ref()
                .map(|host| {
                    host.read(cx)
                        .index()
                        .project_filters()
                        .iter()
                        .filter_map(|filter| filter.value.on_project_added.clone())
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default()
        })
        .map_err(|error| error.to_string())?;
    let cx = cx.clone();
    let steps = filters.into_iter().map(|filter| {
        let project = project.clone();
        let mut cx = cx.clone();
        move || async move {
            let task = cx.update(|_, app| filter(project, app))?;
            task.await
        }
    });
    run_in_order(steps).await?;
    Ok(Some((added.id, cwd.to_string())))
}

/// `SidebarWorkspaceMenuItems`, in the Electron order, for the items whose target is in scope.
fn workspace_menu(
    menu: PopupMenu,
    weak: &WeakEntity<Sidebar>,
    row: &WorkspaceRow,
    cx: &App,
) -> PopupMenu {
    let mut menu = menu.min_w(zoomed(260., cx)).item(menu_item(
        "Copy path",
        IconName::Copy,
        weak.clone(),
        row.cwd.clone(),
        cx,
        |this, path, window, cx| this.copy_path(path, window, cx),
    ));
    menu = menu.item(menu_item(
        "Rename workspace",
        IconName::Pencil,
        weak.clone(),
        row.id.clone(),
        cx,
        {
            let current = row.name.clone();
            move |this, id, window, cx| this.rename(id, current.clone(), window, cx)
        },
    ));
    // `canMarkUnread`: a workspace with nothing to look at.
    if row.bucket == StatusBucket::Done {
        menu = menu.item(menu_item(
            "Mark as unread",
            IconName::Circle,
            weak.clone(),
            row.id.clone(),
            cx,
            |this, id, _, cx| this.mark_unread(&id, cx),
        ));
    }
    let pinned = row.pinned;
    menu = menu.item(menu_item(
        if pinned { "Unpin" } else { "Pin to top" },
        if pinned {
            IconName::PinOff
        } else {
            IconName::Pin
        },
        weak.clone(),
        row.id.clone(),
        cx,
        move |this, id, _, cx| this.pin(&id, !pinned, cx),
    ));
    let name = row.name.clone();
    menu.item(menu_item(
        "Archive",
        IconName::Archive,
        weak.clone(),
        row.id.clone(),
        cx,
        move |this, id, window, cx| this.archive(id, name.clone(), window, cx),
    ))
}

/// A hover kebab with its menu. `group` is the row's group, which shows the button.
/// `NewWorktreeButton`: a 24 px "+" that fades in on row hover. Opacity keeps its slot, so the
/// row never shifts, and the pointer reaches it only through the hovered row.
fn new_workspace_button(
    ix: usize,
    project_name: &str,
    project_id: String,
    group: SharedString,
    t: &Tokens,
    cx: &mut Context<Sidebar>,
) -> impl IntoElement + use<> {
    let fg = t.foreground;
    div()
        .id(("project-new-workspace", ix))
        .role(Role::Button)
        .aria_label(format!("Create a new workspace for {project_name}"))
        .w(zoomed(PROJECT_KEBAB, cx))
        .h(zoomed(PROJECT_KEBAB, cx))
        .flex_shrink_0()
        .rounded(zoomed(RADIUS_MD, cx))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_color(t.foreground_muted)
        .opacity(0.)
        .group_hover(group, |style| style.opacity(1.))
        .hover(move |style| style.bg(t.surface_sidebar_hover).text_color(fg))
        .on_press(cx, move |_, _, cx| {
            cx.emit(SidebarNavEvent::NewWorkspace {
                project_id: Some(project_id.clone()),
            })
        })
        .child(icon(IconName::Plus).with_size(zoomed(PROJECT_PLUS_GLYPH, cx)))
}

fn kebab<B>(
    id: (&'static str, usize),
    label: &'static str,
    side: f32,
    radius: f32,
    group: SharedString,
    t: &Tokens,
    cx: &App,
    build: B,
) -> impl IntoElement + use<B>
where
    B: Fn(PopupMenu, &mut Window, &mut Context<PopupMenu>) -> PopupMenu + 'static,
{
    let fg = t.foreground;
    let base = div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .w(zoomed(side, cx))
        .h(zoomed(side, cx))
        .rounded(zoomed(radius, cx))
        .flex()
        .items_center()
        .justify_center()
        .cursor_pointer()
        .text_color(t.foreground_muted)
        .hover(move |style| style.bg(t.surface2).text_color(fg))
        // No mouse-down listener here: a child's listener runs before the popover wrapper's,
        // so stopping the event here would keep the menu shut. The wrapper stops it itself,
        // which also keeps the row under the kebab from taking the click.
        .child(icon(IconName::MoreVertical).with_size(zoomed(ICON_SM, cx)));
    KebabButton {
        base,
        open: false,
        group,
        fill: t.surface2,
    }
    .dropdown_menu_with_anchor(Anchor::TopRight, build)
}

impl Render for Sidebar {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        // The app may create the plugin host after the sidebar.
        self.attach_host(window, cx);
        let t = tokens(cx);
        // The filter answers a swipe within the same effect cycle. No answer means no Space
        // that way, or a save that queued the swipe; its change slides in later.
        if let Some((direction, from)) = self.awaiting.take() {
            self.swipe_edge = Some(direction);
            self.snap_back(from, cx);
        }
        let frame = match self.drag {
            Some(offset) => Some(drag_frame(offset, self.width, self.blocked(offset))),
            None => {
                let frame = self.slide.as_ref().and_then(|slide| {
                    slide_frame(
                        slide.target,
                        slide.from,
                        slide.started.elapsed().as_secs_f32() * 1000.,
                    )
                });
                if frame.is_some() {
                    window.request_animation_frame();
                } else {
                    self.slide = None;
                }
                frame
            }
        };
        let width = self.width;
        let sidebar = cx.weak_entity();
        // The swipe takes the wheel in the capture phase over the tree, which stays put while
        // the rows move with the finger, so it can keep a horizontal swipe from the list.
        let wheel = canvas(
            |bounds, window, _| window.insert_hitbox(bounds, HitboxBehavior::Normal),
            move |_, hitbox, window, _| {
                window.on_mouse_event(move |event: &ScrollWheelEvent, phase, window, cx| {
                    if phase == DispatchPhase::Capture && hitbox.should_handle_scroll(window) {
                        sidebar.update(cx, |this, cx| this.on_wheel(event, cx)).ok();
                    }
                });
            },
        )
        .absolute()
        .size_full();
        let tree = div()
            .id("sidebar-tree")
            .role(Role::Tree)
            .aria_label("Projects and workspaces")
            .flex_1()
            .min_h_0()
            .pt(zoomed(SPACE_0_5, cx))
            .overflow_hidden()
            .child(wheel)
            .child(
                // Offset with `left`, as Electron does, and only while a drag or a slide runs.
                div()
                    .size_full()
                    .relative()
                    .when_some(frame, |this, frame| {
                        this.left(px(frame.offset * width)).opacity(frame.opacity)
                    })
                    .child(list(self.list.clone(), cx.processor(Self::render_row)).size_full()),
            );
        div()
            .id("sidebar")
            .role(Role::Navigation)
            .aria_label("Sidebar")
            .relative()
            .flex()
            .flex_col()
            .h_full()
            .w(px(self.width))
            .flex_shrink_0()
            .bg(t.surface_sidebar)
            .border_r_1()
            .border_color(t.border)
            .text_color(t.foreground)
            .on_drag_move::<ResizeDrag>(cx.listener(
                |this, event: &DragMoveEvent<ResizeDrag>, _, cx| {
                    let width =
                        clamp_width(f32::from(event.event.position.x - event.bounds.origin.x));
                    if (width - this.width).abs() >= 0.5 {
                        this.width = width;
                        cx.notify();
                    }
                },
            ))
            .child(self.render_nav(&t, cx))
            .child(tree)
            .child(self.render_sections())
            .child(self.render_footer(&t, cx))
            // The handle hangs 5 px outside the sidebar, so it paints above its neighbor.
            .child(deferred(self.render_resize_handle(&t, cx)).with_priority(1))
    }
}
