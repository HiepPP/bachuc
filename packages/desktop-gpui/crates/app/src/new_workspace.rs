//! The New workspace screen, after Electron's `new-workspace-screen.tsx`: the title, a row of
//! project, isolation, and base pickers, and the draft composer under them. A submit sends one
//! `workspace.create.request`: an empty text creates the workspace alone, a text creates it with
//! its first agent. The shell talks to one host, so the host picker never shows, and terminal
//! launch is out, so the row ends at the base picker. A plugin's New workspace panel shows at
//! the right edge.

use crate::plugins::{side_panel_available, side_panel_index};
use bachuc_client::{
    BranchSuggestion, BranchValidation, Client, ClientEvent, CreateWorkspace, CreatedWorkspace,
    DirectoryEvent, InitialAgent, ProjectNode, WorkspaceSource, WorktreeAction,
};
use bachuc_composer::{Composer, ComposerEvent, DraftOptions, DraftSubmission, composer_dock};
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{NewWorkspacePanelInput, PluginHostEvent, ViewFactory};
use bachuc_theme::metrics::{
    FONT_2XL, FONT_BASE, ICON_MD, ICON_SM, RADIUS_2XL, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2,
    SPACE_4, SPACE_6, SPACE_8, WEIGHT_NORMAL,
};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::BTreeMap;
use std::rc::Rc;

/// `SIDE_PANEL_WIDTH` of the New workspace side panel.
const SIDE_PANEL_WIDTH: f32 = 360.;
/// `BADGE_HEIGHT`: one picker chip.
const BADGE_HEIGHT: f32 = 28.;
/// The chip's `maxWidth`.
const BADGE_MAX_WIDTH: f32 = 240.;
/// The opacity of a disabled chip.
const BADGE_DISABLED_OPACITY: f32 = 0.6;
/// The `errorText` line height.
const ERROR_LINE_HEIGHT: f32 = 20.;
/// `composerTitle` sets no line height, so Electron draws the 20 px title at about 1.2;
/// GPUI's default 1.618 put the chip row 8 px lower.
const TITLE_LINE_HEIGHT: f32 = 24.;
/// The width of a picker list.
const PICKER_WIDTH: f32 = 280.;
const PICKER_MAX_HEIGHT: f32 = 320.;
/// How many branches the base picker asks for.
const BRANCH_LIMIT: u32 = 50;

pub const CHOOSE_PROJECT: &str = "Choose a project";
pub const SELECT_MODEL: &str = "Select a model";
/// `newWorkspace.errors.createWorktreeFailed`: a worktree on a folder that is not a git
/// checkout.
pub const CREATE_WORKTREE_FAILED: &str = "Failed to create worktree";

/// Where the screen opens: a project, a folder, and a title for the workspace.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct NewWorkspacePreset {
    pub cwd: Option<String>,
    pub project_id: Option<String>,
    pub title: Option<String>,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub enum Isolation {
    #[default]
    Local,
    Worktree,
}

impl Isolation {
    pub fn label(self) -> &'static str {
        match self {
            Isolation::Local => "Local",
            Isolation::Worktree => "New worktree",
        }
    }
}

/// Which chips show and what a submit creates (`useWorkspaceIsolation`). `is_git` is `None`
/// while the checkout status loads.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Visibility {
    pub isolation_chip: bool,
    pub base_chip: bool,
    pub creates_worktree: bool,
}

pub fn visibility(multiplicity: bool, is_git: Option<bool>, isolation: Isolation) -> Visibility {
    let can_create_worktree = multiplicity && is_git != Some(false);
    let is_worktree = isolation == Isolation::Worktree && can_create_worktree;
    Visibility {
        isolation_chip: can_create_worktree,
        base_chip: !multiplicity || is_worktree,
        // Without multiplicity every create is a worktree (line 2068).
        creates_worktree: !multiplicity || is_worktree,
    }
}

/// `defaultBasePickerItem`: the current branch's local ref. The client gets no upstream ref.
pub fn default_base_ref(current_branch: Option<&str>) -> Option<String> {
    current_branch
        .filter(|branch| !branch.is_empty())
        .map(|branch| format!("refs/heads/{branch}"))
}

/// `branchNameFromRef`: the row label of a ref.
pub fn branch_name_from_ref(ref_name: &str) -> &str {
    if let Some(name) = ref_name.strip_prefix("refs/heads/") {
        return name;
    }
    if let Some(rest) = ref_name.strip_prefix("refs/remotes/") {
        return rest.split_once('/').map_or(rest, |(_, name)| name);
    }
    ref_name
}

/// The 8 hex digits of `worktreeSlug`. GPUI has no `mnemonic-id` word list; without a slug the
/// daemon names every worktree "worktree", and a second create collides.
pub fn worktree_slug(id: uuid::Uuid) -> String {
    id.simple().to_string()[..8].to_string()
}

/// A project the picker lists: the client's projects by name, as the sidebar sorts them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct ProjectOption {
    pub id: String,
    pub name: String,
    pub root: String,
}

pub fn project_options(projects: &[ProjectNode]) -> Vec<ProjectOption> {
    let mut options: Vec<ProjectOption> = projects
        .iter()
        .map(|project| ProjectOption {
            id: project.id.clone(),
            name: project.name.clone(),
            root: project.root.clone(),
        })
        .collect();
    options.sort_by_key(|option| option.name.to_lowercase());
    options
}

/// The preset project, else the project of the preset folder, else the first one.
pub fn default_project(projects: &[ProjectNode], preset: &NewWorkspacePreset) -> Option<String> {
    let listed = |id: &str| projects.iter().any(|project| project.id == id);
    if let Some(id) = preset.project_id.as_deref().filter(|id| listed(id)) {
        return Some(id.to_string());
    }
    if let Some(cwd) = preset.cwd.as_deref() {
        let owner = projects.iter().find(|project| {
            project.root == cwd
                || project
                    .workspaces
                    .iter()
                    .any(|workspace| workspace.cwd == cwd)
        });
        if let Some(project) = owner {
            return Some(project.id.clone());
        }
    }
    project_options(projects)
        .into_iter()
        .next()
        .map(|option| option.id)
}

