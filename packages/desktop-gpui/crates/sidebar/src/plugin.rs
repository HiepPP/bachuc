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
use gpui_kit::{ScrollDelta, SharedString, TouchPhase};
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

/// What one scroll event, or a quiet timeout, does to the swipe. Offsets are pixels in the web
/// sign: a finger moving left is positive and points at the next Space.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum SwipeStep {
    /// Not part of a swipe: a vertical scroll, a momentum tail, or too little travel.
    Ignore,
    /// The list follows the finger.
    Drag { offset: f32 },
    /// The fingers lifted past half the width or with a flick: go one Space in `direction`.
    Commit {
        direction: SwipeDirection,
        offset: f32,
    },
    /// The fingers lifted short of a switch, or the system took the gesture.
    SnapBack { offset: f32 },
}

#[derive(Clone, Copy, Debug, PartialEq)]
enum Mode {
    Idle,
    /// Travel before the gesture picks an axis.
    Pending {
        x: f32,
        y: f32,
    },
    Dragging {
        offset: f32,
    },
    /// A vertical gesture: the list scrolls until it ends.
    Vertical,
    /// After a release: the momentum tail, ignored until a new gesture.
    Tail,
}

/// The two-finger swipe over the project list as a state machine. GPUI gives each scroll event
/// a phase: `Started` for macOS MayBegin and Began, `Ended`, and `Moved` for everything else,
/// momentum and Cancelled included. A mouse wheel never sends `Started`, so a quiet gap ends
/// its gesture.
#[derive(Clone, Debug)]
pub struct SwipeGesture {
    mode: Mode,
    /// The gesture began with `Started`, so `Ended` will close it.
    phased: bool,
    last: f64,
    last_move: f64,
    /// Pixels per millisecond, smoothed over the last moves.
    velocity: f32,
}

impl Default for SwipeGesture {
    fn default() -> Self {
        Self {
            mode: Mode::Idle,
            phased: false,
            last: f64::NEG_INFINITY,
            last_move: f64::NEG_INFINITY,
            velocity: 0.,
        }
    }
}

/// A quiet gap that ends a gesture without phases, and the tail after a release.
const QUIET_GAP_MS: f64 = 120.;
/// A phased gesture ends without `Ended` only after this long; macOS sends Cancelled as `Moved`.
const PHASED_TIMEOUT_MS: f64 = 1500.;
/// Travel before the gesture picks an axis.
const AXIS_LOCK: f32 = 8.;
const HORIZONTAL_RATIO: f32 = 1.5;
/// A lift this fast, toward the offset, switches even short of half the width.
const FLICK_VELOCITY: f32 = 0.5;
const FLICK_MIN_TRAVEL: f32 = 24.;
/// Fingers that rested this long before lifting carry no velocity.
const STALE_MOVE_MS: f64 = 60.;
/// A line delta counts as this many pixels (`WHEEL_DELTA_SCALE`).
const LINE_PIXELS: f32 = 16.;

impl SwipeGesture {
    /// Feeds one scroll event. `width` is the list width in pixels.
    pub fn push(
        &mut self,
        dx: f32,
        dy: f32,
        phase: TouchPhase,
        time: f64,
        width: f32,
    ) -> SwipeStep {
        let quiet = time - self.last > QUIET_GAP_MS;
        self.last = time;
        match phase {
            TouchPhase::Started => {
                // A new gesture during a drag means the old one never ended.
                let lost = match self.mode {
                    Mode::Dragging { offset } => Some(SwipeStep::SnapBack { offset }),
                    _ => None,
                };
                self.begin(true);
                let step = self.travel(dx, dy, time);
                return lost.unwrap_or(step);
            }
            TouchPhase::Ended => {
                self.travel(dx, dy, time);
                return self.release(time, width);
            }
            TouchPhase::Cancelled => return self.cancel(),
            TouchPhase::Moved => {}
        }
        match self.mode {
            Mode::Idle | Mode::Tail | Mode::Vertical if quiet => self.begin(false),
            Mode::Idle | Mode::Tail | Mode::Vertical => return SwipeStep::Ignore,
            Mode::Pending { .. } | Mode::Dragging { .. } => {}
        }
        self.travel(dx, dy, time)
    }

    /// How long after the last event [`Self::tick`] should run; `None` when no gesture is open.
    pub fn timeout_ms(&self) -> Option<f64> {
        match self.mode {
            Mode::Pending { .. } | Mode::Dragging { .. } => Some(if self.phased {
                PHASED_TIMEOUT_MS
            } else {
                QUIET_GAP_MS
            }),
            _ => None,
        }
    }

    /// Ends an open gesture whose events stopped. Without phases the last event is the lift, so
    /// its velocity counts; a phased gesture that lost its `Ended` decides by travel alone.
    pub fn tick(&mut self, time: f64, width: f32) -> SwipeStep {
        match self.timeout_ms() {
            Some(timeout) if time - self.last >= timeout => {
                let lift = if self.phased { time } else { self.last };
                self.release(lift, width)
            }
            _ => SwipeStep::Ignore,
        }
    }

    fn begin(&mut self, phased: bool) {
        self.mode = Mode::Pending { x: 0., y: 0. };
        self.phased = phased;
        self.velocity = 0.;
    }

