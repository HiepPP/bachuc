//! The sidebar tree without any drawing: status buckets, the flat row list, the archive rule,
//! and the project icon colors. Everything here is a port of the Electron sidebar and has tests.
//!
//! Sources: `utils/sidebar-agent-state.ts`, `protocol/src/agent-state-bucket.ts`,
//! `components/sidebar-workspace-list.tsx`, `git/worktree-archive-warning.ts`, and
//! `styles/identity-colors.ts` under `packages/app/src`.

use crate::prefs::{ChecksDisplay, RowItems, TitleSource, Trailing};
use bachuc_client::{
    AgentSummary, AttentionReason, ChangedFile, CheckSummary, Client, ClientCore, DiffStat,
    ProjectNode, PullRequestState, WorkspaceLabel, WorkspaceNode, WorkspaceScript,
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

    /// The order of the status groups (`STATUS_BUCKET_ORDER`): what needs a person first, then
    /// what runs on its own. It differs from [`StatusBucket::rank`], which puts running above
    /// ready to review.
    pub const GROUP_ORDER: [StatusBucket; 5] = [
        StatusBucket::NeedsInput,
        StatusBucket::Failed,
        StatusBucket::Attention,
        StatusBucket::Running,
        StatusBucket::Done,
    ];

    /// The stored name of a status group.
    pub fn key(self) -> &'static str {
        match self {
            StatusBucket::NeedsInput => "needs_input",
            StatusBucket::Failed => "failed",
            StatusBucket::Running => "running",
            StatusBucket::Attention => "attention",
            StatusBucket::Done => "done",
        }
    }

    pub fn from_key(key: &str) -> Option<StatusBucket> {
        Self::GROUP_ORDER
            .into_iter()
            .find(|bucket| bucket.key() == key)
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
    fn workspace_scripts(&self, workspace_id: &str) -> Vec<WorkspaceScript>;
    /// The daemon's server id, which prefixes every thread key. `None` before the first answer.
    fn server_id(&self) -> Option<String>;
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

    fn workspace_scripts(&self, workspace_id: &str) -> Vec<WorkspaceScript> {
        Client::workspace_scripts(self, workspace_id)
    }

    fn server_id(&self) -> Option<String> {
        self.server_info().map(|info| info.server_id.clone())
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

    fn workspace_scripts(&self, workspace_id: &str) -> Vec<WorkspaceScript> {
        ClientCore::workspace_scripts(self, workspace_id)
    }

    fn server_id(&self) -> Option<String> {
        let id = self.server_info()?["serverId"].as_str()?;
        Some(id.to_string())
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
    /// Workspace id to the service its row reports.
    pub services: HashMap<String, ServiceSummary>,
    /// The daemon's server id. Without it a thread key cannot be built, so threads keep the name
    /// order and are not seeded.
    pub server_id: Option<String>,
}

/// The one running service a row reports, by name (`WorkspaceServiceSummary`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ServiceSummary {
    pub name: String,
    /// `healthy` or `unhealthy`; `None` while unknown.
    pub health: Option<String>,
}

impl ServiceSummary {
    pub fn unhealthy(&self) -> bool {
        self.health.as_deref() == Some("unhealthy")
    }
}

/// `selectWorkspaceServiceSummary`: only running services count, not plain scripts. An unhealthy
/// service wins, because it is the one a person acts on; otherwise the first running one.
pub fn select_service_summary(scripts: &[WorkspaceScript]) -> Option<ServiceSummary> {
    let mut first = None;
    for script in scripts {
        if script.lifecycle != "running" || script.kind != "service" {
            continue;
        }
        let summary = ServiceSummary {
            name: script.name.clone(),
            health: script.health.clone(),
        };
        if summary.unhealthy() {
            return Some(summary);
        }
        first.get_or_insert(summary);
    }
    first
}

impl Facts {
    pub fn read(
        source: &impl TreeSource,
        projects: &[ProjectNode],
        agents: &[AgentSummary],
    ) -> Facts {
        let mut facts = Facts {
            server_id: source.server_id(),
            ..Facts::default()
        };
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
                if let Some(service) = select_service_summary(&source.workspace_scripts(&node.id)) {
                    facts.services.insert(node.id.clone(), service);
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
    /// A status heading in status grouping.
    Status(StatusBucket),
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

/// The part of a workspace pull request that a row draws.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct RowPullRequest {
    pub number: u64,
    pub url: String,
    pub state: PullRequestState,
    pub forge: String,
    pub checks: Option<CheckSummary>,
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
    /// When the workspace entered its status, in ms since the epoch. Status groups sort by it,
    /// and the trailing slot can show it.
    pub status_entered_at: Option<i64>,
    /// The pull request the meta row draws, copied when the rows build.
    pub pull_request: Option<RowPullRequest>,
    pub labels: Vec<WorkspaceLabel>,
    /// The running service the meta row draws.
    pub service: Option<ServiceSummary>,
    pub group: Group,
    /// The last row of a section, which keeps extra space under it.
    pub block_end: bool,
}

// A workspace row is most rows, so boxing it would add an allocation to the common case.
#[allow(clippy::large_enum_variant)]
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Row {
    PinnedHeader {
        collapsed: bool,
        block_end: bool,
    },
    /// The "Workspaces" heading above the project rows.
    SectionTitle,
    Project(ProjectRow),
    /// The heading of one status group in status grouping.
    StatusHeader {
        bucket: StatusBucket,
        collapsed: bool,
        block_end: bool,
    },
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
    /// A workspace row that draws a meta line is taller than one that does not.
    meta_line: bool,
    key: String,
}

impl Row {
    /// The shape of a row with no meta line.
    pub fn shape(&self) -> RowShape {
        self.shape_with(&RowItems::none(), ChecksDisplay::Hidden)
    }

    /// The shape under the Show choices: a workspace row with a meta line is taller.
    pub fn shape_with(&self, items: &RowItems, checks: ChecksDisplay) -> RowShape {
        let meta_line = matches!(self, Row::Workspace(row) if has_meta_line(row, items, checks));
        let (kind, block_end, key) = match self {
            Row::PinnedHeader { block_end, .. } => (0, *block_end, String::new()),
            Row::SectionTitle => (1, false, String::new()),
            Row::Project(project) => (2, false, project.id.clone()),
            Row::Workspace(row) => (3, row.block_end, row.id.clone()),
            Row::ShowMore {
                group, block_end, ..
            } => (4, *block_end, format!("{group:?}")),
            Row::Empty => (5, false, String::new()),
            Row::StatusHeader {
                bucket, block_end, ..
            } => (6, *block_end, bucket.key().to_string()),
        };
        RowShape {
            kind,
            block_end,
            meta_line,
            key,
        }
    }
}

/// How the unpinned workspaces group.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Grouping {
    /// One heading per project (`groupMode` "project").
    #[default]
    Project,
    /// One heading per status bucket (`groupMode` "status").
    Status,
}

/// The user-chosen view state of the tree.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ViewState {
    pub collapsed_projects: HashSet<String>,
    pub pinned_collapsed: bool,
    pub expanded_groups: HashSet<Group>,
    pub grouping: Grouping,
    /// The collapsed status headings of status grouping.
    pub collapsed_statuses: HashSet<StatusBucket>,
    /// Project ids to show. Empty shows every project.
    pub project_filter: HashSet<String>,
    /// [`label_key`]s to show; the empty key stands for "no label". Empty shows every workspace.
    pub label_filter: HashSet<String>,
    /// The saved order of projects, threads, and pinned rows.
    pub order: SidebarOrder,
}

/// The saved sidebar order, with Electron's keys (`sidebar-order-store.ts`): a project is its view
/// key, a thread is [`workspace_key`]. Keys of things that went are kept, so a thread that comes
/// back keeps its place.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct SidebarOrder {
    pub project_order: Vec<String>,
    /// Project view key to the thread keys of that project.
    pub workspace_order_by_project: HashMap<String, Vec<String>>,
    pub pinned_workspace_order: Vec<String>,
}

