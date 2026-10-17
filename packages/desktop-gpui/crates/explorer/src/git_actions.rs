//! The git actions policy of the workspace header split button, ported from the app's
//! `git/policy.ts` (`buildGitActions`) and the inputs `git/use-actions.tsx` derives for it.
//! Pure functions over `GitStatus` and `PrStatusReply`; `git_actions_ui` draws the result.

use bachuc_client::{
    GitAction, GitStatus, MergeMethod, PrMergeCapability, PrStatus, PrStatusReply,
};

/// What the user ships with when nothing else is pending (`shipDefault`). Kept per repo.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum ShipDefault {
    #[default]
    Pr,
    Merge,
}

/// The `GitActionId` of `policy.ts`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ActionId {
    Commit,
    Pull,
    Push,
    PullAndPush,
    /// Create PR, or View PR when the branch has one.
    Pr,
    MergePr(MergeMethod),
    EnableAutoMerge(MergeMethod),
    DisableAutoMerge,
    MergeBranch,
    MergeFromBase,
    ArchiveWorkspace,
}

/// One built action.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Action {
    pub id: ActionId,
    pub label: String,
    pub pending_label: String,
    pub disabled: bool,
    /// Why the action cannot run now. A press shows it instead of running the action.
    pub unavailable: Option<String>,
    /// A separator goes above this item when it is not the first.
    pub starts_group: bool,
}

/// The split button contents. Electron's overflow `menu` is always empty, so it is left out.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct GitActions {
    pub primary: Option<Action>,
    pub secondary: Vec<Action>,
}

/// What the button knows besides the git and PR status.
#[derive(Clone, Copy, Debug, Default)]
pub struct PolicyState {
    /// The host serves an auto-merge request (`checkoutForgeSetAutoMerge` or the GitHub one).
    pub auto_merge_actions_enabled: bool,
    pub ship_default: ShipDefault,
    /// A merge just succeeded here, so Archive is promoted.
    pub post_ship_archive_suggested: bool,
    /// The status loads or failed, or an action runs: every action is disabled.
    pub actions_disabled: bool,
}

const FORGE_BRAND: &str = "GitHub";
const FORGE_NOUN: &str = "PR";
/// `workspace.git.diff.base`, the label when the daemon names no base ref.
const BASE_FALLBACK: &str = "base";

/// Direct merge and auto-merge items come in this order (`PULL_REQUEST_*_ACTION_MODELS`).
const METHOD_ORDER: [MergeMethod; 3] =
    [MergeMethod::Squash, MergeMethod::Merge, MergeMethod::Rebase];

/// The neutral `MergeCapability` of `client-forge-module.ts`.
#[derive(Clone, Debug, PartialEq, Eq)]
struct Capability {
    direct_merge_ready: bool,
    can_enable_auto_merge: bool,
    auto_merge_enabled: bool,
    can_disable_auto_merge: bool,
    merge_blocked_by_queue: bool,
    allowed_methods: Vec<MergeMethod>,
    preferred_method: Option<MergeMethod>,
}

/// `deriveGithubMergeCapability`. A PR without `github` facts parses to the default
/// capability, which stands for Electron's `null` (no facts, fall back to git state).
/// The client reads neither the merge queue flags nor `viewerDefaultMergeMethod`, so the
/// queue never blocks and the first allowed method is preferred.
fn capability(merge: &PrMergeCapability) -> Option<Capability> {
    if *merge == PrMergeCapability::default() {
        return None;
    }
    Some(Capability {
        direct_merge_ready: matches!(
            merge.merge_state_status.as_deref(),
            Some("CLEAN" | "HAS_HOOKS")
        ),
        can_enable_auto_merge: merge.can_enable_auto_merge,
        auto_merge_enabled: merge.auto_merge_enabled,
        can_disable_auto_merge: merge.can_disable_auto_merge,
        merge_blocked_by_queue: false,
        allowed_methods: merge.allowed_methods.clone(),
        preferred_method: None,
    })
}

/// `formatBaseRefLabel`: `refs/heads/main` and `origin/main` read `main`.
pub fn base_ref_label(base_ref: Option<&str>) -> String {
    let Some(base_ref) = base_ref.filter(|base| !base.is_empty()) else {
        return BASE_FALLBACK.to_string();
    };
    let trimmed = base_ref
        .strip_prefix("refs/heads/")
        .or_else(|| base_ref.strip_prefix("refs/remotes/"))
        .unwrap_or(base_ref)
        .trim();
    trimmed
        .strip_prefix("origin/")
        .unwrap_or(trimmed)
        .to_string()
}

/// `BuildGitActionsInput`, derived as `use-actions.tsx` derives it.
struct Input<'a> {
    github: bool,
    auto_merge_actions: bool,
    pr: Option<&'a PrStatus>,
    pr_state: Option<&'a str>,
    mergeable: &'a str,
    capability: Option<Capability>,
    has_remote: bool,
    paseo_worktree: bool,
    on_base: bool,
    dirty: bool,
    base_available: bool,
    base_label: String,
    ahead: u32,
    behind_base: u32,
    /// `None` without an upstream, as the daemon's null counts.
    ahead_of_origin: Option<u32>,
    behind_of_origin: Option<u32>,
    promote_archive: bool,
    ship_default: ShipDefault,
    disabled: bool,
}

