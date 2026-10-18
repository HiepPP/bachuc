//! The Space rules without GPUI: `shared/spaces.ts`, `client/sidebar-state.ts`,
//! `client/spaces-sidebar.ts`, `client/gesture.ts`, and the menu and page rules of
//! `space-menu.tsx` and `page.tsx`. Each rule is a plain function so a test can run it.

use bachuc_plugin_host::SwipeDirection;
use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::cell::RefCell;
use std::collections::{BTreeMap, HashMap};

/// The catalog refreshes every 15 s (`index.client.tsx:32`).
pub(crate) const REFRESH_MS: u64 = 15_000;
pub(crate) const INVALID: &str = "Invalid Space membership";
pub(crate) const UNAVAILABLE: &str = "Spaces unavailable. Refresh and try again.";
pub(crate) const ASSIGN_FAILED: &str = "Unable to assign project to Space. Try again.";
pub(crate) const SAVE_FAILED: &str = "Save failed";
const GONE_REFRESH: &str = "Workspace no longer exists. Refresh first.";
const GONE_RETRY: &str = "Workspace no longer exists. Refresh and try again.";
const LAST_SPACE: &str = "Keep at least one workspace.";
const BLANK_NAME: &str = "Enter a workspace name.";
const NO_PROJECT: &str = "Project unavailable. Refresh first.";
const NO_VIEW_KEY: &str = "Project unavailable. Reopen its menu.";

/// The menu's width, its gap above the tab, and its distance from the window edge
/// (`space-menu.tsx:19-21`).
pub(crate) const MENU_WIDTH: f32 = 220.;
pub(crate) const MENU_GAP: f32 = 6.;
pub(crate) const MENU_EDGE: f32 = 8.;

// ---- state (shared/spaces.ts) ----

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Space {
    pub id: String,
    pub name: String,
}

fn default_spaces() -> Vec<Space> {
    vec![Space {
        id: "space-1".into(),
        name: "Workspace 1".into(),
    }]
}

/// One host's Spaces and the Space of each project key (`stateSchema`).
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct HostSpaces {
    #[serde(default = "default_spaces")]
    pub spaces: Vec<Space>,
    #[serde(default)]
    pub members: BTreeMap<String, String>,
}

impl Default for HostSpaces {
    fn default() -> Self {
        Self {
            spaces: default_spaces(),
            members: BTreeMap::new(),
        }
    }
}

impl HostSpaces {
    pub(crate) fn has(&self, id: &str) -> bool {
        self.spaces.iter().any(|space| space.id == id)
    }

    pub(crate) fn first(&self) -> &str {
        self.spaces.first().map_or("", |space| space.id.as_str())
    }

    pub(crate) fn name_of(&self, id: &str) -> Option<&str> {
        self.spaces
            .iter()
            .find(|space| space.id == id)
            .map(|space| space.name.as_str())
    }

    /// `validSpaces` with the `min(1)` rules of `spacesShape`: unique ids, at least one Space,
    /// and every member names a Space.
    fn valid(&self) -> bool {
        !self.spaces.is_empty()
            && self
                .spaces
                .iter()
                .all(|space| !space.id.is_empty() && !space.name.is_empty())
            && self
                .spaces
                .iter()
                .enumerate()
                .all(|(index, space)| self.spaces[..index].iter().all(|b| b.id != space.id))
            && self.members.values().all(|id| self.has(id))
    }
}

fn validated(state: HostSpaces) -> Result<HostSpaces, String> {
    if state.valid() {
        Ok(state)
    } else {
        Err(INVALID.into())
    }
}

/// The saved values of the `spaces` settings (`preferencesSchema`): the root set of the "All
/// hosts" view, which this client keeps untouched, and one entry per host.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct Preferences {
    #[serde(flatten)]
    pub root: HostSpaces,
    #[serde(default)]
    pub hosts: BTreeMap<String, HostSpaces>,
}

impl Preferences {
    /// Rejects values that break `validSpaces` in the root set or in any host.
    pub(crate) fn parse(values: &Value) -> Result<Self, String> {
        let prefs: Preferences =
            serde_json::from_value(values.clone()).map_err(|e| e.to_string())?;
        if prefs.root.valid() && prefs.hosts.values().all(HostSpaces::valid) {
            Ok(prefs)
        } else {
            Err(INVALID.into())
        }
    }

    pub(crate) fn to_value(&self) -> Value {
        serde_json::to_value(self).unwrap_or(Value::Null)
    }
}

/// A host with no entry gets the default one Space.
pub(crate) fn host_state(prefs: &Preferences, host: &str) -> HostSpaces {
    prefs.hosts.get(host).cloned().unwrap_or_default()
}

/// `prefs` with `host`'s entry replaced; the root set and the other hosts stay as they are.
pub(crate) fn with_host_state(
    prefs: &Preferences,
    host: &str,
    state: HostSpaces,
) -> Result<Preferences, String> {
    let mut next = prefs.clone();
    next.hosts.insert(host.to_string(), validated(state)?);
    Ok(next)
}

/// `JSON.stringify([scope, id])`.
pub(crate) fn project_key(scope: &str, id: &str) -> String {
    serde_json::to_string(&[scope, id]).unwrap_or_default()
}

/// The second item of a key that is a JSON array with a string there (a placement key).
fn pair_second(key: &str) -> Option<String> {
    match serde_json::from_str::<Value>(key).ok()? {
        Value::Array(items) => items.get(1)?.as_str().map(str::to_string),
        _ => None,
    }
}

/// The Space of a project key; the first Space when the key is not stored (`membership`).
pub(crate) fn membership<'a>(state: &'a HostSpaces, key: &str) -> &'a str {
    state
        .members
        .get(key)
        .map_or_else(|| state.first(), String::as_str)
}

fn space_number(id: &str) -> i64 {
    id.get(6..)
        .and_then(|digits| digits.trim().parse::<i64>().ok())
        .unwrap_or(0)
}

/// Adds `space-<n>` "Workspace <n>", where `n` is one above the highest number in use.
pub(crate) fn add_space(state: &HostSpaces) -> Result<HostSpaces, String> {
    let number = state
        .spaces
        .iter()
        .map(|space| space_number(&space.id))
        .max()
        .unwrap_or(0)
        + 1;
    let mut next = state.clone();
    next.spaces.push(Space {
        id: format!("space-{number}"),
        name: format!("Workspace {number}"),
    });
    validated(next)
}

pub(crate) fn rename_space(state: &HostSpaces, id: &str, name: &str) -> Result<HostSpaces, String> {
    if !state.has(id) {
        return Err(GONE_REFRESH.into());
    }
    let trimmed = name.trim();
    if trimmed.is_empty() {
        return Err(BLANK_NAME.into());
    }
    let mut next = state.clone();
    for space in next.spaces.iter_mut().filter(|space| space.id == id) {
        space.name = trimmed.to_string();
    }
    validated(next)
}

pub(crate) fn move_project(
    state: &HostSpaces,
    key: &str,
    target: &str,
) -> Result<HostSpaces, String> {
    if !state.has(target) {
        return Err(GONE_RETRY.into());
    }
    let mut next = state.clone();
    next.members.insert(key.to_string(), target.to_string());
    validated(next)
}