impl SidebarOrder {
    pub fn is_empty(&self) -> bool {
        self.project_order.is_empty()
            && self.workspace_order_by_project.is_empty()
            && self.pinned_workspace_order.is_empty()
    }
}

/// The key of a thread in the saved order: `<serverId>:<workspaceId>`.
pub fn workspace_key(server_id: &str, workspace_id: &str) -> String {
    format!("{server_id}:{workspace_id}")
}

/// `applyStoredOrdering`: an item that is not in the saved order keeps its slot, and the saved
/// items fill their own slots in saved order. `key` is `None` for an item that has no key, which
/// is never saved. Saved keys that match no item are skipped. The keys are borrowed, so a large
/// list allocates no string per item.
fn apply_stored_ordering<T>(
    items: Vec<T>,
    stored: &[&str],
    key: impl Fn(&T) -> Option<&str>,
) -> Vec<T> {
    if items.len() <= 1 || stored.is_empty() {
        return items;
    }
    // A sorted vector beats a hash map here: most lists have a handful of items, and a debug
    // build hashes slowly.
    let mut keyed: Vec<(&str, usize)> = items
        .iter()
        .enumerate()
        .filter_map(|(at, item)| key(item).map(|key| (key, at)))
        .collect();
    keyed.sort_unstable();
    let mut ordered = vec![false; items.len()];
    let mut pruned: Vec<usize> = Vec::new();
    for &saved in stored {
        // The last item with the key wins, as in the Electron map.
        let end = keyed.partition_point(|(key, _)| *key <= saved);
        if let Some(&(key, at)) = end.checked_sub(1).and_then(|last| keyed.get(last))
            && key == saved
            && !ordered[at]
        {
            ordered[at] = true;
            pruned.push(at);
        }
    }
    drop(keyed);
    if pruned.is_empty() {
        return items;
    }
    let mut next = pruned.iter();
    let permutation: Vec<usize> = (0..items.len())
        .map(|at| {
            if ordered[at] {
                next.next().copied().unwrap_or(at)
            } else {
                at
            }
        })
        .collect();
    let mut slots: Vec<Option<T>> = items.into_iter().map(Some).collect();
    permutation
        .into_iter()
        .filter_map(|at| slots[at].take())
        .collect()
}