/// One row of the base picker.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct BaseItem {
    pub name: String,
    pub ref_name: String,
    pub label: String,
}

/// `buildBranchPickerItems` without the divergence text: the origin row first, then the local
/// row when the refs differ or their divergence is unknown.
pub fn base_items(branches: &[BranchSuggestion]) -> Vec<BaseItem> {
    let mut items = Vec::new();
    for branch in branches {
        let name = &branch.name;
        if branch.has_local.is_none() && branch.has_remote.is_none() {
            items.push(BaseItem {
                name: name.clone(),
                ref_name: name.clone(),
                label: format!("{name}, branch"),
            });
            continue;
        }
        let has_local = branch.has_local == Some(true);
        let has_remote = branch.has_remote == Some(true);
        let divergence = match (branch.local_ahead, branch.local_behind) {
            (Some(ahead), Some(behind)) if has_local && has_remote => Some(ahead + behind > 0),
            _ => None,
        };
        let both = has_local && has_remote && divergence != Some(false);
        if has_remote {
            items.push(BaseItem {
                name: name.clone(),
                ref_name: format!("refs/remotes/origin/{name}"),
                label: format!("{name}, origin branch"),
            });
        }
        if has_local && (both || !has_remote) {
            items.push(BaseItem {
                name: if both {
                    format!("{name} (local)")
                } else {
                    name.clone()
                },
                ref_name: format!("refs/heads/{name}"),
                label: format!("{name}, local branch"),
            });
        }
    }
    items
}

/// The rows for a search: the listed branches, plus the typed name when it is not listed and
/// `validate_branch` found it.
pub fn base_rows(
    branches: &[BranchSuggestion],
    query: &str,
    typed: Option<&BranchValidation>,
) -> Vec<BaseItem> {
    let mut rows = base_items(branches);
    let query = query.trim();
    let listed = branches.iter().any(|branch| branch.name == query);
    if !query.is_empty()
        && !listed
        && let Some(validation) = typed.filter(|validation| validation.exists)
    {
        let ref_name = validation
            .resolved_ref
            .clone()
            .unwrap_or_else(|| query.to_string());
        rows.push(BaseItem {
            name: query.to_string(),
            label: format!("{query}, branch"),
            ref_name,
        });
    }
    rows
}

/// What a submit reads from the form.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Form {
    /// The chosen project id and folder.
    pub project: Option<(String, String)>,
    pub multiplicity: bool,
    pub is_git: Option<bool>,
    pub isolation: Isolation,
    /// The picked base ref, else the default from the current branch.
    pub base_ref: Option<String>,
    pub worktree_slug: String,
    pub title: Option<String>,
}

/// `isEmptyWorkspaceSubmission`: no text, images, or attachments.
pub fn is_empty_submission(submission: &DraftSubmission) -> bool {
    submission.text.trim().is_empty()
        && submission.images.is_empty()
        && submission.attachments.is_empty()
}

/// The one `workspace.create.request` of a submit.
pub fn build_request(form: &Form, submission: &DraftSubmission) -> Result<CreateWorkspace, String> {
    let Some((project_id, cwd)) = form.project.clone() else {
        return Err(CHOOSE_PROJECT.to_string());
    };
    let shown = visibility(form.multiplicity, form.is_git, form.isolation);
    let source = if shown.creates_worktree {
        // The daemon would answer with an error for a folder that is not a git checkout.
        if form.is_git != Some(true) {
            return Err(CREATE_WORKTREE_FAILED.to_string());
        }
        WorkspaceSource::Worktree {
            cwd,
            project_id: Some(project_id),
            action: form.base_ref.as_ref().map(|_| WorktreeAction::BranchOff),
            ref_name: form.base_ref.clone(),
            base_branch: None,
            branch_name: None,
            worktree_slug: Some(form.worktree_slug.clone()),
        }
    } else {
        WorkspaceSource::Directory {
            path: cwd,
            project_id: Some(project_id),
        }
    };
    let (agent, first_agent_prompt) = if is_empty_submission(submission) {
        (None, None)
    } else {
        let Some(config) = submission.config.clone() else {
            return Err(SELECT_MODEL.to_string());
        };
        let agent = InitialAgent {
            config,
            feature_values: submission.feature_values.clone(),
            env: submission.environment.clone(),
            initial_prompt: submission.text.clone(),
            images: submission.images.clone(),
            attachments: submission.attachments.clone(),
            labels: BTreeMap::new(),
        };
        (Some(agent), Some(submission.text.clone()))
    };
    Ok(CreateWorkspace {
        source,
        title: form.title.clone(),
        first_agent_prompt,
        agent,
    })
}

/// One submit per draft (`pendingWorkspaceSubmissions`): a second submit while the first is
/// pending sends nothing.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct SubmitGuard {
    pending: bool,
}

impl SubmitGuard {
    /// True when the submit may send; the guard then holds until `finish`.
    pub fn begin(&mut self) -> bool {
        !std::mem::replace(&mut self.pending, true)
    }

    pub fn finish(&mut self) {
        self.pending = false;
    }

    pub fn pending(&self) -> bool {
        self.pending
    }
}

/// The screen created a workspace; the shell opens it when the client lists it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Created {
    pub key: String,
    pub created: CreatedWorkspace,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Picker {
    Project,
    Isolation,
    Base,
}

/// The checkout of the chosen project: git or not, and its branch.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
struct Checkout {
    root: String,
    is_git: Option<bool>,
    branch: Option<String>,
}

