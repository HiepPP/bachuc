//! Subagent trees and the two Runs columns (`shared/tree.ts`), and the cluster rules of
//! `client/page.tsx`.

use super::run::{ProjectGroup, Run, group_runs, starred_first};
use super::text::time_value;
use std::collections::{HashMap, HashSet};

/// A run with its subagents. The flags and `ended_at` cover the whole subtree.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct RunTree {
    pub run: Run,
    pub children: Vec<RunTree>,
    pub count: usize,
    pub running: bool,
    pub starred: bool,
    pub needs_input: bool,
    pub ended_at: i64,
}

/// `buildRunTrees`. A missing parent or a parent cycle makes the run a root, so a malformed
/// parent never hides a conversation.
pub(crate) fn build_run_trees(runs: &[Run]) -> Vec<RunTree> {
    // A `Map` keyed by agent id: the first position, the last value.
    let mut nodes: Vec<Run> = Vec::new();
    let mut index: HashMap<String, usize> = HashMap::new();
    for run in runs {
        match index.get(&run.agent_id) {
            Some(&at) => nodes[at] = run.clone(),
            None => {
                index.insert(run.agent_id.clone(), nodes.len());
                nodes.push(run.clone());
            }
        }
    }
    let parent_of = |at: usize| {
        index
            .get(nodes[at].parent_agent_id.as_deref().unwrap_or(""))
            .copied()
    };
    let mut children: Vec<Vec<usize>> = vec![Vec::new(); nodes.len()];
    let mut roots = Vec::new();
    for at in 0..nodes.len() {
        let parent = parent_of(at);
        let mut seen = HashSet::from([at]);
        let mut ancestor = parent;
        while let Some(up) = ancestor.filter(|up| !seen.contains(up)) {
            seen.insert(up);
            ancestor = parent_of(up);
        }
        match (parent, ancestor) {
            (Some(parent), None) => children[parent].push(at),
            _ => roots.push(at),
        }
    }
    fn build(at: usize, nodes: &[Run], children: &[Vec<usize>]) -> RunTree {
        let run = nodes[at].clone();
        let mut tree = RunTree {
            count: 1,
            running: run.running(),
            starred: run.starred,
            needs_input: run.needs_input,
            ended_at: time_value(run.ended_at.as_deref()).unwrap_or(0),
            children: Vec::new(),
            run,
        };
        for &child in &children[at] {
            let child = build(child, nodes, children);
            tree.count += child.count;
            tree.running |= child.running;
            tree.starred |= child.starred;
            tree.needs_input |= child.needs_input;
            tree.ended_at = tree.ended_at.max(child.ended_at);
            tree.children.push(child);
        }
        tree
    }
    roots
        .into_iter()
        .map(|at| build(at, &nodes, &children))
        .collect()
}

#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct Columns {
    pub running: Vec<RunTree>,
    pub finished: Vec<RunTree>,
}

/// `boardColumns`: a tree with any running run is Running; finished trees sort starred first,
/// then by the newest end in the tree.
pub(crate) fn board_columns(runs: &[Run]) -> Columns {
    let (mut running, mut finished): (Vec<RunTree>, Vec<RunTree>) = build_run_trees(runs)
        .into_iter()
        .partition(|tree| tree.running);
    running.sort_by(|left, right| starred_first(left.starred, right.starred));
    finished.sort_by(|left, right| {
        starred_first(left.starred, right.starred).then(right.ended_at.cmp(&left.ended_at))
    });
    Columns { running, finished }
}

/// The order of a column's cards (`RunColumn`): starred trees above all others, then grouped
/// by project in first-seen order.
pub(crate) fn column_order(trees: &[RunTree]) -> Vec<usize> {
    let indexed: Vec<(usize, &RunTree)> = trees.iter().enumerate().collect();
    let mut order: Vec<(usize, &RunTree)> = group_runs(
        &indexed,
        |item: &(usize, &RunTree)| &item.1.run,
        |item| item.1.starred,
    )
    .into_iter()
    .flat_map(|group: ProjectGroup<(usize, &RunTree)>| group.runs)
    .collect();
    order.sort_by(|left, right| starred_first(left.1.starred, right.1.starred));
    order.into_iter().map(|(at, _)| at).collect()
}