/// The saved keys as plain strings, for [`apply_stored_ordering`].
fn as_strs(keys: &[String]) -> Vec<&str> {
    keys.iter().map(String::as_str).collect()
}

/// The workspace ids of the saved thread keys of one server: `<serverId>:<id>` gives `<id>`. A
/// key of another server matches no thread.
fn workspace_ids<'a>(keys: &'a [String], server_id: &str) -> Vec<&'a str> {
    keys.iter()
        .filter_map(|key| key.strip_prefix(server_id)?.strip_prefix(':'))
        .collect()
}

/// A drop (`handleProjectDragEnd`, `handleWorkspaceReorder`): `dragged` leaves the visible keys
/// and goes in at the index that `target` had, so a drag down lands after the target and a drag
/// up lands before it. An unknown key, or a drop on the dragged key itself, changes nothing.
pub fn reorder_visible(visible: &[String], dragged: &str, target: &str) -> Vec<String> {
    let position = |key: &str| visible.iter().position(|item| item == key);
    let (Some(from), Some(to)) = (position(dragged), position(target)) else {
        return visible.to_vec();
    };
    let mut reordered = visible.to_vec();
    let key = reordered.remove(from);
    reordered.insert(to, key);
    reordered
}

/// `mergeWithRemainder`: the visible keys in their new order, then the saved keys that were not
/// visible (hidden by a filter, "Show more", a collapse, or a pin).
pub fn merge_with_remainder(current: &[String], reordered_visible: &[String]) -> Vec<String> {
    let visible: HashSet<&str> = reordered_visible.iter().map(String::as_str).collect();
    let mut merged = reordered_visible.to_vec();
    merged.extend(
        current
            .iter()
            .filter(|key| !visible.contains(key.as_str()))
            .cloned(),
    );
    merged
}

/// `hasVisibleOrderChanged`: whether the visible keys differ from the saved order of the same
/// keys. A visible key the saved order lacks counts as a change.
pub fn has_visible_order_changed(current: &[String], reordered_visible: &[String]) -> bool {
    let visible: HashSet<&str> = reordered_visible.iter().map(String::as_str).collect();
    let current_visible: Vec<&str> = current
        .iter()
        .map(String::as_str)
        .filter(|key| visible.contains(key))
        .collect();
    current_visible.len() != reordered_visible.len()
        || reordered_visible
            .iter()
            .zip(&current_visible)
            .any(|(reordered, current)| reordered != current)
}

/// One of the three saved lists a row drag can reorder, as in Electron: the projects, the
/// unpinned threads of one project (by its view key), or the pinned rows.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OrderList {
    Projects,
    Threads(String),
    Pinned,
}