impl<'a> Input<'a> {
    fn new(status: &'a GitStatus, pr: Option<&'a PrStatusReply>, state: PolicyState) -> Self {
        let github = pr.is_some_and(|reply| reply.github_features_enabled);
        let pr_status = pr
            .and_then(|reply| reply.status.as_ref())
            .filter(|pr| !pr.url.is_empty());
        let base_label = base_ref_label(status.base_ref.as_deref());
        let dirty = status.is_dirty;
        let merged = pr_status.is_some_and(|pr| pr.is_merged);
        Input {
            github,
            auto_merge_actions: state.auto_merge_actions_enabled,
            pr: pr_status,
            pr_state: pr_status
                .map(|pr| pr.state.as_str())
                .filter(|state| matches!(*state, "open" | "closed")),
            mergeable: pr_status
                .and_then(|pr| pr.mergeable.as_deref())
                .unwrap_or("UNKNOWN"),
            capability: pr_status.and_then(|pr| capability(&pr.merge)),
            has_remote: status.has_remote,
            paseo_worktree: status.is_paseo_owned_worktree,
            on_base: status.branch.as_deref() == Some(base_label.as_str()),
            dirty,
            base_available: status
                .base_ref
                .as_deref()
                .is_some_and(|base| !base.is_empty()),
            base_label,
            ahead: status.ahead,
            behind_base: status.behind,
            ahead_of_origin: status.has_upstream.then_some(status.ahead_of_origin),
            behind_of_origin: status.has_upstream.then_some(status.behind_of_origin),
            promote_archive: !dirty && (state.post_ship_archive_suggested || merged),
            ship_default: state.ship_default,
            disabled: !status.is_git || state.actions_disabled,
        }
    }

    fn pr_open(&self) -> bool {
        self.pr_state == Some("open")
    }

    fn pr_draft(&self) -> bool {
        self.pr.is_some_and(|pr| pr.is_draft)
    }

    fn pr_merged(&self) -> bool {
        self.pr.is_some_and(|pr| pr.is_merged)
    }

    fn allowed(&self, method: MergeMethod) -> bool {
        self.capability
            .as_ref()
            .is_none_or(|capability| capability.allowed_methods.contains(&method))
    }

    fn allowed_methods(&self) -> Vec<MergeMethod> {
        METHOD_ORDER
            .into_iter()
            .filter(|method| self.allowed(*method))
            .collect()
    }

    fn preferred_method(&self) -> MergeMethod {
        let allowed = self.allowed_methods();
        let preferred = self
            .capability
            .as_ref()
            .and_then(|capability| capability.preferred_method);
        allowed
            .iter()
            .copied()
            .find(|method| Some(*method) == preferred)
            .or_else(|| allowed.first().copied())
            .unwrap_or(MergeMethod::Squash)
    }

    fn can_pull(&self) -> bool {
        self.has_remote && !self.dirty && self.behind_of_origin.unwrap_or(0) > 0
    }

    fn can_push(&self) -> bool {
        self.has_remote && self.has_pushable_commits() && self.behind_of_origin.unwrap_or(0) == 0
    }

    /// No-upstream Paseo worktrees are first-pushable: the daemon push sets the upstream.
    fn has_pushable_commits(&self) -> bool {
        self.ahead_of_origin.unwrap_or(0) > 0
            || (self.paseo_worktree && self.ahead_of_origin.is_none() && self.ahead > 0)
    }

    fn can_merge_from_base(&self) -> bool {
        !self.on_base && self.base_available && !self.dirty && self.behind_base > 0
    }

    fn can_use_pr_as_ship_default(&self) -> bool {
        if self.on_base || !self.github {
            return false;
        }
        self.pr.is_some() || self.ahead > 0
    }

    fn can_merge_pr(&self) -> bool {
        let from_status = self.github
            && self.pr.is_some()
            && self.pr_open()
            && !self.pr_draft()
            && !self.pr_merged()
            && self.mergeable != "CONFLICTING"
            && self.ahead > 0
            && !self.dirty;
        if !from_status {
            return false;
        }
        match &self.capability {
            None => {
                self.mergeable == "MERGEABLE"
                    && self.behind_of_origin == Some(0)
                    && self.ahead_of_origin == Some(0)
                    && !self.can_merge_from_base()
            }
            Some(capability) => {
                capability.direct_merge_ready
                    && !capability.auto_merge_enabled
                    && !capability.merge_blocked_by_queue
                    && !self.allowed_methods().is_empty()
            }
        }
    }

