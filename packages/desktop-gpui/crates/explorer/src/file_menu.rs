//! The right-click menu of a Files or Changes row, a port of `FileActionsContextMenuContent` in
//! `packages/app/src/components/file-actions-menu.tsx`, plus the confirm texts and path rules its
//! actions need. It holds no GPUI state, so the rules are plain functions.

use crate::tree::{ROOT, TreeRow};
use bachuc_icons::IconName;

/// What a row stands for.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum EntryKind {
    File,
    Directory,
}

/// The git state of a changed file of the Changes tree.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct GitState {
    /// A new file: discard deletes it.
    pub untracked: bool,
    pub deleted: bool,
    /// The tree lists uncommitted changes, which discard can revert. The Committed mode cannot.
    pub uncommitted: bool,
}

/// The host features the file operations need (`fsEntryOps`, `fsEntryDuplicate`).
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct EntryFeatures {
    pub entry_ops: bool,
    pub duplicate: bool,
}

/// One menu item, by what it does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileAction {
    NewFile,
    NewFolder,
    OpenToSide,
    CopyPath,
    CopyRelativePath,
    AddToChat,
    Rename,
    Duplicate,
    Revert,
    Delete,
}

/// `FileActionGroup`: a separator goes between two groups.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Group {
    Create,
    Open,
    Reference,
    Manage,
    Destructive,
}

impl FileAction {
    fn group(self) -> Group {
        match self {
            FileAction::NewFile | FileAction::NewFolder => Group::Create,
            FileAction::OpenToSide => Group::Open,
            FileAction::CopyPath | FileAction::CopyRelativePath | FileAction::AddToChat => {
                Group::Reference
            }
            FileAction::Rename | FileAction::Duplicate => Group::Manage,
            FileAction::Revert | FileAction::Delete => Group::Destructive,
        }
    }

    /// The `workspace.fileActions` labels.
    pub fn label(self) -> &'static str {
        match self {
            FileAction::NewFile => "New file",
            FileAction::NewFolder => "New folder",
            FileAction::OpenToSide => "Open to the side",
            FileAction::CopyPath => "Copy path",
            FileAction::CopyRelativePath => "Copy relative path",
            FileAction::AddToChat => "Add to chat",
            FileAction::Rename => "Rename",
            FileAction::Duplicate => "Duplicate",
            FileAction::Revert => "Discard changes",
            FileAction::Delete => "Delete",
        }
    }

    pub fn icon(self) -> IconName {
        match self {
            FileAction::NewFile => IconName::FilePlus,
            FileAction::NewFolder => IconName::FolderPlus,
            FileAction::OpenToSide => IconName::ArrowRightToLine,
            FileAction::CopyPath | FileAction::CopyRelativePath => IconName::Copy,
            FileAction::AddToChat => IconName::MessageSquarePlus,
            FileAction::Rename => IconName::Pencil,
            FileAction::Duplicate => IconName::CopyPlus,
            FileAction::Revert => IconName::Undo2,
            FileAction::Delete => IconName::Trash2,
        }
    }

    pub fn destructive(self) -> bool {
        self.group() == Group::Destructive
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum FileMenuEntry {
    Item(FileAction),
    Separator,
}

/// The menu of a row in the Electron order. `git` is `None` for a Files row and set for a changed
/// file of the Changes tree. Create, Rename, and Delete need `entry_ops`; Duplicate needs
/// `duplicate`. Open to the side and Add to chat need a file that exists.
pub fn menu_entries(
    kind: EntryKind,
    git: Option<GitState>,
    features: EntryFeatures,
) -> Vec<FileMenuEntry> {
    let file = kind == EntryKind::File;
    let candidates: Vec<(bool, FileAction)> = match git {
        None => vec![
            (features.entry_ops, FileAction::NewFile),
            (features.entry_ops, FileAction::NewFolder),
            (file, FileAction::OpenToSide),
            (true, FileAction::CopyPath),
            (true, FileAction::CopyRelativePath),
            (file, FileAction::AddToChat),
            (features.entry_ops, FileAction::Rename),
            (features.duplicate, FileAction::Duplicate),
            (features.entry_ops, FileAction::Delete),
        ],
        Some(git) => {
            let exists = file && !git.deleted;
            vec![
                (exists, FileAction::OpenToSide),
                (true, FileAction::CopyPath),
                (true, FileAction::CopyRelativePath),
                (exists, FileAction::AddToChat),
                (file && git.uncommitted, FileAction::Revert),
            ]
        }
    };
    let mut entries = Vec::new();
    let mut last: Option<Group> = None;
    for action in candidates
        .into_iter()
        .filter_map(|(shown, action)| shown.then_some(action))
    {
        if last.is_some_and(|group| group != action.group()) {
            entries.push(FileMenuEntry::Separator);
        }
        last = Some(action.group());
        entries.push(FileMenuEntry::Item(action));
    }
    entries
}

/// The texts of a destructive confirm.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Confirm {
    pub title: String,
    pub message: String,
    pub confirm: &'static str,
}