/// The Space `direction` steps from `current`, never past either end. An unknown `current`
/// counts as the first Space.
pub(crate) fn adjacent(ids: &[String], current: &str, direction: i32) -> String {
    let index = ids.iter().position(|id| id == current).unwrap_or(0) as i64;
    let last = ids.len() as i64 - 1;
    let next = (index + i64::from(direction)).clamp(0, last.max(0)) as usize;
    ids.get(next).cloned().unwrap_or_default()
}

/// The Space that takes the projects of a removed Space: the one before it, or the second when
/// the first goes.
pub(crate) fn removal_target(state: &HostSpaces, id: &str) -> Result<String, String> {
    let Some(index) = state.spaces.iter().position(|space| space.id == id) else {
        return Err(GONE_REFRESH.into());
    };
    if state.spaces.len() == 1 {
        return Err(LAST_SPACE.into());
    }
    Ok(state.spaces[if index > 0 { index - 1 } else { 1 }]
        .id
        .clone())
}

pub(crate) fn remove_space(state: &HostSpaces, id: &str) -> Result<HostSpaces, String> {
    let target = removal_target(state, id)?;
    validated(HostSpaces {
        spaces: state
            .spaces
            .iter()
            .filter(|space| space.id != id)
            .cloned()
            .collect(),
        members: state
            .members
            .iter()
            .map(|(key, value)| {
                let moved = if value == id { &target } else { value };
                (key.clone(), moved.clone())
            })
            .collect(),
    })
}

// ---- the catalog (spaces.catalog) ----

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
pub(crate) struct CatalogWorkspace {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CatalogProject {
    pub id: String,
    pub name: String,
    pub path: String,
    #[serde(default)]
    pub view_key: Option<String>,
    #[serde(default)]
    pub workspaces: Vec<CatalogWorkspace>,
}

impl CatalogProject {
    /// A project with a view key moves by that key; an empty one counts as none.
    pub(crate) fn view_key(&self) -> Option<&str> {
        self.view_key.as_deref().filter(|key| !key.is_empty())
    }
}

#[derive(Clone, Debug, Deserialize)]
pub(crate) struct Catalog {
    pub projects: Vec<CatalogProject>,
}

// ---- the lookup (client/sidebar-state.ts) ----

/// A host's Spaces with the catalog, indexed once so that a sidebar redraw runs no JSON parse
/// per project. Build it when the settings or the catalog change.
pub(crate) struct Snapshot {
    pub state: HostSpaces,
    pub projects: Vec<CatalogProject>,
    /// Project id to Space, from the canonical `["sidebar", id]` keys.
    sidebar: HashMap<String, String>,
    /// View key to Space, from the canonical `["view", key]` keys.
    views: HashMap<String, String>,
    /// Project id to Space, from the first key whose second item is that id.
    legacy: HashMap<String, String>,
    by_view: HashMap<String, usize>,
    by_id: HashMap<String, usize>,
    /// The Space of each view key asked so far.
    resolved: RefCell<HashMap<String, String>>,
}

impl Snapshot {
    pub(crate) fn new(state: HostSpaces, projects: Vec<CatalogProject>) -> Self {
        let mut sidebar = HashMap::new();
        let mut views = HashMap::new();
        let mut legacy = HashMap::new();
        for (key, space) in &state.members {
            let Ok(Value::Array(items)) = serde_json::from_str::<Value>(key) else {
                continue;
            };
            let Some(second) = items.get(1).and_then(Value::as_str) else {
                continue;
            };
            legacy
                .entry(second.to_string())
                .or_insert_with(|| space.clone());
            match items.first().and_then(Value::as_str) {
                Some("sidebar") if *key == project_key("sidebar", second) => {
                    sidebar.insert(second.to_string(), space.clone());
                }
                Some("view") if *key == project_key("view", second) => {
                    views.insert(second.to_string(), space.clone());
                }
                _ => {}
            }
        }
        let mut by_view = HashMap::new();
        let mut by_id = HashMap::new();
        for (index, project) in projects.iter().enumerate() {
            if let Some(key) = &project.view_key {
                by_view.entry(key.clone()).or_insert(index);
            }
            by_id.entry(project.id.clone()).or_insert(index);
        }
        Self {
            state,
            projects,
            sidebar,
            views,
            legacy,
            by_view,
            by_id,
            resolved: RefCell::new(HashMap::new()),
        }
    }

    /// The Space of a project row id: its sidebar key, else a legacy key, else the first Space.
    pub(crate) fn sidebar_membership(&self, id: &str) -> &str {
        self.sidebar
            .get(id)
            .or_else(|| self.legacy.get(id))
            .map_or_else(|| self.state.first(), String::as_str)
    }

    /// The Space of a view key: its own key, else the matched project's sidebar Space, else the
    /// first Space.
    pub(crate) fn view_membership(&self, key: &str, project: Option<&CatalogProject>) -> &str {
        match self.views.get(key) {
            Some(space) => space,
            None => match project {
                Some(project) => self.sidebar_membership(&project.id),
                None => self.state.first(),
            },
        }
    }

    /// The catalog project of a view key: a view key match, else the project a placement key
    /// `["<host>", "<id>"]` names.
    pub(crate) fn match_project(&self, key: &str) -> Option<&CatalogProject> {
        let index = match self.by_view.get(key) {
            Some(index) => Some(index),
            None => pair_second(key).and_then(|id| self.by_id.get(&id)),
        };
        index.map(|index| &self.projects[*index])
    }

    /// Whether the project row with this view key belongs to `space`. It answers from a cache
    /// after the first ask.
    pub(crate) fn in_space(&self, key: &str, space: &str) -> bool {
        if let Some(found) = self.resolved.borrow().get(key) {
            return found == space;
        }
        let found = self
            .view_membership(key, self.match_project(key))
            .to_string();
        let same = found == space;
        self.resolved.borrow_mut().insert(key.to_string(), found);
        same
    }

    /// The Space a catalog project belongs to.
    pub(crate) fn project_space(&self, project: &CatalogProject) -> &str {
        match project.view_key() {
            Some(key) => self.view_membership(key, Some(project)),
            None => self.sidebar_membership(&project.id),
        }
    }
}

/// `controller.move`: the project row by its sidebar key, and every key whose second item is
/// the project id.
pub(crate) fn move_sidebar(snap: &Snapshot, id: &str, target: &str) -> Result<HostSpaces, String> {
    if !snap.projects.iter().any(|project| project.id == id) {
        return Err(NO_PROJECT.into());
    }
    let mut next = move_project(&snap.state, &project_key("sidebar", id), target)?;
    rewrite_legacy(&mut next, id, target);
    Ok(next)
}

/// `controller.moveView`: the view key, then the matched project's sidebar key and legacy keys.
pub(crate) fn move_view(snap: &Snapshot, key: &str, target: &str) -> Result<HostSpaces, String> {
    if key.is_empty() {
        return Err(NO_VIEW_KEY.into());
    }
    let mut next = move_project(&snap.state, &project_key("view", key), target)?;
    if let Some(project) = snap.match_project(key) {
        next.members
            .insert(project_key("sidebar", &project.id), target.to_string());
        rewrite_legacy(&mut next, &project.id, target);
    }
    Ok(next)
}

fn rewrite_legacy(state: &mut HostSpaces, id: &str, target: &str) {
    for (key, value) in state.members.iter_mut() {
        if pair_second(key).as_deref() == Some(id) {
            *value = target.to_string();
        }
    }
}

/// `controller.addProject`: a project that any key already names stays where it is.
pub(crate) fn add_project(
    state: &HostSpaces,
    id: &str,
    view_key: Option<&str>,
    target: &str,
) -> Result<HostSpaces, String> {
    let sidebar_key = project_key("sidebar", id);
    let view_key = view_key
        .filter(|key| !key.is_empty())
        .map(|key| project_key("view", key));
    let assigned = state.members.keys().any(|key| {
        *key == sidebar_key
            || view_key.as_ref() == Some(key)
            || pair_second(key).as_deref() == Some(id)
    });
    if assigned {
        return Ok(state.clone());
    }
    let next = move_project(state, &sidebar_key, target)?;
    match view_key {
        Some(key) => move_project(&next, &key, target),
        None => Ok(next),
    }
}

// ---- the sidebar contributions (client/spaces-sidebar.ts) ----

/// The selected Space: the saved choice when it still exists, else the first one.
pub(crate) fn selected_space<'a>(state: &'a HostSpaces, selected: Option<&str>) -> &'a str {
    selected
        .and_then(|id| state.spaces.iter().find(|space| space.id == id))
        .map_or_else(|| state.first(), |space| space.id.as_str())
}

