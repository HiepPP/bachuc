//! The Changes tree model: changed files grouped by folder, single-child folder chains merged,
//! and flattened into rows. A port of `packages/app/src/git/diff-tree.ts`, which owns the order
//! and the compression; keep them in step.

use bachuc_client::{ChangedFile, ChangedFileDetails, CheckoutDetails, FileDiff, FileDiffStatus};
use std::collections::HashSet;

/// A diff file as a Changes row file, with the `status` names of the wire.
pub fn changed_file_of(diff: &FileDiff) -> ChangedFileDetails {
    ChangedFileDetails {
        path: diff.path.clone(),
        old_path: diff.old_path.clone().filter(|old| !old.is_empty()),
        is_new: diff.is_new,
        is_deleted: diff.is_deleted,
        status: match diff.status {
            FileDiffStatus::Ok => "ok",
            FileDiffStatus::TooLarge => "too_large",
            FileDiffStatus::Binary => "binary",
        }
        .to_string(),
        additions: diff.additions,
        deletions: diff.deletions,
    }
}

/// How a changed file differs, as `FileChangeIcon` marks it. A rename is a modified file.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Change {
    Added,
    Deleted,
    Modified,
}

impl Change {
    pub fn of(is_new: bool, is_deleted: bool) -> Change {
        if is_new {
            Change::Added
        } else if is_deleted {
            Change::Deleted
        } else {
            Change::Modified
        }
    }

    /// `newFile`, `deletedFile`, and `modifiedFile`.
    pub fn label(self) -> &'static str {
        match self {
            Change::Added => "New",
            Change::Deleted => "Deleted",
            Change::Modified => "Modified",
        }
    }
}

/// The label of the branch toolbar when HEAD is detached (`branchUnknown`).
pub const UNKNOWN_BRANCH: &str = "Unknown";

/// What the Changes tab shows for a git checkout.
#[derive(Debug)]
pub struct CheckoutView {
    /// The branch, or `UNKNOWN_BRANCH` for a detached HEAD.
    pub branch: String,
    pub tree: ChangeTree,
    /// The files in daemon order; `ChangeRow::File` indexes point here too.
    pub files: Vec<ChangedFileDetails>,
}

impl CheckoutView {
    /// `None` when the folder is not in a git repository. A detached HEAD has no branch but is
    /// still a git checkout.
    pub fn new(details: CheckoutDetails) -> Option<CheckoutView> {
        if !details.is_git {
            return None;
        }
        let plain: Vec<ChangedFile> = details.files.iter().cloned().map(Into::into).collect();
        Some(CheckoutView {
            branch: details.branch.unwrap_or_else(|| UNKNOWN_BRANCH.to_string()),
            tree: ChangeTree::build(&plain),
            files: details.files,
        })
    }

    /// The Committed tree: the files of a `Base` diff under the branch of the status answer.
    pub fn from_diffs(branch: Option<String>, diffs: &[FileDiff]) -> CheckoutView {
        let files: Vec<ChangedFileDetails> = diffs.iter().map(changed_file_of).collect();
        let plain: Vec<ChangedFile> = files.iter().cloned().map(Into::into).collect();
        CheckoutView {
            branch: branch.unwrap_or_else(|| UNKNOWN_BRANCH.to_string()),
            tree: ChangeTree::build(&plain),
            files,
        }
    }

    pub fn change(&self, index: usize) -> Change {
        self.files.get(index).map_or(Change::Modified, |file| {
            Change::of(file.is_new, file.is_deleted)
        })
    }

    /// The row label: the path, `old -> new` for a rename, then the counts.
    pub fn label(&self, index: usize) -> String {
        let Some(file) = self.files.get(index) else {
            return String::new();
        };
        let path = match &file.old_path {
            Some(old) if *old != file.path => format!("{old} -> {}", file.path),
            _ => file.path.clone(),
        };
        format!(
            "{path}, {}, +{}, -{}",
            self.change(index).label(),
            file.additions,
            file.deletions
        )
    }
}

#[derive(Debug)]
enum Node {
    File {
        index: usize,
        name: String,
    },
    Dir {
        /// The full path of the deepest folder this node stands for; its identity for collapsing.
        dir_path: String,
        /// The label: a merged chain joins its segments, for example `packages/app/src`.
        name: String,
        children: Vec<Node>,
    },
}

/// One visible row of the Changes tree.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ChangeRow {
    Folder {
        dir_path: String,
        display_name: String,
        depth: usize,
        added: u32,
        removed: u32,
    },
    File {
        /// The index into the files that built the tree.
        index: usize,
        name: String,
        depth: usize,
    },
}

/// The tree of a set of changed files.
#[derive(Debug)]
pub struct ChangeTree {
    root: Vec<Node>,
    files: Vec<ChangedFile>,
}

fn name_order(a: &Node, b: &Node) -> std::cmp::Ordering {
    let key = |node: &Node| -> (bool, String) {
        match node {
            // Folders sort before files; plain ASCII, so the order does not depend on the locale.
            Node::Dir { name, .. } => (false, name.clone()),
            Node::File { name, .. } => (true, name.clone()),
        }
    };
    key(a).cmp(&key(b))
}

fn sort(nodes: &mut [Node]) {
    nodes.sort_by(name_order);
    for node in nodes {
        if let Node::Dir { children, .. } = node {
            sort(children);
        }
    }
}