pub struct NewWorkspaceScreen {
    client: Entity<Client>,
    key: String,
    preset: NewWorkspacePreset,
    composer: Entity<Composer>,
    project_id: Option<String>,
    /// A project `add_project` made that the client does not list yet.
    added: Option<ProjectOption>,
    isolation: Isolation,
    checkout: Checkout,
    checkout_generation: u64,
    /// The picked base; `None` uses the default from the branch.
    base: Option<BaseItem>,
    picker: Option<Picker>,
    base_query: Entity<InputState>,
    branches: Vec<BranchSuggestion>,
    typed: Option<BranchValidation>,
    branches_loading: bool,
    branch_generation: u64,
    slug: String,
    guard: SubmitGuard,
    error: Option<String>,
    /// The side panel shows; the shell keeps the choice for the session.
    side_panel_open: bool,
    /// The panel a draft pill asked for, as plugin id and panel id.
    side_panel_request: Option<(&'static str, &'static str)>,
    side_panel_view: Option<(SidePanelKey, ViewFactory<NewWorkspacePanelInput>, AnyView)>,
    _subscriptions: Vec<Subscription>,
}

type SidePanelKey = (&'static str, &'static str, NewWorkspacePanelInput);

/// The side panel was shown or hidden from the screen.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct SidePanelChanged(pub bool);

impl EventEmitter<Created> for NewWorkspaceScreen {}
impl EventEmitter<SidePanelChanged> for NewWorkspaceScreen {}

impl NewWorkspaceScreen {
    pub fn new(
        client: Entity<Client>,
        key: String,
        preset: NewWorkspacePreset,
        window: &mut Window,
        cx: &mut Context<NewWorkspaceScreen>,
    ) -> NewWorkspaceScreen {
        let project_id = default_project(client.read(cx).projects(), &preset);
        let composer = cx.new(|cx| {
            Composer::new_draft(
                client.clone(),
                key.clone(),
                DraftOptions::default(),
                window,
                cx,
            )
        });
        let base_query =
            cx.new(|cx| InputState::new(window, cx).placeholder("Search branches and PRs"));
        let subscriptions = vec![
            cx.subscribe_in(
                &composer,
                window,
                |this: &mut NewWorkspaceScreen, _, event: &ComposerEvent, window, cx| match event {
                    ComposerEvent::DraftSubmit => this.submit(window, cx),
                    ComposerEvent::ShowNewWorkspacePanel {
                        plugin_id,
                        panel_id,
                    } => {
                        this.side_panel_request = Some((plugin_id, panel_id));
                        this.set_side_panel_open(true, cx);
                        cx.emit(SidePanelChanged(true));
                    }
                    _ => {}
                },
            ),
            cx.subscribe(
                &base_query,
                |this: &mut NewWorkspaceScreen, _, event: &InputEvent, cx| {
                    if matches!(event, InputEvent::Change) {
                        this.load_branches(cx);
                    }
                },
            ),
            cx.subscribe(
                &client,
                |this: &mut NewWorkspaceScreen, _, event: &ClientEvent, cx| {
                    // A request sent before `Connected` is lost; load again after each connect.
                    if matches!(event, ClientEvent::Connected) {
                        this.checkout = Checkout::default();
                        this.sync_project(cx);
                        cx.notify();
                    }
                },
            ),
            cx.subscribe(
                &client,
                |this: &mut NewWorkspaceScreen, _, event: &DirectoryEvent, cx| {
                    if *event == DirectoryEvent::ProjectsChanged {
                        this.sync_project(cx);
                        cx.notify();
                    }
                },
            ),
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
        ];
        // The shell passes the session choice after creation.
        let side_panel_open = false;
        let mut subscriptions = subscriptions;
        if let Some(host) = bachuc_plugin_host::host(cx) {
            subscriptions.push(cx.subscribe(
                &host,
                |_: &mut NewWorkspaceScreen, _, event: &PluginHostEvent, cx| {
                    if matches!(event, PluginHostEvent::ContributionsChanged) {
                        cx.notify();
                    }
                },
            ));
        }
        let mut screen = NewWorkspaceScreen {
            client,
            key,
            preset,
            composer,
            project_id,
            added: None,
            isolation: Isolation::default(),
            checkout: Checkout::default(),
            checkout_generation: 0,
            base: None,
            picker: None,
            base_query,
            branches: Vec::new(),
            typed: None,
            branches_loading: false,
            branch_generation: 0,
            slug: worktree_slug(uuid::Uuid::new_v4()),
            guard: SubmitGuard::default(),
            error: None,
            side_panel_open,
            side_panel_request: None,
            side_panel_view: None,
            _subscriptions: subscriptions,
        };
        screen.sync_project(cx);
        screen
    }

    /// The header toggle shows once a project is chosen and a plugin has a panel.
    pub fn side_panel_available(&self, cx: &App) -> bool {
        let panels = bachuc_plugin_host::host(cx)
            .map_or(0, |host| host.read(cx).index().new_workspace_panels().len());
        side_panel_available(self.side_panel_target().as_ref(), panels)
    }

    pub fn side_panel_open(&self) -> bool {
        self.side_panel_open
    }

    pub fn set_side_panel_open(&mut self, open: bool, cx: &mut Context<Self>) {
        if self.side_panel_open != open {
            self.side_panel_open = open;
            cx.notify();
        }
    }

    /// The open side panel: the requested panel, else the first, built once per input.
    fn side_panel(&mut self, window: &mut Window, cx: &mut Context<Self>) -> Option<AnyElement> {
        if !self.side_panel_open {
            return None;
        }
        let (project_id, cwd) = self.side_panel_target()?;
        let index = bachuc_plugin_host::host(cx)?.read(cx).index().clone();
        let panels = index.new_workspace_panels();
        let ids: Vec<(&str, &str)> = panels
            .iter()
            .map(|entry| (entry.plugin_id, entry.value.id))
            .collect();
        let entry = &panels[side_panel_index(&ids, self.side_panel_request)?];
        let input = NewWorkspacePanelInput { project_id, cwd };
        let key = (entry.plugin_id, entry.value.id, input.clone());
        let cached = self
            .side_panel_view
            .as_ref()
            .filter(|(cached, factory, _)| {
                *cached == key && Rc::ptr_eq(factory, &entry.value.view)
            });
        let view = match cached {
            Some((_, _, view)) => view.clone(),
            None => {
                let view = (entry.value.view)(input, window, cx);
                self.side_panel_view = Some((key, entry.value.view.clone(), view.clone()));
                view
            }
        };
        let t = tokens(cx);
        Some(
            div()
                .id("new-workspace-side-panel")
                .role(Role::Complementary)
                .aria_label(entry.value.title.clone())
                .flex_none()
                .w(px(SIDE_PANEL_WIDTH))
                .h_full()
                .border_l_1()
                .border_color(t.border)
                .bg(t.surface0)
                .child(view)
                .into_any_element(),
        )
    }

