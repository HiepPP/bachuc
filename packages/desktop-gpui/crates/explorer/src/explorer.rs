//! The Explorer sidebar: the Files / Changes tab rail of `explorer-sidebar-tab-rail.tsx` over
//! the Files tree (`file-explorer-pane.tsx`) or the Changes tree (`changed-files-tree.tsx`).
//!
//! The view does not open anything itself. A click on a file emits `ExplorerEvent::OpenFile`, a
//! click on a changed file emits `ExplorerEvent::OpenDiff`, and the app opens the viewer pane.

use crate::changes::{ChangeRow, ChangeTree};
use crate::tree::{FileTree, RootState, Toggled, TreeRow};
use crate::ui::{
    CHANGE_ROW_HEIGHT, Change, FILE_ROW_HEIGHT, TRAILING_RAIL, TREE_ICON_GAP, centered_text,
    change_icon, chevron, diff_stat, file_icon_element, loading_marker, state_message,
    tree_padding_left,
};
use bachuc_client::{ChangedFile, Client, ClientEvent, DirectoryEvent};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_SM, RADIUS_MD, SPACE_1, SPACE_2, SPACE_3, SPACE_4, TAB_BAR_HEIGHT,
};
use bachuc_theme::{tokens, ui_font_size};
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

/// What the Explorer tells its owner.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum ExplorerEvent {
    /// A file of the Files tree was picked.
    OpenFile { cwd: String, path: String },
    /// A changed file of the Changes tree was picked.
    OpenDiff { cwd: String, path: String },
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
    /// The folder is not in a git repository (`branch` is none).
    NotGit,
    Ready {
        branch: String,
        tree: ChangeTree,
    },
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
    selected_file: Option<String>,
    files_scroll: UniformListScrollHandle,
    changes: ChangesState,
    change_rows: Vec<ChangeRow>,
    collapsed: HashSet<String>,
    selected_change: Option<String>,
    changes_scroll: UniformListScrollHandle,
    /// The status request in flight or last applied; an older answer is dropped.
    status_request: u64,
    watching: bool,
    _subscriptions: Vec<Subscription>,
}

impl EventEmitter<ExplorerEvent> for Explorer {}

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
            selected_file: None,
            files_scroll: UniformListScrollHandle::new(),
            changes: ChangesState::Idle,
            change_rows: Vec::new(),
            collapsed: HashSet::new(),
            selected_change: None,
            changes_scroll: UniformListScrollHandle::new(),
            status_request: 0,
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
        self.load_directory(crate::tree::ROOT, cx);
        if self.tab == ExplorerTab::Changes && self.visible {
            self.start_watching(cx);
            self.request_status(cx);
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
        self.rows = self.tree.rows();
        cx.notify();
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
        self.status_request += 1;
        let request = self.status_request;
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.checkout_status(&cwd, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            let _ = this.update(cx, |this, cx| {
                if this.generation != generation || this.status_request != request {
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
        result: anyhow::Result<bachuc_client::CheckoutStatus>,
        cx: &mut Context<Explorer>,
    ) {
        self.changes = match result {
            Err(error) => {
                log::warn!("checkout_status: {error:#}");
                ChangesState::Failed(format!("{error:#}"))
            }
            Ok(bachuc_client::CheckoutStatus {
                branch: Some(branch),
                files,
            }) => ChangesState::Ready {
                branch,
                tree: ChangeTree::build(&files),
            },
            Ok(_) => ChangesState::NotGit,
        };
        self.rebuild_change_rows();
        cx.notify();
    }

    fn rebuild_change_rows(&mut self) {
        self.change_rows = match &self.changes {
            ChangesState::Ready { tree, .. } => tree.rows(&self.collapsed),
            _ => Vec::new(),
        };
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
                let ChangesState::Ready { tree, .. } = &self.changes else {
                    return;
                };
                let Some(file) = tree.files().get(index) else {
                    return;
                };
                let path = file.path.clone();
                self.selected_change = Some(path.clone());
                cx.emit(ExplorerEvent::OpenDiff {
                    cwd: self.cwd.clone(),
                    path,
                });
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
            RootState::Empty => {
                centered_text("explorer-files-empty", "No files", false, cx).into_any_element()
            }
            RootState::Ready => div()
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
                )
                .into_any_element(),
        }
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
            ChangesState::Ready { branch, tree } => {
                let header = self.render_changes_header(branch.clone(), tree.files(), cx);
                let body = if tree.files().is_empty() {
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
                v_flex()
                    .flex_1()
                    .min_h_0()
                    .child(header)
                    .child(body)
                    .into_any_element()
            }
        }
    }

    /// The two toolbar rows over the Changes tree (`ChangesRepositoryToolbar` and
    /// `ChangesComparisonToolbar`): the branch, then the diff mode with the total count and the
    /// refresh button. Branch switching and the other diff mode are out of scope, so the branch
    /// and the mode show as passive labels without a menu.
    fn render_changes_header(
        &self,
        branch: String,
        files: &[ChangedFile],
        cx: &mut Context<Self>,
    ) -> impl IntoElement + use<> {
        let colors = tokens(cx);
        let added = files.iter().map(|file| file.added).sum();
        let removed = files.iter().map(|file| file.removed).sum();
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
                            .child(label("changes-diff-mode", "Uncommitted".into()))
                            .child(diff_stat(added, removed, cx)),
                    )
                    .child(
                        div()
                            .id("changes-refresh")
                            .role(Role::Button)
                            .aria_label("Refresh")
                            .flex_shrink_0()
                            .size(px(TOOLBAR_BUTTON))
                            .rounded(px(RADIUS_MD))
                            .flex()
                            .items_center()
                            .justify_center()
                            .cursor_pointer()
                            .hover(|style| style.bg(colors.interaction_highlight))
                            .on_click(cx.listener(|this, _, _, cx| this.request_status(cx)))
                            .child(
                                icon(IconName::RotateCw)
                                    .with_size(px(ICON_SM))
                                    .text_color(colors.foreground_extra_muted),
                            ),
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
        let files: &[ChangedFile] = match &self.changes {
            ChangesState::Ready { tree, .. } => tree.files(),
            _ => &[],
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
                        let file = files.get(*file_index)?;
                        let selected = self.selected_change.as_deref() == Some(file.path.as_str());
                        let group = SharedString::from(format!("explorer-change-row-{index}"));
                        let label = format!("{}, +{}, -{}", file.path, file.added, file.removed);
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
                                    .child(diff_stat(file.added, file.removed, cx))
                                    // ChangedFile does not say whether a file is new or deleted,
                                    // so every row shows the modified mark.
                                    .child(change_icon(Change::Modified, &colors)),
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
