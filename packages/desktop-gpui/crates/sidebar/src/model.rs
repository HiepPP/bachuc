//! The sidebar tree without any drawing: status buckets, the flat row list, the archive rule,
//! and the project icon colors. Everything here is a port of the Electron sidebar and has tests.
//!
//! Sources: `utils/sidebar-agent-state.ts`, `protocol/src/agent-state-bucket.ts`,
//! `components/sidebar-workspace-list.tsx`, `git/worktree-archive-warning.ts`, and
//! `styles/identity-colors.ts` under `packages/app/src`.

use bachuc_client::{
    AgentSummary, AttentionReason, ChangedFile, Client, ClientCore, DiffStat, ProjectNode,
    WorkspaceNode,
};
use bachuc_theme::metrics::{SIDEBAR_MAX_WIDTH, SIDEBAR_MIN_WIDTH};
use std::cmp::Ordering;
use std::collections::{HashMap, HashSet};
use std::ops::Range;

/// A group shows this many workspaces before "Show more" (`use-limited-sidebar-group.ts`).
pub const INITIAL_VISIBLE: usize = 20;

/// `SidebarStateBucket`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum StatusBucket {
    /// A permission waits for an answer.
    NeedsInput,
    Failed,
    Running,
    /// Finished and not yet looked at.
    Attention,
    Done,
}

impl StatusBucket {
    /// `STATUS_BUCKET_LABELS`.
    pub fn label(self) -> &'static str {
        match self {
            StatusBucket::NeedsInput => "Needs input",
            StatusBucket::Failed => "Failed",
            StatusBucket::Running => "Working",
            StatusBucket::Attention => "Ready to review",
            StatusBucket::Done => "Done",
        }
    }

    /// Smaller is more urgent: `STATUS_BUCKET_PRIORITY`, which is also the daemon's
    /// `WORKSPACE_STATE_BUCKET_PRIORITY`.
    fn rank(self) -> u8 {
        match self {
            StatusBucket::NeedsInput => 0,
            StatusBucket::Failed => 1,
            StatusBucket::Running => 2,
            StatusBucket::Attention => 3,
            StatusBucket::Done => 4,
        }
    }
}

/// What the tree reads from the client besides the project and agent lists. `Client` and
/// `ClientCore` answer the same way, so a test can read a fake daemon without GPUI.
pub trait TreeSource {
    fn agent_attention(&self, agent_id: &str) -> Option<(bool, Option<AttentionReason>)>;
    fn project_view_key(&self, project_id: &str) -> Option<String>;
    fn workspace_diff_stat(&self, workspace_id: &str) -> Option<DiffStat>;
    fn workspace_pinned_at(&self, workspace_id: &str) -> Option<String>;
}

impl TreeSource for Client {
    fn agent_attention(&self, agent_id: &str) -> Option<(bool, Option<AttentionReason>)> {
        Client::agent_attention(self, agent_id)
    }

    fn project_view_key(&self, project_id: &str) -> Option<String> {
        Client::project_view_key(self, project_id)
    }

    fn workspace_diff_stat(&self, workspace_id: &str) -> Option<DiffStat> {
        Client::workspace_diff_stat(self, workspace_id)
    }

    fn workspace_pinned_at(&self, workspace_id: &str) -> Option<String> {
        Client::workspace_pinned_at(self, workspace_id)
    }
}

impl TreeSource for ClientCore {
    fn agent_attention(&self, agent_id: &str) -> Option<(bool, Option<AttentionReason>)> {
        ClientCore::agent_attention(self, agent_id)
    }

    fn project_view_key(&self, project_id: &str) -> Option<String> {
        ClientCore::project_view_key(self, project_id)
    }

    fn workspace_diff_stat(&self, workspace_id: &str) -> Option<DiffStat> {
        ClientCore::workspace_diff_stat(self, workspace_id)
    }

    fn workspace_pinned_at(&self, workspace_id: &str) -> Option<String> {
        ClientCore::workspace_pinned_at(self, workspace_id)
    }
}

/// The client fields the rows need, read once per refresh. An id without an entry has none.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct Facts {
    /// Agent id to (requires attention, reason).
    pub attention: HashMap<String, (bool, Option<AttentionReason>)>,
    /// Project id to the key its letter color hashes (`projectKey`).
    pub project_keys: HashMap<String, String>,
    /// Workspace id to the (added, deleted) counts of its base-ref diff.
    pub diff_stats: HashMap<String, (u32, u32)>,
    /// Workspace id to the ISO time it was pinned.
    pub pinned_at: HashMap<String, String>,
}