    fn can_enable_auto_merge(&self) -> bool {
        self.github
            && self.auto_merge_actions
            && self.pr.is_some()
            && self.pr_open()
            && !self.pr_draft()
            && !self.pr_merged()
            && self.mergeable != "CONFLICTING"
            && self.capability.as_ref().is_some_and(|capability| {
                !capability.auto_merge_enabled
                    && capability.can_enable_auto_merge
                    && !capability.merge_blocked_by_queue
            })
            && !self.allowed_methods().is_empty()
    }

    fn auto_merge_on(&self) -> bool {
        self.github
            && self.pr.is_some()
            && self
                .capability
                .as_ref()
                .is_some_and(|capability| capability.auto_merge_enabled)
    }

    /// `getPrimaryActionId`: the first match wins.
    fn primary_id(&self) -> Option<ActionId> {
        if self.promote_archive {
            return Some(ActionId::ArchiveWorkspace);
        }
        if self.dirty {
            return Some(ActionId::Commit);
        }
        if self.can_pull() {
            return Some(ActionId::Pull);
        }
        if self.can_push() {
            return Some(ActionId::Push);
        }
        if self.can_merge_pr() {
            return Some(ActionId::MergePr(self.preferred_method()));
        }
        if self.can_enable_auto_merge() {
            return Some(ActionId::EnableAutoMerge(self.preferred_method()));
        }
        if self.auto_merge_on() {
            return Some(ActionId::Pr);
        }
        if self.ship_default == ShipDefault::Pr && self.can_use_pr_as_ship_default() {
            return Some(ActionId::Pr);
        }
        if !self.on_base && self.ahead > 0 {
            return Some(ActionId::MergeBranch);
        }
        if !self.on_base && self.can_merge_from_base() {
            return Some(ActionId::MergeFromBase);
        }
        if self.github && self.pr.is_some() {
            return Some(ActionId::Pr);
        }
        // Only Paseo-owned worktrees get Archive as the fallback primary action.
        self.paseo_worktree.then_some(ActionId::ArchiveWorkspace)
    }

    /// `shouldShowPullRequestAction` for the merge and auto-merge items.
    fn shows(&self, id: ActionId) -> bool {
        match id {
            ActionId::MergePr(method) => self.can_merge_pr() && self.allowed(method),
            ActionId::EnableAutoMerge(method) => {
                self.can_enable_auto_merge() && self.allowed(method)
            }
            ActionId::DisableAutoMerge => {
                self.auto_merge_actions
                    && self
                        .capability
                        .as_ref()
                        .is_some_and(|capability| capability.auto_merge_enabled)
            }
            _ => true,
        }
    }

