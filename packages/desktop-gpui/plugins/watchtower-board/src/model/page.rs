//! The view rules without GPUI: the page's workspace choice and placeholders
//! (`client/page.tsx`), the opener's task, the panel's folder name, and the list item diff.

use super::dto::Task;
use bachuc_client::ProjectNode;
use std::collections::BTreeMap;
use std::ops::Range;

/// What the page shows: one workspace, or a placeholder line.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum PageWorkspace {
    Ready(String),
    Message(&'static str),
}

/// The page shows one workspace's board only: `params.workspaceId`, else the preselected
/// workspace, else the active one, else the first workspace. A named workspace never falls back
/// to another one, because the overview's actions send prompts to that workspace's agent.
pub(crate) fn choose_workspace(
    param: Option<&str>,
    preselected: Option<&str>,
    active: Option<&str>,
    workspaces: &[&str],
    connected: bool,
) -> PageWorkspace {
    let named = [param, preselected, active]
        .into_iter()
        .flatten()
        .find(|id| !id.is_empty());
    match named {
        Some(id) if workspaces.contains(&id) => PageWorkspace::Ready(id.to_string()),
        Some(_) if connected => PageWorkspace::Message("Workspace not found."),
        Some(_) => PageWorkspace::Message("Loading workspace…"),
        None => match workspaces.first() {
            Some(first) => PageWorkspace::Ready(first.to_string()),
            None if connected => PageWorkspace::Message("No workspaces on this host."),
            None => PageWorkspace::Message("Loading workspaces…"),
        },
    }
}

/// The name of the project that holds the workspace.
pub(crate) fn project_name(projects: &[ProjectNode], workspace_id: &str) -> Option<String> {
    projects
        .iter()
        .find(|project| project.workspaces.iter().any(|w| w.id == workspace_id))
        .map(|project| project.name.clone())
}

/// The opener's task, `params.task`.
pub(crate) fn initial_task(params: &BTreeMap<String, String>) -> Option<String> {
    params.get("task").filter(|task| !task.is_empty()).cloned()
}

/// The task the list selects once the board loads: the opener's task when the board has it.
pub(crate) fn initial_selection(tasks: &[Task], initial: Option<&str>) -> Option<String> {
    let initial = initial?;
    tasks
        .iter()
        .find(|task| task.id == initial)
        .map(|task| task.id.clone())
}

/// The New workspace panel names the project by the last segment of its folder.
pub(crate) fn folder_name(cwd: &str) -> Option<String> {
    cwd.split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .next_back()
        .map(str::to_string)
}

/// The range of `old` that `new` replaces and the new item count there, for `ListState::splice`;
/// `None` when nothing changed.
pub(crate) fn changed_range<T: PartialEq>(old: &[T], new: &[T]) -> Option<(Range<usize>, usize)> {
    let prefix = old.iter().zip(new).take_while(|(a, b)| a == b).count();
    if prefix == old.len() && prefix == new.len() {
        return None;
    }
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(a, b)| a == b)
        .count();
    Some((prefix..old.len() - suffix, new.len() - prefix - suffix))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::dashboard::tests::task;
    use bachuc_client::WorkspaceNode;

    #[test]
    fn a_named_workspace_wins_in_order() {
        let all = ["w1", "w2", "w3"];
        assert_eq!(
            choose_workspace(Some("w2"), Some("w3"), Some("w1"), &all, true),
            PageWorkspace::Ready("w2".into())
        );
        assert_eq!(
            choose_workspace(None, Some("w3"), Some("w1"), &all, true),
            PageWorkspace::Ready("w3".into())
        );
        assert_eq!(
            choose_workspace(Some(""), None, Some("w1"), &all, true),
            PageWorkspace::Ready("w1".into())
        );
        assert_eq!(
            choose_workspace(None, None, None, &all, true),
            PageWorkspace::Ready("w1".into())
        );
    }

    #[test]
    fn a_named_workspace_missing_from_the_client_is_not_found_and_never_another() {
        let all = ["w1", "w2"];
        assert_eq!(
            choose_workspace(Some("gone"), None, Some("w1"), &all, true),
            PageWorkspace::Message("Workspace not found.")
        );
        assert_eq!(
            choose_workspace(None, Some("gone"), None, &all, true),
            PageWorkspace::Message("Workspace not found.")
        );
        assert_eq!(
            choose_workspace(Some("gone"), None, None, &[], false),
            PageWorkspace::Message("Loading workspace…")
        );
    }

    #[test]
    fn no_workspace_shows_the_electron_placeholders() {
        assert_eq!(
            choose_workspace(None, None, None, &[], true),
            PageWorkspace::Message("No workspaces on this host.")
        );
        assert_eq!(
            choose_workspace(None, None, None, &[], false),
            PageWorkspace::Message("Loading workspaces…")
        );
    }

    #[test]
    fn the_project_name_is_the_project_that_holds_the_workspace() {
        let workspace = |id: &str| WorkspaceNode {
            id: id.into(),
            name: id.into(),
            cwd: format!("/p/{id}"),
            branch: None,
            pinned: false,
            agent_ids: vec![],
            ..Default::default()
        };
        let projects = [
            ProjectNode {
                id: "p1".into(),
                name: "bachuc".into(),
                root: "/p".into(),
                workspaces: vec![workspace("w1")],
            },
            ProjectNode {
                id: "p2".into(),
                name: "other".into(),
                root: "/o".into(),
                workspaces: vec![workspace("w2")],
            },
        ];
        assert_eq!(project_name(&projects, "w2").as_deref(), Some("other"));
        assert_eq!(project_name(&projects, "w9"), None);
    }

    #[test]
    fn a_surface_opened_with_params_task_selects_that_task_and_an_unknown_id_selects_none() {
        let tasks = [task("TASK-001", "DONE"), task("TASK-002", "TODO")];
        let params = BTreeMap::from([("task".to_string(), "TASK-002".to_string())]);
        assert_eq!(
            initial_selection(&tasks, initial_task(&params).as_deref()).as_deref(),
            Some("TASK-002")
        );
        let unknown = BTreeMap::from([("task".to_string(), "TASK-099".to_string())]);
        assert_eq!(
            initial_selection(&tasks, initial_task(&unknown).as_deref()),
            None
        );
        assert_eq!(initial_task(&BTreeMap::new()), None);
        let empty = BTreeMap::from([("task".to_string(), String::new())]);
        assert_eq!(initial_task(&empty), None);
    }

    #[test]
    fn the_folder_name_is_the_last_path_segment() {
        assert_eq!(
            folder_name("/Users/hiep/Projects/bachuc/").as_deref(),
            Some("bachuc")
        );
        assert_eq!(folder_name(r"C:\work\app").as_deref(), Some("app"));
        assert_eq!(folder_name("/"), None);
    }

    #[test]
    fn changed_range_covers_only_the_items_that_differ() {
        assert_eq!(changed_range(&[1, 2, 3], &[1, 2, 3]), None);
        assert_eq!(changed_range(&[1, 2, 3], &[1, 9, 3]), Some((1..2, 1)));
        assert_eq!(changed_range(&[1, 2, 3], &[1, 2, 4, 5, 3]), Some((2..2, 2)));
        assert_eq!(changed_range(&[1, 2, 3], &[1]), Some((1..3, 0)));
        assert_eq!(changed_range::<i32>(&[], &[1, 2]), Some((0..0, 2)));
        assert_eq!(changed_range(&[1, 1], &[1, 1, 1]), Some((2..2, 1)));
    }
}