impl Facts {
    pub fn read(
        source: &impl TreeSource,
        projects: &[ProjectNode],
        agents: &[AgentSummary],
    ) -> Facts {
        let mut facts = Facts::default();
        for agent in agents {
            if let Some(attention) = source.agent_attention(&agent.id) {
                facts.attention.insert(agent.id.clone(), attention);
            }
        }
        for project in projects {
            if let Some(key) = source.project_view_key(&project.id) {
                facts.project_keys.insert(project.id.clone(), key);
            }
            for node in &project.workspaces {
                if let Some(stat) = source.workspace_diff_stat(&node.id) {
                    facts
                        .diff_stats
                        .insert(node.id.clone(), (stat.additions, stat.deletions));
                }
                if let Some(at) = source.workspace_pinned_at(&node.id) {
                    facts.pinned_at.insert(node.id.clone(), at);
                }
            }
        }
        facts
    }

    /// The key a project's letter color hashes: the view key, else the project id.
    fn color_key(&self, project_id: &str) -> String {
        self.project_keys
            .get(project_id)
            .cloned()
            .unwrap_or_else(|| project_id.to_string())
    }
}

fn reason_name(reason: AttentionReason) -> &'static str {
    match reason {
        AttentionReason::Finished => "finished",
        AttentionReason::Error => "error",
        AttentionReason::Permission => "permission",
    }
}

/// `deriveAgentStateBucket`.
pub fn bucket_for_agent(
    status: &str,
    pending_permissions: usize,
    requires_attention: bool,
    attention_reason: Option<&str>,
) -> StatusBucket {
    if pending_permissions > 0 || attention_reason == Some("permission") {
        StatusBucket::NeedsInput
    } else if status == "error" || attention_reason == Some("error") {
        StatusBucket::Failed
    } else if status == "running" {
        StatusBucket::Running
    } else if requires_attention {
        StatusBucket::Attention
    } else {
        StatusBucket::Done
    }
}

/// The most urgent bucket, for a workspace over its agents and for a collapsed project over its
/// workspaces (`aggregateSidebarStateBuckets`).
pub fn most_urgent(buckets: impl IntoIterator<Item = StatusBucket>) -> StatusBucket {
    buckets
        .into_iter()
        .min_by_key(|bucket| bucket.rank())
        .unwrap_or(StatusBucket::Done)
}

fn workspace_bucket(
    node: &WorkspaceNode,
    agents: &HashMap<&str, &AgentSummary>,
    facts: &Facts,
) -> StatusBucket {
    most_urgent(
        node.agent_ids
            .iter()
            .filter_map(|id| agents.get(id.as_str()))
            .map(|agent| {
                let (requires_attention, reason) = facts
                    .attention
                    .get(&agent.id)
                    .copied()
                    .unwrap_or((false, None));
                bucket_for_agent(
                    &agent.status,
                    agent.pending_permissions,
                    requires_attention,
                    reason.map(reason_name),
                )
            }),
    )
}

/// A list section whose rows are cut at [`INITIAL_VISIBLE`].
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
pub enum Group {
    Pinned,
    Project(String),
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectRow {
    pub id: String,
    pub name: String,
    pub root: String,
    pub collapsed: bool,
    /// The most urgent bucket of the project's workspaces. The view shows it only while the
    /// project is collapsed, because open rows show their own.
    pub bucket: StatusBucket,
    /// The key the letter color hashes.
    pub color_key: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub struct WorkspaceRow {
    pub id: String,
    pub name: String,
    pub cwd: String,
    pub branch: Option<String>,
    pub pinned: bool,
    pub bucket: StatusBucket,
    /// The (added, deleted) counts at the right of the row. `None` shows nothing; `(0, 0)` shows
    /// `+0 -0`, as Electron does.
    pub diff: Option<(u32, u32)>,
    pub project_id: String,
    pub project_name: String,
    /// The key the project letter color hashes, for the pinned rows that lead with it.
    pub color_key: String,
    pub group: Group,
    /// The last row of a section, which keeps extra space under it.
    pub block_end: bool,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    PinnedHeader {
        collapsed: bool,
        block_end: bool,
    },
    /// The "Workspaces" heading above the project rows.
    SectionTitle,
    Project(ProjectRow),
    Workspace(WorkspaceRow),
    ShowMore {
        group: Group,
        expanded: bool,
        block_end: bool,
    },
    /// No projects at all.
    Empty,
}

/// What decides the height and identity of a row. Two rows with the same shape take the same
/// space, so a change between them needs a redraw but no new measure.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowShape {
    kind: u8,
    block_end: bool,
    key: String,
}

impl Row {
    pub fn shape(&self) -> RowShape {
        let (kind, block_end, key) = match self {
            Row::PinnedHeader { block_end, .. } => (0, *block_end, String::new()),
            Row::SectionTitle => (1, false, String::new()),
            Row::Project(project) => (2, false, project.id.clone()),
            Row::Workspace(row) => (3, row.block_end, row.id.clone()),
            Row::ShowMore {
                group, block_end, ..
            } => (4, *block_end, format!("{group:?}")),
            Row::Empty => (5, false, String::new()),
        };
        RowShape {
            kind,
            block_end,
            key,
        }
    }
}

/// The user-chosen view state of the tree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewState {
    pub collapsed_projects: HashSet<String>,
    pub pinned_collapsed: bool,
    pub expanded_groups: HashSet<Group>,
}