/// Which way the list slides when the selection moves to `to`; `None` for the same Space or an
/// unknown one.
pub(crate) fn select_direction(
    state: &HostSpaces,
    selected: Option<&str>,
    to: &str,
) -> Option<SwipeDirection> {
    let from = selected_space(state, selected);
    let position = |id: &str| state.spaces.iter().position(|space| space.id == id);
    let (from, to) = (position(from)?, position(to)?);
    match to.cmp(&from) {
        std::cmp::Ordering::Greater => Some(SwipeDirection::Next),
        std::cmp::Ordering::Less => Some(SwipeDirection::Previous),
        std::cmp::Ordering::Equal => None,
    }
}

/// The Space that "+" just made, which is the last one.
pub(crate) fn created_space(state: &HostSpaces) -> Option<&str> {
    state.spaces.last().map(|space| space.id.as_str())
}

/// The Space to select before a removal saves: the removal target, only when the removed Space
/// is the selected one. Any other removal keeps the selection.
pub(crate) fn removal_selection(
    state: &HostSpaces,
    selected: Option<&str>,
    id: &str,
) -> Option<String> {
    if state.spaces.len() < 2 || !state.has(id) {
        return None;
    }
    let target = removal_target(state, id).ok()?;
    (selected_space(state, selected) == id).then_some(target)
}

/// `error`, or `fallback` when a failed action left no error text.
pub(crate) fn text_or(error: &str, fallback: &str) -> String {
    if error.is_empty() { fallback } else { error }.to_string()
}

/// While loading and on an error every project shows, so a failure never hides work.
pub(crate) fn is_visible(
    snap: Option<&Snapshot>,
    error: bool,
    selected: Option<&str>,
    view_key: &str,
) -> bool {
    let Some(snap) = snap else {
        return true;
    };
    error || snap.in_space(view_key, selected_space(&snap.state, selected))
}

/// The sidebar heading: the selected Space's name. `None` keeps "Workspaces".
pub(crate) fn title(
    snap: Option<&Snapshot>,
    error: bool,
    selected: Option<&str>,
) -> Option<String> {
    let snap = snap.filter(|_| !error)?;
    snap.state
        .name_of(selected_space(&snap.state, selected))
        .map(str::to_string)
}

/// The Space one step from the selected one; `None` at either end, because a swipe never wraps.
pub(crate) fn adjacent_space(
    state: &HostSpaces,
    selected: Option<&str>,
    direction: SwipeDirection,
) -> Option<String> {
    let ids: Vec<String> = state.spaces.iter().map(|space| space.id.clone()).collect();
    let from = selected_space(state, selected);
    let step = match direction {
        SwipeDirection::Next => 1,
        SwipeDirection::Previous => -1,
    };
    let next = adjacent(&ids, from, step);
    (next != from).then_some(next)
}

/// The Space where sidebar swipes land, one step each from the selected Space, in order. A
/// step past either end stays, because a swipe never wraps. `None` when they land on the
/// selected Space or the Spaces are still loading.
pub(crate) fn swipe_target(
    snap: Option<&Snapshot>,
    selected: Option<&str>,
    swipes: &[SwipeDirection],
) -> Option<String> {
    let state = &snap?.state;
    let from = selected_space(state, selected);
    let mut at = from.to_string();
    for &direction in swipes {
        if let Some(next) = adjacent_space(state, Some(&at), direction) {
            at = next;
        }
    }
    (at != from).then_some(at)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct Choice {
    pub id: String,
    pub name: String,
    pub checked: bool,
}

/// The "Move to" children of a project row: every Space, with the project's own checked. Empty
/// while loading or saving.
pub(crate) fn move_choices(snap: Option<&Snapshot>, busy: bool, view_key: &str) -> Vec<Choice> {
    let Some(snap) = snap.filter(|_| !busy) else {
        return Vec::new();
    };
    let member = snap.view_membership(view_key, snap.match_project(view_key));
    snap.state
        .spaces
        .iter()
        .map(|space| Choice {
            id: space.id.clone(),
            name: space.name.clone(),
            checked: space.id == member,
        })
        .collect()
}

/// A catalog project with its Space, for the Move projects page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MoveProject {
    pub id: String,
    pub name: String,
    pub space_id: String,
}

pub(crate) fn move_projects(snap: Option<&Snapshot>) -> Vec<MoveProject> {
    let Some(snap) = snap else {
        return Vec::new();
    };
    snap.projects
        .iter()
        .map(|project| MoveProject {
            id: project.id.clone(),
            name: project.name.clone(),
            space_id: snap.project_space(project).to_string(),
        })
        .collect()
}

/// `moveProject` of the sidebar controller: a project with a view key moves by that key, so
/// its sidebar row follows. `None` for an unknown project.
pub(crate) fn move_catalog_project(
    snap: &Snapshot,
    project_id: &str,
    target: &str,
) -> Option<Result<HostSpaces, String>> {
    let project = snap
        .projects
        .iter()
        .find(|project| project.id == project_id)?;
    Some(match project.view_key() {
        Some(key) => move_view(snap, key, target),
        None => move_sidebar(snap, &project.id, target),
    })
}

// ---- refresh (index.client.tsx:30-32) ----

/// A catalog refresh is due 15 s after the last one started, and never while one is in flight.
pub(crate) fn refresh_due(elapsed_ms: Option<u64>, in_flight: bool) -> bool {
    !in_flight && elapsed_ms.is_none_or(|elapsed| elapsed >= REFRESH_MS)
}

/// How long the refresh timer waits before it looks again.
pub(crate) fn next_wait_ms(elapsed_ms: Option<u64>) -> u64 {
    elapsed_ms
        .map_or(REFRESH_MS, |elapsed| REFRESH_MS.saturating_sub(elapsed))
        .max(1)
}

// ---- gestures (client/gesture.ts:42) ----

/// `1` for a swipe left (the next Space), `-1` for a swipe right, `0` below the threshold or
/// for a diagonal.
pub(crate) fn touch_direction(dx: f32, dy: f32) -> i32 {
    if dx.abs() >= 70. && dx.abs() > dy.abs() * 1.5 {
        if dx < 0. { 1 } else { -1 }
    } else {
        0
    }
}