/// The keys of a list as the rows show them now, top to bottom. A project row without a view key
/// has none. Thread and pinned keys need the server id.
pub fn visible_keys(
    rows: &[Row],
    list: &OrderList,
    project_keys: &HashMap<String, String>,
    server_id: Option<&str>,
) -> Vec<String> {
    let thread_key = |row: &WorkspaceRow| server_id.map(|server| workspace_key(server, &row.id));
    rows.iter()
        .filter_map(|row| match (list, row) {
            (OrderList::Projects, Row::Project(project)) => project_keys.get(&project.id).cloned(),
            (OrderList::Threads(view_key), Row::Workspace(row)) => match &row.group {
                Group::Project(id) if project_keys.get(id) == Some(view_key) => thread_key(row),
                _ => None,
            },
            (OrderList::Pinned, Row::Workspace(row)) if row.group == Group::Pinned => {
                thread_key(row)
            }
            _ => None,
        })
        .collect()
}

/// Saves a drop in `order` with Electron's rules: the visible keys are reordered, and the saved
/// list changes only when the visible order did. Hidden keys stay behind the visible ones.
/// Returns whether the saved order changed, so a no-op drop writes nothing.
pub fn apply_drop(
    order: &mut SidebarOrder,
    list: &OrderList,
    visible: &[String],
    dragged: &str,
    target: &str,
) -> bool {
    if dragged == target {
        return false;
    }
    let reordered = reorder_visible(visible, dragged, target);
    let none = Vec::new();
    let current = match list {
        OrderList::Projects => &order.project_order,
        OrderList::Threads(view_key) => order
            .workspace_order_by_project
            .get(view_key)
            .unwrap_or(&none),
        OrderList::Pinned => &order.pinned_workspace_order,
    };
    if !has_visible_order_changed(current, &reordered) {
        return false;
    }
    let merged = merge_with_remainder(current, &reordered);
    match list {
        OrderList::Projects => order.project_order = merged,
        OrderList::Threads(view_key) => {
            order
                .workspace_order_by_project
                .insert(view_key.clone(), merged);
        }
        OrderList::Pinned => order.pinned_workspace_order = merged,
    }
    true
}

/// `computeSidebarOrderUpdates`, applied in place: a project key the order lacks goes last, and a
/// thread key it lacks goes first, in name order. Nothing is pruned. A project without a view
/// key is skipped, and without a server id no thread is seeded. Returns whether the order
/// changed, so a refresh with nothing new writes nothing.
pub fn seed_order(order: &mut SidebarOrder, projects: &[ProjectNode], facts: &Facts) -> bool {
    let mut changed = false;
    let mut known: HashSet<String> = order.project_order.iter().cloned().collect();
    for entry in name_sorted(projects) {
        let Some(view_key) = facts.project_keys.get(&entry.project.id) else {
            continue;
        };
        if known.insert(view_key.clone()) {
            order.project_order.push(view_key.clone());
            changed = true;
        }
        let Some(server_id) = &facts.server_id else {
            continue;
        };
        let saved = order
            .workspace_order_by_project
            .get(view_key)
            .map_or(&[][..], Vec::as_slice);
        let present: HashSet<&str> = saved.iter().map(String::as_str).collect();
        let missing: Vec<String> = entry
            .workspaces
            .iter()
            .map(|node| workspace_key(server_id, &node.id))
            .filter(|key| !present.contains(key.as_str()))
            .collect();
        if !missing.is_empty() {
            let saved = order
                .workspace_order_by_project
                .entry(view_key.clone())
                .or_default();
            saved.splice(0..0, missing);
            changed = true;
        }
    }
    changed
}

// ---- row meta items (TASK-262) ----

