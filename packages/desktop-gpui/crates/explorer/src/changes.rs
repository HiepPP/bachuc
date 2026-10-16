//! The Changes tree model: changed files grouped by folder, single-child folder chains merged,
//! and flattened into rows. A port of `packages/app/src/git/diff-tree.ts`, which owns the order
//! and the compression; keep them in step.

use bachuc_client::ChangedFile;
use std::collections::HashSet;

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