/// `confirmDelete`.
pub fn delete_confirm(name: &str, kind: EntryKind) -> Confirm {
    Confirm {
        title: match kind {
            EntryKind::File => "Delete file?",
            EntryKind::Directory => "Delete folder?",
        }
        .to_string(),
        message: format!("\"{name}\" will be permanently deleted."),
        confirm: "Delete",
    }
}

/// `confirmRevert` for a tracked file. Discarding an untracked file deletes it, so its confirm
/// says that instead.
pub fn revert_confirm(name: &str, untracked: bool) -> Confirm {
    if untracked {
        Confirm {
            title: format!("Delete {name}?"),
            message: "This file is not tracked.".to_string(),
            confirm: "Delete",
        }
    } else {
        Confirm {
            title: "Discard changes?".to_string(),
            message: format!("Changes to \"{name}\" will be permanently discarded."),
            confirm: "Discard",
        }
    }
}

/// `parentExplorerPath`: the folder of `path`, `ROOT` for a top-level entry.
pub fn parent_path(path: &str) -> &str {
    match path.rsplit_once('/') {
        Some((parent, _)) if !parent.is_empty() => parent,
        _ => ROOT,
    }
}

/// The path of `name` inside folder `parent`.
pub fn child_path(parent: &str, name: &str) -> String {
    if parent == ROOT || parent.is_empty() {
        name.to_string()
    } else {
        format!("{parent}/{name}")
    }
}

/// The folder New file and New folder create in: the folder itself, or the folder of a file.
pub fn create_parent(kind: EntryKind, path: &str) -> &str {
    match kind {
        EntryKind::Directory => path,
        EntryKind::File => parent_path(path),
    }
}

/// `buildAbsoluteExplorerPath`, for Copy path.
pub fn absolute_path(cwd: &str, path: &str) -> String {
    let root = cwd.trim().trim_end_matches(['/', '\\']);
    let path = path.trim();
    if root.is_empty() {
        return path.to_string();
    }
    if path.is_empty() || path == ROOT {
        return root.to_string();
    }
    if path.starts_with('/') {
        return path.to_string();
    }
    let separator = if root.contains('\\') { "\\" } else { "/" };
    let segments: Vec<&str> = path
        .split(['/', '\\'])
        .filter(|part| !part.is_empty())
        .collect();
    format!("{root}{separator}{}", segments.join(separator))
}

/// The trimmed name an inline row commits; an empty name cancels.
pub fn entry_name(input: &str) -> Option<String> {
    let name = input.trim();
    (!name.is_empty()).then(|| name.to_string())
}

/// The paths a discard sends: a rename also restores its old path.
pub fn discard_paths(path: &str, old_path: Option<&str>) -> Vec<String> {
    let mut paths = vec![path.to_string()];
    if let Some(old) = old_path.filter(|old| !old.is_empty() && *old != path) {
        paths.push(old.to_string());
    }
    paths
}

/// Where the New file or New folder row goes among `rows`: its list index and depth. It goes
/// first under its folder, or first in the list for the root or a folder that is not shown.
pub fn draft_slot(rows: &[TreeRow], parent: &str) -> (usize, usize) {
    if parent == ROOT {
        return (0, 0);
    }
    rows.iter()
        .position(|row| row.entry.path == parent)
        .map_or((0, 0), |index| (index + 1, rows[index].depth + 1))
}

#[cfg(test)]
mod tests {
    use super::{
        Confirm, EntryFeatures, EntryKind, FileAction, FileMenuEntry, GitState, absolute_path,
        child_path, create_parent, delete_confirm, discard_paths, draft_slot, entry_name,
        menu_entries, parent_path, revert_confirm,
    };
    use crate::tree::TreeRow;
    use bachuc_client::FileEntry;

    const ALL: EntryFeatures = EntryFeatures {
        entry_ops: true,
        duplicate: true,
    };