    fn travel(&mut self, dx: f32, dy: f32, time: f64) -> SwipeStep {
        match self.mode {
            Mode::Pending { x, y } => {
                let (x, y) = (x + dx, y + dy.abs());
                self.mode = if x.abs() >= AXIS_LOCK && x.abs() >= y * HORIZONTAL_RATIO {
                    self.last_move = time;
                    Mode::Dragging { offset: x }
                } else if y >= AXIS_LOCK {
                    Mode::Vertical
                } else {
                    Mode::Pending { x, y }
                };
                match self.mode {
                    Mode::Dragging { offset } => SwipeStep::Drag { offset },
                    _ => SwipeStep::Ignore,
                }
            }
            Mode::Dragging { offset } => {
                if dx == 0. {
                    return SwipeStep::Drag { offset };
                }
                let elapsed = (time - self.last_move).max(1.) as f32;
                self.velocity = 0.7 * (dx / elapsed) + 0.3 * self.velocity;
                self.last_move = time;
                let offset = offset + dx;
                self.mode = Mode::Dragging { offset };
                SwipeStep::Drag { offset }
            }
            _ => SwipeStep::Ignore,
        }
    }

    fn release(&mut self, lift: f64, width: f32) -> SwipeStep {
        let mode = std::mem::replace(&mut self.mode, Mode::Tail);
        let Mode::Dragging { offset } = mode else {
            return SwipeStep::Ignore;
        };
        let velocity = if lift - self.last_move > STALE_MOVE_MS {
            0.
        } else {
            self.velocity
        };
        let flick = velocity.abs() >= FLICK_VELOCITY
            && velocity.signum() == offset.signum()
            && offset.abs() >= FLICK_MIN_TRAVEL;
        if offset.abs() >= width / 2. || flick {
            SwipeStep::Commit {
                direction: if offset > 0. {
                    SwipeDirection::Next
                } else {
                    SwipeDirection::Previous
                },
                offset,
            }
        } else {
            SwipeStep::SnapBack { offset }
        }
    }

    fn cancel(&mut self) -> SwipeStep {
        match std::mem::replace(&mut self.mode, Mode::Tail) {
            Mode::Dragging { offset } => SwipeStep::SnapBack { offset },
            _ => SwipeStep::Ignore,
        }
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

/// A drag toward a Space that does not exist gives way less and less: `offset` pixels draw as
/// this many, never past half the width.
pub fn rubber_band(offset: f32, width: f32) -> f32 {
    let limit = width.max(1.) / 2.;
    offset.signum() * limit * (1. - 1. / (offset.abs() * 0.55 / limit + 1.))
}

// ---- slide ----

pub const SLIDE_OUT_MS: f32 = 65.;
pub const SLIDE_IN_MS: f32 = 105.;
pub const SNAP_BACK_MS: f32 = 180.;

/// Where the list is during a slide. `offset` is a fraction of the list width: -1 is fully off
/// to the left, 1 fully off to the right.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct SlideFrame {
    pub offset: f32,
    pub opacity: f32,
    /// The filter change shows from this frame on (the list is out of sight).
    pub applied: bool,
}

/// The list fades as it leaves, the same way during a drag and a slide.
fn opacity_at(offset: f32) -> f32 {
    1. - offset.abs().min(1.)
}

/// The list under the finger: `offset` web-sign pixels move it left. `blocked` is a drag toward
/// a Space that a swipe already found missing, which rubber-bands.
pub fn drag_frame(offset: f32, width: f32, blocked: bool) -> SlideFrame {
    let shown = if blocked {
        rubber_band(offset, width)
    } else {
        offset
    };
    let offset = (-shown / width.max(1.)).clamp(-1., 1.);
    SlideFrame {
        offset,
        opacity: opacity_at(offset),
        applied: false,
    }
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

fn sign(direction: SwipeDirection) -> f32 {
    match direction {
        SwipeDirection::Next => 1.,
        SwipeDirection::Previous => -1.,
    }
}

/// How long the out phase of a slide runs when it starts at `from` (a drag fraction): the part
/// of the way the finger did not cover.
pub fn slide_out_ms(direction: SwipeDirection, from: f32) -> f32 {
    SLIDE_OUT_MS * (1. - (-sign(direction) * from).clamp(0., 1.))
}

/// The slide at `elapsed_ms` after it starts at `from`. With a `target`, the list goes out
/// against it, the rows change, and the new list comes in from it over 105 ms; a strip click
/// starts at 0 and runs 65 ms out. With no target the list snaps back to 0. `None` once it ends.
pub fn slide_frame(
    target: Option<SwipeDirection>,
    from: f32,
    elapsed_ms: f32,
) -> Option<SlideFrame> {
    let elapsed_ms = elapsed_ms.max(0.);
    let Some(direction) = target else {
        if elapsed_ms >= SNAP_BACK_MS {
            return None;
        }
        let eased = cubic_bezier(0.16, 1., 0.3, 1., elapsed_ms / SNAP_BACK_MS);
        let offset = from * (1. - eased);
        return Some(SlideFrame {
            offset,
            opacity: opacity_at(offset),
            applied: false,
        });
    };
    let sign = sign(direction);
    let out_ms = slide_out_ms(direction, from);
    if elapsed_ms < out_ms {
        let eased = cubic_bezier(0.4, 0., 1., 1., elapsed_ms / out_ms);
        let offset = from + (-sign - from) * eased;
        Some(SlideFrame {
            offset,
            opacity: opacity_at(offset),
            applied: false,
        })
    } else if elapsed_ms < out_ms + SLIDE_IN_MS {
        let eased = cubic_bezier(0.16, 1., 0.3, 1., (elapsed_ms - out_ms) / SLIDE_IN_MS);
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