/// One item of the line under a workspace title, in the order it is read: where the workspace
/// lives, what change it belongs to, whether that change passes, what it runs, and what someone
/// filed it under (`selectMetaRowItems`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum MetaItem<'a> {
    Branch(&'a str),
    Project(&'a str),
    PullRequest(&'a RowPullRequest),
    Checks {
        summary: CheckSummary,
        /// Icon and text, not the icon alone.
        label: bool,
    },
    Services(&'a ServiceSummary),
    Labels(&'a [WorkspaceLabel]),
}

/// The items a row draws. Each shows only when its toggle is on and its data exists. The project
/// name shows only on a row that leads with the project icon (`leading_project`): under a
/// project heading it would repeat the heading. Checks follow the Checks choice alone, so they
/// show without the pull request beside them, as in Electron.
pub fn meta_items<'a>(
    row: &'a WorkspaceRow,
    items: &RowItems,
    checks: ChecksDisplay,
    leading_project: bool,
) -> Vec<MetaItem<'a>> {
    let mut shown = Vec::new();
    if let Some(branch) = row.branch.as_deref().filter(|branch| !branch.is_empty())
        && items.branch
    {
        shown.push(MetaItem::Branch(branch));
    }
    if leading_project && items.project && !row.project_name.is_empty() {
        shown.push(MetaItem::Project(&row.project_name));
    }
    if let Some(pr) = &row.pull_request
        && items.pull_request
    {
        shown.push(MetaItem::PullRequest(pr));
    }
    if checks != ChecksDisplay::Hidden
        && let Some(summary) = row.pull_request.as_ref().and_then(|pr| pr.checks)
    {
        shown.push(MetaItem::Checks {
            summary,
            label: checks == ChecksDisplay::IconAndText,
        });
    }
    if let Some(service) = &row.service
        && items.services
    {
        shown.push(MetaItem::Services(service));
    }
    if items.labels && !row.labels.is_empty() {
        shown.push(MetaItem::Labels(&row.labels));
    }
    shown
}

/// Whether the row draws a meta line, which makes it taller.
pub fn has_meta_line(row: &WorkspaceRow, items: &RowItems, checks: ChecksDisplay) -> bool {
    !meta_items(row, items, checks, !matches!(row.group, Group::Project(_))).is_empty()
}

/// What the slot right of the title holds: the diff counts or the last activity, never both.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TrailingSlot {
    Diff {
        added: u32,
        removed: u32,
    },
    /// ms since the epoch at which the workspace entered its status.
    LastActivity(i64),
}

pub fn trailing_slot(row: &WorkspaceRow, trailing: Option<Trailing>) -> Option<TrailingSlot> {
    match trailing? {
        Trailing::DiffStats => row
            .diff
            .map(|(added, removed)| TrailingSlot::Diff { added, removed }),
        Trailing::LastActivity => row.status_entered_at.map(TrailingSlot::LastActivity),
    }
}

/// `resolveSidebarWorkspacePrimaryLabel`: the branch name falls back to the workspace name.
pub fn row_title(row: &WorkspaceRow, source: TitleSource) -> &str {
    match source {
        TitleSource::Branch => row
            .branch
            .as_deref()
            .filter(|branch| !branch.is_empty())
            .unwrap_or(&row.name),
        TitleSource::Title => &row.name,
    }
}

/// `describeCompactTimeAgo`: "now", "5m", "3h", "2d", and from a week on the date, "Oct 5" (UTC).
pub fn compact_time_ago(then_ms: i64, now_ms: i64) -> String {
    const MINUTE: i64 = 60_000;
    const HOUR: i64 = 60 * MINUTE;
    const DAY: i64 = 24 * HOUR;
    let elapsed = now_ms - then_ms;
    if elapsed < MINUTE {
        "now".to_string()
    } else if elapsed < HOUR {
        format!("{}m", elapsed / MINUTE)
    } else if elapsed < DAY {
        format!("{}h", elapsed / HOUR)
    } else if elapsed < 7 * DAY {
        format!("{}d", elapsed / DAY)
    } else {
        let (month, day) = month_and_day(then_ms.div_euclid(DAY));
        format!("{month} {day}")
    }
}

/// The month name and day of the month of a day count since 1970-01-01 (proleptic Gregorian).
fn month_and_day(days: i64) -> (&'static str, i64) {
    const MONTHS: [&str; 12] = [
        "Jan", "Feb", "Mar", "Apr", "May", "Jun", "Jul", "Aug", "Sep", "Oct", "Nov", "Dec",
    ];
    let z = days + 719_468;
    let day_of_era = z.rem_euclid(146_097);
    let year_of_era =
        (day_of_era - day_of_era / 1_460 + day_of_era / 36_524 - day_of_era / 146_096) / 365;
    let day_of_year = day_of_era - (365 * year_of_era + year_of_era / 4 - year_of_era / 100);
    let shifted_month = (5 * day_of_year + 2) / 153;
    let day = day_of_year - (153 * shifted_month + 2) / 5 + 1;
    let month = if shifted_month < 10 {
        shifted_month + 3
    } else {
        shifted_month - 9
    };
    (MONTHS[(month - 1) as usize], day)
}

/// The identity color of a label color name (`violet` ... `blue`, the order of
/// [`IDENTITY_COLORS`]); `None` for a name this client does not know.
pub fn label_color(name: &str) -> Option<u32> {
    const NAMES: [&str; 10] = [
        "violet", "sky", "emerald", "orange", "pink", "indigo", "teal", "red", "amber", "blue",
    ];
    NAMES
        .iter()
        .position(|candidate| *candidate == name)
        .map(|at| IDENTITY_COLORS[at])
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
        status_entered_at: node.status_entered_at,
        pull_request: node.pull_request.as_ref().map(|pr| RowPullRequest {
            number: pr.number,
            url: pr.url.clone(),
            state: pr.state,
            forge: pr.forge.clone(),
            checks: pr.checks,
        }),
        labels: node.labels.clone(),
        service: facts.services.get(&node.id).cloned(),
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
    // Equal text is the common tie in a sort (the same project name on every row of a group).
    if left == right {
        return Ordering::Equal;
    }
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

/// A project with its workspaces in order. It borrows from the client list, so sorting copies
/// no workspace.
struct SortedProject<'a> {
    project: &'a ProjectNode,
    workspaces: Vec<&'a WorkspaceNode>,
}

/// `buildWorkspaceStructureProjects` order: projects by name, workspaces by name, each with the
/// id as the tie-break.
fn name_sorted(projects: &[ProjectNode]) -> Vec<SortedProject<'_>> {
    let mut sorted: Vec<SortedProject> = projects
        .iter()
        .map(|project| {
            let mut workspaces: Vec<&WorkspaceNode> = project.workspaces.iter().collect();
            workspaces.sort_by(|left, right| {
                natural_compare(&left.name, &right.name)
                    .then_with(|| left.id.to_lowercase().cmp(&right.id.to_lowercase()))
            });
            SortedProject {
                project,
                workspaces,
            }
        })
        .collect();
    sorted.sort_by(|left, right| {
        natural_compare(&left.project.name, &right.project.name)
            .then_with(|| left.project.id.cmp(&right.project.id))
    });
    sorted
}