/// Conversations in the trees, subagents included.
pub(crate) fn total(trees: &[RunTree]) -> usize {
    trees.iter().map(|tree| tree.count).sum()
}

/// Finished trees whose root the owner has not read.
pub(crate) fn unread_count(trees: &[RunTree]) -> usize {
    trees
        .iter()
        .filter(|tree| !tree.running && tree.run.unread == Some(true))
        .count()
}

/// `clusterSummary`: running and waiting subagents under the root.
pub(crate) fn cluster_summary(tree: &RunTree) -> (usize, usize) {
    fn visit(node: &RunTree, total: &mut (usize, usize)) {
        if node.run.running() {
            total.0 += 1;
        }
        if node.run.needs_input {
            total.1 += 1;
        }
        node.children.iter().for_each(|child| visit(child, total));
    }
    let mut total = (0, 0);
    tree.children
        .iter()
        .for_each(|child| visit(child, &mut total));
    total
}

/// The cluster's own read state covers its subagents (`RunCluster`): a finished root that the
/// host reports read.
pub(crate) fn cluster_seen(tree: &RunTree) -> bool {
    !tree.running && tree.run.unread == Some(false)
}

/// A lone subagent keeps the full row; two cells need a 406 px panel at the Board scale.
pub(crate) fn two_columns(children: usize, panel_width: f32, scale: f32) -> bool {
    children > 1 && panel_width >= 406. * scale
}

/// One item of a Runs column list. Items stay small: an expanded cluster's subagent rows are
/// their own items.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum ColumnItem {
    /// The column has no cards.
    Empty,
    /// A top-level card, or a cluster's head: its root card and toggle row.
    Card { tree: usize },
    /// One or two subagent cells of an expanded cluster's panel.
    Children {
        tree: usize,
        cells: Vec<usize>,
        first: bool,
        last: bool,
    },
}

/// The items of a column, in card order. `panel_width` is the width inside a top-level
/// cluster's panel, which decides one or two cells per row.
pub(crate) fn column_items(
    trees: &[RunTree],
    collapsed: &HashSet<String>,
    panel_width: f32,
    scale: f32,
) -> Vec<ColumnItem> {
    if trees.is_empty() {
        return vec![ColumnItem::Empty];
    }
    let mut items = Vec::new();
    for at in column_order(trees) {
        let tree = &trees[at];
        items.push(ColumnItem::Card { tree: at });
        if tree.children.is_empty() || collapsed.contains(&tree.run.agent_id) {
            continue;
        }
        let per_row = if two_columns(tree.children.len(), panel_width, scale) {
            2
        } else {
            1
        };
        let rows: Vec<Vec<usize>> = (0..tree.children.len())
            .collect::<Vec<_>>()
            .chunks(per_row)
            .map(<[usize]>::to_vec)
            .collect();
        let count = rows.len();
        for (row, cells) in rows.into_iter().enumerate() {
            items.push(ColumnItem::Children {
                tree: at,
                cells,
                first: row == 0,
                last: row + 1 == count,
            });
        }
    }
    items
}

/// A stable key for an item, so a new snapshot replaces only the items that changed.
pub(crate) fn item_key(trees: &[RunTree], item: &ColumnItem) -> String {
    match item {
        ColumnItem::Empty => "empty".into(),
        ColumnItem::Card { tree } => format!("card:{}", trees[*tree].run.id),
        ColumnItem::Children { tree, cells, .. } => {
            let tree = &trees[*tree];
            let ids: Vec<&str> = cells
                .iter()
                .map(|&cell| tree.children[cell].run.id.as_str())
                .collect();
            format!("row:{}:{}", tree.run.id, ids.join(","))
        }
    }
}

