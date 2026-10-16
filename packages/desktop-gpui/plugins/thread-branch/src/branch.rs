//! The branch rules of `client/label.ts`, `client/buttons.ts`, and `client/sync-icon.tsx`, one
//! function per TS function with the same results, over the `thread-branch.get` answer.

use bachuc_icons::IconName;
use bachuc_theme::Tokens;
use gpui_kit::Hsla;
use serde::Deserialize;

/// The most characters a pill label shows (`MAX_LABEL`).
const MAX_LABEL: usize = 24;

/// `branchInfoSchema` without `prLookup`, which no pill reads. The derived equality is the
/// `signature` of `pills.ts:28`: every field a pill shows.
#[derive(Clone, Debug, PartialEq, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchInfo {
    /// False when the folder is not in a git work tree.
    pub repo: bool,
    pub branch: Option<String>,
    pub detached: bool,
    pub sha: Option<String>,
    pub dirty: bool,
    pub changes: Option<Changes>,
    pub upstream: Option<String>,
    pub ahead: Option<u32>,
    pub behind: Option<u32>,
    pub pr: Option<PullRequest>,
    pub remote_url: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct Changes {
    pub files: u32,
    /// Null when `git diff --numstat HEAD` fails, for example before the first commit.
    pub added: Option<u32>,
    pub deleted: Option<u32>,
}

#[derive(Clone, Debug, PartialEq, Deserialize)]
pub(crate) struct PullRequest {
    pub number: u32,
    pub url: String,
    pub state: String,
}

fn truncate(text: &str, max: usize) -> String {
    if text.chars().count() <= max {
        return text.to_string();
    }
    let head: String = text.chars().take(max - 1).collect();
    format!("{head}…")
}

pub(crate) fn pill_label(info: &BranchInfo) -> String {
    let name = if info.detached {
        format!("@{}", info.sha.as_deref().unwrap_or("HEAD"))
    } else {
        info.branch.clone().unwrap_or_else(|| "unknown".into())
    };
    truncate(&name, MAX_LABEL)
}

/// The New workspace label: the branch of the selected project, before any agent exists.
pub(crate) fn draft_pill_label(cwd: Option<&str>, info: Option<&BranchInfo>) -> String {
    match (cwd.filter(|cwd| !cwd.is_empty()), info) {
        (None, _) => "No project".into(),
        (Some(_), None) => "…".into(),
        (Some(_), Some(info)) if info.repo => pill_label(info),
        (Some(_), Some(_)) => "Not a git repo".into(),
    }
}

pub(crate) fn pill_title(info: &BranchInfo) -> String {
    let mut parts = vec![if info.detached {
        format!("Detached at {}", info.sha.as_deref().unwrap_or("HEAD"))
    } else {
        format!("Branch {}", info.branch.as_deref().unwrap_or("unknown"))
    }];
    if let Some(upstream) = &info.upstream {
        parts.push(format!("tracks {upstream}"));
        if let (Some(ahead), Some(behind)) = (info.ahead, info.behind) {
            parts.push(format!("ahead {ahead} / behind {behind}"));
        }
    }
    if info.dirty {
        parts.push("uncommitted changes".into());
    }
    if let Some(pr) = &info.pr {
        parts.push(format!("PR #{} {}", pr.number, pr.state.to_lowercase()));
    }
    parts.join(" · ")
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum SyncState {
    Synced,
    Ahead,
    Behind,
}

/// Pull first: anything behind needs attention before pushing (`syncState`).
pub(crate) fn sync_state(ahead: u32, behind: u32) -> SyncState {
    if behind > 0 {
        SyncState::Behind
    } else if ahead > 0 {
        SyncState::Ahead
    } else {
        SyncState::Synced
    }
}

/// The branch pill color: the host's muted tone when in sync, accent while commits wait to
/// push, and the warning tone while commits wait to pull (`branchIcons`).
pub(crate) fn sync_tone(sync: SyncState, tokens: &Tokens) -> Option<Hsla> {
    match sync {
        SyncState::Synced => None,
        SyncState::Ahead => Some(tokens.accent),
        SyncState::Behind => Some(tokens.status_warning),
    }
}

/// Last path segment of the `origin` web URL, such as `bachuc` (`repoName`).
pub(crate) fn repo_name(url: &str) -> String {
    let path = ["https://", "http://"]
        .iter()
        .find_map(|scheme| url.strip_prefix(scheme))
        .and_then(|rest| rest.split_once('/'))
        .map_or(url, |(_, path)| path);
    match path.rsplit('/').next() {
        Some(name) if !name.is_empty() => name.to_string(),
        _ => "repository".into(),
    }
}

/// What the branch pill shows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BranchPill {
    pub title: String,
    /// The name plus the nonzero ahead and behind counts, as `main ↑2 ↓1`.
    pub label: String,
    pub sync: SyncState,
}

