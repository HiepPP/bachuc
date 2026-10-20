//! The sidebar prefs that survive a restart, stored in `sidebar.json` in the GPUI data dir.
//! Same pattern as the app's `LayoutPrefs`: each field falls back to its default on its own,
//! and a save goes through a temp file and a rename.

use crate::electron_order::read_electron_order;
use crate::model::{Group, Grouping, SidebarOrder, StatusBucket, ViewState};
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::Path;

const FILE: &str = "sidebar.json";
const PINNED: &str = "pinned";
const PROJECT_PREFIX: &str = "project:";
const STATUS_PREFIX: &str = "status:";

/// What a workspace row titles itself with (`WorkspaceTitleSource`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum TitleSource {
    #[default]
    Title,
    /// The branch name, else the workspace name.
    Branch,
}

/// The facts a row may show under its title (`SidebarRowItems`). Each is independent. The host
/// item is out of scope for the one-host GPUI client.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct RowItems {
    pub branch: bool,
    pub project: bool,
    pub pull_request: bool,
    pub services: bool,
    pub labels: bool,
}

impl Default for RowItems {
    /// `DEFAULT_SIDEBAR_ROW_ITEMS`.
    fn default() -> Self {
        Self {
            branch: false,
            project: false,
            pull_request: true,
            services: true,
            labels: true,
        }
    }
}

/// One toggle of [`RowItems`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum RowItem {
    Branch,
    Project,
    PullRequest,
    Services,
    Labels,
}

impl RowItems {
    /// Every item off.
    pub fn none() -> Self {
        Self {
            branch: false,
            project: false,
            pull_request: false,
            services: false,
            labels: false,
        }
    }

    pub fn is_on(&self, item: RowItem) -> bool {
        match item {
            RowItem::Branch => self.branch,
            RowItem::Project => self.project,
            RowItem::PullRequest => self.pull_request,
            RowItem::Services => self.services,
            RowItem::Labels => self.labels,
        }
    }

    pub fn toggle(&mut self, item: RowItem) {
        let flag = match item {
            RowItem::Branch => &mut self.branch,
            RowItem::Project => &mut self.project,
            RowItem::PullRequest => &mut self.pull_request,
            RowItem::Services => &mut self.services,
            RowItem::Labels => &mut self.labels,
        };
        *flag = !*flag;
    }
}

/// How a pull request's CI shows (`SidebarChecksDisplay`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ChecksDisplay {
    #[default]
    IconAndText,
    IconOnly,
    Hidden,
}

/// The one thing the slot right of the title holds (`SidebarWorkspaceTrailing` without "none").
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Trailing {
    DiffStats,
    LastActivity,
}