fn workspace_row(
    project: &ProjectNode,
    node: &WorkspaceNode,
    group: Group,
    agents: &HashMap<&str, &AgentSummary>,
    facts: &Facts,
) -> WorkspaceRow {
    WorkspaceRow {
        id: node.id.clone(),
        name: node.name.clone(),
        cwd: node.cwd.clone(),
        branch: node.branch.clone(),
        pinned: node.pinned,
        bucket: workspace_bucket(node, agents, facts),
        diff: facts.diff_stats.get(&node.id).copied(),
        project_id: project.id.clone(),
        project_name: project.name.clone(),
        color_key: facts.color_key(&project.id),
        group,
        block_end: false,
    }
}

/// Pushes the rows of one section, with "Show more" after a cut list, and marks the last row.
fn push_section(rows: &mut Vec<Row>, group: Group, items: Vec<WorkspaceRow>, state: &ViewState) {
    let expanded = state.expanded_groups.contains(&group);
    let limited = items.len() > INITIAL_VISIBLE;
    let shown = if expanded || !limited {
        items.len()
    } else {
        INITIAL_VISIBLE
    };
    rows.extend(items.into_iter().take(shown).map(Row::Workspace));
    if limited {
        rows.push(Row::ShowMore {
            group,
            expanded,
            block_end: false,
        });
    }
    match rows.last_mut() {
        Some(Row::Workspace(row)) => row.block_end = true,
        Some(Row::ShowMore { block_end, .. }) => *block_end = true,
        _ => {}
    }
}

#[derive(Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
enum CharClass {
    Space,
    Punctuation,
    Digit,
    Letter,
}

fn char_class(c: char) -> CharClass {
    if c.is_whitespace() {
        CharClass::Space
    } else if c.is_numeric() {
        CharClass::Digit
    } else if c.is_alphabetic() {
        CharClass::Letter
    } else {
        CharClass::Punctuation
    }
}

/// `localeCompare(right, undefined, { numeric: true, sensitivity: "base" })` for the names the
/// sidebar sorts: case is ignored, digit runs compare by value, and the ICU root order puts
/// spaces, then punctuation, then digits before letters. Accents are not folded.
pub fn natural_compare(left: &str, right: &str) -> Ordering {
    let mut a = left.chars().peekable();
    let mut b = right.chars().peekable();
    loop {
        let (Some(&x), Some(&y)) = (a.peek(), b.peek()) else {
            return a.peek().is_some().cmp(&b.peek().is_some());
        };
        if x.is_ascii_digit() && y.is_ascii_digit() {
            let take = |chars: &mut std::iter::Peekable<std::str::Chars>| {
                let mut run = String::new();
                while let Some(c) = chars.next_if(char::is_ascii_digit) {
                    run.push(c);
                }
                run
            };
            let (x, y) = (take(&mut a), take(&mut b));
            let (x, y) = (x.trim_start_matches('0'), y.trim_start_matches('0'));
            let order = x.len().cmp(&y.len()).then_with(|| x.cmp(y));
            if order != Ordering::Equal {
                return order;
            }
            continue;
        }
        let order = char_class(x)
            .cmp(&char_class(y))
            .then_with(|| x.to_lowercase().cmp(y.to_lowercase()));
        if order != Ordering::Equal {
            return order;
        }
        a.next();
        b.next();
    }
}

