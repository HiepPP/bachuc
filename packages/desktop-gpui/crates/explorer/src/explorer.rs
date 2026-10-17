//! The Explorer sidebar: the Files / Changes tab rail of `explorer-sidebar-tab-rail.tsx` over
//! the Files tree (`file-explorer-pane.tsx`) or the Changes tree (`changed-files-tree.tsx`).
//!
//! The view does not open anything itself. A click on a file emits `ExplorerEvent::OpenFile`, a
//! click on a changed file emits `ExplorerDiffEvent::OpenAllDiffs`, a click on a file of a
//! commit emits `ExplorerDiffEvent::OpenCommitDiff`, and the app opens the viewer pane.

use crate::changes::{ChangeRow, CheckoutView};
use crate::commits::{CommitRow, CommitsState, commit_rows, time_ago};
use crate::diff_doc::Latest;
use crate::diff_prefs::DiffPrefs;
use crate::tree::{FileTree, RootState, SortBy, Toggled, TreeRow};
use crate::ui::{
    CHANGE_ROW_HEIGHT, FILE_ROW_HEIGHT, TRAILING_RAIL, TREE_ICON_GAP, centered_text, change_icon,
    chevron, diff_mode_menu, diff_stat, file_icon_element, loading_marker, read_prefs,
    state_message, tree_padding_left, update_prefs,
};
use bachuc_client::{Client, ClientEvent, CommitSummary, DiffCompare, DirectoryEvent};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_SM, RADIUS_MD, SPACE_1, SPACE_2, SPACE_3, SPACE_4, TAB_BAR_HEIGHT,
};
use bachuc_theme::{mono_font_family, tokens, ui_font_size};
use gpui_kit::component::theme::Theme;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::HashSet;
use std::ops::Range;

/// `EXPLORER_TAB_RAIL_INSET`, the space left of the first tab.
const TAB_RAIL_INSET: f32 = 4.;
/// Spacing between two tabs (`TAB_GAP`), half on each side.
const TAB_GAP: f32 = 4.;
/// `HEADER_CONTROL_HEIGHT`, the height of one tab.
const TAB_HEIGHT: f32 = 26.;
/// `WORKSPACE_SECONDARY_HEADER_HEIGHT`, the height of a Changes toolbar row.
const TOOLBAR_HEIGHT: f32 = 36.;
/// `SMALL_ICON_BUTTON_SIZE`, the hitbox of a toolbar icon button.
const TOOLBAR_BUTTON: f32 = 20.;
/// The Files sort trigger height (`sortTrigger`).
const SORT_TRIGGER_HEIGHT: f32 = 24.;
/// `CommitGraphNode`: the dot size, the rail width, and the short sha column.
const COMMIT_DOT: f32 = 8.;
const COMMIT_RAIL: f32 = 2.;
const COMMIT_SHA_WIDTH: f32 = 70.;
/// A commit row: 4 px above and below one 20 px line.
const COMMIT_ROW_HEIGHT: f32 = 28.;
/// The open Commits footer stops growing here and scrolls.
const COMMITS_MAX_HEIGHT: f32 = 240.;

/// What the Explorer tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerEvent {
    /// A file of the Files tree was picked.
    OpenFile { cwd: String, path: String },
    /// A changed file of the Changes tree was picked.
    OpenDiff { cwd: String, path: String },
}

/// The diff requests of the Changes tab, apart from `ExplorerEvent` so its owners keep matching
/// that enum as before.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerDiffEvent {
    /// Open every uncommitted file in one diff, scrolled to `focus_path` when it is set.
    OpenAllDiffs {
        cwd: String,
        focus_path: Option<String>,
    },
    /// Open the diff of one file of one commit.
    OpenCommitDiff {
        cwd: String,
        sha: String,
        path: String,
    },
}