/// [`name_sorted`], then the saved order on top (`composeWorkspaceStructure`): each project's
/// threads by `workspace_order_by_project[view key]`, then the projects by `project_order`. A
/// project without a view key keeps its name slot, and without a server id the threads keep
/// theirs.
fn sorted<'a>(
    projects: &'a [ProjectNode],
    facts: &'a Facts,
    order: &SidebarOrder,
) -> Vec<SortedProject<'a>> {
    let mut sorted = name_sorted(projects);
    if order.is_empty() {
        return sorted;
    }
    let view_key = |entry: &SortedProject| facts.project_keys.get(&entry.project.id);
    if let Some(server_id) = &facts.server_id {
        for entry in &mut sorted {
            let Some(stored) =
                view_key(entry).and_then(|key| order.workspace_order_by_project.get(key))
            else {
                continue;
            };
            let workspaces = std::mem::take(&mut entry.workspaces);
            entry.workspaces =
                apply_stored_ordering(workspaces, &workspace_ids(stored, server_id), |node| {
                    Some(node.id.as_str())
                });
        }
    }
    apply_stored_ordering(sorted, &as_strs(&order.project_order), |entry| {
        view_key(entry).map(String::as_str)
    })
}

/// `workspaceLabelKey`: whitespace collapsed, trimmed, lower case.
pub fn label_key(name: &str) -> String {
    name.split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
        .to_lowercase()
}

/// `filterWorkspacesByLabels` for one workspace: it matches when it carries any selected label,
/// and the empty key selects a workspace with no label at all.
pub fn label_filter_matches(filter: &HashSet<String>, labels: &[WorkspaceLabel]) -> bool {
    if filter.is_empty() {
        return true;
    }
    let keys: Vec<String> = labels
        .iter()
        .map(|label| label_key(&label.name))
        .filter(|key| !key.is_empty())
        .collect();
    filter.iter().any(|selected| {
        if selected.is_empty() {
            keys.is_empty()
        } else {
            keys.contains(selected)
        }
    })
}

/// One row of the Label filter page.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct LabelOption {
    pub key: String,
    pub name: String,
    pub color: String,
}

/// The labels the page can filter by: each distinct [`label_key`] on a workspace, by name. The
/// client gives a workspace only labels of the host catalog, so a label no workspace carries has
/// no row.
pub fn label_options(projects: &[ProjectNode]) -> Vec<LabelOption> {
    let mut options: Vec<LabelOption> = Vec::new();
    for label in projects
        .iter()
        .flat_map(|project| &project.workspaces)
        .flat_map(|node| &node.labels)
    {
        let key = label_key(&label.name);
        if !key.is_empty() && !options.iter().any(|option| option.key == key) {
            options.push(LabelOption {
                key,
                name: label.name.clone(),
                color: label.color.clone(),
            });
        }
    }
    options.sort_by(|left, right| natural_compare(&left.name, &right.name));
    options
}