/// The branch pill, hidden outside a git work tree. Counts show only with an upstream.
pub(crate) fn branch_pill(info: &BranchInfo) -> Option<BranchPill> {
    if !info.repo {
        return None;
    }
    let tracked = info.upstream.is_some();
    let ahead = if tracked { info.ahead.unwrap_or(0) } else { 0 };
    let behind = if tracked { info.behind.unwrap_or(0) } else { 0 };
    let mut label = pill_label(info);
    if ahead > 0 {
        label.push_str(&format!(" ↑{ahead}"));
    }
    if behind > 0 {
        label.push_str(&format!(" ↓{behind}"));
    }
    Some(BranchPill {
        title: pill_title(info),
        label,
        sync: sync_state(ahead, behind),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct PrPill {
    pub title: String,
    pub label: String,
    pub url: String,
}

/// One click opens the pull request; hidden when the branch has none.
pub(crate) fn pr_pill(info: &BranchInfo) -> Option<PrPill> {
    let pr = info.pr.as_ref().filter(|_| info.repo)?;
    Some(PrPill {
        title: format!("Open PR #{} ({})", pr.number, pr.state.to_lowercase()),
        label: format!("#{}", pr.number),
        url: pr.url.clone(),
    })
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct ChangesPill {
    pub title: String,
    pub label: String,
}

/// `3 files +14 -3` for uncommitted tracked changes; hidden when the work tree is clean.
pub(crate) fn changes_pill(info: &BranchInfo) -> Option<ChangesPill> {
    let changes = info.changes.as_ref().filter(|_| info.repo && info.dirty)?;
    let files = format!(
        "{} {}",
        changes.files,
        if changes.files == 1 { "file" } else { "files" }
    );
    let lines = match (changes.added, changes.deleted) {
        (Some(added), Some(deleted)) => format!(" +{added} -{deleted}"),
        _ => String::new(),
    };
    let detail = if lines.is_empty() {
        String::new()
    } else {
        format!(",{lines} lines vs HEAD")
    };
    Some(ChangesPill {
        title: format!("{files} uncommitted{detail} · click to refresh"),
        label: format!("{files}{lines}"),
    })
}

/// What a menu row does.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum MenuAction {
    Copy,
    OpenRepo,
    Fetch,
    Refresh,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct MenuRow {
    pub id: &'static str,
    pub title: String,
    pub icon: IconName,
    pub action: MenuAction,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum MenuEntry {
    Row(MenuRow),
    Separator,
}

fn row(id: &'static str, title: &str, icon: IconName, action: MenuAction) -> MenuEntry {
    MenuEntry::Row(MenuRow {
        id,
        title: title.into(),
        icon,
        action,
    })
}

/// Drops hidden rows, then separators that lead, repeat, or trail (`visibleMenuItems`).
fn visible_entries(entries: Vec<(MenuEntry, bool)>) -> Vec<MenuEntry> {
    let mut shown: Vec<MenuEntry> = Vec::new();
    for (entry, visible) in entries {
        match entry {
            MenuEntry::Row(_) if !visible => {}
            MenuEntry::Separator if matches!(shown.last(), None | Some(MenuEntry::Separator)) => {}
            entry => shown.push(entry),
        }
    }
    if shown.last() == Some(&MenuEntry::Separator) {
        shown.pop();
    }
    shown
}

/// The agent branch pill menu: Copy branch name, Open repository, a divider, Fetch, Refresh.
pub(crate) fn branch_menu(info: &BranchInfo) -> Vec<MenuEntry> {
    let url = info.remote_url.as_deref();
    let github = url.is_some_and(|url| url.contains("github.com"));
    let open = match url {
        Some(url) => format!(
            "Open {}{}",
            repo_name(url),
            if github { " on GitHub" } else { "" }
        ),
        None => "Open repository".into(),
    };
    visible_entries(vec![
        (
            row(
                "copy-branch",
                "Copy branch name",
                IconName::Copy,
                MenuAction::Copy,
            ),
            !info.detached && info.branch.is_some(),
        ),
        (
            row(
                "open-repo",
                &open,
                if github {
                    IconName::Github
                } else {
                    IconName::Globe
                },
                MenuAction::OpenRepo,
            ),
            url.is_some(),
        ),
        (MenuEntry::Separator, true),
        (
            row("fetch", "Fetch", IconName::CloudDownload, MenuAction::Fetch),
            info.upstream.is_some(),
        ),
        (
            row(
                "refresh",
                "Refresh",
                IconName::RefreshCw,
                MenuAction::Refresh,
            ),
            true,
        ),
    ])
}

/// The New workspace branch pill menu: every row shows, as `draft-pill.ts:78-120`.
pub(crate) fn draft_menu() -> Vec<MenuEntry> {
    visible_entries(vec![
        (
            row(
                "copy-branch",
                "Copy branch name",
                IconName::Copy,
                MenuAction::Copy,
            ),
            true,
        ),
        (MenuEntry::Separator, true),
        (
            row("fetch", "Fetch", IconName::CloudDownload, MenuAction::Fetch),
            true,
        ),
        (
            row(
                "refresh",
                "Refresh",
                IconName::RefreshCw,
                MenuAction::Refresh,
            ),
            true,
        ),
    ])
}

/// The text a Copy press writes. The agent pill refuses only a missing name; the draft pill
/// also refuses a detached head (`pills.ts:77`, `draft-pill.ts:77`).
pub(crate) fn copy_text(info: &BranchInfo, draft: bool) -> Result<String, &'static str> {
    match &info.branch {
        Some(branch) if !(draft && info.detached) => Ok(branch.clone()),
        _ => Err("No branch name to copy."),
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BranchInfo, Changes, MenuAction, MenuEntry, PullRequest, SyncState, branch_menu,
        branch_pill, changes_pill, copy_text, draft_menu, draft_pill_label, pill_label, pill_title,
        pr_pill, repo_name, sync_state, sync_tone,
    };
    use bachuc_theme::Tokens;

    fn base() -> BranchInfo {
        BranchInfo {
            repo: true,
            branch: Some("feat/board-avatars".into()),
            detached: false,
            sha: Some("a1b2c3d".into()),
            dirty: false,
            changes: None,
            upstream: None,
            ahead: None,
            behind: None,
            pr: None,
            remote_url: None,
        }
    }

    fn open_pr() -> PullRequest {
        PullRequest {
            number: 42,
            url: "https://github.com/o/r/pull/42".into(),
            state: "OPEN".into(),
        }
    }

    fn row_ids(entries: &[MenuEntry]) -> Vec<&str> {
        entries
            .iter()
            .map(|entry| match entry {
                MenuEntry::Row(row) => row.id,
                MenuEntry::Separator => "-",
            })
            .collect()
    }

    #[test]
    fn label_shows_branch_without_pr_suffix_detached_sha_and_truncation() {
        assert_eq!(pill_label(&base()), "feat/board-avatars");
        let with_pr = BranchInfo {
            pr: Some(open_pr()),
            ..base()
        };
        assert_eq!(pill_label(&with_pr), "feat/board-avatars");
        let detached = BranchInfo {
            branch: None,
            detached: true,
            ..base()
        };
        assert_eq!(pill_label(&detached), "@a1b2c3d");
        let long = BranchInfo {
            branch: Some("feature/very-long-branch-name-that-overflows".into()),
            ..base()
        };
        assert_eq!(pill_label(&long), "feature/very-long-branc…");
        assert_eq!(pill_label(&long).chars().count(), 24);
        // A 24-character name stays whole.
        let exact = BranchInfo {
            branch: Some("a".repeat(24)),
            ..base()
        };
        assert_eq!(pill_label(&exact), "a".repeat(24));
    }

    #[test]
    fn new_workspace_label_names_the_selected_projects_branch_or_why_there_is_none() {
        assert_eq!(draft_pill_label(None, None), "No project");
        assert_eq!(draft_pill_label(Some("/repo"), None), "…");
        assert_eq!(
            draft_pill_label(Some("/repo"), Some(&base())),
            "feat/board-avatars"
        );
        let plain = BranchInfo {
            repo: false,
            branch: None,
            ..base()
        };
        assert_eq!(
            draft_pill_label(Some("/tmp"), Some(&plain)),
            "Not a git repo"
        );
    }

    #[test]
    fn title_carries_upstream_sync_counts_dirty_state_and_pr_state() {
        let info = BranchInfo {
            upstream: Some("origin/feat/board-avatars".into()),
            ahead: Some(2),
            behind: Some(0),
            dirty: true,
            pr: Some(open_pr()),
            ..base()
        };
        assert_eq!(
            pill_title(&info),
            "Branch feat/board-avatars · tracks origin/feat/board-avatars · ahead 2 / behind 0 \
             · uncommitted changes · PR #42 open"
        );
        let detached = BranchInfo {
            detached: true,
            ..base()
        };
        assert_eq!(pill_title(&detached), "Detached at a1b2c3d");
    }

    #[test]
    fn pr_pill_is_a_one_click_action_hidden_without_a_pull_request() {
        assert_eq!(pr_pill(&base()), None);
        let info = BranchInfo {
            pr: Some(PullRequest {
                number: 7,
                url: "https://github.com/o/r/pull/7".into(),
                state: "OPEN".into(),
            }),
            ..base()
        };
        let pill = pr_pill(&info).unwrap();
        assert_eq!(pill.label, "#7");
        assert_eq!(pill.title, "Open PR #7 (open)");
        assert_eq!(pill.url, "https://github.com/o/r/pull/7");
    }

    #[test]
    fn branch_pill_adds_nonzero_push_and_pull_counts_and_tints_its_icon() {
        let tracked = BranchInfo {
            upstream: Some("origin/feat/board-avatars".into()),
            ..base()
        };
        let pill = |info: &BranchInfo| branch_pill(info).unwrap();
        let untracked = BranchInfo {
            ahead: Some(4),
            behind: Some(4),
            ..base()
        };
        assert_eq!(pill(&untracked).label, "feat/board-avatars");
        assert_eq!(pill(&untracked).sync, SyncState::Synced);
        let synced = BranchInfo {
            ahead: Some(0),
            behind: Some(0),
            ..tracked.clone()
        };
        assert_eq!(pill(&synced).label, "feat/board-avatars");
        assert_eq!(pill(&synced).sync, SyncState::Synced);
        let ahead = BranchInfo {
            ahead: Some(2),
            behind: Some(0),
            ..tracked.clone()
        };
        assert_eq!(pill(&ahead).label, "feat/board-avatars ↑2");
        assert_eq!(pill(&ahead).sync, SyncState::Ahead);
        let behind = BranchInfo {
            ahead: Some(0),
            behind: Some(1),
            ..tracked.clone()
        };
        assert_eq!(pill(&behind).label, "feat/board-avatars ↓1");
        let both = BranchInfo {
            ahead: Some(2),
            behind: Some(3),
            ..tracked
        };
        assert_eq!(pill(&both).label, "feat/board-avatars ↑2 ↓3");
        assert_eq!(pill(&both).sync, SyncState::Behind);
        let tokens = Tokens::light();
        assert_eq!(sync_tone(SyncState::Synced, &tokens), None);
        assert_eq!(sync_tone(SyncState::Ahead, &tokens), Some(tokens.accent));
        assert_eq!(
            sync_tone(SyncState::Behind, &tokens),
            Some(tokens.status_warning)
        );
    }

    #[test]
    fn branch_menu_opens_the_origin_repository_hidden_without_a_web_remote() {
        assert!(!row_ids(&branch_menu(&base())).contains(&"open-repo"));
        let github = BranchInfo {
            remote_url: Some("https://github.com/o/r".into()),
            ..base()
        };
        let menu = branch_menu(&github);
        let MenuEntry::Row(open) = &menu[1] else {
            panic!("expected the open row second: {menu:?}");
        };
        assert_eq!(open.title, "Open r on GitHub");
        assert_eq!(open.icon, bachuc_icons::IconName::Github);
        assert_eq!(open.action, MenuAction::OpenRepo);
        let gitlab = BranchInfo {
            remote_url: Some("https://gitlab.com/o/r".into()),
            ..base()
        };
        let menu = branch_menu(&gitlab);
        let MenuEntry::Row(open) = &menu[1] else {
            panic!("expected the open row second: {menu:?}");
        };
        assert_eq!(open.title, "Open r");
        assert_eq!(open.icon, bachuc_icons::IconName::Globe);
    }

    #[test]
    fn branch_menu_offers_fetch_only_when_the_branch_has_an_upstream() {
        assert!(!row_ids(&branch_menu(&base())).contains(&"fetch"));
        let tracked = BranchInfo {
            upstream: Some("origin/feat/board-avatars".into()),
            ..base()
        };
        let menu = branch_menu(&tracked);
        assert!(row_ids(&menu).contains(&"fetch"));
        let fetch = menu
            .iter()
            .find_map(|entry| match entry {
                MenuEntry::Row(row) if row.id == "fetch" => Some(row),
                _ => None,
            })
            .unwrap();
        assert_eq!(fetch.title, "Fetch");
        assert_eq!(fetch.action, MenuAction::Fetch);
    }

    #[test]
    fn sync_state_puts_pulling_first() {
        assert_eq!(sync_state(0, 0), SyncState::Synced);
        assert_eq!(sync_state(3, 0), SyncState::Ahead);
        assert_eq!(sync_state(0, 1), SyncState::Behind);
        assert_eq!(sync_state(3, 1), SyncState::Behind);
    }

    #[test]
    fn changes_pill_shows_file_and_line_counts_only_when_the_work_tree_is_dirty() {
        assert_eq!(changes_pill(&base()), None);
        let dirty = BranchInfo {
            dirty: true,
            changes: Some(Changes {
                files: 3,
                added: Some(14),
                deleted: Some(3),
            }),
            ..base()
        };
        let pill = changes_pill(&dirty).unwrap();
        assert_eq!(pill.label, "3 files +14 -3");
        assert_eq!(
            pill.title,
            "3 files uncommitted, +14 -3 lines vs HEAD · click to refresh"
        );
        let single = BranchInfo {
            dirty: true,
            changes: Some(Changes {
                files: 1,
                added: None,
                deleted: None,
            }),
            ..base()
        };
        let pill = changes_pill(&single).unwrap();
        assert_eq!(pill.label, "1 file");
        assert_eq!(pill.title, "1 file uncommitted · click to refresh");
    }

    #[test]
    fn branch_label_counts() {
        let tracked = |ahead, behind| BranchInfo {
            upstream: Some("origin/main".into()),
            branch: Some("main".into()),
            ahead: Some(ahead),
            behind: Some(behind),
            ..base()
        };
        assert_eq!(branch_pill(&tracked(2, 0)).unwrap().label, "main ↑2");
        assert_eq!(branch_pill(&tracked(0, 1)).unwrap().label, "main ↓1");
        assert_eq!(branch_pill(&tracked(2, 1)).unwrap().label, "main ↑2 ↓1");
        assert_eq!(branch_pill(&tracked(0, 0)).unwrap().label, "main");
        // No upstream: the counts are left out.
        let no_upstream = BranchInfo {
            branch: Some("main".into()),
            ahead: Some(2),
            behind: Some(1),
            ..base()
        };
        assert_eq!(branch_pill(&no_upstream).unwrap().label, "main");
    }

    #[test]
    fn pills_hidden_rules() {
        let plain = BranchInfo {
            repo: false,
            dirty: true,
            changes: Some(Changes {
                files: 1,
                added: Some(1),
                deleted: Some(0),
            }),
            pr: Some(open_pr()),
            ..base()
        };
        assert_eq!(branch_pill(&plain), None);
        assert_eq!(pr_pill(&plain), None);
        assert_eq!(changes_pill(&plain), None);
        // A clean tree has no changes pill, even with counts; no PR has no PR pill.
        let clean = BranchInfo {
            changes: Some(Changes {
                files: 1,
                added: Some(1),
                deleted: Some(0),
            }),
            ..base()
        };
        assert!(branch_pill(&clean).is_some());
        assert_eq!(changes_pill(&clean), None);
        assert_eq!(pr_pill(&clean), None);
        // A dirty tree whose counts are unknown has no changes pill either.
        let unknown = BranchInfo {
            dirty: true,
            ..base()
        };
        assert_eq!(changes_pill(&unknown), None);
    }

    #[test]
    fn draft_label_rule() {
        assert_eq!(draft_pill_label(None, None), "No project");
        assert_eq!(draft_pill_label(Some(""), Some(&base())), "No project");
        assert_eq!(draft_pill_label(Some("/p"), None), "\u{2026}");
        let plain = BranchInfo {
            repo: false,
            ..base()
        };
        assert_eq!(draft_pill_label(Some("/p"), Some(&plain)), "Not a git repo");
        assert_eq!(
            draft_pill_label(Some("/p"), Some(&base())),
            "feat/board-avatars"
        );
    }

    #[test]
    fn menu_rows_visibility() {
        // A tracked branch with a GitHub remote shows every row.
        let full = BranchInfo {
            upstream: Some("origin/main".into()),
            remote_url: Some("https://github.com/o/r".into()),
            ..base()
        };
        assert_eq!(
            row_ids(&branch_menu(&full)),
            ["copy-branch", "open-repo", "-", "fetch", "refresh"]
        );
        // No remote and no upstream: the divider stays between Copy and Refresh.
        assert_eq!(
            row_ids(&branch_menu(&base())),
            ["copy-branch", "-", "refresh"]
        );
        // A detached head has nothing to copy; the leading divider goes.
        let detached = BranchInfo {
            detached: true,
            ..base()
        };
        assert_eq!(row_ids(&branch_menu(&detached)), ["refresh"]);
        let unnamed = BranchInfo {
            branch: None,
            ..base()
        };
        assert_eq!(row_ids(&branch_menu(&unnamed)), ["refresh"]);
        // The draft menu shows its four entries always.
        assert_eq!(
            row_ids(&draft_menu()),
            ["copy-branch", "-", "fetch", "refresh"]
        );
    }

    #[test]
    fn copy_writes_the_branch_and_refuses_a_missing_name() {
        assert_eq!(copy_text(&base(), false).unwrap(), "feat/board-avatars");
        let unnamed = BranchInfo {
            branch: None,
            ..base()
        };
        assert_eq!(copy_text(&unnamed, false), Err("No branch name to copy."));
        // A detached head still names a branch for the agent pill, but not for the draft pill.
        let detached = BranchInfo {
            detached: true,
            ..base()
        };
        assert!(copy_text(&detached, false).is_ok());
        assert_eq!(copy_text(&detached, true), Err("No branch name to copy."));
    }

    #[test]
    fn repo_name_is_the_last_segment_of_the_remote() {
        assert_eq!(repo_name("https://github.com/o/r"), "r");
        assert_eq!(repo_name("https://github.com/o/bachuc"), "bachuc");
        assert_eq!(repo_name("https://github.com/o/r/"), "repository");
        assert_eq!(repo_name("https://github.com"), "github.com");
        assert_eq!(repo_name("ssh://git@host/o/r"), "r");
    }
}