impl ExplorerDiffEvent {
    /// The request for file `file` of `commit`; none when the commit has no such file.
    pub fn commit_file(cwd: &str, commit: &CommitSummary, file: usize) -> Option<Self> {
        let file = commit.files.get(file)?;
        Some(ExplorerDiffEvent::OpenCommitDiff {
            cwd: cwd.to_string(),
            sha: commit.sha.clone(),
            path: file.path.clone(),
        })
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum ExplorerTab {
    Files,
    Changes,
}

enum ChangesState {
    /// Not asked yet.
    Idle,
    Loading,
    Failed(String),
    /// The folder is not in a git repository (`is_git` is false).
    NotGit,
    Ready(CheckoutView),
}

pub struct Explorer {
    client: Entity<Client>,
    cwd: String,
    tab: ExplorerTab,
    visible: bool,
    /// Bumped when `cwd` changes, so an answer for the old folder is dropped.
    generation: u64,
    tree: FileTree,
    rows: Vec<TreeRow>,
    sort: SortBy,
    show_hidden: bool,
    selected_file: Option<String>,
    files_scroll: UniformListScrollHandle,
    changes: ChangesState,
    change_rows: Vec<ChangeRow>,
    collapsed: HashSet<String>,
    selected_change: Option<String>,
    changes_scroll: UniformListScrollHandle,
    /// The status request in flight or last applied; an older answer is dropped.
    status_request: Latest,
    /// The `Base` diff that lists the Committed files, and its request.
    tree_compare: Option<DiffCompare>,
    base_request: Latest,
    /// The Commits footer starts closed, as Electron's does, and loads when it opens.
    commits_open: bool,
    commits: CommitsState,
    commits_request: Latest,
    /// The commits whose files show.
    open_commits: HashSet<String>,
    watching: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ExplorerEvent> for Explorer {}
impl EventEmitter<ExplorerDiffEvent> for Explorer {}

impl Explorer {
    pub fn new(
        client: Entity<Client>,
        cwd: String,
        _window: &mut Window,
        cx: &mut Context<Explorer>,
    ) -> Explorer {
        let subscriptions = vec![
            cx.subscribe(
                &client,
                |this: &mut Explorer, _, event: &DirectoryEvent, cx| {
                    if let DirectoryEvent::CheckoutChanged { cwd } = event
                        && *cwd == this.cwd
                        && this.watching
                    {
                        this.request_status(cx);
                        if this.commits_open {
                            this.request_commits(cx);
                        }
                    }
                },
            ),
            // A request sent while the client connects fails with "not connected", so the views
            // wait for the connection and list again after each one.
            cx.subscribe(
                &client,
                |this: &mut Explorer, _, event: &ClientEvent, cx| {
                    if let ClientEvent::Connected = event {
                        this.refresh(cx);
                    }
                },
            ),
            // set_theme and set_font_scale write this global.
            cx.observe_global::<Theme>(|_, cx| cx.notify()),
            cx.observe_global::<DiffPrefs>(|this, cx| this.prefs_changed(cx)),
            cx.on_release(|this, cx| {
                if this.watching {
                    let cwd = this.cwd.clone();
                    this.client
                        .update(cx, |client, cx| client.unwatch_checkout(&cwd, cx));
                }
            }),
        ];
        let mut explorer = Explorer {
            client,
            cwd,
            tab: ExplorerTab::Files,
            visible: true,
            generation: 0,
            tree: FileTree::new(),
            rows: Vec::new(),
            sort: SortBy::Name,
            // `explorerShowHiddenFiles` defaults to true.
            show_hidden: true,
            selected_file: None,
            files_scroll: UniformListScrollHandle::new(),
            changes: ChangesState::Idle,
            change_rows: Vec::new(),
            collapsed: HashSet::new(),
            selected_change: None,
            changes_scroll: UniformListScrollHandle::new(),
            status_request: Latest::default(),
            tree_compare: None,
            base_request: Latest::default(),
            commits_open: false,
            commits: CommitsState::Loading,
            commits_request: Latest::default(),
            open_commits: HashSet::new(),
            watching: false,
            _subscriptions: subscriptions,
        };
        explorer.load_directory(crate::tree::ROOT, cx);
        explorer
    }

    /// Shows another workspace folder: both trees start again.
    pub fn set_cwd(&mut self, cwd: String, cx: &mut Context<Explorer>) {
        if cwd == self.cwd {
            return;
        }
        self.stop_watching(cx);
        self.cwd = cwd;
        self.generation += 1;
        self.tree.reset();
        self.rows.clear();
        self.selected_file = None;
        self.changes = ChangesState::Idle;
        self.change_rows.clear();
        self.collapsed.clear();
        self.selected_change = None;
        self.commits = CommitsState::Loading;
        self.open_commits.clear();
        self.load_directory(crate::tree::ROOT, cx);
        if self.tab == ExplorerTab::Changes && self.visible {
            self.start_watching(cx);
            self.request_status(cx);
            if self.commits_open {
                self.request_commits(cx);
            }
        }
        cx.notify();
    }

    pub fn tab(&self) -> ExplorerTab {
        self.tab
    }

    pub fn select_tab(&mut self, tab: ExplorerTab, cx: &mut Context<Explorer>) {
        if self.tab == tab {
            return;
        }
        self.tab = tab;
        match tab {
            ExplorerTab::Files => self.stop_watching(cx),
            ExplorerTab::Changes if self.visible => {
                self.start_watching(cx);
                self.request_status(cx);
            }
            ExplorerTab::Changes => {}
        }
        cx.notify();
    }

    /// The owner calls this when the sidebar is shown or hidden. A hidden Explorer stops
    /// watching the checkout, and a shown Changes tab starts again with a fresh list.
    pub fn set_visible(&mut self, visible: bool, cx: &mut Context<Explorer>) {
        if self.visible == visible {
            return;
        }
        self.visible = visible;
        if !visible {
            self.stop_watching(cx);
        } else if self.tab == ExplorerTab::Changes {
            self.start_watching(cx);
            self.request_status(cx);
        }
    }

    /// Lists the root and every open folder again, and asks for the changes again.
    pub fn refresh(&mut self, cx: &mut Context<Explorer>) {
        let open = self.tree.expanded_paths();
        self.tree.invalidate();
        self.load_directory(crate::tree::ROOT, cx);
        for path in open {
            self.load_directory(&path, cx);
        }
        if self.watching {
            self.request_status(cx);
            if self.commits_open {
                self.request_commits(cx);
            }
        }
    }

    fn start_watching(&mut self, cx: &mut Context<Explorer>) {
        if !self.watching {
            self.watching = true;
            let cwd = self.cwd.clone();
            self.client
                .update(cx, |client, cx| client.watch_checkout(&cwd, cx));
        }
    }

    fn stop_watching(&mut self, cx: &mut Context<Explorer>) {
        if self.watching {
            self.watching = false;
            let cwd = self.cwd.clone();
            self.client
                .update(cx, |client, cx| client.unwatch_checkout(&cwd, cx));
        }
    }

    // ----- files -----

    fn load_directory(&mut self, path: &str, cx: &mut Context<Explorer>) {
        self.tree.start_load(path);
        if !self.client.read(cx).is_connected() {
            // `ClientEvent::Connected` lists the root and the open folders.
            return;
        }
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let path = path.to_string();
        let task = self
            .client
            .update(cx, |client, cx| client.list_directory(&cwd, &path, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                match result {
                    Ok(entries) => this.tree.set_listing(&path, entries),
                    Err(error) => {
                        log::warn!("list_directory {path}: {error:#}");
                        this.tree.set_failed(&path, format!("{error:#}"));
                    }
                }
                this.rebuild_rows(cx);
            });
        })
        .detach();
    }

    fn rebuild_rows(&mut self, cx: &mut Context<Explorer>) {
        self.rows = self.tree.rows_with(self.sort, self.show_hidden);
        cx.notify();
    }

    fn cycle_sort(&mut self, cx: &mut Context<Explorer>) {
        self.sort = self.sort.next();
        self.rebuild_rows(cx);
    }

    fn toggle_hidden(&mut self, cx: &mut Context<Explorer>) {
        self.show_hidden = !self.show_hidden;
        self.rebuild_rows(cx);
    }

    fn click_file_row(&mut self, index: usize, cx: &mut Context<Explorer>) {
        let Some(row) = self.rows.get(index) else {
            return;
        };
        let entry = row.entry.clone();
        self.selected_file = Some(entry.path.clone());
        if entry.is_dir {
            if let Toggled::Expanded { needs_load: true } = self.tree.toggle(&entry.path) {
                self.load_directory(&entry.path, cx);
            }
            self.rebuild_rows(cx);
        } else {
            cx.emit(ExplorerEvent::OpenFile {
                cwd: self.cwd.clone(),
                path: entry.path,
            });
            cx.notify();
        }
    }

    // ----- changes -----

    fn request_status(&mut self, cx: &mut Context<Explorer>) {
        if matches!(self.changes, ChangesState::Idle) {
            self.changes = ChangesState::Loading;
        }
        if !self.client.read(cx).is_connected() {
            // `ClientEvent::Connected` asks again.
            cx.notify();
            return;
        }
        let request = self.status_request.begin();
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.checkout_details(&cwd, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation || !this.status_request.is_current(request) {
                    return;
                }
                this.apply_status(result, cx);
            });
        })
        .detach();
        cx.notify();
    }

    fn apply_status(
        &mut self,
        result: anyhow::Result<bachuc_client::CheckoutDetails>,
        cx: &mut Context<Explorer>,
    ) {
        if let Ok(details) = &result
            && details.is_git
        {
            let (cwd, dirty, branch) = (
                self.cwd.clone(),
                !details.files.is_empty(),
                details.branch.clone(),
            );
            update_prefs(cx, |prefs| prefs.set_status(&cwd, dirty, branch));
        }
        self.tree_compare = read_prefs(cx, |prefs| prefs.tree_compare(&self.cwd));
        // A base answer still in flight is for the mode or request before this one.
        self.base_request.begin();
        self.changes = match result {
            Err(error) => {
                log::warn!("checkout_details: {error:#}");
                ChangesState::Failed(format!("{error:#}"))
            }
            Ok(details) if details.is_git && self.tree_compare.is_some() => {
                // Committed: the tree lists the files of the base diff instead.
                self.request_base_files(details.branch, cx);
                return;
            }
            Ok(details) => match CheckoutView::new(details) {
                Some(view) => ChangesState::Ready(view),
                None => ChangesState::NotGit,
            },
        };
        self.rebuild_change_rows();
        cx.notify();
    }

    /// Lists the Committed files with the `Base` diff of `tree_compare`. The shown tree stays
    /// until the answer arrives.
    fn request_base_files(&mut self, branch: Option<String>, cx: &mut Context<Explorer>) {
        let Some(compare) = self.tree_compare.clone() else {
            return;
        };
        let request = self.base_request.begin();
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let task = self.client.update(cx, |client, cx| {
            client.checkout_diff_with(&cwd, compare, cx)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation || !this.base_request.is_current(request) {
                    return;
                }
                this.changes = match result {
                    Ok(diffs) => ChangesState::Ready(CheckoutView::from_diffs(branch, &diffs)),
                    Err(error) => {
                        log::warn!("checkout_diff: {error:#}");
                        ChangesState::Failed(format!("{error:#}"))
                    }
                };
                this.rebuild_change_rows();
                cx.notify();
            });
        })
        .detach();
    }

    /// A mode pick, a whitespace toggle, or a new base ref changes what the tree lists.
    fn prefs_changed(&mut self, cx: &mut Context<Explorer>) {
        let wanted = read_prefs(cx, |prefs| prefs.tree_compare(&self.cwd));
        if self.watching
            && matches!(
                self.changes,
                ChangesState::Ready(_) | ChangesState::Failed(_)
            )
            && wanted != self.tree_compare
        {
            self.request_status(cx);
        }
        cx.notify();
    }

    fn rebuild_change_rows(&mut self) {
        self.change_rows = match &self.changes {
            ChangesState::Ready(view) => view.tree.rows(&self.collapsed),
            _ => Vec::new(),
        };
    }

    /// The app only activates an open diff tab, so a focus request also goes to the shared
    /// prefs, which the open all-files view of this folder follows.
    fn open_all_diffs(&mut self, focus_path: Option<String>, cx: &mut Context<Explorer>) {
        if let Some(path) = &focus_path {
            let cwd = self.cwd.clone();
            update_prefs(cx, |prefs| {
                prefs.request_focus(&cwd, path);
                true
            });
        }
        cx.emit(ExplorerDiffEvent::OpenAllDiffs {
            cwd: self.cwd.clone(),
            focus_path,
        });
    }

    // ----- commits -----

    fn toggle_commits(&mut self, cx: &mut Context<Explorer>) {
        self.commits_open = !self.commits_open;
        if self.commits_open {
            self.request_commits(cx);
        }
        cx.notify();
    }

    fn request_commits(&mut self, cx: &mut Context<Explorer>) {
        if !self.client.read(cx).is_connected() {
            // `ClientEvent::Connected` refreshes, which asks again.
            return;
        }
        let request = self.commits_request.begin();
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.commits(&cwd, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation || !this.commits_request.is_current(request) {
                    return;
                }
                match &result {
                    Err(error) => log::warn!("commits: {error:#}"),
                    Ok(list) => {
                        let (cwd, base_ref) = (this.cwd.clone(), list.base_ref.clone());
                        update_prefs(cx, |prefs| prefs.set_base_ref(&cwd, base_ref));
                    }
                }
                this.commits = CommitsState::from_result(result);
                cx.notify();
            });
        })
        .detach();
    }

    fn click_commit_row(&mut self, row: CommitRow, cx: &mut Context<Explorer>) {
        let CommitsState::Ready(commits) = &self.commits else {
            return;
        };
        match row {
            CommitRow::Commit { index } => {
                let Some(commit) = commits.get(index) else {
                    return;
                };
                if !self.open_commits.remove(&commit.sha) {
                    self.open_commits.insert(commit.sha.clone());
                }
            }
            CommitRow::File { commit, file } => {
                let Some(event) = commits
                    .get(commit)
                    .and_then(|commit| ExplorerDiffEvent::commit_file(&self.cwd, commit, file))
                else {
                    return;
                };
                cx.emit(event);
            }
        }
        cx.notify();
    }

    fn click_change_row(&mut self, index: usize, cx: &mut Context<Explorer>) {
        let Some(row) = self.change_rows.get(index).cloned() else {
            return;
        };
        match row {
            ChangeRow::Folder { dir_path, .. } => {
                self.selected_change = Some(dir_path.clone());
                if !self.collapsed.remove(&dir_path) {
                    self.collapsed.insert(dir_path);
                }
                self.rebuild_change_rows();
            }
            ChangeRow::File { index, .. } => {
                let ChangesState::Ready(view) = &self.changes else {
                    return;
                };
                let Some(file) = view.files.get(index) else {
                    return;
                };
                let path = file.path.clone();
                self.selected_change = Some(path.clone());
                self.open_all_diffs(Some(path), cx);
            }
        }
        cx.notify();
    }

    // ----- render -----

    fn render_tab(&self, tab: ExplorerTab, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let active = self.tab == tab;
        let (label, glyph, id) = match tab {
            ExplorerTab::Files => ("Files", IconName::Files, "explorer-tab-files"),
            ExplorerTab::Changes => ("Changes", IconName::FileDiff, "explorer-tab-changes"),
        };
        let color = if active {
            colors.foreground
        } else {
            colors.foreground_muted
        };
        div().mx(px(TAB_GAP / 2.)).child(
            div()
                .id(id)
                .role(Role::Tab)
                .aria_label(label)
                .aria_selected(active)
                .h(px(TAB_HEIGHT))
                .max_w(px(180.))
                .px(px(SPACE_2))
                .rounded(px(RADIUS_MD))
                .flex()
                .items_center()
                .gap(px(SPACE_1))
                .cursor_pointer()
                .when(active, |this| this.bg(colors.interaction_highlight))
                .hover(|style| style.bg(colors.interaction_highlight))
                .on_click(cx.listener(move |this, _, _, cx| this.select_tab(tab, cx)))
                .child(icon(glyph).with_size(px(ICON_SM)).text_color(color))
                .child(
                    div()
                        .min_w_0()
                        .truncate()
                        .text_size(ui_font_size(FONT_BASE, cx))
                        .text_color(color)
                        .child(label),
                ),
        )
    }

    fn render_tab_rail(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        div()
            .id("explorer-tab-rail")
            .role(Role::TabList)
            .aria_label("Explorer views")
            .relative()
            .flex_shrink_0()
            .h(px(TAB_BAR_HEIGHT))
            .px(px(TAB_RAIL_INSET))
            .flex()
            .items_center()
            .bg(colors.surface_sidebar)
            .child(self.render_tab(ExplorerTab::Files, cx))
            .child(self.render_tab(ExplorerTab::Changes, cx))
            .child(
                div()
                    .absolute()
                    .left_0()
                    .right_0()
                    .bottom_0()
                    .h(px(1.))
                    .bg(colors.border),
            )
    }

    fn render_files(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match self.tree.root_state() {
            RootState::Loading => {
                centered_text("explorer-files-loading", "Loading files...", false, cx)
                    .into_any_element()
            }
            RootState::Failed(message) => self.render_files_failed(message, cx),
            RootState::Empty => v_flex()
                .flex_1()
                .min_h_0()
                .child(self.render_files_toolbar(cx))
                .child(centered_text("explorer-files-empty", "No files", false, cx))
                .into_any_element(),
            RootState::Ready if self.rows.is_empty() => v_flex()
                .flex_1()
                .min_h_0()
                .child(self.render_files_toolbar(cx))
                .child(centered_text(
                    "explorer-files-empty",
                    if self.tree.only_hidden() {
                        "No visible files"
                    } else {
                        "No files"
                    },
                    false,
                    cx,
                ))
                .into_any_element(),
            RootState::Ready => v_flex()
                .flex_1()
                .min_h_0()
                .child(self.render_files_toolbar(cx))
                .child(
                    div()
                        .id("explorer-files")
                        .role(Role::Tree)
                        .aria_label("Files")
                        .flex_1()
                        .min_h_0()
                        .child(
                            uniform_list(
                                "explorer-files-rows",
                                self.rows.len(),
                                cx.processor(Self::render_file_rows),
                            )
                            .track_scroll(&self.files_scroll)
                            .size_full()
                            .pt(px(SPACE_2))
                            .pb(px(SPACE_4)),
                        ),
                )
                .into_any_element(),
        }
    }

    /// A 20 px icon button of a toolbar row, its glyph in the extra muted color.
    fn toolbar_button(
        &self,
        id: &'static str,
        label: &'static str,
        glyph: IconName,
        cx: &mut Context<Self>,
    ) -> Stateful<Div> {
        let colors = tokens(cx);
        div()
            .id(id)
            .role(Role::Button)
            .aria_label(label)
            .flex_shrink_0()
            .size(px(TOOLBAR_BUTTON))
            .rounded(px(RADIUS_MD))
            .flex()
            .items_center()
            .justify_center()
            .cursor_pointer()
            .hover(|style| style.bg(colors.interaction_highlight))
            .child(
                icon(glyph)
                    .with_size(px(ICON_SM))
                    .text_color(colors.foreground_extra_muted),
            )
    }

    /// The Files toolbar (`files-pane-header`): the sort trigger, then the hidden files toggle
    /// and refresh. New file and New folder need a create request the client does not have, and
    /// the Modified sort needs a modified time that `FileEntry` does not have.
    fn render_files_toolbar(&self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let sort_label = self.sort.label();
        let (hidden_label, hidden_icon) = if self.show_hidden {
            ("Hide hidden files", IconName::Eye)
        } else {
            ("Show hidden files", IconName::EyeOff)
        };
        h_flex()
            .id("files-pane-header")
            .role(Role::Toolbar)
            .aria_label("Files")
            .flex_shrink_0()
            .h(px(TOOLBAR_HEIGHT))
            .pr(px(TRAILING_RAIL - (TOOLBAR_BUTTON - ICON_SM) / 2.))
            .justify_between()
            .bg(colors.surface_sidebar)
            .border_b_1()
            .border_color(colors.border)
            .child(
                h_flex()
                    .id("files-sort-trigger")
                    .role(Role::Button)
                    .aria_label(format!("Sort by {sort_label}"))
                    .ml(px(SPACE_3 - SPACE_1))
                    .px(px(SPACE_1))
                    .h(px(SORT_TRIGGER_HEIGHT))
                    .gap(px(SPACE_1))
                    .rounded(px(RADIUS_MD))
                    .cursor_pointer()
                    .hover(|style| style.bg(colors.interaction_highlight))
                    .on_click(cx.listener(|this, _, _, cx| this.cycle_sort(cx)))
                    .child(
                        div()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.foreground_muted)
                            .child(sort_label),
                    )
                    .child(
                        icon(IconName::ChevronDown)
                            .with_size(px(12.))
                            .text_color(colors.foreground_muted),
                    ),
            )
            .child(
                h_flex()
                    .gap(px(SPACE_1))
                    .child(
                        self.toolbar_button("files-hidden-toggle", hidden_label, hidden_icon, cx)
                            .aria_selected(!self.show_hidden)
                            .when(!self.show_hidden, |this| {
                                this.bg(colors.interaction_highlight)
                            })
                            .on_click(cx.listener(|this, _, _, cx| this.toggle_hidden(cx))),
                    )
                    .child(
                        self.toolbar_button(
                            "files-refresh",
                            "Refresh files",
                            IconName::RotateCw,
                            cx,
                        )
                        .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
                    ),
            )
    }

    fn render_files_failed(&self, message: String, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        v_flex()
            .id("explorer-files-error")
            .flex_1()
            .items_center()
            .justify_center()
            .gap(px(SPACE_2))
            .p(px(SPACE_4))
            .text_size(ui_font_size(FONT_BASE, cx))
            .child(
                div()
                    .text_center()
                    .text_color(colors.destructive)
                    .child(message),
            )
            .child(
                div()
                    .id("explorer-files-retry")
                    .role(Role::Button)
                    .aria_label("Retry")
                    .cursor_pointer()
                    .px(px(SPACE_3))
                    .py(px(SPACE_1))
                    .rounded_full()
                    .border_1()
                    .border_color(colors.border)
                    .text_color(colors.foreground_muted)
                    .font_weight(FontWeight::SEMIBOLD)
                    .child("Retry")
                    .on_click(cx.listener(|this, _, _, cx| this.refresh(cx))),
            )
            .into_any_element()
    }

    fn render_file_rows(
        &mut self,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let colors = tokens(cx);
        range
            .filter_map(|index| {
                let row = self.rows.get(index)?;
                let selected = self.selected_file.as_deref() == Some(row.entry.path.as_str());
                let name = row.entry.name.clone();
                let lead = if !row.entry.is_dir {
                    file_icon_element(&name).into_any_element()
                } else if row.loading {
                    loading_marker(&colors).into_any_element()
                } else {
                    chevron(row.expanded, &colors).into_any_element()
                };
                let group = SharedString::from(format!("explorer-file-row-{index}"));
                Some(
                    div()
                        .id(("explorer-file-row", index))
                        .group(group.clone())
                        .role(Role::TreeItem)
                        .aria_label(name.clone())
                        .aria_selected(selected)
                        .when(row.entry.is_dir, |this| this.aria_expanded(row.expanded))
                        .w_full()
                        .h(px(FILE_ROW_HEIGHT))
                        .pl(px(tree_padding_left(row.depth)))
                        .pr(px(TRAILING_RAIL))
                        .flex()
                        .items_center()
                        .gap(px(TREE_ICON_GAP))
                        .cursor_pointer()
                        .when(selected, |this| this.bg(colors.surface_sidebar_hover))
                        .hover(|style| style.bg(colors.surface_sidebar_hover))
                        .on_click(cx.listener(move |this, _, _, cx| this.click_file_row(index, cx)))
                        .child(lead)
                        .child(
                            div()
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .text_size(ui_font_size(FONT_BASE, cx))
                                .text_color(colors.foreground.opacity(0.76))
                                .group_hover(group, |style| style.text_color(colors.foreground))
                                .child(name),
                        )
                        .into_any_element(),
                )
            })
            .collect()
    }

    fn render_changes(&mut self, cx: &mut Context<Self>) -> AnyElement {
        match &self.changes {
            ChangesState::Idle | ChangesState::Loading => centered_text(
                "explorer-changes-loading",
                "Checking repository...",
                false,
                cx,
            )
            .into_any_element(),
            ChangesState::Failed(message) => {
                state_message("explorer-changes-error", message.clone(), true, cx)
                    .into_any_element()
            }
            ChangesState::NotGit => state_message(
                "explorer-changes-not-git",
                "Not a git repository",
                false,
                cx,
            )
            .into_any_element(),
            ChangesState::Ready(view) => {
                let added = view.files.iter().map(|file| file.additions).sum();
                let removed = view.files.iter().map(|file| file.deletions).sum();
                let has_files = !view.files.is_empty();
                let header =
                    self.render_changes_header(view.branch.clone(), added, removed, has_files, cx);
                let body = if !has_files {
                    state_message("explorer-changes-empty", "No changes to display", false, cx)
                        .into_any_element()
                } else {
                    div()
                        .id("explorer-changes")
                        .role(Role::Tree)
                        .aria_label("Changes")
                        .flex_1()
                        .min_h_0()
                        .child(
                            uniform_list(
                                "explorer-changes-rows",
                                self.change_rows.len(),
                                cx.processor(Self::render_change_rows),
                            )
                            .track_scroll(&self.changes_scroll)
                            .size_full()
                            .pb(px(SPACE_4 * 2.)),
                        )
                        .into_any_element()
                };
                let footer = self.render_commits(cx);
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .child(header)
                    .child(body)
                    .child(footer)
                    .into_any_element()
            }
        }
    }

    /// The Commits footer (`CommitsSection`): a header that opens and closes it, then the
    /// branch commits, each opening to its files.
    fn render_commits(&mut self, cx: &mut Context<Self>) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let open = self.commits_open;
        let count = if open { self.commits.count() } else { None };
        let header = h_flex()
            .id("commits-section-header")
            .role(Role::Button)
            .aria_label("Commits")
            .aria_expanded(open)
            .flex_shrink_0()
            .pl(px(tree_padding_left(0)))
            .pr(px(SPACE_3))
            .py(px(SPACE_2))
            .gap(px(SPACE_2))
            .cursor_pointer()
            .hover(|style| style.bg(colors.surface_sidebar_hover))
            .on_click(cx.listener(|this, _, _, cx| this.toggle_commits(cx)))
            .child(chevron(open, &colors))
            .child(
                div()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(colors.foreground)
                    .child("Commits"),
            )
            .child(
                div()
                    .id("commits-section-count")
                    .flex_1()
                    .text_size(ui_font_size(FONT_SM, cx))
                    .text_color(colors.foreground_muted)
                    .when_some(count, |this, count| {
                        this.role(Role::Label)
                            .aria_label(format!("{count} workspace commits"))
                            .child(count.to_string())
                    }),
            );
        let message = |id: &'static str, text: String, error: bool| {
            div()
                .id(id)
                .role(Role::Status)
                .aria_label(text.clone())
                .pl(px(SPACE_2))
                .pr(px(SPACE_3))
                .py(px(SPACE_2))
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(if error {
                    colors.status_danger
                } else {
                    colors.foreground_muted
                })
                .child(text)
                .into_any_element()
        };
        let body = if !open {
            None
        } else {
            Some(match &self.commits {
                CommitsState::Loading => message(
                    "commits-section-skeleton",
                    "Loading commits...".into(),
                    false,
                ),
                CommitsState::Failed => message(
                    "commits-section-error",
                    "Failed to load commits".into(),
                    true,
                ),
                CommitsState::Empty { base } => message(
                    "commits-section-no-workspace-commits",
                    format!("No commits ahead of {base} yet"),
                    false,
                ),
                CommitsState::Ready(commits) => {
                    let rows = commit_rows(commits, &self.open_commits);
                    let now = std::time::SystemTime::now()
                        .duration_since(std::time::UNIX_EPOCH)
                        .map_or(0, |elapsed| elapsed.as_secs() as i64);
                    let elements: Vec<AnyElement> = rows
                        .iter()
                        .enumerate()
                        .filter_map(|(position, row)| {
                            self.render_commit_row(position, *row, commits, now, cx)
                        })
                        .collect();
                    div()
                        .id("commits-section-list")
                        .role(Role::List)
                        .aria_label("Commits")
                        .max_h(px(COMMITS_MAX_HEIGHT))
                        .overflow_y_scroll()
                        .pb(px(SPACE_1))
                        .children(elements)
                        .into_any_element()
                }
            })
        };
        v_flex()
            .flex_shrink_0()
            .border_t_1()
            .border_color(colors.border)
            .child(header)
            .children(body)
    }

    fn render_commit_row(
        &self,
        position: usize,
        row: CommitRow,
        commits: &[CommitSummary],
        now: i64,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let colors = tokens(cx);
        let base = h_flex()
            .id(("commit-row", position))
            .role(Role::Button)
            .h(px(COMMIT_ROW_HEIGHT))
            .px(px(SPACE_2))
            .gap(px(SPACE_2))
            .cursor_pointer()
            .hover(|style| style.bg(colors.surface_sidebar_hover))
            .on_click(cx.listener(move |this, _, _, cx| this.click_commit_row(row, cx)));
        Some(match row {
            CommitRow::Commit { index } => {
                let commit = commits.get(index)?;
                let open = self.open_commits.contains(&commit.sha);
                let (first, last) = (index == 0, index + 1 == commits.len());
                // Local commits are rings; commits on the remote are filled dots.
                let dot = div()
                    .size(px(COMMIT_DOT))
                    .rounded_full()
                    .border_2()
                    .border_color(colors.accent)
                    .bg(if commit.is_on_remote {
                        colors.accent
                    } else {
                        colors.surface0
                    });
                let node = div()
                    .relative()
                    .w(px(COMMIT_DOT))
                    .h_full()
                    .flex_shrink_0()
                    .flex()
                    .items_center()
                    .justify_center()
                    .when(!(first && last), |this| {
                        this.child(
                            div()
                                .absolute()
                                .left(px((COMMIT_DOT - COMMIT_RAIL) / 2.))
                                .w(px(COMMIT_RAIL))
                                .top(if first {
                                    px(COMMIT_ROW_HEIGHT / 2.)
                                } else {
                                    px(0.)
                                })
                                .bottom(if last {
                                    px(COMMIT_ROW_HEIGHT / 2.)
                                } else {
                                    px(0.)
                                })
                                .bg(colors.accent),
                        )
                    })
                    .child(dot);
                base.aria_label(format!("{} {}", commit.short_sha, commit.subject))
                    .aria_expanded(open)
                    .child(node)
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(SPACE_1))
                            .child(
                                div()
                                    .w(px(COMMIT_SHA_WIDTH))
                                    .flex_shrink_0()
                                    .truncate()
                                    .font_family(mono_font_family(cx))
                                    .text_size(ui_font_size(FONT_SM, cx))
                                    .text_color(colors.foreground_muted)
                                    .child(commit.short_sha.clone()),
                            )
                            .child(
                                div()
                                    .flex_1()
                                    .min_w_0()
                                    .truncate()
                                    .text_size(ui_font_size(FONT_BASE, cx))
                                    .text_color(colors.foreground)
                                    .child(commit.subject.clone()),
                            ),
                    )
                    .child(
                        div()
                            .flex_shrink_0()
                            .text_size(ui_font_size(FONT_SM, cx))
                            .text_color(colors.foreground_muted)
                            .child(time_ago(&commit.author_date, now)),
                    )
                    .child(chevron(open, &colors))
                    .into_any_element()
            }
            CommitRow::File { commit, file } => {
                let entry = commits.get(commit)?.files.get(file)?;
                let name = entry
                    .path
                    .rsplit_once('/')
                    .map_or(entry.path.as_str(), |(_, name)| name)
                    .to_string();
                let change = match entry.status.as_deref() {
                    Some("added") => crate::changes::Change::Added,
                    Some("deleted") => crate::changes::Change::Deleted,
                    _ => crate::changes::Change::Modified,
                };
                base.aria_label(format!(
                    "{}, {}, +{}, -{}",
                    entry.path,
                    change.label(),
                    entry.additions,
                    entry.deletions
                ))
                .pl(px(tree_padding_left(1)))
                .child(
                    h_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(px(TREE_ICON_GAP))
                        .child(file_icon_element(&name))
                        .child(
                            div()
                                .min_w_0()
                                .truncate()
                                .text_size(ui_font_size(FONT_BASE, cx))
                                .text_color(colors.foreground.opacity(0.76))
                                .child(name),
                        ),
                )
                .child(diff_stat(entry.additions, entry.deletions, cx))
                .child(change_icon(change, &colors))
                .into_any_element()
            }
        })
    }

    /// The two toolbar rows over the Changes tree (`ChangesRepositoryToolbar` and
    /// `ChangesComparisonToolbar`): the branch, then the diff mode menu with the total count,
    /// the Open Diff tab button, and the refresh button. Branch switching is out of scope, so
    /// the branch shows as a passive label.
    fn render_changes_header(
        &self,
        branch: String,
        added: u32,
        removed: u32,
        has_files: bool,
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let toolbar_row = |id: &'static str| {
            h_flex()
                .id(id)
                .role(Role::Toolbar)
                .flex_shrink_0()
                .h(px(TOOLBAR_HEIGHT))
                .pl(px(SPACE_2))
                // A bare glyph's hitbox overhangs its ink, so the glyph sits on the trailing rail.
                .pr(px(TRAILING_RAIL - (TOOLBAR_BUTTON - ICON_SM) / 2.))
                .gap(px(SPACE_2))
                .bg(colors.surface_sidebar)
                .border_b_1()
                .border_color(colors.border)
        };
        let label = |id: &'static str, text: SharedString| {
            div()
                .id(id)
                .role(Role::Label)
                .aria_label(text.clone())
                .min_w_0()
                .p(px(SPACE_1))
                .truncate()
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(colors.foreground_muted)
                .child(text)
        };
        v_flex()
            .flex_shrink_0()
            .child(
                toolbar_row("changes-repository-header")
                    .aria_label("Repository")
                    .child(label("changes-branch", branch.into())),
            )
            .child(
                toolbar_row("changes-header")
                    .aria_label("Diff")
                    .child(
                        h_flex()
                            .flex_1()
                            .min_w_0()
                            .gap(px(SPACE_1))
                            .child(diff_mode_menu("changes-diff-mode", &self.cwd, cx))
                            .child(diff_stat(added, removed, cx)),
                    )
                    .when(has_files, |this| {
                        this.child(
                            self.toolbar_button(
                                "changes-open-tab",
                                "Open Diff tab",
                                IconName::FileDiff,
                                cx,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.open_all_diffs(None, cx))),
                        )
                    })
                    .child(
                        self.toolbar_button("changes-refresh", "Refresh", IconName::RotateCw, cx)
                            .on_click(cx.listener(|this, _, _, cx| {
                                this.request_status(cx);
                                if this.commits_open {
                                    this.request_commits(cx);
                                }
                            })),
                    ),
            )
    }

    fn render_change_rows(
        &mut self,
        range: Range<usize>,
        _window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Vec<AnyElement> {
        let colors = tokens(cx);
        let ChangesState::Ready(view) = &self.changes else {
            return Vec::new();
        };
        range
            .filter_map(|index| {
                let row = self.change_rows.get(index)?;
                let base = div()
                    .w_full()
                    .h(px(CHANGE_ROW_HEIGHT))
                    .pr(px(TRAILING_RAIL))
                    .flex()
                    .items_center()
                    .justify_between()
                    .gap(px(SPACE_1))
                    .cursor_pointer();
                let element = match row {
                    ChangeRow::Folder {
                        dir_path,
                        display_name,
                        depth,
                        added,
                        removed,
                    } => {
                        let selected = self.selected_change.as_deref() == Some(dir_path.as_str());
                        let collapsed = self.collapsed.contains(dir_path);
                        let group = SharedString::from(format!("explorer-change-row-{index}"));
                        base.id(("explorer-change-row", index))
                            .group(group.clone())
                            .role(Role::TreeItem)
                            .aria_label(display_name.clone())
                            .aria_selected(selected)
                            .aria_expanded(!collapsed)
                            .pl(px(tree_padding_left(*depth)))
                            .when(selected, |this| this.bg(colors.surface_sidebar_hover))
                            .hover(|style| style.bg(colors.surface_sidebar_hover))
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.click_change_row(index, cx)),
                            )
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(TREE_ICON_GAP))
                                    .child(chevron(!collapsed, &colors))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(ui_font_size(FONT_BASE, cx))
                                            .text_color(colors.foreground.opacity(0.76))
                                            .group_hover(group, |style| {
                                                style.text_color(colors.foreground)
                                            })
                                            .child(display_name.clone()),
                                    ),
                            )
                            .child(diff_stat(*added, *removed, cx))
                    }
                    ChangeRow::File {
                        index: file_index,
                        name,
                        depth,
                    } => {
                        let file = view.files.get(*file_index)?;
                        let selected = self.selected_change.as_deref() == Some(file.path.as_str());
                        let group = SharedString::from(format!("explorer-change-row-{index}"));
                        let label = view.label(*file_index);
                        let change = view.change(*file_index);
                        base.id(("explorer-change-row", index))
                            .group(group.clone())
                            .role(Role::TreeItem)
                            .aria_label(label)
                            .aria_selected(selected)
                            .pl(px(tree_padding_left(*depth)))
                            .when(selected, |this| this.bg(colors.surface_sidebar_hover))
                            .hover(|style| style.bg(colors.surface_sidebar_hover))
                            .on_click(
                                cx.listener(move |this, _, _, cx| this.click_change_row(index, cx)),
                            )
                            .child(
                                h_flex()
                                    .flex_1()
                                    .min_w_0()
                                    .gap(px(TREE_ICON_GAP))
                                    .child(file_icon_element(name))
                                    .child(
                                        div()
                                            .min_w_0()
                                            .truncate()
                                            .text_size(ui_font_size(FONT_BASE, cx))
                                            .text_color(colors.foreground.opacity(0.76))
                                            .group_hover(group, |style| {
                                                style.text_color(colors.foreground)
                                            })
                                            .child(name.clone()),
                                    ),
                            )
                            .child(
                                h_flex()
                                    .flex_shrink_0()
                                    .gap(px(SPACE_2))
                                    .child(diff_stat(file.additions, file.deletions, cx))
                                    .child(change_icon(change, &colors)),
                            )
                    }
                };
                Some(element.into_any_element())
            })
            .collect()
    }
}

impl Render for Explorer {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let colors = tokens(cx);
        let body = match self.tab {
            ExplorerTab::Files => self.render_files(cx),
            ExplorerTab::Changes => self.render_changes(cx),
        };
        div()
            .id("explorer")
            .role(Role::Pane)
            .aria_label("Explorer")
            .size_full()
            .flex()
            .flex_col()
            .bg(colors.surface_sidebar)
            .text_color(colors.foreground)
            .child(self.render_tab_rail(cx))
            .child(body)
    }
}