/// Electron shows the Project filter only when there is more than one project to choose from
/// (`menu.tsx` line 189).
pub fn project_filter_visible(project_count: usize) -> bool {
    project_count > 1
}

/// Electron shows the Label filter when labels exist, or when a label filter is on so that it
/// can be undone (`menu.tsx` line 193).
pub fn label_filter_visible(has_labels: bool, filter_active: bool) -> bool {
    has_labels || filter_active
}

/// The value at the right of the Project row: "All projects", the one chosen name, or a count.
/// Chosen ids that no project has any more count as no choice.
pub fn project_filter_value(selected: &HashSet<String>, projects: &[(String, String)]) -> String {
    let chosen: Vec<&str> = projects
        .iter()
        .filter(|(id, _)| selected.contains(id))
        .map(|(_, name)| name.as_str())
        .collect();
    match chosen.as_slice() {
        [] => "All projects".to_string(),
        [name] => (*name).to_string(),
        many => format!("{} projects", many.len()),
    }
}

/// The project filter that applies: the selected ids that still exist. A filter whose projects
/// all went filters nothing (`resolveActiveProjectFilters`).
fn applied_project_filter<'a>(
    projects: &[SortedProject],
    filter: &'a HashSet<String>,
) -> Option<&'a HashSet<String>> {
    (!filter.is_empty()
        && projects
            .iter()
            .any(|entry| filter.contains(&entry.project.id)))
    .then_some(filter)
}

/// The project filter keeps a chosen project even without workspaces, because it still owns a
/// header to create the first workspace under. The label filter can only ask about workspaces,
/// so a project it empties goes (`sidebar-model.tsx`).
fn filter_projects<'a>(
    mut projects: Vec<SortedProject<'a>>,
    state: &ViewState,
) -> Vec<SortedProject<'a>> {
    if let Some(filter) = applied_project_filter(&projects, &state.project_filter) {
        projects.retain(|entry| filter.contains(&entry.project.id));
    }
    if !state.label_filter.is_empty() {
        for entry in &mut projects {
            entry
                .workspaces
                .retain(|node| label_filter_matches(&state.label_filter, &node.labels));
        }
        projects.retain(|entry| !entry.workspaces.is_empty());
    }
    projects
}

/// `compareStatusRows`: the workspace that entered its status last comes first and one without a
/// time comes last; then project name, workspace name, and id.
fn compare_status_rows(left: &WorkspaceRow, right: &WorkspaceRow) -> Ordering {
    match (left.status_entered_at, right.status_entered_at) {
        (Some(a), Some(b)) if a != b => return b.cmp(&a),
        (Some(_), None) => return Ordering::Less,
        (None, Some(_)) => return Ordering::Greater,
        _ => {}
    }
    natural_compare(&left.project_name, &right.project_name)
        .then_with(|| natural_compare(&left.name, &right.name))
        .then_with(|| natural_compare(&left.id, &right.id))
}

/// `buildStatusGroups`: the workspaces by status in [`StatusBucket::GROUP_ORDER`], each group
/// sorted by [`compare_status_rows`]. A bucket without rows has no group. Each row takes
/// [`Group::Status`] of its group.
pub fn status_groups(rows: Vec<WorkspaceRow>) -> Vec<(StatusBucket, Vec<WorkspaceRow>)> {
    let mut buckets: Vec<(StatusBucket, Vec<WorkspaceRow>)> = StatusBucket::GROUP_ORDER
        .into_iter()
        .map(|bucket| (bucket, Vec::new()))
        .collect();
    for row in rows {
        if let Some((_, group)) = buckets.iter_mut().find(|(bucket, _)| *bucket == row.bucket) {
            group.push(row);
        }
    }
    buckets.retain(|(_, rows)| !rows.is_empty());
    for (bucket, rows) in &mut buckets {
        rows.sort_by(compare_status_rows);
        for row in rows.iter_mut() {
            row.group = Group::Status(*bucket);
        }
    }
    buckets
}