/// `buildWorkspaceStructureProjects` order: projects by name, workspaces by name, each with the
/// id as the tie-break. A user drag order (`applyStoredOrdering`) is not ported.
fn sorted(projects: &[ProjectNode]) -> Vec<ProjectNode> {
    let mut projects = projects.to_vec();
    projects.sort_by(|left, right| {
        natural_compare(&left.name, &right.name).then_with(|| left.id.cmp(&right.id))
    });
    for project in &mut projects {
        project.workspaces.sort_by(|left, right| {
            natural_compare(&left.name, &right.name)
                .then_with(|| left.id.to_lowercase().cmp(&right.id.to_lowercase()))
        });
    }
    projects
}

/// The rows of the tree, top to bottom: the pinned section (when any workspace is pinned), then
/// "Workspaces" with each project and its unpinned workspaces, in the Electron sort order. The
/// pinned rows go by `pinnedAt`, newest first, and keep the tree order when the times are equal
/// or missing.
pub fn build_rows(
    projects: &[ProjectNode],
    agents: &[AgentSummary],
    facts: &Facts,
    state: &ViewState,
) -> Vec<Row> {
    if projects.is_empty() {
        return vec![Row::Empty];
    }
    let projects = sorted(projects);
    let by_id: HashMap<&str, &AgentSummary> = agents.iter().map(|a| (a.id.as_str(), a)).collect();
    let mut rows = Vec::new();

    let mut pinned: Vec<WorkspaceRow> = projects
        .iter()
        .flat_map(|project| {
            let by_id = &by_id;
            project
                .workspaces
                .iter()
                .filter(|node| node.pinned)
                .map(move |node| workspace_row(project, node, Group::Pinned, by_id, facts))
        })
        .collect();
    // `sortPinnedChats`: a missing time compares as "", and the sort is stable.
    let pinned_at = |row: &WorkspaceRow| {
        facts
            .pinned_at
            .get(&row.id)
            .map(String::as_str)
            .unwrap_or_default()
    };
    pinned.sort_by(|left, right| pinned_at(right).cmp(pinned_at(left)));
    if !pinned.is_empty() {
        rows.push(Row::PinnedHeader {
            collapsed: state.pinned_collapsed,
            block_end: state.pinned_collapsed,
        });
        if !state.pinned_collapsed {
            push_section(&mut rows, Group::Pinned, pinned, state);
        }
    }

    rows.push(Row::SectionTitle);
    for project in &projects {
        let items: Vec<WorkspaceRow> = project
            .workspaces
            .iter()
            .filter(|node| !node.pinned)
            .map(|node| {
                workspace_row(
                    project,
                    node,
                    Group::Project(project.id.clone()),
                    &by_id,
                    facts,
                )
            })
            .collect();
        let collapsed = state.collapsed_projects.contains(&project.id);
        rows.push(Row::Project(ProjectRow {
            id: project.id.clone(),
            name: project.name.clone(),
            root: project.root.clone(),
            collapsed,
            bucket: most_urgent(
                project
                    .workspaces
                    .iter()
                    .map(|node| workspace_bucket(node, &by_id, facts)),
            ),
            color_key: facts.color_key(&project.id),
        }));
        if !collapsed && !items.is_empty() {
            push_section(&mut rows, Group::Project(project.id.clone()), items, state);
        }
    }
    rows
}

/// The part of a list that changed between two frames: the range of old rows that went away and
/// how many new rows replace it. `None` when the shapes are equal, so the list keeps its
/// measured heights and its scroll position.
pub fn changed_range(old: &[RowShape], new: &[RowShape]) -> Option<(Range<usize>, usize)> {
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

/// The confirm shown before an archive.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ArchivePrompt {
    pub title: String,
    pub message: String,
}

fn lines(count: u32, one: &str, many: &str) -> String {
    if count == 1 {
        format!("{count} {one}")
    } else {
        format!("{count} {many}")
    }
}

/// `buildWorktreeArchiveRiskReasons` for the uncommitted case. A workspace is risky when its
/// checkout lists changed files. Unpushed commits are not checked, because the client does not
/// expose how far a branch is ahead of its remote. A clean workspace gives `None` and archives
/// at once.
pub fn archive_prompt(workspace_name: &str, files: &[ChangedFile]) -> Option<ArchivePrompt> {
    if files.is_empty() {
        return None;
    }
    let added: u32 = files.iter().map(|file| file.added).sum();
    let removed: u32 = files.iter().map(|file| file.removed).sum();
    Some(archive_confirm(workspace_name, added, removed))
}