/// The smallest splice that turns `old` into `new`: the range of `old` to replace and the
/// count of new items. Items before and after keep their place and their scroll position.
pub(crate) fn splice_range(old: &[String], new: &[String]) -> (std::ops::Range<usize>, usize) {
    let prefix = old
        .iter()
        .zip(new)
        .take_while(|(left, right)| left == right)
        .count();
    let suffix = old[prefix..]
        .iter()
        .rev()
        .zip(new[prefix..].iter().rev())
        .take_while(|(left, right)| left == right)
        .count();
    (prefix..old.len() - suffix, new.len() - prefix - suffix)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::model::run::RunStatus;
    use crate::model::run::tests::card;

    fn run(id: &str, parent: Option<&str>) -> Run {
        let mut run = card(id, "app", false);
        run.parent_agent_id = parent.map(str::to_string);
        run.status = RunStatus::Completed;
        run.ended_at = Some("2026-09-22T00:00:00Z".into());
        run
    }

    fn roots(trees: &[RunTree]) -> Vec<&str> {
        trees.iter().map(|tree| tree.run.id.as_str()).collect()
    }

    #[test]
    fn children_before_parents_form_one_nested_cluster() {
        let input = [
            run("child", Some("parent")),
            run("grandchild", Some("child")),
            run("parent", None),
            run("other", None),
        ];
        let trees = build_run_trees(&input);
        assert_eq!(roots(&trees), ["parent", "other"]);
        assert_eq!(trees[0].count, 3);
        assert_eq!(trees[0].children[0].run.id, "child");
        assert_eq!(trees[0].children[0].children[0].run.id, "grandchild");
    }

    #[test]
    fn a_running_descendant_keeps_its_parent_in_running_with_attention_and_star() {
        let mut child = run("child", Some("parent"));
        child.status = RunStatus::Running;
        child.needs_input = true;
        child.starred = true;
        let mut rows = vec![run("parent", None), child];
        let columns = board_columns(&rows);
        assert_eq!((columns.running.len(), columns.finished.len()), (1, 0));
        let tree = &columns.running[0];
        assert_eq!(tree.run.status, RunStatus::Completed);
        assert!(tree.starred && tree.needs_input && !tree.run.starred);
        rows[1].status = RunStatus::Completed;
        let columns = board_columns(&rows);
        assert!(columns.running.is_empty());
        assert_eq!(columns.finished[0].count, 2);
    }

    #[test]
    fn a_missing_parent_makes_the_child_a_root() {
        let trees = build_run_trees(&[
            run("child", Some("removed")),
            run("other", None),
            run("self", Some("self")),
        ]);
        assert_eq!(roots(&trees), ["child", "other", "self"]);
        assert!(trees.iter().all(|tree| tree.count == 1));
    }

    #[test]
    fn a_parent_cycle_makes_both_runs_roots_each_once() {
        let trees = build_run_trees(&[
            run("a", Some("b")),
            run("b", Some("a")),
            run("c", Some("a")),
        ]);
        assert_eq!(roots(&trees), ["a", "b", "c"]);
        assert_eq!(total(&trees), 3);
    }

    #[test]
    fn running_trees_go_to_running_and_finished_sort_starred_then_newest_end() {
        let mut late = run("child", Some("parent"));
        late.ended_at = Some("2026-09-22T01:00:00Z".into());
        let mut starred = run("starred", None);
        starred.starred = true;
        let mut live = run("live", None);
        live.status = RunStatus::Running;
        let columns = board_columns(&[run("old", None), run("parent", None), late, starred, live]);
        assert_eq!(roots(&columns.running), ["live"]);
        assert_eq!(roots(&columns.finished), ["starred", "parent", "old"]);
        assert_eq!(total(&columns.finished), 4);
    }

    #[test]
    fn column_order_groups_projects_and_puts_starred_trees_first() {
        let mut a = run("a", None);
        a.project_key = "one".into();
        let mut b = run("b", None);
        b.project_key = "two".into();
        let mut c = run("c", None);
        c.project_key = "one".into();
        let mut child = run("child", Some("c"));
        child.starred = true;
        let trees = build_run_trees(&[a, b, c, child]);
        assert_eq!(column_order(&trees), [2, 0, 1]);
    }

    #[test]
    fn column_order_puts_starred_trees_above_unstarred_trees_of_other_projects() {
        let rows: Vec<Run> = [
            ("a", "one", true),
            ("b", "one", false),
            ("c", "two", true),
            ("d", "two", false),
        ]
        .into_iter()
        .map(|(id, project, starred)| {
            let mut row = run(id, None);
            row.project_key = project.into();
            row.starred = starred;
            row
        })
        .collect();
        let trees = build_run_trees(&rows);
        assert_eq!(column_order(&trees), [0, 2, 1, 3]);
    }

    #[test]
    fn the_cluster_summary_counts_subagents_only() {
        let mut root = run("root", None);
        root.status = RunStatus::Running;
        let mut waiting = run("waiting", Some("root"));
        waiting.status = RunStatus::Running;
        waiting.needs_input = true;
        let mut busy = run("busy", Some("waiting"));
        busy.status = RunStatus::Running;
        let trees = build_run_trees(&[root, waiting, busy, run("done", Some("root"))]);
        assert_eq!(cluster_summary(&trees[0]), (2, 1));
    }

    #[test]
    fn the_child_grid_follows_its_panel_width_and_the_board_scale() {
        assert!(two_columns(2, 440., 1.));
        assert!(!two_columns(2, 320., 1.));
        assert!(!two_columns(2, 440., 1.5));
        assert!(!two_columns(1, 900., 1.));
    }

    #[test]
    fn an_expanded_cluster_gives_each_child_row_its_own_item() {
        let trees = build_run_trees(&[
            run("parent", None),
            run("a", Some("parent")),
            run("b", Some("parent")),
            run("c", Some("parent")),
            run("solo", None),
        ]);
        let mut collapsed = HashSet::new();
        let items = column_items(&trees, &collapsed, 440., 1.);
        assert_eq!(
            items,
            vec![
                ColumnItem::Card { tree: 0 },
                ColumnItem::Children {
                    tree: 0,
                    cells: vec![0, 1],
                    first: true,
                    last: false,
                },
                ColumnItem::Children {
                    tree: 0,
                    cells: vec![2],
                    first: false,
                    last: true,
                },
                ColumnItem::Card { tree: 1 },
            ]
        );
        let keys: Vec<String> = items.iter().map(|item| item_key(&trees, item)).collect();
        assert_eq!(keys[1], "row:parent:a,b");
        // A narrow panel puts one subagent per row.
        assert_eq!(column_items(&trees, &collapsed, 320., 1.).len(), 5);
        collapsed.insert("parent".to_string());
        assert_eq!(
            column_items(&trees, &collapsed, 440., 1.),
            vec![ColumnItem::Card { tree: 0 }, ColumnItem::Card { tree: 1 }]
        );
        assert_eq!(
            column_items(&[], &collapsed, 440., 1.),
            vec![ColumnItem::Empty]
        );
    }

    #[test]
    fn a_splice_keeps_the_items_around_a_change() {
        let keys = |list: &[&str]| list.iter().map(|key| key.to_string()).collect::<Vec<_>>();
        let old = keys(&["a", "b", "c", "d"]);
        assert_eq!(splice_range(&old, &old), (4..4, 0));
        assert_eq!(splice_range(&old, &keys(&["a", "x", "c", "d"])), (1..2, 1));
        assert_eq!(splice_range(&old, &keys(&["a", "d"])), (1..3, 0));
        assert_eq!(
            splice_range(&old, &keys(&["z", "a", "b", "c", "d"])),
            (0..0, 1)
        );
        assert_eq!(splice_range(&[], &keys(&["a"])), (0..0, 1));
        assert_eq!(splice_range(&keys(&["a", "a"]), &keys(&["a"])), (1..2, 0));
    }

    #[test]
    fn a_read_finished_root_dims_its_cluster() {
        let mut root = run("root", None);
        root.unread = Some(false);
        let trees = build_run_trees(&[root.clone()]);
        assert!(cluster_seen(&trees[0]));
        root.unread = None;
        assert!(!cluster_seen(&build_run_trees(&[root])[0]));
        assert_eq!(unread_count(&trees), 0);
    }
}