impl Trailing {
    /// `toggleTrailing`: a click on the choice that already shows clears the slot.
    pub fn toggled(current: Option<Trailing>, choice: Trailing) -> Option<Trailing> {
        (current != Some(choice)).then_some(choice)
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SidebarPrefs {
    /// Which projects and sections the owner collapsed or expanded, the grouping, and the
    /// project and label filters.
    pub view: ViewState,
    pub title_source: TitleSource,
    pub row_items: RowItems,
    pub checks_display: ChecksDisplay,
    /// `None` leaves the slot empty. The default is the diff stats, as in Electron's settings.
    pub trailing: Option<Trailing>,
    /// Whether the one-time import of Electron's saved order already ran (ADR-0057).
    pub electron_order_imported: bool,
}

impl Default for SidebarPrefs {
    fn default() -> Self {
        Self {
            view: ViewState::default(),
            title_source: TitleSource::default(),
            row_items: RowItems::default(),
            checks_display: ChecksDisplay::default(),
            trailing: Some(Trailing::DiffStats),
            electron_order_imported: false,
        }
    }
}

impl SidebarPrefs {
    /// Reads the stored prefs. A bad file, a bad field, or a bad list item gives the default.
    /// Unknown keys are ignored.
    pub fn parse(text: &str) -> Self {
        let value: serde_json::Value = serde_json::from_str(text).unwrap_or_default();
        let strings = |key: &str| -> Vec<&str> {
            value[key]
                .as_array()
                .map(|items| items.iter().filter_map(|item| item.as_str()).collect())
                .unwrap_or_default()
        };
        let defaults = Self::default();
        let items = &value["rowItems"];
        let flag = |key: &str, default: bool| items[key].as_bool().unwrap_or(default);
        Self {
            view: ViewState {
                collapsed_projects: strings("collapsedProjects")
                    .into_iter()
                    .map(String::from)
                    .collect(),
                pinned_collapsed: value["pinnedCollapsed"].as_bool().unwrap_or(false),
                expanded_groups: strings("expandedGroups")
                    .into_iter()
                    .filter_map(parse_group)
                    .collect(),
                grouping: match value["grouping"].as_str() {
                    Some("status") => Grouping::Status,
                    _ => Grouping::Project,
                },
                collapsed_statuses: strings("collapsedStatuses")
                    .into_iter()
                    .filter_map(StatusBucket::from_key)
                    .collect(),
                project_filter: strings("projectFilter")
                    .into_iter()
                    .map(String::from)
                    .collect(),
                label_filter: strings("labelFilter")
                    .into_iter()
                    .map(String::from)
                    .collect(),
                order: SidebarOrder {
                    project_order: string_list(&value["projectOrder"]),
                    workspace_order_by_project: value["workspaceOrderByProject"]
                        .as_object()
                        .map(|map| {
                            map.iter()
                                .map(|(key, list)| (key.clone(), string_list(list)))
                                .collect::<HashMap<_, _>>()
                        })
                        .unwrap_or_default(),
                    pinned_workspace_order: string_list(&value["pinnedWorkspaceOrder"]),
                },
            },
            title_source: match value["titleSource"].as_str() {
                Some("branch") => TitleSource::Branch,
                _ => TitleSource::Title,
            },
            row_items: RowItems {
                branch: flag("branch", defaults.row_items.branch),
                project: flag("project", defaults.row_items.project),
                pull_request: flag("pullRequest", defaults.row_items.pull_request),
                services: flag("services", defaults.row_items.services),
                labels: flag("labels", defaults.row_items.labels),
            },
            checks_display: match value["checksDisplay"].as_str() {
                Some("icon") => ChecksDisplay::IconOnly,
                Some("none") => ChecksDisplay::Hidden,
                _ => ChecksDisplay::IconAndText,
            },
            trailing: match value["trailing"].as_str() {
                Some("timestamp") => Some(Trailing::LastActivity),
                Some("none") => None,
                _ => defaults.trailing,
            },
            electron_order_imported: value["electronOrderImported"].as_bool().unwrap_or(false),
        }
    }

    /// The lists are sorted, so the same prefs always give the same text.
    pub fn to_json(&self) -> String {
        let sorted = |items: &HashSet<String>| -> Vec<String> {
            let mut items: Vec<String> = items.iter().cloned().collect();
            items.sort_unstable();
            items
        };
        let mut expanded: Vec<String> = self.view.expanded_groups.iter().map(group_key).collect();
        expanded.sort_unstable();
        let mut collapsed_statuses: Vec<&str> = self
            .view
            .collapsed_statuses
            .iter()
            .map(|bucket| bucket.key())
            .collect();
        collapsed_statuses.sort_unstable();
        serde_json::json!({
            "collapsedProjects": sorted(&self.view.collapsed_projects),
            "pinnedCollapsed": self.view.pinned_collapsed,
            "expandedGroups": expanded,
            "grouping": match self.view.grouping {
                Grouping::Project => "project",
                Grouping::Status => "status",
            },
            "collapsedStatuses": collapsed_statuses,
            "projectFilter": sorted(&self.view.project_filter),
            "labelFilter": sorted(&self.view.label_filter),
            "projectOrder": self.view.order.project_order,
            "workspaceOrderByProject": self
                .view
                .order
                .workspace_order_by_project
                .iter()
                .collect::<BTreeMap<_, _>>(),
            "pinnedWorkspaceOrder": self.view.order.pinned_workspace_order,
            "titleSource": match self.title_source {
                TitleSource::Title => "title",
                TitleSource::Branch => "branch",
            },
            "rowItems": {
                "branch": self.row_items.branch,
                "project": self.row_items.project,
                "pullRequest": self.row_items.pull_request,
                "services": self.row_items.services,
                "labels": self.row_items.labels,
            },
            "checksDisplay": match self.checks_display {
                ChecksDisplay::IconAndText => "iconAndText",
                ChecksDisplay::IconOnly => "icon",
                ChecksDisplay::Hidden => "none",
            },
            "trailing": match self.trailing {
                Some(Trailing::DiffStats) => "diff",
                Some(Trailing::LastActivity) => "timestamp",
                None => "none",
            },
            "electronOrderImported": self.electron_order_imported,
        })
        .to_string()
    }

    pub fn load_from(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join(FILE))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
    }

    /// [`SidebarPrefs::load_from`], then the one-time import of Electron's saved order when the
    /// flag is not set. The flag is set after the first try, also when the folder is missing or
    /// the read fails, so Electron's store is read at most once. A successful read replaces the
    /// saved order; the order held before holds only keys that were seeded by name. `None` skips
    /// the import and leaves the flag as it is.
    pub fn load_with_import(dir: &Path, electron_user_data: Option<&Path>) -> Self {
        let mut prefs = Self::load_from(dir);
        let Some(user_data) = electron_user_data else {
            return prefs;
        };
        if prefs.electron_order_imported {
            return prefs;
        }
        if let Some(order) = read_electron_order(user_data) {
            prefs.view.order = order;
        }
        prefs.electron_order_imported = true;
        if let Err(error) = prefs.save_to(dir) {
            log::warn!("cannot save sidebar.json after the import: {error}");
        }
        prefs
    }

    /// Writes through a temp file and a rename, so a crash leaves the old file.
    pub fn save_to(&self, dir: &Path) -> std::io::Result<()> {
        std::fs::create_dir_all(dir)?;
        let temp = dir.join(format!("{FILE}.tmp"));
        std::fs::write(&temp, self.to_json())?;
        std::fs::rename(temp, dir.join(FILE))
    }

    /// Drops the project ids and `Project(id)` groups of projects that no longer exist. `Pinned`
    /// stays. Returns whether anything went.
    pub fn retain_projects(&mut self, known: &HashSet<&str>) -> bool {
        let collapsed = self.view.collapsed_projects.len();
        let expanded = self.view.expanded_groups.len();
        let filtered = self.view.project_filter.len();
        self.view
            .collapsed_projects
            .retain(|id| known.contains(id.as_str()));
        self.view.expanded_groups.retain(|group| match group {
            Group::Pinned | Group::Status(_) => true,
            Group::Project(id) => known.contains(id.as_str()),
        });
        self.view
            .project_filter
            .retain(|id| known.contains(id.as_str()));
        collapsed != self.view.collapsed_projects.len()
            || expanded != self.view.expanded_groups.len()
            || filtered != self.view.project_filter.len()
    }
}

/// The strings of a JSON array. A value that is not an array, or an item that is not a string,
/// gives nothing.
fn string_list(value: &serde_json::Value) -> Vec<String> {
    value
        .as_array()
        .map(|items| {
            items
                .iter()
                .filter_map(|item| item.as_str().map(String::from))
                .collect()
        })
        .unwrap_or_default()
}

fn group_key(group: &Group) -> String {
    match group {
        Group::Pinned => PINNED.to_string(),
        Group::Project(id) => format!("{PROJECT_PREFIX}{id}"),
        Group::Status(bucket) => format!("{STATUS_PREFIX}{}", bucket.key()),
    }
}

fn parse_group(key: &str) -> Option<Group> {
    if key == PINNED {
        return Some(Group::Pinned);
    }
    if let Some(bucket) = key.strip_prefix(STATUS_PREFIX) {
        return StatusBucket::from_key(bucket).map(Group::Status);
    }
    key.strip_prefix(PROJECT_PREFIX)
        .map(|id| Group::Project(id.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    fn sample() -> SidebarPrefs {
        SidebarPrefs {
            view: ViewState {
                collapsed_projects: ["p1".to_string(), "p2".to_string()].into(),
                pinned_collapsed: true,
                expanded_groups: [
                    Group::Pinned,
                    Group::Project("p1".into()),
                    Group::Status(StatusBucket::Running),
                ]
                .into(),
                grouping: Grouping::Status,
                collapsed_statuses: [StatusBucket::Done, StatusBucket::Failed].into(),
                project_filter: ["p1".to_string(), "p9".to_string()].into(),
                label_filter: ["bug".to_string(), String::new()].into(),
                order: SidebarOrder {
                    project_order: vec!["key-b".into(), "key-a".into()],
                    workspace_order_by_project: [
                        (
                            "key-a".to_string(),
                            vec!["srv:a2".to_string(), "srv:a1".into()],
                        ),
                        ("key-b".to_string(), vec!["srv:b1".to_string()]),
                    ]
                    .into(),
                    pinned_workspace_order: vec!["srv:p2".into(), "srv:p1".into()],
                },
            },
            title_source: TitleSource::Branch,
            row_items: RowItems {
                branch: true,
                project: true,
                pull_request: false,
                services: false,
                labels: false,
            },
            checks_display: ChecksDisplay::Hidden,
            trailing: Some(Trailing::LastActivity),
            electron_order_imported: true,
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("bachuc-sidebar-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn every_field_round_trips() {
        let prefs = sample();
        assert_eq!(SidebarPrefs::parse(&prefs.to_json()), prefs);
        let dir = temp_dir("round-trip");
        prefs.save_to(&dir).unwrap();
        assert_eq!(SidebarPrefs::load_from(&dir), prefs);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn every_choice_of_every_display_field_round_trips() {
        for grouping in [Grouping::Project, Grouping::Status] {
            for title_source in [TitleSource::Title, TitleSource::Branch] {
                for checks_display in [
                    ChecksDisplay::IconAndText,
                    ChecksDisplay::IconOnly,
                    ChecksDisplay::Hidden,
                ] {
                    for trailing in [
                        None,
                        Some(Trailing::DiffStats),
                        Some(Trailing::LastActivity),
                    ] {
                        let mut prefs = SidebarPrefs::default();
                        prefs.view.grouping = grouping;
                        prefs.title_source = title_source;
                        prefs.checks_display = checks_display;
                        prefs.trailing = trailing;
                        assert_eq!(SidebarPrefs::parse(&prefs.to_json()), prefs);
                    }
                }
            }
        }
        for item in [
            RowItem::Branch,
            RowItem::Project,
            RowItem::PullRequest,
            RowItem::Services,
            RowItem::Labels,
        ] {
            let mut prefs = SidebarPrefs::default();
            prefs.row_items.toggle(item);
            assert_ne!(prefs.row_items, RowItems::default());
            assert_eq!(SidebarPrefs::parse(&prefs.to_json()), prefs);
        }
    }

    #[test]
    fn the_defaults_follow_electron() {
        let prefs = SidebarPrefs::default();
        assert_eq!(prefs.view.grouping, Grouping::Project);
        assert_eq!(prefs.title_source, TitleSource::Title);
        assert_eq!(
            prefs.row_items,
            RowItems {
                branch: false,
                project: false,
                pull_request: true,
                services: true,
                labels: true,
            }
        );
        assert_eq!(prefs.checks_display, ChecksDisplay::IconAndText);
        assert_eq!(prefs.trailing, Some(Trailing::DiffStats));
        assert_eq!(SidebarPrefs::parse("{}"), prefs);
    }

    #[test]
    fn a_second_click_clears_the_trailing_slot() {
        let shown = Some(Trailing::DiffStats);
        assert_eq!(Trailing::toggled(shown, Trailing::DiffStats), None);
        assert_eq!(
            Trailing::toggled(shown, Trailing::LastActivity),
            Some(Trailing::LastActivity)
        );
        assert_eq!(
            Trailing::toggled(None, Trailing::DiffStats),
            Some(Trailing::DiffStats)
        );
    }

    #[test]
    fn a_bad_display_field_falls_back_alone() {
        let prefs = SidebarPrefs::parse(
            r#"{"grouping":"sideways","titleSource":"branch","rowItems":{"branch":true,"labels":"yes"},
                "checksDisplay":7,"trailing":"none","projectFilter":["a",3],"labelFilter":"x",
                "collapsedStatuses":["done","bogus"]}"#,
        );
        assert_eq!(prefs.view.grouping, Grouping::Project);
        assert_eq!(prefs.title_source, TitleSource::Branch);
        assert!(prefs.row_items.branch);
        assert!(prefs.row_items.labels);
        assert_eq!(prefs.checks_display, ChecksDisplay::IconAndText);
        assert_eq!(prefs.trailing, None);
        assert_eq!(prefs.view.project_filter, HashSet::from(["a".to_string()]));
        assert!(prefs.view.label_filter.is_empty());
        assert_eq!(
            prefs.view.collapsed_statuses,
            HashSet::from([StatusBucket::Done])
        );
    }

    #[test]
    fn the_same_prefs_write_the_same_text() {
        assert_eq!(sample().to_json(), sample().to_json());
    }

    #[test]
    fn a_missing_empty_or_bad_file_gives_defaults() {
        let default = SidebarPrefs::default();
        assert_eq!(SidebarPrefs::parse(""), default);
        assert_eq!(SidebarPrefs::parse("not json"), default);
        assert_eq!(SidebarPrefs::parse("[1,2]"), default);
        assert_eq!(SidebarPrefs::parse("{}"), default);
        let missing = temp_dir("missing");
        assert_eq!(SidebarPrefs::load_from(&missing), default);
        let empty = temp_dir("empty");
        std::fs::create_dir_all(&empty).unwrap();
        std::fs::write(empty.join(FILE), "").unwrap();
        assert_eq!(SidebarPrefs::load_from(&empty), default);
        let _ = std::fs::remove_dir_all(&empty);
    }

    #[test]
    fn a_bad_field_falls_back_alone() {
        let prefs = SidebarPrefs::parse(
            r#"{"collapsedProjects":["a",3,null],"pinnedCollapsed":"yes",
                "expandedGroups":["pinned","other","project:b",7]}"#,
        );
        assert_eq!(
            prefs.view.collapsed_projects,
            HashSet::from(["a".to_string()])
        );
        assert!(!prefs.view.pinned_collapsed);
        assert_eq!(
            prefs.view.expanded_groups,
            HashSet::from([Group::Pinned, Group::Project("b".into())])
        );
    }

    #[test]
    fn the_saved_order_round_trips_and_a_bad_field_gives_empty() {
        let prefs = sample();
        assert!(!prefs.view.order.is_empty());
        let back = SidebarPrefs::parse(&prefs.to_json());
        assert_eq!(back.view.order, prefs.view.order);
        let bad = SidebarPrefs::parse(
            r#"{"projectOrder":5,"workspaceOrderByProject":[1],"pinnedWorkspaceOrder":["a",3]}"#,
        );
        assert!(bad.view.order.project_order.is_empty());
        assert!(bad.view.order.workspace_order_by_project.is_empty());
        assert_eq!(bad.view.order.pinned_workspace_order, ["a"]);
    }

    #[test]
    fn the_import_flag_round_trips_and_defaults_to_false() {
        let mut prefs = SidebarPrefs::default();
        assert!(!prefs.electron_order_imported);
        prefs.electron_order_imported = true;
        assert!(SidebarPrefs::parse(&prefs.to_json()).electron_order_imported);
        assert!(!SidebarPrefs::parse("{}").electron_order_imported);
        assert!(!SidebarPrefs::parse(r#"{"electronOrderImported":"yes"}"#).electron_order_imported);
    }

    /// An Electron user data folder whose store holds `order` under the sidebar order key.
    fn electron_store(name: &str, project_key: &str) -> PathBuf {
        let user_data = temp_dir(name);
        let options = rusty_leveldb::Options {
            create_if_missing: true,
            ..rusty_leveldb::Options::default()
        };
        let mut db =
            rusty_leveldb::DB::open(user_data.join("Local Storage/leveldb"), options).unwrap();
        let json = format!(
            r#"{{"state":{{"projectOrder":["{project_key}"],"workspaceOrderByProject":{{}},"pinnedWorkspaceOrder":[]}},"version":1}}"#
        );
        let mut value = vec![1u8];
        value.extend(json.bytes());
        db.put(
            b"_paseo://app\x00\x01sidebar-project-workspace-order",
            &value,
        )
        .unwrap();
        db.flush().unwrap();
        user_data
    }

    #[test]
    fn a_missing_electron_folder_sets_the_flag_keeps_the_order_and_writes_the_file() {
        let dir = temp_dir("import-missing-data");
        let mut saved = SidebarPrefs::default();
        saved.view.order.project_order = vec!["kept".into()];
        saved.save_to(&dir).unwrap();
        let missing = temp_dir("import-missing-electron");
        let prefs = SidebarPrefs::load_with_import(&dir, Some(&missing));
        assert!(prefs.electron_order_imported);
        assert_eq!(prefs.view.order.project_order, ["kept"]);
        let written = SidebarPrefs::load_from(&dir);
        assert!(written.electron_order_imported);
        assert_eq!(written.view.order.project_order, ["kept"]);
        let _ = std::fs::remove_dir_all(&dir);
    }

    #[test]
    fn the_import_replaces_the_order_once() {
        let dir = temp_dir("import-once-data");
        let electron = electron_store("import-once-electron", "from-electron");
        let mut saved = SidebarPrefs::default();
        saved.view.order.project_order = vec!["seeded".into()];
        saved.save_to(&dir).unwrap();
        let prefs = SidebarPrefs::load_with_import(&dir, Some(&electron));
        assert_eq!(prefs.view.order.project_order, ["from-electron"]);
        assert!(prefs.electron_order_imported);
        assert_eq!(
            SidebarPrefs::load_from(&dir).view.order.project_order,
            ["from-electron"]
        );
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&electron);
    }

    #[test]
    fn a_set_flag_never_reads_the_store() {
        let dir = temp_dir("import-flag-data");
        let electron = electron_store("import-flag-electron", "from-electron");
        let mut saved = SidebarPrefs {
            electron_order_imported: true,
            ..SidebarPrefs::default()
        };
        saved.view.order.project_order = vec!["mine".into()];
        saved.save_to(&dir).unwrap();
        let prefs = SidebarPrefs::load_with_import(&dir, Some(&electron));
        assert_eq!(prefs.view.order.project_order, ["mine"]);
        let _ = std::fs::remove_dir_all(&dir);
        let _ = std::fs::remove_dir_all(&electron);
    }

    #[test]
    fn unknown_keys_are_ignored() {
        let mut text: serde_json::Value = serde_json::from_str(&sample().to_json()).unwrap();
        text["futureField"] = serde_json::json!({"a": 1});
        assert_eq!(SidebarPrefs::parse(&text.to_string()), sample());
    }

    #[test]
    fn unknown_projects_are_dropped_and_pinned_stays() {
        let mut prefs = sample();
        let known = HashSet::from(["p1"]);
        assert!(prefs.retain_projects(&known));
        assert_eq!(
            prefs.view.collapsed_projects,
            HashSet::from(["p1".to_string()])
        );
        assert_eq!(
            prefs.view.expanded_groups,
            HashSet::from([
                Group::Pinned,
                Group::Project("p1".into()),
                Group::Status(StatusBucket::Running)
            ])
        );
        // The project filter drops the id of a project that went; the label filter stays.
        assert_eq!(prefs.view.project_filter, HashSet::from(["p1".to_string()]));
        assert_eq!(prefs.view.label_filter.len(), 2);
        assert!(!prefs.retain_projects(&known));
        let mut all_gone = sample();
        all_gone.retain_projects(&HashSet::new());
        assert_eq!(
            all_gone.view.expanded_groups,
            HashSet::from([Group::Pinned, Group::Status(StatusBucket::Running)])
        );
        assert!(all_gone.view.project_filter.is_empty());
        assert!(all_gone.view.collapsed_projects.is_empty());
        assert!(all_gone.view.pinned_collapsed);
    }

    #[test]
    fn a_save_to_a_read_only_dir_is_an_error() {
        use std::os::unix::fs::PermissionsExt;
        let dir = temp_dir("read-only");
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o555)).unwrap();
        let result = sample().save_to(&dir);
        std::fs::set_permissions(&dir, std::fs::Permissions::from_mode(0o755)).unwrap();
        let _ = std::fs::remove_dir_all(&dir);
        // A root user ignores the mode, so only a non-root run proves the error.
        if std::env::var("USER").as_deref() != Ok("root") {
            assert!(result.is_err());
        }
    }
}
