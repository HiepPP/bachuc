//! The plugin rules of the sidebar without any drawing: nav rows, project filters, the title,
//! the swipe gesture, the slide, menu values, and the Add project sequence. Each rule is a
//! plain function, because GPUI view crates have no headless test context here.
//!
//! Sources: `plugins/sidebar-items.tsx`, `plugins/sidebar/model.ts`, `wheel-gesture.ts`,
//! `swipe.web.ts`, and `slide.tsx` under `packages/app/src`.

use crate::model::{Facts, Row, ViewState, build_rows};
use bachuc_client::{AddedProject, AgentSummary, ProjectNode};
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    Contributed, ProjectMenuItem, SidebarItem, SidebarProject, SwipeDirection,
};
use gpui_kit::{ScrollDelta, SharedString};
use std::collections::HashMap;
use std::future::Future;

// ---- nav rows ----

/// One plugin row under Search, as the view draws it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct NavRow {
    pub plugin_id: &'static str,
    pub id: &'static str,
    pub label: SharedString,
    pub icon: IconName,
    pub surface: &'static str,
    /// An item that needs a workspace while none is selected.
    pub disabled: bool,
    /// The row of the surface that is open, drawn with the hover colors.
    pub active: bool,
    pub has_action: bool,
}

/// The rows of `items` in index order. `active_surface` is (plugin id, surface id).
pub fn nav_rows(
    items: &[Contributed<SidebarItem>],
    selected_workspace: Option<&str>,
    active_surface: Option<(&str, &str)>,
) -> Vec<NavRow> {
    items
        .iter()
        .map(|contributed| {
            let item = &contributed.value;
            NavRow {
                plugin_id: contributed.plugin_id,
                id: item.id,
                label: item.title.clone(),
                icon: item.icon,
                surface: item.surface,
                disabled: item
                    .action
                    .as_ref()
                    .is_some_and(|action| action.requires_workspace)
                    && selected_workspace.is_none(),
                active: active_surface == Some((contributed.plugin_id, item.surface)),
                has_action: item.action.is_some(),
            }
        })
        .collect()
}

/// What a press on a nav row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum NavPress {
    Ignored,
    /// Call `on_press` with the selected workspace id.
    Action,
    OpenSurface,
}

pub fn nav_press(row: &NavRow) -> NavPress {
    if row.disabled {
        NavPress::Ignored
    } else if row.has_action {
        NavPress::Action
    } else {
        NavPress::OpenSurface
    }
}

// ---- project filters ----

/// The project as filters and menus see it. The view key is the daemon project key, else the
/// project id, as the row icon color hashes it.
pub fn sidebar_project(project: &ProjectNode, view_key: Option<&String>) -> SidebarProject {
    SidebarProject {
        view_key: view_key.cloned().unwrap_or_else(|| project.id.clone()),
        name: project.name.clone(),
        project_ids: vec![project.id.clone()],
    }
}

/// The project a finished `project.add` made.
pub fn added_sidebar_project(added: &AddedProject) -> SidebarProject {
    SidebarProject {
        view_key: added.view_key.clone(),
        name: added.name.clone(),
        project_ids: vec![added.id.clone()],
    }
}

/// The projects that every filter shows, with their workspaces; a hidden project drops its
/// pinned workspaces too. `keys` is [`Facts::project_keys`].
pub fn visible_projects(
    projects: &[ProjectNode],
    keys: &HashMap<String, String>,
    is_visible: impl Fn(&SidebarProject) -> bool,
) -> Vec<ProjectNode> {
    projects
        .iter()
        .filter(|project| is_visible(&sidebar_project(project, keys.get(&project.id))))
        .cloned()
        .collect()
}

/// The rows of the visible projects. When filters hide every project the list shows its title
/// and no onboarding row, because the user does have projects (`sidebar-workspace-list.tsx`
/// line 2433).
pub fn filtered_rows(
    all: &[ProjectNode],
    visible: &[ProjectNode],
    agents: &[AgentSummary],
    facts: &Facts,
    state: &ViewState,
) -> Vec<Row> {
    if visible.is_empty() && !all.is_empty() {
        return vec![Row::SectionTitle];
    }
    build_rows(visible, agents, facts, state)
}

/// The first non-empty trimmed title; `None` keeps "Workspaces".
pub fn pick_title(titles: impl IntoIterator<Item = Option<SharedString>>) -> Option<SharedString> {
    titles.into_iter().flatten().find_map(|title| {
        let trimmed = title.trim();
        (!trimmed.is_empty()).then(|| SharedString::from(trimmed.to_string()))
    })
}

// ---- swipe ----

/// Turns trackpad wheel deltas into at most one swipe per gesture (`wheel-gesture.ts`). A quiet
/// gap ends a gesture, and the momentum tail does not fire again unless a new stroke
/// accelerates through it. Deltas use the web sign: a finger moving left is positive `x`.
#[derive(Clone, Debug)]
pub struct WheelGesture {
    last: f64,
    x: f32,
    y: f32,
    fired: bool,
    fired_at: f64,
    previous_magnitude: f32,
    tail_minimum: f32,
    rising: u32,
}

impl Default for WheelGesture {
    fn default() -> Self {
        Self {
            last: f64::NEG_INFINITY,
            x: 0.,
            y: 0.,
            fired: false,
            fired_at: f64::NEG_INFINITY,
            previous_magnitude: 0.,
            tail_minimum: f32::INFINITY,
            rising: 0,
        }
    }
}

/// A quiet gap that ends a gesture.
const QUIET_GAP_MS: f64 = 120.;
const MIN_TRAVEL: f32 = 28.;
const HORIZONTAL_RATIO: f32 = 1.5;
/// A line delta counts as this many pixels (`WHEEL_DELTA_SCALE`).
const LINE_PIXELS: f32 = 16.;