    fn pull_unavailable(&self) -> Option<&'static str> {
        if !self.has_remote {
            return Some(
                "Pull isn't available here because this branch is not connected to a remote yet",
            );
        }
        if self.dirty {
            return Some(
                "Pull isn't available while you have local changes so commit or stash them first",
            );
        }
        match self.behind_of_origin {
            None => Some(
                "Pull isn't available here because this branch is not connected to a remote yet",
            ),
            Some(0) => Some("Pull isn't available because this branch is already up to date"),
            Some(_) => None,
        }
    }

    fn push_unavailable(&self) -> Option<&'static str> {
        if !self.has_remote {
            return Some(
                "Push isn't available here because this branch is not connected to a remote yet",
            );
        }
        if self.behind_of_origin.unwrap_or(0) > 0 {
            return Some(
                "Push isn't available yet because there are newer changes to bring in first",
            );
        }
        if !self.has_pushable_commits() {
            return Some("Push isn't available because there is nothing new to send");
        }
        None
    }

    fn pull_and_push_unavailable(&self) -> Option<&'static str> {
        if !self.has_remote {
            return Some(
                "Pull and push isn't available here because this branch is not connected to a \
                 remote yet",
            );
        }
        if self.dirty {
            return Some(
                "Pull and push isn't available while you have local changes so commit or stash \
                 them first",
            );
        }
        let no_incoming =
            "Pull and push isn't available because there are no incoming changes to pull first";
        match (self.behind_of_origin, self.ahead_of_origin) {
            (None, _) => Some(no_incoming),
            (Some(0), Some(0)) => {
                Some("Pull and push isn't available because this branch is already in sync")
            }
            (Some(0), _) => Some(no_incoming),
            (_, ahead) if ahead.unwrap_or(0) == 0 => Some(
                "Pull and push isn't available because there is nothing new to send after pulling",
            ),
            _ => None,
        }
    }

    fn create_pr_unavailable(&self) -> Option<String> {
        if !self.github {
            return Some(format!(
                "Create {FORGE_NOUN} isn't available right now because {FORGE_BRAND} isn't \
                 connected"
            ));
        }
        (self.ahead == 0).then(|| {
            "Create PR isn't available because this branch doesn't have any new commits yet"
                .to_string()
        })
    }

    fn merge_branch_unavailable(&self) -> Option<&'static str> {
        if !self.base_available {
            return Some("Merge isn't available because we couldn't determine the base branch");
        }
        if self.dirty {
            return Some(
                "Merge isn't available while you have local changes so commit or stash them first",
            );
        }
        (self.ahead == 0).then_some(
            "Merge isn't available because this branch doesn't have anything new to merge yet",
        )
    }

    fn merge_from_base_unavailable(&self) -> Option<String> {
        if !self.base_available {
            return Some(
                "Update isn't available because we couldn't determine the base branch".into(),
            );
        }
        if self.dirty {
            return Some(
                "Update isn't available while you have local changes so commit or stash them first"
                    .into(),
            );
        }
        (self.behind_base == 0).then(|| {
            format!(
                "Update isn't available because this branch is already up to date with {}",
                self.base_label
            )
        })
    }

    fn merge_pr_unavailable(&self) -> Option<String> {
        if !self.github {
            return Some(format!(
                "Merge {FORGE_NOUN} isn't available right now because {FORGE_BRAND} isn't \
                 connected"
            ));
        }
        let reason = if self.pr.is_none() {
            "Merge PR isn't available because there isn't a pull request yet"
        } else if self.pr_draft() {
            "Merge PR isn't available because the pull request is still a draft"
        } else if self.pr_merged() {
            "Merge PR isn't available because the pull request is already merged"
        } else if self.pr_state == Some("closed") {
            "Merge PR isn't available because the pull request is closed"
        } else if self.mergeable == "CONFLICTING" {
            "Merge PR isn't available because the pull request has conflicts"
        } else {
            let capability = self.capability.as_ref()?;
            if capability.merge_blocked_by_queue {
                "Merge PR isn't available here because this repository uses a merge queue"
            } else if !capability.direct_merge_ready {
                return Some(format!(
                    "Merge {FORGE_NOUN} isn't available until {FORGE_BRAND} reports the \
                     {FORGE_NOUN} is ready to merge"
                ));
            } else {
                return None;
            }
        };
        Some(reason.to_string())
    }

    /// One action. As in `policy.ts`, an unavailable message is dropped while the action is
    /// disabled by the runtime (no status, an action runs).
    fn action(&self, id: ActionId) -> Action {
        let make = |label: String, pending: &str, unavailable: Option<String>| Action {
            id,
            label,
            pending_label: pending.to_string(),
            disabled: self.disabled,
            unavailable: if self.disabled { None } else { unavailable },
            starts_group: false,
        };
        match id {
            ActionId::Commit => make("Commit".into(), "Committing...", None),
            ActionId::Pull => make(
                "Pull".into(),
                "Pulling...",
                self.pull_unavailable().map(str::to_string),
            ),
            ActionId::Push => make(
                "Push".into(),
                "Pushing...",
                self.push_unavailable().map(str::to_string),
            ),
            ActionId::PullAndPush => make(
                "Pull and push".into(),
                "Pulling and pushing...",
                self.pull_and_push_unavailable().map(str::to_string),
            ),
            ActionId::Pr if self.pr.is_some() => {
                let unavailable = (!self.github).then(|| {
                    format!(
                        "View {FORGE_NOUN} isn't available right now because {FORGE_BRAND} \
                         isn't connected"
                    )
                });
                make("View PR".into(), "View PR", unavailable)
            }
            ActionId::Pr => make(
                "Create PR".into(),
                "Creating PR...",
                self.create_pr_unavailable(),
            ),
            ActionId::MergePr(method) => Action {
                disabled: self.disabled || !self.can_merge_pr(),
                starts_group: method == MergeMethod::Squash,
                ..make(
                    format!("Merge PR ({})", method_name(method)),
                    "Merging PR...",
                    self.merge_pr_unavailable(),
                )
            },
            ActionId::EnableAutoMerge(method) => Action {
                starts_group: method == MergeMethod::Squash,
                ..make(
                    format!("Auto merge ({})", method_name(method)),
                    "Enabling auto-merge...",
                    None,
                )
            },
            ActionId::DisableAutoMerge => {
                let can_disable = self
                    .capability
                    .as_ref()
                    .is_some_and(|capability| capability.can_disable_auto_merge);
                Action {
                    disabled: self.disabled || !can_disable,
                    starts_group: true,
                    ..make(
                        "Auto-merge enabled".into(),
                        "Disabling auto-merge...",
                        (!can_disable).then(|| {
                            "Auto-merge is enabled, but this account can't disable it".into()
                        }),
                    )
                }
            }
            ActionId::MergeBranch => make(
                "Merge locally".into(),
                "Merging...",
                self.merge_branch_unavailable().map(str::to_string),
            ),
            ActionId::MergeFromBase => Action {
                starts_group: true,
                ..make(
                    format!("Update from {}", self.base_label),
                    "Updating...",
                    self.merge_from_base_unavailable(),
                )
            },
            ActionId::ArchiveWorkspace => Action {
                starts_group: true,
                ..make("Archive workspace".into(), "Archiving...", None)
            },
        }
    }
}

