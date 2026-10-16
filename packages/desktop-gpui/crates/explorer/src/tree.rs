//! The Files tree model: listings per folder, the expanded folders, and the visible rows.
//!
//! A folder is listed the first time it opens (`list_directory`), so a workspace of any size costs
//! one request per expanded folder, and `rows` walks only the expanded folders. It follows
//! `flattenExplorerTree` in `packages/app/src/file-explorer/tree.ts` with the default `name` sort.

use bachuc_client::FileEntry;
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};

/// The path of the workspace root in `list_directory`.
pub const ROOT: &str = ".";

#[derive(Clone, Debug, PartialEq, Eq)]
enum Listing {
    Loading,
    Loaded(Vec<FileEntry>),
    Failed(String),
}

/// One visible row.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TreeRow {
    pub entry: FileEntry,
    pub depth: usize,
    /// A folder that is open.
    pub expanded: bool,
    /// An open folder whose listing has not arrived.
    pub loading: bool,
}

/// What the root listing shows when there are no rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum RootState {
    Loading,
    Failed(String),
    /// The listing arrived and has no entries.
    Empty,
    Ready,
}

#[derive(Clone, Debug, Default)]
pub struct FileTree {
    listings: HashMap<String, Listing>,
    expanded: HashSet<String>,
}

/// The result of `toggle`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Toggled {
    Collapsed,
    /// Opened. `needs_load` is true when the folder has no listing yet.
    Expanded {
        needs_load: bool,
    },
}

/// Folders first, then names without regard to case, as `localeCompare` orders plain names.
pub fn compare_entries(a: &FileEntry, b: &FileEntry) -> Ordering {
    b.is_dir
        .cmp(&a.is_dir)
        .then_with(|| a.name.to_lowercase().cmp(&b.name.to_lowercase()))
        .then_with(|| a.name.cmp(&b.name))
}

impl FileTree {
    pub fn new() -> Self {
        Self::default()
    }

    /// Forgets every listing and keeps the open folders, for a refresh. The caller lists the root
    /// and the open folders again.
    pub fn invalidate(&mut self) {
        self.listings.clear();
    }

    /// Forgets everything, for a new workspace folder.
    pub fn reset(&mut self) {
        self.listings.clear();
        self.expanded.clear();
    }

    /// The open folders, for a refresh that lists them again.
    pub fn expanded_paths(&self) -> Vec<String> {
        let mut paths: Vec<String> = self.expanded.iter().cloned().collect();
        paths.sort();
        paths
    }

    pub fn is_expanded(&self, path: &str) -> bool {
        self.expanded.contains(path)
    }

    /// True when `path` has no listing and none is on its way.
    pub fn needs_load(&self, path: &str) -> bool {
        !self.listings.contains_key(path)
    }

    /// Marks `path` as requested.
    pub fn start_load(&mut self, path: &str) {
        self.listings
            .entry(path.to_string())
            .or_insert(Listing::Loading);
    }

    pub fn set_listing(&mut self, path: &str, mut entries: Vec<FileEntry>) {
        entries.sort_by(compare_entries);
        self.listings
            .insert(path.to_string(), Listing::Loaded(entries));
    }

    pub fn set_failed(&mut self, path: &str, message: String) {
        self.listings
            .insert(path.to_string(), Listing::Failed(message));
    }

    /// Opens or closes a folder.
    pub fn toggle(&mut self, path: &str) -> Toggled {
        if self.expanded.remove(path) {
            return Toggled::Collapsed;
        }
        self.expanded.insert(path.to_string());
        Toggled::Expanded {
            needs_load: self.needs_load(path),
        }
    }

    pub fn root_state(&self) -> RootState {
        match self.listings.get(ROOT) {
            None | Some(Listing::Loading) => RootState::Loading,
            Some(Listing::Failed(message)) => RootState::Failed(message.clone()),
            Some(Listing::Loaded(entries)) if entries.is_empty() => RootState::Empty,
            Some(Listing::Loaded(_)) => RootState::Ready,
        }
    }

    /// The visible rows: the root entries, then the entries of each open folder under it.
    pub fn rows(&self) -> Vec<TreeRow> {
        let mut rows = Vec::new();
        self.push_rows(ROOT, 0, &mut rows);
        rows
    }

    fn push_rows(&self, path: &str, depth: usize, rows: &mut Vec<TreeRow>) {
        let Some(Listing::Loaded(entries)) = self.listings.get(path) else {
            return;
        };
        for entry in entries {
            let expanded = entry.is_dir && self.expanded.contains(&entry.path);
            let loading = expanded
                && matches!(
                    self.listings.get(&entry.path),
                    None | Some(Listing::Loading)
                );
            rows.push(TreeRow {
                entry: entry.clone(),
                depth,
                expanded,
                loading,
            });
            if expanded {
                self.push_rows(&entry.path, depth + 1, rows);
            }
        }
    }
}