    pub fn composer(&self) -> Entity<Composer> {
        self.composer.clone()
    }

    /// The chosen project id and its folder, for the New workspace side panel.
    pub fn side_panel_target(&self) -> Option<(String, String)> {
        let id = self.project_id.clone()?;
        Some((id, self.checkout.root.clone())).filter(|(_, root)| !root.is_empty())
    }

    pub fn focus(&self, window: &mut Window, cx: &mut App) {
        self.composer()
            .update(cx, |composer, cx| composer.focus(window, cx));
    }

    fn project(&self, cx: &App) -> Option<ProjectOption> {
        let id = self.project_id.as_deref()?;
        project_options(self.client.read(cx).projects())
            .into_iter()
            .find(|option| option.id == id)
            .or_else(|| self.added.clone().filter(|added| added.id == id))
    }

    fn multiplicity(&self, cx: &App) -> bool {
        self.client
            .read(cx)
            .server_info()
            .and_then(|info| info.features.as_ref())
            .and_then(|features| features.workspace_multiplicity)
            .unwrap_or(false)
    }

    /// Picks the default project when none is chosen, and loads the checkout of the chosen one.
    fn sync_project(&mut self, cx: &mut Context<Self>) {
        if self.project(cx).is_none() {
            self.project_id = default_project(self.client.read(cx).projects(), &self.preset)
                .or_else(|| self.added.as_ref().map(|added| added.id.clone()));
        }
        let root = self.project(cx).map(|project| project.root);
        let Some(root) = root else {
            self.checkout = Checkout::default();
            self.composer
                .update(cx, |composer, cx| composer.set_draft_cwd(None, cx));
            return;
        };
        if self.checkout.root == root && self.checkout.is_git.is_some() {
            return;
        }
        self.checkout = Checkout {
            root: root.clone(),
            is_git: None,
            branch: None,
        };
        self.composer.update(cx, |composer, cx| {
            composer.set_draft_cwd(Some(root.clone()), cx)
        });
        if !self.client.read(cx).is_connected() {
            return;
        }
        self.checkout_generation += 1;
        let generation = self.checkout_generation;
        let task = self
            .client
            .update(cx, |client, cx| client.checkout_details(&root, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.checkout_generation != generation {
                    return;
                }
                match result {
                    Ok(details) => {
                        this.checkout.is_git = Some(details.is_git);
                        this.checkout.branch = details.branch;
                    }
                    Err(error) => {
                        log::warn!("checkout status of {}: {error:#}", this.checkout.root);
                        this.checkout.is_git = Some(false);
                    }
                }
                cx.notify();
            });
        })
        .detach();
    }

    fn select_project(&mut self, id: String, cx: &mut Context<Self>) {
        self.picker = None;
        if self.project_id.as_deref() != Some(id.as_str()) {
            self.project_id = Some(id);
            self.base = None;
            self.error = None;
            self.sync_project(cx);
        }
        cx.notify();
    }

    /// "Add project": a folder dialog, then `add_project`, then the new project is chosen.
    fn add_project(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        self.picker = None;
        let picked = cx.prompt_for_paths(PathPromptOptions {
            files: false,
            directories: true,
            multiple: false,
            prompt: Some("Add project".into()),
        });
        let client = self.client.clone();
        cx.spawn_in(window, async move |this, cx| {
            let Ok(Ok(Some(paths))) = picked.await else {
                return;
            };
            let Some(path) = paths.first().map(|path| path.to_string_lossy().to_string()) else {
                return;
            };
            let Ok(task) = cx.update(|_, cx| client.update(cx, |c, cx| c.add_project(&path, cx)))
            else {
                return;
            };
            let result = task.await;
            let _ = this.update(cx, |this, cx| match result {
                Ok(added) => {
                    this.added = Some(ProjectOption {
                        id: added.id.clone(),
                        name: added.name,
                        root: added.root,
                    });
                    this.select_project(added.id, cx);
                }
                Err(error) => {
                    this.error = Some(format!("{error:#}"));
                    cx.notify();
                }
            });
        })
        .detach();
        cx.notify();
    }

    fn open_picker(&mut self, picker: Picker, window: &mut Window, cx: &mut Context<Self>) {
        if self.picker == Some(picker) {
            self.picker = None;
        } else {
            self.picker = Some(picker);
            if picker == Picker::Base {
                self.base_query
                    .update(cx, |query, cx| query.focus(window, cx));
                self.load_branches(cx);
            }
        }
        cx.notify();
    }

    /// Loads the branches for the typed query, and checks a typed name the list lacks.
    fn load_branches(&mut self, cx: &mut Context<Self>) {
        let root = self.checkout.root.clone();
        if root.is_empty() {
            return;
        }
        let query = self.base_query.read(cx).value().trim().to_string();
        self.branch_generation += 1;
        let generation = self.branch_generation;
        self.branches_loading = true;
        let search = self.client.update(cx, |client, cx| {
            client.branch_suggestions(
                &root,
                Some(query.as_str()).filter(|query| !query.is_empty()),
                Some(BRANCH_LIMIT),
                cx,
            )
        });
        let validate = (!query.is_empty()).then(|| {
            self.client
                .update(cx, |client, cx| client.validate_branch(&root, &query, cx))
        });
        cx.spawn(async move |this, cx| {
            let branches = search.await;
            let typed = match validate {
                Some(task) => task.await.ok(),
                None => None,
            };
            let _ = this.update(cx, |this, cx| {
                if this.branch_generation != generation {
                    return;
                }
                this.branches_loading = false;
                this.branches = branches.unwrap_or_default();
                this.typed = typed;
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    fn form(&self, cx: &App) -> Form {
        let base_ref = self
            .base
            .as_ref()
            .map(|base| base.ref_name.clone())
            .or_else(|| default_base_ref(self.checkout.branch.as_deref()));
        Form {
            project: self.project(cx).map(|project| (project.id, project.root)),
            multiplicity: self.multiplicity(cx),
            is_git: self.checkout.is_git,
            isolation: self.isolation,
            base_ref,
            worktree_slug: self.slug.clone(),
            title: self.preset.title.clone(),
        }
    }

    fn submit(&mut self, _window: &mut Window, cx: &mut Context<Self>) {
        let Some(submission) = self.composer.read(cx).draft_submission(cx) else {
            return;
        };
        if !self.guard.begin() {
            return;
        }
        let request = match build_request(&self.form(cx), &submission) {
            Ok(request) => request,
            Err(message) => {
                self.guard.finish();
                self.error = Some(message);
                cx.notify();
                return;
            }
        };
        self.error = None;
        self.picker = None;
        self.composer
            .update(cx, |composer, cx| composer.set_submitting(true, cx));
        let task = self
            .client
            .update(cx, |client, cx| client.create_workspace(request, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                this.guard.finish();
                this.composer
                    .update(cx, |composer, cx| composer.set_submitting(false, cx));
                match result {
                    Ok(created) => cx.emit(Created {
                        key: this.key.clone(),
                        created,
                    }),
                    Err(error) => this.error = Some(format!("{error:#}")),
                }
                cx.notify();
            });
        })
        .detach();
        cx.notify();
    }

    /// A picker chip (`badge`): 28 px, a muted label, and the shared extra-muted chevron.
    fn chip(
        &self,
        id: &'static str,
        label: String,
        text: String,
        glyph: IconName,
        picker: Picker,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let t = tokens(cx);
        let disabled = self.guard.pending();
        let open = self.picker == Some(picker);
        h_flex()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .aria_expanded(open)
            .flex_shrink(1.)
            .min_w_0()
            .h(px(BADGE_HEIGHT))
            .max_w(px(BADGE_MAX_WIDTH))
            .overflow_hidden()
            .px(px(SPACE_2))
            .gap(px(SPACE_1))
            .items_center()
            .rounded(px(RADIUS_2XL))
            .when(open, |chip| chip.bg(t.surface2))
            .when(disabled, |chip| chip.opacity(BADGE_DISABLED_OPACITY))
            .when(!disabled, |chip| {
                chip.cursor_pointer()
                    .hover(|style| style.bg(t.surface2))
                    .active(|style| style.bg(t.surface0))
                    .on_click(
                        cx.listener(move |this, _, window, cx| {
                            this.open_picker(picker, window, cx)
                        }),
                    )
            })
            .child(
                div()
                    .flex_none()
                    .size(px(ICON_MD))
                    .flex()
                    .items_center()
                    .justify_center()
                    .child(
                        icon(glyph)
                            .with_size(px(ICON_SM))
                            .text_color(t.foreground_muted),
                    ),
            )
            .child(
                div()
                    .min_w_0()
                    .flex_shrink(1.)
                    .truncate()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(t.foreground_muted)
                    .child(text),
            )
            .child(
                // `MetaChevron`: 1 px lower.
                div().flex_none().relative().top(px(1.)).child(
                    icon(IconName::ChevronDown)
                        .with_size(px(ICON_SM))
                        .text_color(t.foreground_extra_muted),
                ),
            )
    }

    fn row(
        &self,
        id: (&'static str, usize),
        label: String,
        text: String,
        checked: bool,
        glyph: Option<IconName>,
        cx: &App,
    ) -> Stateful<Div> {
        let t = tokens(cx);
        h_flex()
            .id(id)
            .role(Role::MenuItem)
            .aria_label(if checked {
                format!("{label}, selected")
            } else {
                label
            })
            .aria_selected(checked)
            .min_h(px(BADGE_HEIGHT))
            .px(px(SPACE_2))
            .gap(px(SPACE_2))
            .items_center()
            .rounded(px(RADIUS_MD))
            .cursor_pointer()
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_color(t.foreground)
            .hover(move |row| row.bg(t.surface2))
            .child(
                div()
                    .flex_none()
                    .w(px(ICON_SM))
                    .when_some(glyph, |slot, glyph| {
                        slot.child(
                            icon(glyph)
                                .with_size(px(ICON_SM))
                                .text_color(t.foreground_muted),
                        )
                    })
                    .when(glyph.is_none() && checked, |slot| {
                        slot.child(icon(IconName::Check).with_size(px(ICON_SM)))
                    }),
            )
            .child(div().flex_1().min_w_0().truncate().child(text))
    }

    /// The list under an open chip, closed by a press outside it.
    fn picker_list(&self, picker: Picker, cx: &mut Context<Self>) -> AnyElement {
        let t = tokens(cx);
        let rows: Vec<AnyElement> = match picker {
            Picker::Project => {
                let mut rows: Vec<AnyElement> = project_options(self.client.read(cx).projects())
                    .into_iter()
                    .enumerate()
                    .map(|(ix, option)| {
                        let checked = self.project_id.as_deref() == Some(option.id.as_str());
                        let id = option.id.clone();
                        self.row(
                            ("new-workspace-project", ix),
                            format!("Project {}", option.name),
                            option.name,
                            checked,
                            None,
                            cx,
                        )
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.select_project(id.clone(), cx)),
                        )
                        .into_any_element()
                    })
                    .collect();
                rows.push(
                    self.row(
                        ("new-workspace-add-project", 0),
                        "Add project".into(),
                        "Add project".into(),
                        false,
                        Some(IconName::FolderPlus),
                        cx,
                    )
                    .on_click(cx.listener(|this, _, window, cx| this.add_project(window, cx)))
                    .into_any_element(),
                );
                rows
            }
            Picker::Isolation => [Isolation::Local, Isolation::Worktree]
                .into_iter()
                .enumerate()
                .map(|(ix, isolation)| {
                    self.row(
                        ("new-workspace-isolation", ix),
                        isolation.label().into(),
                        isolation.label().into(),
                        self.isolation == isolation,
                        None,
                        cx,
                    )
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.isolation = isolation;
                        this.picker = None;
                        cx.notify();
                    }))
                    .into_any_element()
                })
                .collect(),
            Picker::Base => {
                let query = self.base_query.read(cx).value().to_string();
                let current = self.form(cx).base_ref;
                let rows: Vec<AnyElement> = base_rows(&self.branches, &query, self.typed.as_ref())
                    .into_iter()
                    .enumerate()
                    .map(|(ix, item)| {
                        let checked = current.as_deref() == Some(item.ref_name.as_str());
                        let picked = item.clone();
                        self.row(
                            ("new-workspace-base", ix),
                            item.label,
                            item.name,
                            checked,
                            None,
                            cx,
                        )
                        .on_click(cx.listener(move |this, _, _, cx| {
                            this.base = Some(picked.clone());
                            this.picker = None;
                            cx.notify();
                        }))
                        .into_any_element()
                    })
                    .collect();
                let empty = if self.branches_loading {
                    "Searching..."
                } else {
                    "No matching refs."
                };
                let mut list: Vec<AnyElement> = vec![
                    div()
                        .px(px(SPACE_1))
                        .pb(px(SPACE_1))
                        .child(
                            Input::new(&self.base_query)
                                .accessibility_id("new-workspace-base-search")
                                .aria_label("Search branches and PRs"),
                        )
                        .into_any_element(),
                ];
                if rows.is_empty() {
                    list.push(
                        div()
                            .id("new-workspace-base-empty")
                            .role(Role::Status)
                            .aria_label(empty)
                            .px(px(SPACE_2))
                            .py(px(SPACE_1))
                            .text_size(ui_font_size(FONT_BASE, cx))
                            .text_color(t.foreground_muted)
                            .child(empty)
                            .into_any_element(),
                    );
                }
                list.extend(rows);
                list
            }
        };
        let title = match picker {
            Picker::Project => "Project",
            Picker::Isolation => "Isolation",
            Picker::Base => "Start from",
        };
        deferred(
            anchored().snap_to_window_with_margin(px(8.)).child(
                v_flex()
                    .id("new-workspace-picker")
                    .role(Role::Menu)
                    .aria_label(title)
                    .mt(px(BADGE_HEIGHT + SPACE_1))
                    .w(px(PICKER_WIDTH))
                    .max_h(px(PICKER_MAX_HEIGHT))
                    .overflow_y_scroll()
                    .p(px(SPACE_1))
                    .bg(t.popover)
                    .border_1()
                    .border_color(t.border)
                    .rounded(px(RADIUS_LG))
                    .shadow(vec![shadow(ShadowSize::Md, cx)])
                    .occlude()
                    .on_mouse_down_out(cx.listener(|this, _, _, cx| {
                        this.picker = None;
                        cx.notify();
                    }))
                    .children(rows),
            ),
        )
        .with_priority(gpui_kit::base::POPUP_PRIORITY)
        .into_any_element()
    }

    /// The chip with its list when open.
    fn control(&self, chip: Stateful<Div>, picker: Picker, cx: &mut Context<Self>) -> Div {
        let list = (self.picker == Some(picker)).then(|| self.picker_list(picker, cx));
        div()
            .relative()
            .min_w_0()
            .flex_shrink(1.)
            .child(chip)
            .children(list)
    }

    fn form_row(&self, cx: &mut Context<Self>) -> AnyElement {
        let project = self.project(cx);
        let form = self.form(cx);
        let shown = visibility(form.multiplicity, form.is_git, self.isolation);
        let project_name = project.as_ref().map_or_else(
            || CHOOSE_PROJECT.to_string(),
            |project| project.name.clone(),
        );
        let project_chip = self.chip(
            "new-workspace-project-chip",
            format!("Project: {project_name}"),
            project_name,
            IconName::Folder,
            Picker::Project,
            cx,
        );
        let isolation = if shown.creates_worktree {
            Isolation::Worktree
        } else {
            Isolation::Local
        };
        let isolation_chip = shown.isolation_chip.then(|| {
            self.chip(
                "new-workspace-isolation-chip",
                format!("Isolation: {}", isolation.label()),
                isolation.label().into(),
                match isolation {
                    Isolation::Worktree => IconName::GitBranch,
                    Isolation::Local => IconName::Folder,
                },
                Picker::Isolation,
                cx,
            )
        });
        let base_chip = shown.base_chip.then(|| {
            let name = form.base_ref.as_deref().map(branch_name_from_ref);
            let text = name.unwrap_or("Starting ref").to_string();
            self.chip(
                "new-workspace-base-chip",
                format!("Starting ref: {text}"),
                text,
                IconName::GitBranch,
                Picker::Base,
                cx,
            )
        });
        h_flex()
            .id("new-workspace-form")
            .role(Role::Group)
            .aria_label("New workspace options")
            .items_center()
            .mb(px(SPACE_8))
            .px(px(SPACE_4))
            .gap(px(SPACE_2))
            .child(self.control(project_chip, Picker::Project, cx))
            .when_some(isolation_chip, |row, chip| {
                row.child(self.control(chip, Picker::Isolation, cx))
            })
            .when_some(base_chip, |row, chip| {
                row.child(self.control(chip, Picker::Base, cx))
            })
            .into_any_element()
    }
}