/// The same confirm from the workspace `diffStat`, which needs no request: Electron treats a
/// workspace as risky when the daemon gives no dirty flag and the stat has a changed line.
/// `None` and `0/0` archive at once.
pub fn archive_prompt_for_stat(
    workspace_name: &str,
    stat: Option<(u32, u32)>,
) -> Option<ArchivePrompt> {
    let (added, removed) = stat.filter(|(added, removed)| added + removed > 0)?;
    Some(archive_confirm(workspace_name, added, removed))
}

fn archive_confirm(workspace_name: &str, added: u32, removed: u32) -> ArchivePrompt {
    let mut parts = Vec::new();
    if added > 0 {
        parts.push(lines(added, "added line", "added lines"));
    }
    if removed > 0 {
        parts.push(lines(removed, "deleted line", "deleted lines"));
    }
    let message = if parts.is_empty() {
        "Uncommitted changes".to_string()
    } else {
        format!("Uncommitted changes ({})", parts.join(", "))
    };
    ArchivePrompt {
        title: format!("Archive \"{workspace_name}\"?"),
        message,
    }
}

/// `formatDiffCount`: `Intl.NumberFormat("en-US", { notation: "compact",
/// maximumFractionDigits: 1 })` in lower case, so 1234 is "1.2k".
pub fn format_diff_count(value: u32) -> String {
    let mut text = value.to_string();
    // 999,950 rounds to 1000k, which Intl writes in the next unit as 1m.
    let mut carried = false;
    for (size, suffix) in [(1e3, "k"), (1e6, "m"), (1e9, "b")] {
        let value = f64::from(value);
        if value < size && !carried {
            break;
        }
        let scaled = (value / size * 10.).round() / 10.;
        carried = scaled >= 1000.;
        text = format!("{scaled}{suffix}");
    }
    text
}

/// The sidebar width a drag asks for, kept between the Electron limits.
pub fn clamp_width(requested: f32) -> f32 {
    requested.clamp(SIDEBAR_MIN_WIDTH, SIDEBAR_MAX_WIDTH)
}

/// `IDENTITY_COLOR_NAMES` order and `IDENTITY_COLORS` values: violet, sky, emerald, orange,
/// pink, indigo, teal, red, amber, blue. The order picks the color, so it must not change.
pub const IDENTITY_COLORS: [u32; 10] = [
    0x7a6aa8, 0x3d7ea6, 0x388068, 0xa4673a, 0xb05c80, 0x6a70b8, 0x368080, 0xb06260, 0x8f7838,
    0x5179b0,
];

/// `deriveIdentityColorName`: a hash of the key (31 times the hash plus each UTF-16 unit, kept
/// in 32 bits) picks one of the ten colors. The key is the project view key, see
/// [`Facts::project_keys`].
pub fn identity_color(key: &str) -> u32 {
    let mut hash: u32 = 0;
    for character in key.chars() {
        let mut unit = [0u16; 2];
        let first = character.encode_utf16(&mut unit)[0];
        hash = hash.wrapping_mul(31).wrapping_add(u32::from(first));
    }
    IDENTITY_COLORS[hash as usize % IDENTITY_COLORS.len()]
}

/// `projectIconPlaceholderLabelFromDisplayName` and the first letter in capitals.
pub fn project_initial(display_name: &str) -> String {
    let trimmed = display_name.trim();
    let label = trimmed
        .split('/')
        .rfind(|segment| !segment.is_empty())
        .unwrap_or(trimmed);
    label
        .chars()
        .next()
        .map(|c| c.to_uppercase().collect())
        .unwrap_or_default()
}

/// The accessibility label of a workspace row: the workspace name, then its status unless it is
/// done (`resolveSidebarWorkspaceAccessibilityLabel`). Pinned rows lead with the project name.
pub fn workspace_label(row: &WorkspaceRow, leading_project: bool) -> String {
    let mut parts = Vec::new();
    if leading_project {
        parts.push(row.project_name.as_str());
    }
    parts.push(row.name.as_str());
    if row.bucket != StatusBucket::Done {
        parts.push(row.bucket.label());
    }
    parts.join(", ")
}
