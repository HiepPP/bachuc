//! The sidebar prefs that survive a restart, stored in `sidebar.json` in the GPUI data dir.
//! Same pattern as the app's `LayoutPrefs`: each field falls back to its default on its own,
//! and a save goes through a temp file and a rename.

use crate::model::{Group, ViewState};
use std::collections::HashSet;
use std::path::Path;

const FILE: &str = "sidebar.json";
const PINNED: &str = "pinned";
const PROJECT_PREFIX: &str = "project:";

#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SidebarPrefs {
    /// Which projects and sections the owner collapsed or expanded.
    pub view: ViewState,
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
            },
        }
    }

    /// The lists are sorted, so the same prefs always give the same text.
    pub fn to_json(&self) -> String {
        let mut collapsed: Vec<&str> = self
            .view
            .collapsed_projects
            .iter()
            .map(String::as_str)
            .collect();
        collapsed.sort_unstable();
        let mut expanded: Vec<String> = self.view.expanded_groups.iter().map(group_key).collect();
        expanded.sort_unstable();
        serde_json::json!({
            "collapsedProjects": collapsed,
            "pinnedCollapsed": self.view.pinned_collapsed,
            "expandedGroups": expanded,
        })
        .to_string()
    }

    pub fn load_from(dir: &Path) -> Self {
        std::fs::read_to_string(dir.join(FILE))
            .map(|text| Self::parse(&text))
            .unwrap_or_default()
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
        self.view
            .collapsed_projects
            .retain(|id| known.contains(id.as_str()));
        self.view.expanded_groups.retain(|group| match group {
            Group::Pinned => true,
            Group::Project(id) => known.contains(id.as_str()),
        });
        collapsed != self.view.collapsed_projects.len()
            || expanded != self.view.expanded_groups.len()
    }
}

fn group_key(group: &Group) -> String {
    match group {
        Group::Pinned => PINNED.to_string(),
        Group::Project(id) => format!("{PROJECT_PREFIX}{id}"),
    }
}

fn parse_group(key: &str) -> Option<Group> {
    if key == PINNED {
        return Some(Group::Pinned);
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
                expanded_groups: [Group::Pinned, Group::Project("p1".into())].into(),
            },
        }
    }

    fn temp_dir(name: &str) -> PathBuf {
        let dir =
            std::env::temp_dir().join(format!("bachuc-sidebar-{name}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        dir
    }

    #[test]
    fn all_three_fields_round_trip() {
        let prefs = sample();
        assert_eq!(SidebarPrefs::parse(&prefs.to_json()), prefs);
        let dir = temp_dir("round-trip");
        prefs.save_to(&dir).unwrap();
        assert_eq!(SidebarPrefs::load_from(&dir), prefs);
        let _ = std::fs::remove_dir_all(&dir);
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
            HashSet::from([Group::Pinned, Group::Project("p1".into())])
        );
        assert!(!prefs.retain_projects(&known));
        let mut all_gone = sample();
        all_gone.retain_projects(&HashSet::new());
        assert_eq!(
            all_gone.view.expanded_groups,
            HashSet::from([Group::Pinned])
        );
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