    fn render(entries: &[FileMenuEntry]) -> Vec<&'static str> {
        entries
            .iter()
            .map(|entry| match entry {
                FileMenuEntry::Item(action) => action.label(),
                FileMenuEntry::Separator => "---",
            })
            .collect()
    }

    #[test]
    fn file_menu_follows_the_electron_order_with_separators() {
        let entries = menu_entries(EntryKind::File, None, ALL);
        assert_eq!(
            render(&entries),
            vec![
                "New file",
                "New folder",
                "---",
                "Open to the side",
                "---",
                "Copy path",
                "Copy relative path",
                "Add to chat",
                "---",
                "Rename",
                "Duplicate",
                "---",
                "Delete",
            ]
        );
    }

    #[test]
    fn file_menu_of_a_folder_has_no_file_items() {
        let entries = menu_entries(EntryKind::Directory, None, ALL);
        assert_eq!(
            render(&entries),
            vec![
                "New file",
                "New folder",
                "---",
                "Copy path",
                "Copy relative path",
                "---",
                "Rename",
                "Duplicate",
                "---",
                "Delete",
            ]
        );
    }

    #[test]
    fn file_menu_hides_entry_ops_without_the_features() {
        let none = menu_entries(EntryKind::File, None, EntryFeatures::default());
        assert_eq!(
            render(&none),
            vec![
                "Open to the side",
                "---",
                "Copy path",
                "Copy relative path",
                "Add to chat",
            ]
        );
        let no_duplicate = menu_entries(
            EntryKind::File,
            None,
            EntryFeatures {
                entry_ops: true,
                duplicate: false,
            },
        );
        assert!(!no_duplicate.contains(&FileMenuEntry::Item(FileAction::Duplicate)));
        assert!(no_duplicate.contains(&FileMenuEntry::Item(FileAction::Rename)));
        let only_duplicate = menu_entries(
            EntryKind::File,
            None,
            EntryFeatures {
                entry_ops: false,
                duplicate: true,
            },
        );
        assert_eq!(
            render(&only_duplicate)[render(&only_duplicate).len() - 2..],
            ["---", "Duplicate"]
        );
    }

    #[test]
    fn file_menu_of_a_changed_file_ends_with_revert() {
        let git = GitState {
            untracked: false,
            deleted: false,
            uncommitted: true,
        };
        let entries = menu_entries(EntryKind::File, Some(git), ALL);
        assert_eq!(
            render(&entries),
            vec![
                "Open to the side",
                "---",
                "Copy path",
                "Copy relative path",
                "Add to chat",
                "---",
                "Discard changes",
            ]
        );
        assert!(FileAction::Revert.destructive());
        assert!(!FileAction::Rename.destructive());
    }

    #[test]
    fn file_menu_of_a_changed_file_drops_what_does_not_apply() {
        let deleted = GitState {
            untracked: false,
            deleted: true,
            uncommitted: true,
        };
        assert_eq!(
            render(&menu_entries(EntryKind::File, Some(deleted), ALL)),
            vec!["Copy path", "Copy relative path", "---", "Discard changes"]
        );
        let committed = GitState {
            uncommitted: false,
            ..deleted
        };
        assert_eq!(
            render(&menu_entries(EntryKind::File, Some(committed), ALL)),
            vec!["Copy path", "Copy relative path"]
        );
    }

    #[test]
    fn file_menu_revert_confirm_texts() {
        assert_eq!(
            revert_confirm("src/new.rs", true),
            Confirm {
                title: "Delete src/new.rs?".to_string(),
                message: "This file is not tracked.".to_string(),
                confirm: "Delete",
            }
        );
        assert_eq!(
            revert_confirm("src/lib.rs", false),
            Confirm {
                title: "Discard changes?".to_string(),
                message: "Changes to \"src/lib.rs\" will be permanently discarded.".to_string(),
                confirm: "Discard",
            }
        );
    }

    #[test]
    fn file_menu_delete_confirm_texts() {
        let file = delete_confirm("a.txt", EntryKind::File);
        assert_eq!(file.title, "Delete file?");
        assert_eq!(file.message, "\"a.txt\" will be permanently deleted.");
        assert_eq!(file.confirm, "Delete");
        assert_eq!(
            delete_confirm("src", EntryKind::Directory).title,
            "Delete folder?"
        );
    }

    #[test]
    fn file_menu_paths() {
        assert_eq!(parent_path("a/b/c.rs"), "a/b");
        assert_eq!(parent_path("c.rs"), ".");
        assert_eq!(child_path(".", "x.rs"), "x.rs");
        assert_eq!(child_path("a/b", "x.rs"), "a/b/x.rs");
        assert_eq!(create_parent(EntryKind::Directory, "a/b"), "a/b");
        assert_eq!(create_parent(EntryKind::File, "a/b/c.rs"), "a/b");
        assert_eq!(create_parent(EntryKind::File, "c.rs"), ".");
        assert_eq!(absolute_path("/repo/", "a/b.rs"), "/repo/a/b.rs");
        assert_eq!(absolute_path("/repo", "."), "/repo");
        assert_eq!(absolute_path("/repo", "/tmp/x"), "/tmp/x");
        assert_eq!(absolute_path("", "a.rs"), "a.rs");
        assert_eq!(entry_name("  new.rs \n"), Some("new.rs".to_string()));
        assert_eq!(entry_name("   "), None);
        assert_eq!(discard_paths("b.rs", Some("a.rs")), vec!["b.rs", "a.rs"]);
        assert_eq!(discard_paths("b.rs", None), vec!["b.rs"]);
        assert_eq!(discard_paths("b.rs", Some("b.rs")), vec!["b.rs"]);
    }

    #[test]
    fn file_menu_draft_row_goes_first_under_its_folder() {
        let row = |path: &str, depth: usize| TreeRow {
            entry: FileEntry {
                name: path.rsplit('/').next().unwrap_or(path).to_string(),
                path: path.to_string(),
                is_dir: true,
                size: 0,
            },
            depth,
            expanded: true,
            loading: false,
        };
        let rows = vec![row("a", 0), row("a/b", 1), row("c", 0)];
        assert_eq!(draft_slot(&rows, "."), (0, 0));
        assert_eq!(draft_slot(&rows, "a/b"), (2, 2));
        assert_eq!(draft_slot(&rows, "c"), (3, 1));
        assert_eq!(draft_slot(&rows, "gone"), (0, 0));
    }
}