fn ensure_dir<'a>(nodes: &'a mut Vec<Node>, parent: &str, segment: &str) -> &'a mut Vec<Node> {
    let dir_path = if parent.is_empty() {
        segment.to_string()
    } else {
        format!("{parent}/{segment}")
    };
    let position = nodes
        .iter()
        .position(|node| matches!(node, Node::Dir { dir_path: path, .. } if *path == dir_path));
    let position = position.unwrap_or_else(|| {
        nodes.push(Node::Dir {
            dir_path,
            name: segment.to_string(),
            children: Vec::new(),
        });
        nodes.len() - 1
    });
    match &mut nodes[position] {
        Node::Dir { children, .. } => children,
        Node::File { .. } => unreachable!("position found a folder"),
    }
}

fn compress(node: Node) -> Node {
    match node {
        Node::File { .. } => node,
        Node::Dir {
            mut dir_path,
            mut name,
            children,
        } => {
            let mut children: Vec<Node> = children.into_iter().map(compress).collect();
            while children.len() == 1 && matches!(children[0], Node::Dir { .. }) {
                let Some(Node::Dir {
                    dir_path: inner_path,
                    name: inner_name,
                    children: inner_children,
                }) = children.pop()
                else {
                    unreachable!("the only child is a folder");
                };
                name = if name.is_empty() {
                    inner_name
                } else {
                    format!("{name}/{inner_name}")
                };
                dir_path = inner_path;
                children = inner_children;
            }
            Node::Dir {
                dir_path,
                name,
                children,
            }
        }
    }
}

impl ChangeTree {
    /// Builds the compressed tree. The files keep their order in `files`, which row indexes use.
    pub fn build(files: &[ChangedFile]) -> ChangeTree {
        let mut root: Vec<Node> = Vec::new();
        for (index, file) in files.iter().enumerate() {
            let mut parts: Vec<&str> = file.path.split('/').collect();
            let name = parts.pop().unwrap_or_default().to_string();
            let mut nodes = &mut root;
            let mut parent = String::new();
            for segment in parts {
                nodes = ensure_dir(nodes, &parent, segment);
                parent = if parent.is_empty() {
                    segment.to_string()
                } else {
                    format!("{parent}/{segment}")
                };
            }
            nodes.push(Node::File { index, name });
        }
        sort(&mut root);
        // The virtual root is never merged; only its subtrees are.
        let root = root.into_iter().map(compress).collect();
        ChangeTree {
            root,
            files: files.to_vec(),
        }
    }

    pub fn files(&self) -> &[ChangedFile] {
        &self.files
    }

    /// Every folder path of the compressed tree, for "collapse all".
    pub fn folder_paths(&self) -> Vec<String> {
        fn walk(nodes: &[Node], out: &mut Vec<String>) {
            for node in nodes {
                if let Node::Dir {
                    dir_path, children, ..
                } = node
                {
                    out.push(dir_path.clone());
                    walk(children, out);
                }
            }
        }
        let mut out = Vec::new();
        walk(&self.root, &mut out);
        out
    }

    /// The rows in tree order. The children of a folder in `collapsed` are left out; a folder
    /// row always counts every file below it.
    pub fn rows(&self, collapsed: &HashSet<String>) -> Vec<ChangeRow> {
        fn totals(files: &[ChangedFile], children: &[Node]) -> (u32, u32) {
            children.iter().fold((0, 0), |(added, removed), node| {
                let (a, r) = match node {
                    Node::File { index, .. } => (files[*index].added, files[*index].removed),
                    Node::Dir { children, .. } => totals(files, children),
                };
                (added + a, removed + r)
            })
        }
        fn walk(
            files: &[ChangedFile],
            nodes: &[Node],
            depth: usize,
            collapsed: &HashSet<String>,
            rows: &mut Vec<ChangeRow>,
        ) {
            for node in nodes {
                match node {
                    Node::File { index, name } => rows.push(ChangeRow::File {
                        index: *index,
                        name: name.clone(),
                        depth,
                    }),
                    Node::Dir {
                        dir_path,
                        name,
                        children,
                    } => {
                        let (added, removed) = totals(files, children);
                        rows.push(ChangeRow::Folder {
                            dir_path: dir_path.clone(),
                            display_name: name.clone(),
                            depth,
                            added,
                            removed,
                        });
                        if !collapsed.contains(dir_path) {
                            walk(files, children, depth + 1, collapsed, rows);
                        }
                    }
                }
            }
        }
        let mut rows = Vec::new();
        walk(&self.files, &self.root, 0, collapsed, &mut rows);
        rows
    }
}

/// `formatDiffCount`: an `Intl` compact number with one fraction digit, in lower case.
pub fn format_count(value: u32) -> String {
    let mut scaled = value as f64;
    for suffix in ["", "k", "m", "b"] {
        let rounded = (scaled * 10.).round() / 10.;
        // 999,999 rounds up to 1,000k, which reads as 1m.
        if rounded < 1000. || suffix == "b" {
            return if rounded.fract() == 0. {
                format!("{rounded:.0}{suffix}")
            } else {
                format!("{rounded:.1}{suffix}")
            };
        }
        scaled /= 1000.;
    }
    unreachable!("the last suffix returns")
}