impl WheelGesture {
    pub fn push(&mut self, dx: f32, dy: f32, time: f64) -> Option<SwipeDirection> {
        let magnitude = dx.abs();
        let reversed = self.fired && dx * self.x < 0. && magnitude >= 2.;
        if self.fired && time - self.fired_at > 80. {
            self.tail_minimum = self.tail_minimum.min(magnitude);
            self.rising = if magnitude > self.previous_magnitude {
                self.rising + 1
            } else {
                0
            };
        }
        let fresh_push = self.fired
            && time - self.fired_at > 160.
            && magnitude >= 4f32.max(self.tail_minimum * 1.6)
            && (self.rising >= 2 || magnitude >= 8f32.max(self.previous_magnitude * 2.));
        if time - self.last > QUIET_GAP_MS || reversed || fresh_push {
            self.x = 0.;
            self.y = 0.;
            self.fired = false;
            self.tail_minimum = f32::INFINITY;
            self.rising = 0;
        }
        self.last = time;
        self.previous_magnitude = magnitude;
        self.x += dx;
        self.y += dy.abs();
        if self.fired || self.x.abs() < MIN_TRAVEL || self.x.abs() < self.y * HORIZONTAL_RATIO {
            return None;
        }
        self.fired = true;
        self.fired_at = time;
        self.tail_minimum = f32::INFINITY;
        Some(if self.x > 0. {
            SwipeDirection::Next
        } else {
            SwipeDirection::Previous
        })
    }
}

/// The web-sign pixel deltas of a GPUI scroll event. GPUI reports where the content moves, and
/// the web reports where the page scrolls, so the sign flips.
pub fn wheel_deltas(delta: ScrollDelta) -> (f32, f32) {
    let (x, y) = match delta {
        ScrollDelta::Pixels(point) => (f32::from(point.x), f32::from(point.y)),
        ScrollDelta::Lines(point) => (point.x * LINE_PIXELS, point.y * LINE_PIXELS),
    };
    (-x, -y)
}

/// Whether a scroll belongs to the swipe and not to the list: it moves more across than down.
pub fn is_horizontal(dx: f32, dy: f32) -> bool {
    dx.abs() > dy.abs()
}

// ---- slide ----

pub const SLIDE_OUT_MS: f32 = 65.;
pub const SLIDE_IN_MS: f32 = 105.;

/// Where the list is during a slide. `offset` is a fraction of the list width: -1 is fully off
/// to the left, 1 fully off to the right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlideFrame {
    pub offset: f32,
    pub opacity: f32,
    /// The filter change shows from this frame on (the list is out of sight).
    pub applied: bool,
}

/// `cubic-bezier(x1, y1, x2, y2)` at progress `t`.
fn cubic_bezier(x1: f32, y1: f32, x2: f32, y2: f32, t: f32) -> f32 {
    let curve = |a: f32, b: f32, s: f32| {
        let inverse = 1. - s;
        3. * inverse * inverse * s * a + 3. * inverse * s * s * b + s * s * s
    };
    // Bisect for the curve parameter whose x is `t`; x is monotonic for these controls.
    let (mut low, mut high) = (0f32, 1f32);
    for _ in 0..24 {
        let mid = (low + high) / 2.;
        if curve(x1, x2, mid) < t {
            low = mid;
        } else {
            high = mid;
        }
    }
    curve(y1, y2, (low + high) / 2.)
}

/// The slide at `elapsed_ms` after a swipe: out against `direction` over 65 ms, then in from
/// `direction` over 105 ms. `None` once it ends.
pub fn slide_frame(direction: SwipeDirection, elapsed_ms: f32) -> Option<SlideFrame> {
    let sign = match direction {
        SwipeDirection::Next => 1.,
        SwipeDirection::Previous => -1.,
    };
    if elapsed_ms < SLIDE_OUT_MS {
        let eased = cubic_bezier(0.4, 0., 1., 1., (elapsed_ms.max(0.)) / SLIDE_OUT_MS);
        Some(SlideFrame {
            offset: -sign * eased,
            opacity: 1. - eased,
            applied: false,
        })
    } else if elapsed_ms < SLIDE_OUT_MS + SLIDE_IN_MS {
        let eased = cubic_bezier(0.16, 1., 0.3, 1., (elapsed_ms - SLIDE_OUT_MS) / SLIDE_IN_MS);
        Some(SlideFrame {
            offset: sign * (1. - eased),
            opacity: eased,
            applied: true,
        })
    } else {
        None
    }
}

// ---- project menu ----

/// The value shown at the end of a submenu row: the title of its first checked child.
pub fn submenu_trailing(items: &[ProjectMenuItem]) -> Option<SharedString> {
    items
        .iter()
        .find(|item| item.checked)
        .map(|item| item.title.clone())
}

/// The label of a submenu row. The gpui-kit menu has no trailing slot on a submenu row, so the
/// value follows the title.
pub fn submenu_row_label(title: &SharedString, trailing: Option<SharedString>) -> SharedString {
    match trailing {
        Some(value) => format!("{title}: {value}").into(),
        None => title.clone(),
    }
}

// ---- Add project ----

/// Runs the steps one after another and stops at the first `Err`, returning its text. A filter
/// that fails leaves the project added.
pub async fn run_in_order<F, Fut>(steps: impl IntoIterator<Item = F>) -> Result<(), String>
where
    F: FnOnce() -> Fut,
    Fut: Future<Output = anyhow::Result<()>>,
{
    for step in steps {
        step().await.map_err(|error| error.to_string())?;
    }
    Ok(())
}