/// The rows of the tree, top to bottom: the pinned section (when any workspace is pinned), then
/// "Workspaces" with either each project and its unpinned workspaces ([`Grouping::Project`], in
/// the Electron sort order) or one heading per status ([`Grouping::Status`]). The project and
/// label filters of `state` narrow both modes. The pinned rows go by `pinnedAt`, newest first,
/// and keep the tree order when the times are equal or missing; the saved pinned order comes
/// next. Status grouping keeps its own sort and ignores the saved thread and project order.
pub fn build_rows(
    projects: &[ProjectNode],
    agents: &[AgentSummary],
    facts: &Facts,
    state: &ViewState,
) -> Vec<Row> {
    if projects.is_empty() {
        return vec![Row::Empty];
    }
    let projects = filter_projects(sorted(projects, facts, &state.order), state);
    let by_id: HashMap<&str, &AgentSummary> = agents.iter().map(|a| (a.id.as_str(), a)).collect();
    let mut rows = Vec::new();

    let mut pinned: Vec<WorkspaceRow> = projects
        .iter()
        .flat_map(|entry| {
            let by_id = &by_id;
            entry
                .workspaces
                .iter()
                .filter(|node| node.pinned)
                .map(move |node| workspace_row(entry.project, node, Group::Pinned, by_id, facts))
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
    // `splitPinnedSidebarGroups`: the saved pinned order applies after the `pinnedAt` sort.
    if let Some(server_id) = &facts.server_id {
        pinned = apply_stored_ordering(
            pinned,
            &workspace_ids(&state.order.pinned_workspace_order, server_id),
            |row| Some(row.id.as_str()),
        );
    }
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
    match state.grouping {
        Grouping::Project => push_project_rows(&mut rows, &projects, &by_id, facts, state),
        Grouping::Status => push_status_rows(&mut rows, &projects, &by_id, facts, state),
    }
    rows
}

fn push_project_rows(
    rows: &mut Vec<Row>,
    projects: &[SortedProject],
    by_id: &HashMap<&str, &AgentSummary>,
    facts: &Facts,
    state: &ViewState,
) {
    for entry in projects {
        let project = entry.project;
        let items: Vec<WorkspaceRow> = entry
            .workspaces
            .iter()
            .filter(|node| !node.pinned)
            .map(|node| {
                workspace_row(
                    project,
                    node,
                    Group::Project(project.id.clone()),
                    by_id,
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
                entry
                    .workspaces
                    .iter()
                    .map(|node| workspace_bucket(node, by_id, facts)),
            ),
            color_key: facts.color_key(&project.id),
        }));
        if !collapsed && !items.is_empty() {
            push_section(rows, Group::Project(project.id.clone()), items, state);
        }
    }
}

fn push_status_rows(
    rows: &mut Vec<Row>,
    projects: &[SortedProject],
    by_id: &HashMap<&str, &AgentSummary>,
    facts: &Facts,
    state: &ViewState,
) {
    let unpinned: Vec<WorkspaceRow> = projects
        .iter()
        .flat_map(|entry| {
            entry
                .workspaces
                .iter()
                .filter(|node| !node.pinned)
                .map(move |node| {
                    workspace_row(
                        entry.project,
                        node,
                        Group::Project(entry.project.id.clone()),
                        by_id,
                        facts,
                    )
                })
        })
        .collect();
    for (bucket, items) in status_groups(unpinned) {
        let collapsed = state.collapsed_statuses.contains(&bucket);
        rows.push(Row::StatusHeader {
            bucket,
            collapsed,
            block_end: collapsed,
        });
        if !collapsed {
            push_section(rows, Group::Status(bucket), items, state);
        }
    }
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

/// `archive_prompt_for_stat`, and also a confirm when the daemon reports the checkout dirty
/// (`gitRuntime.isDirty`). The stat comes from `git diff --shortstat`, which misses untracked
/// files, and archiving a Paseo-owned worktree removes them. Electron trusts the stat here; this
/// client asks on purpose (owner, Q-095). `None` and `Some(false)` keep the stat rule.
pub fn archive_prompt_for_checkout(
    workspace_name: &str,
    stat: Option<(u32, u32)>,
    is_dirty: Option<bool>,
) -> Option<ArchivePrompt> {
    archive_prompt_for_stat(workspace_name, stat)
        .or_else(|| (is_dirty == Some(true)).then(|| archive_confirm(workspace_name, 0, 0)))
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
    workspace_label_for(row, leading_project, TitleSource::Title)
}

/// [`workspace_label`] with the title the row shows (`resolveSidebarWorkspaceAccessibilityLabel`).
pub fn workspace_label_for(
    row: &WorkspaceRow,
    leading_project: bool,
    source: TitleSource,
) -> String {
    let mut parts = Vec::new();
    if leading_project {
        parts.push(row.project_name.as_str());
    }
    parts.push(row_title(row, source));
    if row.bucket != StatusBucket::Done {
        parts.push(row.bucket.label());
    }
    parts.join(", ")
}