fn method_name(method: MergeMethod) -> &'static str {
    match method {
        MergeMethod::Squash => "squash",
        MergeMethod::Merge => "merge",
        MergeMethod::Rebase => "rebase",
    }
}

/// `buildGitActions`. Outside git there is no primary action and every action is disabled.
pub fn build_git_actions(
    status: &GitStatus,
    pr: Option<&PrStatusReply>,
    state: PolicyState,
) -> GitActions {
    let input = Input::new(status, pr, state);
    let primary_id = if status.is_git {
        input.primary_id()
    } else {
        None
    };
    let mut ids = vec![ActionId::Pull, ActionId::Push, ActionId::PullAndPush];
    if !input.on_base {
        ids.extend([ActionId::MergeFromBase, ActionId::MergeBranch, ActionId::Pr]);
        ids.extend(METHOD_ORDER.map(ActionId::MergePr));
        ids.extend(METHOD_ORDER.map(ActionId::EnableAutoMerge));
        ids.push(ActionId::DisableAutoMerge);
    }
    if primary_id != Some(ActionId::ArchiveWorkspace) {
        ids.push(ActionId::ArchiveWorkspace);
    }
    GitActions {
        primary: primary_id.map(|id| input.action(id)),
        secondary: ids
            .into_iter()
            .filter(|id| input.shows(*id))
            .map(|id| input.action(id))
            .collect(),
    }
}

/// What a press on an item does (`useGitActionRunner`). An item with an unavailable message
/// shows it, even when it is disabled.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Press {
    Run(ActionId),
    Explain(String),
    Ignore,
}

pub fn press(action: &Action) -> Press {
    if let Some(message) = &action.unavailable {
        return Press::Explain(message.clone());
    }
    if action.disabled {
        return Press::Ignore;
    }
    Press::Run(action.id)
}

/// How an action runs.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Step {
    Daemon(GitAction),
    /// View PR opens the PR page.
    OpenPr,
    /// The app runs its archive flow.
    Archive,
}

pub fn step(id: ActionId, has_pr: bool) -> Step {
    match id {
        ActionId::Commit => Step::Daemon(GitAction::Commit),
        ActionId::Pull => Step::Daemon(GitAction::Pull),
        ActionId::Push => Step::Daemon(GitAction::Push),
        ActionId::PullAndPush => Step::Daemon(GitAction::PullAndPush),
        ActionId::Pr if has_pr => Step::OpenPr,
        ActionId::Pr => Step::Daemon(GitAction::CreatePr),
        ActionId::MergePr(method) => Step::Daemon(GitAction::MergePr(method)),
        ActionId::EnableAutoMerge(method) => Step::Daemon(GitAction::SetAutoMerge {
            enabled: true,
            method: Some(method),
        }),
        ActionId::DisableAutoMerge => Step::Daemon(GitAction::SetAutoMerge {
            enabled: false,
            method: None,
        }),
        ActionId::MergeBranch => Step::Daemon(GitAction::MergeBranch),
        ActionId::MergeFromBase => Step::Daemon(GitAction::MergeFromBase),
        ActionId::ArchiveWorkspace => Step::Archive,
    }
}

/// The ship default a pick saves: PR actions save `Pr`, a local merge saves `Merge`.
pub fn ship_default_for(id: ActionId) -> Option<ShipDefault> {
    match id {
        ActionId::Pr | ActionId::MergePr(_) | ActionId::EnableAutoMerge(_) => Some(ShipDefault::Pr),
        ActionId::MergeBranch => Some(ShipDefault::Merge),
        _ => None,
    }
}

/// A merge that succeeded promotes Archive (`setPostShipArchiveSuggested`).
pub fn suggests_archive(id: ActionId) -> bool {
    matches!(id, ActionId::MergePr(_) | ActionId::MergeBranch)
}

/// The success text of a daemon action (`successLabel`).
pub fn success_message(id: ActionId) -> &'static str {
    match id {
        ActionId::Commit => "Committed",
        ActionId::Pull => "Pulled",
        ActionId::Push => "Pushed",
        ActionId::PullAndPush => "Pulled and pushed",
        ActionId::Pr => "PR Created",
        ActionId::MergePr(_) => "PR merged",
        ActionId::EnableAutoMerge(_) => "Auto-merge enabled",
        ActionId::DisableAutoMerge => "Auto-merge disabled",
        ActionId::MergeBranch => "Merged",
        ActionId::MergeFromBase => "Updated",
        ActionId::ArchiveWorkspace => "Archived",
    }
}

/// The error text: the daemon message, or Electron's fallback when it is empty.
pub fn failure_message(id: ActionId, error: &str) -> String {
    if !error.trim().is_empty() {
        return error.to_string();
    }
    match id {
        ActionId::Commit => "Failed to commit",
        ActionId::Pull => "Failed to pull",
        ActionId::Push => "Failed to push",
        ActionId::PullAndPush => "Failed to pull and push",
        ActionId::Pr => "Failed to create PR",
        ActionId::MergePr(_) => "Failed to merge PR",
        ActionId::EnableAutoMerge(_) => "Failed to enable auto-merge",
        ActionId::DisableAutoMerge => "Failed to disable auto-merge",
        ActionId::MergeBranch => "Failed to merge",
        ActionId::MergeFromBase => "Failed to update from base",
        ActionId::ArchiveWorkspace => "Failed to archive",
    }
    .to_string()
}