impl Render for NewWorkspaceScreen {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let side_panel = self.side_panel(window, cx);
        let title = div().mb(px(SPACE_8)).pl(px(SPACE_6)).pr(px(SPACE_4)).child(
            div()
                .id("new-workspace-title")
                .role(Role::Heading)
                .aria_label("New workspace")
                .text_size(ui_font_size(FONT_2XL, cx))
                .line_height(px(TITLE_LINE_HEIGHT))
                .font_weight(FontWeight(WEIGHT_NORMAL))
                .text_color(t.foreground)
                .child("New workspace"),
        );
        let setup = v_flex()
            .child(title)
            .child(self.form_row(cx))
            .into_any_element();
        let composer = v_flex()
            .w_full()
            .child(self.composer.clone())
            .children(self.error.clone().map(|message| {
                div()
                    .id("new-workspace-error")
                    .role(Role::Alert)
                    .aria_label(format!("Error: {message}"))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .line_height(px(ERROR_LINE_HEIGHT))
                    .text_color(t.destructive)
                    .child(message)
            }))
            .into_any_element();
        div()
            .id("new-workspace-screen")
            .role(Role::Group)
            .aria_label("New workspace")
            .size_full()
            .flex()
            .flex_row()
            .bg(t.surface0)
            .child(
                div()
                    .relative()
                    .flex_1()
                    .min_w_0()
                    .h_full()
                    .child(composer_dock(true, setup, composer)),
            )
            .children(side_panel)
    }
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob brings a `test` macro that shadows the std one.
    use super::{
        BADGE_HEIGHT, BaseItem, CHOOSE_PROJECT, CREATE_WORKTREE_FAILED, Form, Isolation,
        NewWorkspacePreset, SELECT_MODEL, SPACE_8, SubmitGuard, TITLE_LINE_HEIGHT, Visibility,
        base_items, base_rows, branch_name_from_ref, build_request, default_base_ref,
        default_project, project_options, visibility, worktree_slug,
    };
    use bachuc_client::{
        BranchSuggestion, BranchValidation, DraftConfig, ProjectNode, WorkspaceNode,
        WorkspaceSource, WorktreeAction,
    };
    use bachuc_composer::DraftSubmission;
    use serde_json::json;

    fn config() -> DraftConfig {
        DraftConfig {
            provider: "claude".into(),
            cwd: "/repo".into(),
            model: Some("opus".into()),
            mode_id: Some("plan".into()),
            thinking_option_id: Some("high".into()),
        }
    }

    fn form(isolation: Isolation, multiplicity: bool, is_git: Option<bool>) -> Form {
        Form {
            project: Some(("p1".into(), "/repo".into())),
            multiplicity,
            is_git,
            isolation,
            base_ref: default_base_ref(Some("main")),
            worktree_slug: "a1b2c3d4".into(),
            title: None,
        }
    }

    fn empty() -> DraftSubmission {
        DraftSubmission {
            config: Some(config()),
            ..DraftSubmission::default()
        }
    }

    fn text(prompt: &str) -> DraftSubmission {
        DraftSubmission {
            text: prompt.into(),
            config: Some(config()),
            feature_values: [("fast".to_string(), json!(true))].into(),
            environment: [("PLUGIN".to_string(), "1".to_string())].into(),
            ..DraftSubmission::default()
        }
    }

    #[test]
    fn the_chip_row_centre_sits_70_px_below_the_title_top() {
        // Electron: a 24 px title line, `marginBottom` 32, and half of the 28 px chip.
        assert_eq!(TITLE_LINE_HEIGHT + SPACE_8 + BADGE_HEIGHT / 2., 70.);
    }

    #[test]
    fn empty_text_on_local_creates_a_directory_workspace_with_no_agent() {
        let request = build_request(&form(Isolation::Local, true, Some(true)), &empty()).unwrap();
        assert_eq!(
            request.source,
            WorkspaceSource::Directory {
                path: "/repo".into(),
                project_id: Some("p1".into()),
            }
        );
        assert!(request.agent.is_none());
        assert!(request.first_agent_prompt.is_none());
    }

    #[test]
    fn text_on_a_new_worktree_branches_off_main_with_a_slug_and_the_first_agent() {
        let request = build_request(
            &form(Isolation::Worktree, true, Some(true)),
            &text("fix it"),
        )
        .unwrap();
        assert_eq!(
            request.source,
            WorkspaceSource::Worktree {
                cwd: "/repo".into(),
                project_id: Some("p1".into()),
                action: Some(WorktreeAction::BranchOff),
                ref_name: Some("refs/heads/main".into()),
                base_branch: None,
                branch_name: None,
                worktree_slug: Some("a1b2c3d4".into()),
            }
        );
        let agent = request.agent.expect("first agent");
        assert_eq!(agent.config, config());
        assert_eq!(agent.initial_prompt, "fix it");
        assert_eq!(agent.feature_values["fast"], json!(true));
        assert_eq!(agent.env["PLUGIN"], "1");
        assert_eq!(request.first_agent_prompt.as_deref(), Some("fix it"));
    }

    #[test]
    fn images_alone_still_create_the_first_agent() {
        let submission = DraftSubmission {
            images: vec![json!({"data": "x"})],
            ..empty()
        };
        let request =
            build_request(&form(Isolation::Local, true, Some(true)), &submission).unwrap();
        assert_eq!(request.agent.expect("agent").images.len(), 1);
    }

    #[test]
    fn a_non_git_project_shows_no_isolation_or_base_chip_and_creates_a_directory() {
        let shown = visibility(true, Some(false), Isolation::Worktree);
        assert_eq!(
            shown,
            Visibility {
                isolation_chip: false,
                base_chip: false,
                creates_worktree: false,
            }
        );
        let request = build_request(&form(Isolation::Worktree, true, Some(false)), &text("hi"));
        assert!(matches!(
            request.unwrap().source,
            WorkspaceSource::Directory { .. }
        ));
    }

    #[test]
    fn the_isolation_and_base_chips_follow_git_and_the_choice() {
        let local = visibility(true, Some(true), Isolation::Local);
        assert!(local.isolation_chip && !local.base_chip && !local.creates_worktree);
        let worktree = visibility(true, Some(true), Isolation::Worktree);
        assert!(worktree.isolation_chip && worktree.base_chip && worktree.creates_worktree);
    }

    #[test]
    fn without_multiplicity_the_base_chip_shows_and_the_request_is_a_worktree() {
        let shown = visibility(false, Some(true), Isolation::Local);
        assert!(!shown.isolation_chip && shown.base_chip && shown.creates_worktree);
        let request = build_request(&form(Isolation::Local, false, Some(true)), &empty()).unwrap();
        assert!(matches!(request.source, WorkspaceSource::Worktree { .. }));
    }

    #[test]
    fn empty_text_on_a_non_git_project_with_a_forced_worktree_builds_no_request() {
        let result = build_request(&form(Isolation::Local, false, Some(false)), &empty());
        assert_eq!(result.unwrap_err(), CREATE_WORKTREE_FAILED);
    }

    #[test]
    fn no_project_asks_for_one_and_text_without_a_model_asks_for_one() {
        let mut no_project = form(Isolation::Local, true, Some(true));
        no_project.project = None;
        assert_eq!(
            build_request(&no_project, &text("hi")).unwrap_err(),
            CHOOSE_PROJECT
        );
        let no_model = DraftSubmission {
            config: None,
            ..text("hi")
        };
        assert_eq!(
            build_request(&form(Isolation::Local, true, Some(true)), &no_model).unwrap_err(),
            SELECT_MODEL
        );
        // An empty submit needs no model.
        let empty_no_model = DraftSubmission::default();
        assert!(build_request(&form(Isolation::Local, true, Some(true)), &empty_no_model).is_ok());
    }

    #[test]
    fn the_guard_sends_one_request_for_two_submits_and_again_after_the_first_resolves() {
        let mut guard = SubmitGuard::default();
        let mut sent = 0;
        for _ in 0..2 {
            if guard.begin() {
                sent += 1;
            }
        }
        assert_eq!(sent, 1);
        assert!(guard.pending());
        guard.finish();
        if guard.begin() {
            sent += 1;
        }
        assert_eq!(sent, 2);
    }

    #[test]
    fn the_base_defaults_to_the_current_branch() {
        assert_eq!(
            default_base_ref(Some("main")).as_deref(),
            Some("refs/heads/main")
        );
        assert_eq!(default_base_ref(None), None);
        assert_eq!(default_base_ref(Some("")), None);
        assert_eq!(branch_name_from_ref("refs/heads/feat/x"), "feat/x");
        assert_eq!(branch_name_from_ref("refs/remotes/origin/main"), "main");
        assert_eq!(branch_name_from_ref("main"), "main");
    }

    #[test]
    fn the_slug_is_eight_hex_digits() {
        let slug = worktree_slug(uuid::Uuid::new_v4());
        assert_eq!(slug.len(), 8);
        assert!(slug.chars().all(|c| c.is_ascii_hexdigit()));
        assert_ne!(slug, worktree_slug(uuid::Uuid::new_v4()));
    }

    fn project(id: &str, name: &str, root: &str) -> ProjectNode {
        ProjectNode {
            id: id.into(),
            name: name.into(),
            root: root.into(),
            workspaces: vec![WorkspaceNode {
                id: format!("{id}-w"),
                name: "w".into(),
                cwd: format!("{root}/wt"),
                branch: None,
                pinned: false,
                agent_ids: Vec::new(),
            }],
        }
    }

    #[test]
    fn the_project_default_is_the_preset_then_its_folder_then_the_first_by_name() {
        let projects = [project("b", "beta", "/b"), project("a", "Alpha", "/a")];
        assert_eq!(
            project_options(&projects)
                .iter()
                .map(|option| option.name.as_str())
                .collect::<Vec<_>>(),
            ["Alpha", "beta"]
        );
        let none = NewWorkspacePreset::default();
        assert_eq!(default_project(&projects, &none).as_deref(), Some("a"));
        let preset = NewWorkspacePreset {
            project_id: Some("b".into()),
            ..NewWorkspacePreset::default()
        };
        assert_eq!(default_project(&projects, &preset).as_deref(), Some("b"));
        let folder = NewWorkspacePreset {
            cwd: Some("/b/wt".into()),
            ..NewWorkspacePreset::default()
        };
        assert_eq!(default_project(&projects, &folder).as_deref(), Some("b"));
        let unknown = NewWorkspacePreset {
            project_id: Some("gone".into()),
            ..NewWorkspacePreset::default()
        };
        assert_eq!(default_project(&projects, &unknown).as_deref(), Some("a"));
        assert_eq!(default_project(&[], &none), None);
    }

    fn branch(name: &str, local: Option<bool>, remote: Option<bool>) -> BranchSuggestion {
        BranchSuggestion {
            name: name.into(),
            committer_date: None,
            has_local: local,
            has_remote: remote,
            local_ahead: None,
            local_behind: None,
        }
    }

    #[test]
    fn base_rows_list_origin_then_local_and_a_typed_branch_only_when_it_exists() {
        let rows = base_items(&[branch("main", Some(true), Some(true))]);
        assert_eq!(
            rows.iter()
                .map(|row| row.ref_name.as_str())
                .collect::<Vec<_>>(),
            ["refs/remotes/origin/main", "refs/heads/main"]
        );
        let mut same = branch("dev", Some(true), Some(true));
        same.local_ahead = Some(0);
        same.local_behind = Some(0);
        assert_eq!(base_items(&[same]).len(), 1);
        assert_eq!(
            base_items(&[branch("plain", None, None)]),
            [BaseItem {
                name: "plain".into(),
                ref_name: "plain".into(),
                label: "plain, branch".into(),
            }]
        );

        let found = BranchValidation {
            exists: true,
            resolved_ref: Some("refs/heads/topic".into()),
            is_remote: false,
        };
        let missing = BranchValidation {
            exists: false,
            resolved_ref: None,
            is_remote: false,
        };
        let listed = [branch("main", Some(true), None)];
        assert_eq!(base_rows(&listed, "topic", Some(&found)).len(), 2);
        assert_eq!(base_rows(&listed, "topic", Some(&missing)).len(), 1);
        assert_eq!(base_rows(&listed, "topic", None).len(), 1);
        assert_eq!(base_rows(&listed, "main", Some(&found)).len(), 1);
    }
}