pub(crate) fn drag_direction(start: (f32, f32), end: (f32, f32)) -> Option<SwipeDirection> {
    match touch_direction(end.0 - start.0, end.1 - start.1) {
        1 => Some(SwipeDirection::Next),
        -1 => Some(SwipeDirection::Previous),
        _ => None,
    }
}

// ---- the Space menu (client/space-menu.tsx) ----

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuPage {
    Menu,
    Rename,
    Remove,
    Projects,
    Target,
}

/// The menu's left edge: the tab's, kept 8 px inside the window.
pub(crate) fn menu_left(anchor_x: f32, window_width: f32) -> f32 {
    anchor_x
        .min(window_width - MENU_WIDTH - MENU_EDGE)
        .max(MENU_EDGE)
}

/// The y of the menu's bottom edge: 6 px above the tab.
pub(crate) fn menu_bottom(anchor_y: f32) -> f32 {
    anchor_y - MENU_GAP
}

/// This Space's projects first, then the rest, each group by name.
pub(crate) fn order_projects(projects: &[MoveProject], space_id: &str) -> Vec<MoveProject> {
    let mut ordered = projects.to_vec();
    ordered.sort_by(|a, b| {
        (b.space_id == space_id)
            .cmp(&(a.space_id == space_id))
            .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
    });
    ordered
}

pub(crate) fn projects_here(projects: &[MoveProject], space_id: &str) -> usize {
    projects
        .iter()
        .filter(|project| project.space_id == space_id)
        .count()
}

/// The name of the Space that receives the projects of `space_id`; `None` for the last Space.
pub(crate) fn remove_target_name(state: &HostSpaces, space_id: &str) -> Option<String> {
    if state.spaces.len() < 2 {
        return None;
    }
    let target = removal_target(state, space_id).ok()?;
    state.name_of(&target).map(str::to_string)
}

pub(crate) fn remove_prompt(name: &str, target: &str) -> String {
    format!("Remove “{name}”? Its projects move to {target}.")
}

pub(crate) fn menu_heading(page: MenuPage, space_name: &str, moving: Option<&str>) -> String {
    match (page, moving) {
        (MenuPage::Target, Some(project)) => format!("Move {project} to"),
        _ => space_name.to_string(),
    }
}

pub(crate) fn remove_label(target: Option<&str>) -> &'static str {
    if target.is_some() {
        "Remove…"
    } else {
        "Keep at least one Space"
    }
}

/// A blank name is never sent, and nothing saves while a save runs.
pub(crate) fn can_save_name(draft: &str, busy: bool) -> bool {
    !busy && !draft.trim().is_empty()
}

// ---- the Spaces page (client/page.tsx) ----