/// Coalesces status refreshes: the first request in a window starts one timer, and the
/// requests that follow before it fires add nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct RefreshGate {
    scheduled: bool,
}

impl RefreshGate {
    /// True when the caller must start the timer.
    pub fn request(&mut self) -> bool {
        !std::mem::replace(&mut self.scheduled, true)
    }

    /// The timer fired; the next request starts a new window.
    pub fn fire(&mut self) {
        self.scheduled = false;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn git() -> GitStatus {
        GitStatus {
            is_git: true,
            branch: Some("feature".into()),
            base_ref: Some("main".into()),
            has_remote: true,
            has_upstream: true,
            ..GitStatus::default()
        }
    }

    fn open_pr(merge: PrMergeCapability) -> PrStatusReply {
        PrStatusReply {
            status: Some(PrStatus {
                number: Some(7),
                url: "https://github.com/o/r/pull/7".into(),
                title: "Title".into(),
                state: "open".into(),
                is_merged: false,
                is_draft: false,
                mergeable: Some("MERGEABLE".into()),
                checks: Vec::new(),
                review_decision: None,
                repo_owner: "o".into(),
                repo_name: "r".into(),
                merge,
            }),
            github_features_enabled: true,
        }
    }

    fn clean_capability() -> PrMergeCapability {
        PrMergeCapability {
            allowed_methods: vec![MergeMethod::Merge, MergeMethod::Squash],
            merge_state_status: Some("CLEAN".into()),
            ..PrMergeCapability::default()
        }
    }

    fn primary(status: &GitStatus, pr: Option<&PrStatusReply>) -> Option<ActionId> {
        build_git_actions(status, pr, PolicyState::default())
            .primary
            .map(|action| action.id)
    }

    fn all(actions: &GitActions) -> impl Iterator<Item = &Action> {
        actions.primary.iter().chain(&actions.secondary)
    }

    #[test]
    fn git_actions_dirty_tree_is_commit() {
        let status = GitStatus {
            is_dirty: true,
            behind_of_origin: 2,
            ..git()
        };
        assert_eq!(primary(&status, None), Some(ActionId::Commit));
    }

    #[test]
    fn git_actions_clean_behind_origin_is_pull() {
        let status = GitStatus {
            behind_of_origin: 2,
            ahead_of_origin: 1,
            ..git()
        };
        assert_eq!(primary(&status, None), Some(ActionId::Pull));
    }

    #[test]
    fn git_actions_clean_ahead_of_origin_is_push() {
        let status = GitStatus {
            ahead: 3,
            ahead_of_origin: 3,
            ..git()
        };
        assert_eq!(primary(&status, None), Some(ActionId::Push));
    }

    #[test]
    fn git_actions_first_push_of_a_paseo_worktree_without_upstream() {
        let status = GitStatus {
            ahead: 1,
            has_upstream: false,
            is_paseo_owned_worktree: true,
            ..git()
        };
        assert_eq!(primary(&status, None), Some(ActionId::Push));
        let plain = GitStatus {
            is_paseo_owned_worktree: false,
            ..status
        };
        assert_ne!(primary(&plain, None), Some(ActionId::Push));
    }

    #[test]
    fn git_actions_open_clean_mergeable_pr_is_merge_pr() {
        let status = GitStatus { ahead: 2, ..git() };
        let pr = open_pr(clean_capability());
        // Squash comes before merge in Electron's model order.
        assert_eq!(
            primary(&status, Some(&pr)),
            Some(ActionId::MergePr(MergeMethod::Squash))
        );
        let actions = build_git_actions(&status, Some(&pr), PolicyState::default());
        let merges: Vec<_> = actions
            .secondary
            .iter()
            .filter(|action| matches!(action.id, ActionId::MergePr(_)))
            .map(|action| action.id)
            .collect();
        assert_eq!(
            merges,
            [
                ActionId::MergePr(MergeMethod::Squash),
                ActionId::MergePr(MergeMethod::Merge)
            ]
        );
    }

    #[test]
    fn git_actions_pr_without_facts_needs_mergeable_and_in_sync() {
        let status = GitStatus { ahead: 2, ..git() };
        let pr = open_pr(PrMergeCapability::default());
        assert_eq!(
            primary(&status, Some(&pr)),
            Some(ActionId::MergePr(MergeMethod::Squash))
        );
        let behind_base = GitStatus {
            behind: 1,
            ..status
        };
        assert_ne!(
            primary(&behind_base, Some(&pr)),
            Some(ActionId::MergePr(MergeMethod::Squash))
        );
    }

    #[test]
    fn git_actions_blocked_pr_enables_auto_merge() {
        let status = GitStatus { ahead: 2, ..git() };
        let pr = open_pr(PrMergeCapability {
            can_enable_auto_merge: true,
            allowed_methods: vec![MergeMethod::Rebase],
            merge_state_status: Some("BLOCKED".into()),
            ..PrMergeCapability::default()
        });
        let on = PolicyState {
            auto_merge_actions_enabled: true,
            ..PolicyState::default()
        };
        assert_eq!(
            build_git_actions(&status, Some(&pr), on)
                .primary
                .map(|a| a.id),
            Some(ActionId::EnableAutoMerge(MergeMethod::Rebase))
        );
        // Without the host feature, the ship default PR takes over.
        assert_eq!(primary(&status, Some(&pr)), Some(ActionId::Pr));
    }

    #[test]
    fn git_actions_auto_merge_on_is_view_pr_and_offers_disable() {
        let status = GitStatus { ahead: 2, ..git() };
        let pr = open_pr(PrMergeCapability {
            auto_merge_enabled: true,
            allowed_methods: vec![MergeMethod::Squash],
            merge_state_status: Some("BLOCKED".into()),
            ..PrMergeCapability::default()
        });
        let state = PolicyState {
            auto_merge_actions_enabled: true,
            ..PolicyState::default()
        };
        let actions = build_git_actions(&status, Some(&pr), state);
        let primary = actions.primary.unwrap();
        assert_eq!(
            (primary.id, primary.label.as_str()),
            (ActionId::Pr, "View PR")
        );
        let disable = actions
            .secondary
            .iter()
            .find(|action| action.id == ActionId::DisableAutoMerge)
            .unwrap();
        assert!(disable.disabled);
        assert_eq!(
            press(disable),
            Press::Explain("Auto-merge is enabled, but this account can't disable it".into())
        );
    }

    #[test]
    fn git_actions_ship_default_picks_pr_or_local_merge() {
        let status = GitStatus { ahead: 2, ..git() };
        let pr = PrStatusReply {
            status: None,
            github_features_enabled: true,
        };
        assert_eq!(primary(&status, Some(&pr)), Some(ActionId::Pr));
        let merge = PolicyState {
            ship_default: ShipDefault::Merge,
            ..PolicyState::default()
        };
        assert_eq!(
            build_git_actions(&status, Some(&pr), merge)
                .primary
                .map(|a| a.id),
            Some(ActionId::MergeBranch)
        );
        // No forge: the local merge wins whatever the default.
        assert_eq!(primary(&status, None), Some(ActionId::MergeBranch));
    }

    #[test]
    fn git_actions_behind_base_is_update_from_base() {
        let status = GitStatus {
            behind: 4,
            base_ref: Some("refs/remotes/origin/main".into()),
            ..git()
        };
        let actions = build_git_actions(&status, None, PolicyState::default());
        let primary = actions.primary.unwrap();
        assert_eq!(primary.id, ActionId::MergeFromBase);
        assert_eq!(primary.label, "Update from main");
    }

    #[test]
    fn git_actions_archive_promoted_after_merge_or_for_merged_pr() {
        let status = git();
        let state = PolicyState {
            post_ship_archive_suggested: true,
            ..PolicyState::default()
        };
        let actions = build_git_actions(&status, None, state);
        assert_eq!(
            actions.primary.map(|a| a.id),
            Some(ActionId::ArchiveWorkspace)
        );
        assert!(
            actions
                .secondary
                .iter()
                .all(|action| action.id != ActionId::ArchiveWorkspace)
        );
        let mut merged = open_pr(clean_capability());
        merged.status.as_mut().unwrap().is_merged = true;
        assert_eq!(
            primary(&status, Some(&merged)),
            Some(ActionId::ArchiveWorkspace)
        );
        // A dirty tree still commits first.
        let dirty = GitStatus {
            is_dirty: true,
            ..status
        };
        assert_eq!(primary(&dirty, Some(&merged)), Some(ActionId::Commit));
    }

    #[test]
    fn git_actions_archive_fallback_only_for_paseo_worktrees() {
        assert_eq!(primary(&git(), None), None);
        let worktree = GitStatus {
            is_paseo_owned_worktree: true,
            ..git()
        };
        assert_eq!(primary(&worktree, None), Some(ActionId::ArchiveWorkspace));
    }

    #[test]
    fn git_actions_secondary_list_order_and_groups() {
        let status = GitStatus { ahead: 1, ..git() };
        let pr = PrStatusReply {
            status: None,
            github_features_enabled: true,
        };
        let actions = build_git_actions(&status, Some(&pr), PolicyState::default());
        let ids: Vec<_> = actions.secondary.iter().map(|a| a.id).collect();
        assert_eq!(
            ids,
            [
                ActionId::Pull,
                ActionId::Push,
                ActionId::PullAndPush,
                ActionId::MergeFromBase,
                ActionId::MergeBranch,
                ActionId::Pr,
                ActionId::ArchiveWorkspace,
            ]
        );
        let groups: Vec<_> = actions.secondary.iter().map(|a| a.starts_group).collect();
        assert_eq!(groups, [false, false, false, true, false, false, true]);
    }

    #[test]
    fn git_actions_on_base_branch_lists_only_remote_actions() {
        let status = GitStatus {
            branch: Some("main".into()),
            base_ref: Some("origin/main".into()),
            ..git()
        };
        let ids: Vec<_> = build_git_actions(&status, None, PolicyState::default())
            .secondary
            .iter()
            .map(|a| a.id)
            .collect();
        assert_eq!(
            ids,
            [
                ActionId::Pull,
                ActionId::Push,
                ActionId::PullAndPush,
                ActionId::ArchiveWorkspace
            ]
        );
    }

    #[test]
    fn git_actions_unavailable_messages() {
        let status = GitStatus {
            has_remote: false,
            ..git()
        };
        let actions = build_git_actions(&status, None, PolicyState::default());
        let pull = &actions.secondary[0];
        assert!(!pull.disabled);
        assert_eq!(
            press(pull),
            Press::Explain(
                "Pull isn't available here because this branch is not connected to a remote yet"
                    .into()
            )
        );
        let in_sync = build_git_actions(&git(), None, PolicyState::default());
        assert_eq!(
            in_sync.secondary[2].unavailable.as_deref(),
            Some("Pull and push isn't available because this branch is already in sync")
        );
        let create = in_sync
            .secondary
            .iter()
            .find(|a| a.id == ActionId::Pr)
            .unwrap();
        assert_eq!(
            create.unavailable.as_deref(),
            Some("Create PR isn't available right now because GitHub isn't connected")
        );
        let ready = GitStatus {
            behind_of_origin: 1,
            ..git()
        };
        let pull = &build_git_actions(&ready, None, PolicyState::default()).secondary[0];
        assert_eq!(press(pull), Press::Run(ActionId::Pull));
    }

    #[test]
    fn git_actions_not_git_disables_every_action() {
        let status = GitStatus {
            is_dirty: true,
            ahead: 2,
            ..GitStatus::default()
        };
        let actions = build_git_actions(&status, None, PolicyState::default());
        assert_eq!(actions.primary, None);
        assert!(!actions.secondary.is_empty());
        for action in all(&actions) {
            assert!(action.disabled, "{:?}", action.id);
            assert_eq!(press(action), Press::Ignore, "{:?}", action.id);
        }
    }

    #[test]
    fn git_actions_status_loading_disables_every_action() {
        let status = GitStatus {
            is_dirty: true,
            ahead: 2,
            ..git()
        };
        let pr = open_pr(clean_capability());
        let loading = PolicyState {
            actions_disabled: true,
            auto_merge_actions_enabled: true,
            ..PolicyState::default()
        };
        let actions = build_git_actions(&status, Some(&pr), loading);
        assert_eq!(
            actions.primary.as_ref().map(|a| a.id),
            Some(ActionId::Commit)
        );
        for action in all(&actions) {
            assert!(action.disabled, "{:?}", action.id);
            assert_eq!(press(action), Press::Ignore, "{:?}", action.id);
        }
    }

    #[test]
    fn git_actions_steps_and_ship_defaults() {
        assert_eq!(step(ActionId::Pr, true), Step::OpenPr);
        assert_eq!(step(ActionId::Pr, false), Step::Daemon(GitAction::CreatePr));
        assert_eq!(step(ActionId::ArchiveWorkspace, false), Step::Archive);
        assert_eq!(
            step(ActionId::DisableAutoMerge, true),
            Step::Daemon(GitAction::SetAutoMerge {
                enabled: false,
                method: None
            })
        );
        assert_eq!(
            ship_default_for(ActionId::MergeBranch),
            Some(ShipDefault::Merge)
        );
        assert_eq!(
            ship_default_for(ActionId::MergePr(MergeMethod::Squash)),
            Some(ShipDefault::Pr)
        );
        assert_eq!(ship_default_for(ActionId::Push), None);
        assert!(suggests_archive(ActionId::MergeBranch));
        assert!(!suggests_archive(ActionId::Push));
    }

    #[test]
    fn git_actions_failed_push_keeps_the_daemon_message() {
        assert_eq!(
            failure_message(ActionId::Push, "rejected: non-fast-forward"),
            "rejected: non-fast-forward"
        );
        assert_eq!(failure_message(ActionId::Push, " "), "Failed to push");
    }

    #[test]
    fn git_actions_refresh_gate_coalesces_a_burst() {
        let mut gate = RefreshGate::default();
        assert!(gate.request());
        assert!(!gate.request());
        assert!(!gate.request());
        gate.fire();
        assert!(gate.request());
    }

    #[test]
    fn git_actions_base_ref_label() {
        assert_eq!(base_ref_label(Some("refs/heads/dev")), "dev");
        assert_eq!(base_ref_label(Some("origin/main")), "main");
        assert_eq!(base_ref_label(None), "base");
    }
}