/// The catalog as the page sees it.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CatalogView<'a> {
    Pending,
    Failed(&'a [CatalogProject]),
    Ready(&'a [CatalogProject]),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PageItem {
    Loading,
    Failed,
    /// No project in this Space; with the first Space's name when this is another Space.
    Empty {
        first: Option<(String, String)>,
    },
    Card {
        project: CatalogProject,
        moving: bool,
    },
}

/// The list rows of the selected Space: a status row, an empty state, or the project cards. A
/// failed refresh keeps the cards of the last answer.
pub(crate) fn page_items(
    catalog: CatalogView<'_>,
    host: &str,
    state: &HostSpaces,
    selected: &str,
    moving: Option<&str>,
) -> Vec<PageItem> {
    let (projects, failed, pending) = match catalog {
        CatalogView::Pending => (&[][..], false, true),
        CatalogView::Failed(projects) => (projects, true, false),
        CatalogView::Ready(projects) => (projects, false, false),
    };
    let here: Vec<&CatalogProject> = projects
        .iter()
        .filter(|project| membership(state, &project_key(host, &project.id)) == selected)
        .collect();
    let mut items = Vec::new();
    if pending {
        items.push(PageItem::Loading);
    }
    if failed {
        items.push(PageItem::Failed);
    }
    if !pending && !failed && here.is_empty() {
        let first = (selected != state.first())
            .then(|| state.spaces.first().map(|s| (s.id.clone(), s.name.clone())))
            .flatten();
        items.push(PageItem::Empty { first });
    }
    items.extend(here.into_iter().map(|project| PageItem::Card {
        moving: moving == Some(project.id.as_str()),
        project: project.clone(),
    }));
    items
}

/// The page subtitle: "<host> · <Space name>".
pub(crate) fn subtitle(
    host_label: &str,
    state: Option<&HostSpaces>,
    selected: Option<&str>,
) -> String {
    let name = state
        .and_then(|state| state.name_of(selected_space(state, selected)))
        .unwrap_or("Loading");
    format!("{host_label} · {name}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn state(ids: &[&str]) -> HostSpaces {
        HostSpaces {
            spaces: ids
                .iter()
                .map(|id| Space {
                    id: (*id).into(),
                    name: format!("Workspace {}", &id[6..]),
                })
                .collect(),
            members: BTreeMap::new(),
        }
    }

    fn with_members(mut state: HostSpaces, members: &[(&str, &str)]) -> HostSpaces {
        for (key, space) in members {
            state.members.insert((*key).into(), (*space).into());
        }
        state
    }

    fn project(id: &str, view_key: Option<&str>) -> CatalogProject {
        CatalogProject {
            id: id.into(),
            name: id.to_uppercase(),
            path: format!("/{id}"),
            view_key: view_key.map(str::to_string),
            workspaces: Vec::new(),
        }
    }

    fn snapshot(state: HostSpaces, projects: Vec<CatalogProject>) -> Snapshot {
        Snapshot::new(state, projects)
    }

    // ---- shared/spaces.ts ----

    #[test]
    fn adding_a_space_uses_the_highest_number_plus_one() {
        let next = add_space(&state(&["space-1", "space-3"])).unwrap();
        let last = next.spaces.last().unwrap();
        assert_eq!(
            (last.id.as_str(), last.name.as_str()),
            ("space-4", "Workspace 4")
        );
        assert_eq!(next.spaces.len(), 3);
        // The default state grows to two Spaces.
        assert_eq!(
            add_space(&HostSpaces::default()).unwrap().spaces[1].id,
            "space-2"
        );
    }

    #[test]
    fn default_membership_and_host_identity_survive_a_round_trip() {
        let initial = HostSpaces::default();
        assert_eq!(membership(&initial, &project_key("a", "p")), "space-1");
        let mut grown = add_space(&add_space(&initial).unwrap()).unwrap();
        grown = move_project(&grown, &project_key("a", "p"), "space-3").unwrap();
        let restored: HostSpaces =
            serde_json::from_str(&serde_json::to_string(&grown).unwrap()).unwrap();
        assert_eq!(membership(&restored, &project_key("a", "p")), "space-3");
        assert_eq!(membership(&restored, &project_key("b", "p")), "space-1");
        assert_eq!(membership(&initial, &project_key("a", "p")), "space-1");
        let back = move_project(&restored, &project_key("a", "p"), "space-1").unwrap();
        assert_eq!(membership(&back, &project_key("a", "p")), "space-1");
        assert!(move_project(&grown, "p", "missing").is_err());
    }

    #[test]
    fn a_project_key_is_the_json_array_electron_writes() {
        assert_eq!(project_key("host", "p"), r#"["host","p"]"#);
        assert_eq!(project_key("view", "repo:p"), r#"["view","repo:p"]"#);
    }

    #[test]
    fn renaming_trims_and_keeps_identities_and_membership() {
        let base = move_project(
            &add_space(&HostSpaces::default()).unwrap(),
            "project",
            "space-2",
        )
        .unwrap();
        let renamed = rename_space(&base, "space-2", "  Công việc  ").unwrap();
        assert_eq!(renamed.spaces[0], base.spaces[0]);
        assert_eq!(
            renamed.spaces[1],
            Space {
                id: "space-2".into(),
                name: "Công việc".into()
            }
        );
        assert_eq!(renamed.members, base.members);
        assert_eq!(base.spaces[1].name, "Workspace 2");
    }

    #[test]
    fn a_blank_name_or_a_missing_id_is_rejected_with_electrons_text() {
        let base = add_space(&HostSpaces::default()).unwrap();
        assert_eq!(
            rename_space(&base, "space-2", " \n "),
            Err("Enter a workspace name.".into())
        );
        assert_eq!(
            rename_space(&base, "missing", "Work"),
            Err("Workspace no longer exists. Refresh first.".into())
        );
        assert_eq!(
            move_project(&base, "p", "missing"),
            Err("Workspace no longer exists. Refresh and try again.".into())
        );
    }

    #[test]
    fn the_removal_target_is_the_next_space_for_the_first_and_the_previous_for_others() {
        let three = state(&["space-1", "space-2", "space-3"]);
        assert_eq!(removal_target(&three, "space-1"), Ok("space-2".into()));
        assert_eq!(removal_target(&three, "space-2"), Ok("space-1".into()));
        assert_eq!(removal_target(&three, "space-3"), Ok("space-2".into()));
    }

    #[test]
    fn the_last_space_cannot_be_removed_and_a_stale_id_is_rejected() {
        let one = HostSpaces::default();
        assert_eq!(
            removal_target(&one, "space-1"),
            Err("Keep at least one workspace.".into())
        );
        assert_eq!(
            remove_space(&one, "space-1"),
            Err("Keep at least one workspace.".into())
        );
        assert_eq!(
            remove_space(&one, "missing"),
            Err("Workspace no longer exists. Refresh first.".into())
        );
    }

    #[test]
    fn removing_a_space_moves_every_member_to_the_target() {
        let base = with_members(
            state(&["space-1", "space-2", "space-3"]),
            &[
                ("legacy", "space-2"),
                (r#"["view","repo"]"#, "space-2"),
                ("other", "space-3"),
            ],
        );
        let result = remove_space(&base, "space-2").unwrap();
        let ids: Vec<&str> = result.spaces.iter().map(|s| s.id.as_str()).collect();
        assert_eq!(ids, ["space-1", "space-3"]);
        assert_eq!(result.members["legacy"], "space-1");
        assert_eq!(result.members[r#"["view","repo"]"#], "space-1");
        assert_eq!(result.members["other"], "space-3");
        // The input is not touched.
        assert_eq!(base.members["legacy"], "space-2");
        // Removing the first Space sends its projects to the second.
        let first = remove_space(
            &with_members(state(&["space-1", "space-2"]), &[("p", "space-1")]),
            "space-1",
        )
        .unwrap();
        assert_eq!(first.members["p"], "space-2");
        assert_eq!(membership(&first, "new-project"), "space-2");
    }

    #[test]
    fn adjacent_never_wraps_at_either_end() {
        let ids: Vec<String> = ["1", "2", "3"].iter().map(|s| s.to_string()).collect();
        assert_eq!(adjacent(&ids, "1", -1), "1");
        assert_eq!(adjacent(&ids, "3", 1), "3");
        assert_eq!(adjacent(&ids, "2", 1), "3");
        assert_eq!(adjacent(&ids, "unknown", 1), "2");
        assert_eq!(adjacent(&[], "1", 1), "");
    }

    #[test]
    fn host_state_of_an_unknown_server_is_the_default_one_space() {
        let prefs = Preferences::default();
        assert_eq!(host_state(&prefs, "srv"), HostSpaces::default());
    }

    #[test]
    fn with_host_state_keeps_the_root_set_and_the_other_hosts() {
        let mut prefs = Preferences::default();
        prefs.root = with_members(state(&["space-1", "space-2"]), &[("r", "space-2")]);
        prefs
            .hosts
            .insert("other".into(), state(&["space-1", "space-5"]));
        let next =
            with_host_state(&prefs, "srv", state(&["space-1", "space-2", "space-3"])).unwrap();
        assert_eq!(next.root, prefs.root);
        assert_eq!(next.hosts["other"], prefs.hosts["other"]);
        assert_eq!(next.hosts["srv"].spaces.len(), 3);
        assert_eq!(prefs.hosts.len(), 1);
    }

    #[test]
    fn saved_values_that_break_the_membership_rules_are_rejected() {
        assert!(Preferences::parse(&json!({})).is_ok());
        assert_eq!(
            Preferences::parse(&json!({"spaces": []})),
            Err(INVALID.into())
        );
        assert_eq!(
            Preferences::parse(&json!({"members": {"p": "absent"}})),
            Err(INVALID.into())
        );
        let twice =
            json!({"spaces": [{"id": "space-1", "name": "1"}, {"id": "space-1", "name": "2"}]});
        assert_eq!(Preferences::parse(&twice), Err(INVALID.into()));
        // A host entry is checked like the root.
        let host = json!({"hosts": {"srv": {"members": {"p": "absent"}}}});
        assert_eq!(Preferences::parse(&host), Err(INVALID.into()));
        // A change that breaks the rules is not built, so nothing is saved.
        let prefs = Preferences::default();
        let broken = with_members(HostSpaces::default(), &[("p", "absent")]);
        assert_eq!(with_host_state(&prefs, "srv", broken), Err(INVALID.into()));
        let empty = HostSpaces {
            spaces: Vec::new(),
            members: BTreeMap::new(),
        };
        assert_eq!(with_host_state(&prefs, "srv", empty), Err(INVALID.into()));
    }

    #[test]
    fn preferences_round_trip_through_the_saved_json() {
        let values = json!({
            "spaces": [{"id": "space-1", "name": "A"}, {"id": "space-2", "name": "B"}],
            "members": {"p": "space-2"},
            "hosts": {"srv": {"spaces": [{"id": "space-1", "name": "C"}], "members": {}}}
        });
        let prefs = Preferences::parse(&values).unwrap();
        assert_eq!(prefs.root.spaces.len(), 2);
        assert_eq!(prefs.hosts["srv"].spaces[0].name, "C");
        assert_eq!(prefs.to_value(), values);
    }

    // ---- client/sidebar-state.ts ----

    #[test]
    fn sidebar_membership_finds_a_legacy_key_and_skips_an_unparsable_one() {
        let members = with_members(
            state(&["space-1", "space-2"]),
            &[("not json", "space-1"), (r#"["host","p"]"#, "space-2")],
        );
        let snap = snapshot(members, Vec::new());
        assert_eq!(snap.sidebar_membership("p"), "space-2");
        assert_eq!(snap.sidebar_membership("unknown"), "space-1");
        // The canonical key wins over a legacy one.
        let both = with_members(
            state(&["space-1", "space-2"]),
            &[
                (r#"["host","p"]"#, "space-2"),
                (r#"["sidebar","p"]"#, "space-1"),
            ],
        );
        assert_eq!(
            snapshot(both, Vec::new()).sidebar_membership("p"),
            "space-1"
        );
    }

    #[test]
    fn view_membership_prefers_the_view_key_then_the_sidebar_space_then_the_first() {
        let members = with_members(
            state(&["space-1", "space-2", "space-3"]),
            &[
                (r#"["view","repo:p"]"#, "space-3"),
                (r#"["sidebar","p"]"#, "space-2"),
                (r#"["sidebar","q"]"#, "space-2"),
            ],
        );
        let projects = vec![project("p", Some("repo:p")), project("q", Some("repo:q"))];
        let snap = snapshot(members, projects.clone());
        assert_eq!(
            snap.view_membership("repo:p", Some(&projects[0])),
            "space-3"
        );
        assert_eq!(
            snap.view_membership("repo:q", Some(&projects[1])),
            "space-2"
        );
        assert_eq!(snap.view_membership("repo:none", None), "space-1");
    }

    #[test]
    fn match_project_takes_a_view_key_then_a_placement_key_by_its_second_item() {
        let snap = snapshot(HostSpaces::default(), vec![project("p", Some("repo:p"))]);
        assert_eq!(
            snap.match_project("repo:p").map(|p| p.id.as_str()),
            Some("p")
        );
        assert_eq!(
            snap.match_project(r#"["host","p"]"#).map(|p| p.id.as_str()),
            Some("p")
        );
        // A name is not a key.
        assert_eq!(snap.match_project("P"), None);
        assert_eq!(snap.match_project(r#"["host","x"]"#), None);
        assert_eq!(snap.match_project("not json"), None);
    }

    #[test]
    fn add_project_does_not_move_an_assigned_project() {
        let assigned = with_members(
            state(&["space-1", "space-2"]),
            &[(r#"["host-a","claude"]"#, "space-2")],
        );
        let same = add_project(&assigned, "claude", Some("directory:.claude"), "space-1").unwrap();
        assert_eq!(same, assigned);
        // A view key alone also counts as assigned.
        let by_view = with_members(
            state(&["space-1", "space-2"]),
            &[(r#"["view","v"]"#, "space-2")],
        );
        assert_eq!(
            add_project(&by_view, "new", Some("v"), "space-1").unwrap(),
            by_view
        );
    }

    #[test]
    fn add_project_gives_a_new_project_both_the_sidebar_and_the_view_key() {
        let base = state(&["space-1", "space-2"]);
        let next = add_project(&base, "claude", Some("directory:.claude"), "space-2").unwrap();
        assert_eq!(next.members[r#"["sidebar","claude"]"#], "space-2");
        assert_eq!(next.members[r#"["view","directory:.claude"]"#], "space-2");
        let no_view = add_project(&base, "claude", None, "space-2").unwrap();
        assert_eq!(no_view.members.len(), 1);
        assert_eq!(
            add_project(&base, "claude", Some("v"), "gone"),
            Err("Workspace no longer exists. Refresh and try again.".into())
        );
    }

    #[test]
    fn move_view_also_rewrites_every_legacy_key_of_the_same_project() {
        let base = with_members(
            state(&["space-1", "space-2"]),
            &[
                (r#"["host","p"]"#, "space-1"),
                (r#"["other","p"]"#, "space-1"),
                (r#"["host","q"]"#, "space-1"),
            ],
        );
        let snap = snapshot(
            base,
            vec![project("p", Some("repo:p")), project("q", Some("repo:q"))],
        );
        let next = move_view(&snap, "repo:p", "space-2").unwrap();
        assert_eq!(next.members[r#"["view","repo:p"]"#], "space-2");
        assert_eq!(next.members[r#"["sidebar","p"]"#], "space-2");
        assert_eq!(next.members[r#"["host","p"]"#], "space-2");
        assert_eq!(next.members[r#"["other","p"]"#], "space-2");
        assert_eq!(next.members[r#"["host","q"]"#], "space-1");
        assert_eq!(move_view(&snap, "", "space-2"), Err(NO_VIEW_KEY.into()));
    }

    #[test]
    fn move_sidebar_needs_a_catalog_project_and_rewrites_its_legacy_keys() {
        let base = with_members(
            state(&["space-1", "space-2"]),
            &[(r#"["host","p"]"#, "space-1")],
        );
        let snap = snapshot(base, vec![project("p", None)]);
        let next = move_sidebar(&snap, "p", "space-2").unwrap();
        assert_eq!(next.members[r#"["sidebar","p"]"#], "space-2");
        assert_eq!(next.members[r#"["host","p"]"#], "space-2");
        assert_eq!(
            move_sidebar(&snap, "gone", "space-2"),
            Err(NO_PROJECT.into())
        );
    }

    #[test]
    fn a_catalog_project_with_a_view_key_moves_by_that_key() {
        let snap = snapshot(
            state(&["space-1", "space-2"]),
            vec![project("p", Some("repo:p")), project("q", None)],
        );
        let by_view = move_catalog_project(&snap, "p", "space-2")
            .unwrap()
            .unwrap();
        assert!(by_view.members.contains_key(r#"["view","repo:p"]"#));
        let by_id = move_catalog_project(&snap, "q", "space-2")
            .unwrap()
            .unwrap();
        assert!(!by_id.members.keys().any(|k| k.contains("view")));
        assert!(move_catalog_project(&snap, "missing", "space-2").is_none());
    }

    // ---- client/spaces-sidebar.ts ----

    fn filter_snapshot() -> Snapshot {
        let members = with_members(
            state(&["space-1", "space-2"]),
            &[(r#"["view","repo:p"]"#, "space-2")],
        );
        snapshot(
            members,
            vec![project("p", Some("repo:p")), project("q", Some("repo:q"))],
        )
    }

    #[test]
    fn a_project_shows_only_in_its_space() {
        let snap = filter_snapshot();
        assert!(is_visible(Some(&snap), false, None, "repo:q"));
        assert!(!is_visible(Some(&snap), false, None, "repo:p"));
        assert!(is_visible(Some(&snap), false, Some("space-2"), "repo:p"));
        assert!(!is_visible(Some(&snap), false, Some("space-2"), "repo:q"));
        // A project added outside the app keeps the first-Space default.
        assert!(!is_visible(Some(&snap), false, Some("space-2"), "repo:new"));
        assert!(is_visible(Some(&snap), false, Some("space-1"), "repo:new"));
    }

    #[test]
    fn an_error_a_load_in_progress_or_an_unknown_host_shows_every_project() {
        let snap = filter_snapshot();
        assert!(is_visible(Some(&snap), true, Some("space-2"), "repo:q"));
        assert!(is_visible(None, false, Some("space-2"), "repo:q"));
        assert!(is_visible(None, true, None, "repo:p"));
    }

    #[test]
    fn the_title_is_the_selected_space_and_an_error_keeps_workspaces() {
        let snap = filter_snapshot();
        assert_eq!(
            title(Some(&snap), false, None).as_deref(),
            Some("Workspace 1")
        );
        assert_eq!(
            title(Some(&snap), false, Some("space-2")).as_deref(),
            Some("Workspace 2")
        );
        assert_eq!(title(Some(&snap), true, None), None);
        assert_eq!(title(None, false, None), None);
        // A selection that no longer exists falls back to the first Space.
        assert_eq!(
            title(Some(&snap), false, Some("gone")).as_deref(),
            Some("Workspace 1")
        );
    }

    #[test]
    fn the_move_menu_is_empty_while_busy_and_checks_the_members_space() {
        let snap = filter_snapshot();
        assert!(move_choices(Some(&snap), true, "repo:q").is_empty());
        assert!(move_choices(None, false, "repo:q").is_empty());
        let choices = move_choices(Some(&snap), false, "repo:q");
        let shown: Vec<(&str, bool)> = choices
            .iter()
            .map(|choice| (choice.name.as_str(), choice.checked))
            .collect();
        assert_eq!(shown, [("Workspace 1", true), ("Workspace 2", false)]);
        let moved = move_choices(Some(&snap), false, "repo:p");
        assert!(moved[1].checked && !moved[0].checked);
    }

    #[test]
    fn selecting_a_later_space_slides_next_and_the_same_space_does_not_slide() {
        let three = state(&["space-1", "space-2", "space-3"]);
        assert_eq!(
            select_direction(&three, Some("space-1"), "space-3"),
            Some(SwipeDirection::Next)
        );
        assert_eq!(
            select_direction(&three, Some("space-3"), "space-1"),
            Some(SwipeDirection::Previous)
        );
        assert_eq!(select_direction(&three, Some("space-2"), "space-2"), None);
        // No selection counts as the first Space.
        assert_eq!(select_direction(&three, None, "space-1"), None);
        assert_eq!(select_direction(&three, None, "gone"), None);
    }

    #[test]
    fn a_swipe_moves_one_space_and_stays_past_either_end() {
        let snap = filter_snapshot();
        let next = |selected, direction| swipe_target(Some(&snap), selected, &[direction]);
        assert_eq!(
            next(None, SwipeDirection::Previous),
            None,
            "before the first"
        );
        assert_eq!(next(None, SwipeDirection::Next).as_deref(), Some("space-2"));
        assert_eq!(
            next(Some("space-2"), SwipeDirection::Next),
            None,
            "past the last"
        );
        assert_eq!(
            next(Some("space-2"), SwipeDirection::Previous).as_deref(),
            Some("space-1")
        );
        assert_eq!(swipe_target(None, None, &[SwipeDirection::Next]), None);
        assert_eq!(swipe_target(Some(&snap), None, &[]), None);
    }

    #[test]
    fn swipes_queued_during_a_save_run_in_order_and_stop_at_the_ends() {
        let three = Snapshot::new(state(&["space-1", "space-2", "space-3"]), Vec::new());
        let land =
            |selected, swipes: &[SwipeDirection]| swipe_target(Some(&three), selected, swipes);
        use SwipeDirection::{Next, Previous};
        assert_eq!(land(None, &[Next, Next]).as_deref(), Some("space-3"));
        assert_eq!(land(None, &[Next, Next, Next]).as_deref(), Some("space-3"));
        // The step past the end stays, so the next one counts from the last Space.
        assert_eq!(
            land(Some("space-3"), &[Next, Previous]).as_deref(),
            Some("space-2")
        );
        assert_eq!(land(Some("space-2"), &[Next, Previous]), None);
        assert_eq!(land(None, &[Previous, Next]).as_deref(), Some("space-2"));
    }

    #[test]
    fn creating_a_space_selects_the_new_last_one() {
        let grown = add_space(&add_space(&HostSpaces::default()).unwrap()).unwrap();
        assert_eq!(created_space(&grown), Some("space-3"));
    }

    #[test]
    fn removing_the_selected_space_selects_its_removal_target_first() {
        let four = state(&["space-1", "space-2", "space-3", "space-4"]);
        // The preceding Space is selected, not Space 1.
        assert_eq!(
            removal_selection(&four, Some("space-4"), "space-4").as_deref(),
            Some("space-3")
        );
        // Removing the first Space selects the next one.
        assert_eq!(
            removal_selection(&four, Some("space-1"), "space-1").as_deref(),
            Some("space-2")
        );
        // No selection counts as the first Space.
        assert_eq!(
            removal_selection(&four, None, "space-1").as_deref(),
            Some("space-2")
        );
    }

    #[test]
    fn removing_another_space_or_a_missing_one_keeps_the_selection() {
        let three = state(&["space-1", "space-2", "space-3"]);
        assert_eq!(removal_selection(&three, Some("space-2"), "space-3"), None);
        assert_eq!(removal_selection(&three, Some("space-2"), "gone"), None);
        assert_eq!(
            removal_selection(&HostSpaces::default(), Some("space-1"), "space-1"),
            None
        );
    }

    #[test]
    fn a_failed_action_without_an_error_text_uses_the_fallback() {
        assert_eq!(text_or("", ASSIGN_FAILED), ASSIGN_FAILED);
        assert_eq!(
            text_or("Revision conflict", ASSIGN_FAILED),
            "Revision conflict"
        );
        assert_eq!(text_or("", SAVE_FAILED), "Save failed");
    }

    #[test]
    fn a_project_added_into_the_selected_space_shows_there_and_nowhere_else() {
        let base = state(&["space-1", "space-2"]);
        let added = add_project(&base, "claude", Some("directory:.claude"), "space-2").unwrap();
        let snap = snapshot(added, vec![project("claude", Some("directory:.claude"))]);
        assert!(is_visible(
            Some(&snap),
            false,
            Some("space-2"),
            "directory:.claude"
        ));
        assert!(!is_visible(
            Some(&snap),
            false,
            Some("space-1"),
            "directory:.claude"
        ));
        assert_eq!(snap.sidebar_membership("claude"), "space-2");
    }

    #[test]
    fn move_projects_lists_each_project_with_its_space() {
        let snap = filter_snapshot();
        let projects = move_projects(Some(&snap));
        assert_eq!(
            projects
                .iter()
                .map(|p| (p.id.as_str(), p.space_id.as_str()))
                .collect::<Vec<_>>(),
            [("p", "space-2"), ("q", "space-1")]
        );
        assert!(move_projects(None).is_empty());
    }

    // ---- client/gesture.ts ----

    #[test]
    fn touch_direction_follows_the_threshold_the_diagonal_and_the_sign() {
        assert_eq!(touch_direction(-80., 10.), 1);
        assert_eq!(touch_direction(80., 10.), -1);
        assert_eq!(touch_direction(60., 0.), 0);
        assert_eq!(touch_direction(80., 60.), 0);
        assert_eq!(touch_direction(70., 0.), -1);
    }

    #[test]
    fn a_drag_across_the_list_switches_only_when_it_passes_touch_direction() {
        assert_eq!(
            drag_direction((300., 100.), (200., 110.)),
            Some(SwipeDirection::Next)
        );
        assert_eq!(
            drag_direction((100., 100.), (200., 100.)),
            Some(SwipeDirection::Previous)
        );
        assert_eq!(drag_direction((100., 100.), (150., 100.)), None);
    }

    // ---- refresh ----

    #[test]
    fn refresh_is_due_after_15_seconds_and_never_while_one_is_in_flight() {
        assert!(refresh_due(None, false));
        assert!(!refresh_due(Some(14_999), false));
        assert!(refresh_due(Some(15_000), false));
        assert!(refresh_due(Some(40_000), false));
        assert!(!refresh_due(None, true));
        assert!(!refresh_due(Some(40_000), true));
        assert_eq!(REFRESH_MS, 15_000);
    }

    #[test]
    fn the_refresh_timer_waits_for_what_is_left_of_the_interval() {
        assert_eq!(next_wait_ms(None), 15_000);
        assert_eq!(next_wait_ms(Some(5_000)), 10_000);
        assert_eq!(next_wait_ms(Some(15_000)), 1);
        assert_eq!(next_wait_ms(Some(99_000)), 1);
    }

    // ---- the menu ----

    #[test]
    fn the_menu_floats_above_the_tab_and_stays_inside_the_window() {
        assert_eq!(menu_left(100., 1000.), 100.);
        assert_eq!(menu_left(2., 1000.), 8.);
        assert_eq!(menu_left(950., 1000.), 1000. - 220. - 8.);
        assert_eq!(menu_bottom(500.), 494.);
    }

    #[test]
    fn menu_projects_list_this_spaces_first_then_the_rest_by_name() {
        let all = |name: &str, space: &str| MoveProject {
            id: name.to_lowercase(),
            name: name.into(),
            space_id: space.into(),
        };
        let projects = vec![
            all("zeta", "space-1"),
            all("Beta", "space-2"),
            all("alpha", "space-2"),
            all("Eta", "space-1"),
        ];
        let order: Vec<String> = order_projects(&projects, "space-2")
            .into_iter()
            .map(|p| p.name)
            .collect();
        assert_eq!(order, ["alpha", "Beta", "Eta", "zeta"]);
        assert_eq!(projects_here(&projects, "space-2"), 2);
        assert_eq!(projects_here(&projects, "space-9"), 0);
    }

    #[test]
    fn the_remove_page_names_the_space_that_receives_the_projects() {
        let three = state(&["space-1", "space-2", "space-3"]);
        assert_eq!(
            remove_target_name(&three, "space-1").as_deref(),
            Some("Workspace 2")
        );
        assert_eq!(
            remove_target_name(&three, "space-3").as_deref(),
            Some("Workspace 2")
        );
        assert_eq!(remove_target_name(&HostSpaces::default(), "space-1"), None);
        assert_eq!(
            remove_prompt("Work", "Home"),
            "Remove “Work”? Its projects move to Home."
        );
        assert_eq!(remove_label(Some("Home")), "Remove…");
        assert_eq!(remove_label(None), "Keep at least one Space");
    }

    #[test]
    fn the_menu_heading_names_the_space_or_the_project_being_moved() {
        assert_eq!(menu_heading(MenuPage::Menu, "Work", None), "Work");
        assert_eq!(menu_heading(MenuPage::Projects, "Work", Some("p")), "Work");
        assert_eq!(
            menu_heading(MenuPage::Target, "Work", Some("P")),
            "Move P to"
        );
        assert_eq!(menu_heading(MenuPage::Target, "Work", None), "Work");
    }

    #[test]
    fn a_blank_name_is_never_saved_and_nothing_saves_while_busy() {
        assert!(can_save_name("Work", false));
        assert!(!can_save_name("  ", false));
        assert!(!can_save_name("", false));
        assert!(!can_save_name("Work", true));
    }

    // ---- the page ----

    #[test]
    fn the_page_lists_the_projects_of_the_selected_space_by_the_pages_own_key() {
        let members = with_members(
            state(&["space-1", "space-2"]),
            &[(r#"["srv","p"]"#, "space-2")],
        );
        let projects = vec![project("p", None), project("q", None)];
        let items = page_items(
            CatalogView::Ready(&projects),
            "srv",
            &members,
            "space-2",
            Some("p"),
        );
        assert_eq!(
            items,
            [PageItem::Card {
                project: projects[0].clone(),
                moving: true
            }]
        );
        let first = page_items(
            CatalogView::Ready(&projects),
            "srv",
            &members,
            "space-1",
            None,
        );
        assert_eq!(
            first,
            [PageItem::Card {
                project: projects[1].clone(),
                moving: false
            }]
        );
    }

    #[test]
    fn an_empty_space_offers_the_first_space_unless_it_is_the_first() {
        let two = state(&["space-1", "space-2"]);
        let items = page_items(CatalogView::Ready(&[]), "srv", &two, "space-2", None);
        assert_eq!(
            items,
            [PageItem::Empty {
                first: Some(("space-1".into(), "Workspace 1".into()))
            }]
        );
        let first = page_items(CatalogView::Ready(&[]), "srv", &two, "space-1", None);
        assert_eq!(first, [PageItem::Empty { first: None }]);
    }

    #[test]
    fn the_page_shows_the_catalog_status_and_keeps_old_cards_after_a_failed_refresh() {
        let one = HostSpaces::default();
        assert_eq!(
            page_items(CatalogView::Pending, "srv", &one, "space-1", None),
            [PageItem::Loading]
        );
        assert_eq!(
            page_items(CatalogView::Failed(&[]), "srv", &one, "space-1", None),
            [PageItem::Failed]
        );
        let projects = vec![project("p", None)];
        let kept = page_items(CatalogView::Failed(&projects), "srv", &one, "space-1", None);
        assert_eq!(kept.len(), 2);
        assert_eq!(kept[0], PageItem::Failed);
    }

    #[test]
    fn the_subtitle_joins_the_host_and_the_space_name() {
        let two = state(&["space-1", "space-2"]);
        assert_eq!(
            subtitle("Mac", Some(&two), Some("space-2")),
            "Mac · Workspace 2"
        );
        assert_eq!(subtitle("Mac", Some(&two), None), "Mac · Workspace 1");
        assert_eq!(subtitle("Mac", None, None), "Mac · Loading");
    }

    #[test]
    fn the_catalog_answer_decodes_with_and_without_a_view_key() {
        let catalog: Catalog = serde_json::from_value(json!({"projects": [
            {"id": "p", "name": "P", "path": "/p", "viewKey": "repo:p",
             "workspaces": [{"id": "w1", "name": "One"}]},
            {"id": "q", "name": "Q", "path": "/q", "workspaces": []}
        ]}))
        .unwrap();
        assert_eq!(catalog.projects[0].view_key(), Some("repo:p"));
        assert_eq!(catalog.projects[0].workspaces[0].name, "One");
        assert_eq!(catalog.projects[1].view_key(), None);
    }

    // ---- anti-goal ----

    #[test]
    fn filtering_500_projects_with_500_member_keys_stays_under_20_ms() {
        let spaces = state(&["space-1", "space-2", "space-3"]);
        let mut members = BTreeMap::new();
        let mut projects = Vec::new();
        for index in 0..500 {
            let space = format!("space-{}", index % 3 + 1);
            let key = format!("repo:{index}");
            let id = format!("p{index}");
            // A mix of view keys, sidebar keys, and legacy keys.
            let member = match index % 3 {
                0 => project_key("view", &key),
                1 => project_key("sidebar", &id),
                _ => project_key("host", &id),
            };
            members.insert(member, space);
            projects.push(project(&id, Some(&key)));
        }
        let snap = snapshot(HostSpaces { members, ..spaces }, projects);
        assert_eq!(snap.state.members.len(), 500);
        let started = std::time::Instant::now();
        let shown = (0..500)
            .filter(|index| {
                is_visible(
                    Some(&snap),
                    false,
                    Some("space-2"),
                    &format!("repo:{index}"),
                )
            })
            .count();
        let elapsed = started.elapsed();
        println!("filter: 500 calls over 500 projects took {elapsed:?}");
        assert_eq!(shown, (0..500).filter(|i| i % 3 == 1).count());
        assert!(
            elapsed < std::time::Duration::from_millis(20),
            "500 visibility calls took {elapsed:?}"
        );
    }
}
