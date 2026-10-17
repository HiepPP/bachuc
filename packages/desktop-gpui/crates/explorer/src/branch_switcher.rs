//! The branch switcher of the Changes header, drawn and run as `branch-switcher.tsx` and
//! `use-branch-switcher.ts`: the branch label opens a searchable list of up to 200 branches,
//! loaded only when the list opens and kept for 15 s. A switch that fails on uncommitted
//! changes offers "Stash & Switch"; a switch that lands on a branch with a Paseo stash offers
//! to restore it. Electron shows both confirms in an app dialog; here they draw inline under the
//! label, because the GPUI client has no app dialog. A failed step and a restored stash emit
//! `ExplorerNoticeEvent`, which the app shows as Electron's toast.

use crate::explorer::ExplorerNoticeEvent;
use bachuc_client::{Client, StashEntry};
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_SM, ICON_XS, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2, SPACE_3,
};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::time::{Duration, Instant};

/// `getBranchSuggestions(200)`.
pub(crate) const BRANCH_LIMIT: u32 = 200;
/// The `staleTime` of the branch query.
const BRANCH_CACHE: Duration = Duration::from_secs(15);
/// `desktopMinWidth` of the combobox.
const LIST_MIN_WIDTH: f32 = 280.;
const LIST_MAX_HEIGHT: f32 = 400.;
const DIALOG_WIDTH: f32 = 280.;
const ROW_HEIGHT: f32 = 28.;
const TEXT_LINE_FACTOR: f32 = 1.4;

// ----- rules -----

/// The branches that match `query`, best first, as `filterAndRankComboboxOptions` ranks them
/// without its fuzzy tier (the pickers leave it off): an exact match, a whole word, a prefix, a
/// word start, a substring, then the letters in order within one word. Ties sort by name. An
/// empty query keeps the daemon order.
pub(crate) fn rank_branches(branches: &[String], query: &str) -> Vec<String> {
    let query = query.trim().to_lowercase();
    if query.is_empty() {
        return branches.to_vec();
    }
    let mut scored: Vec<((u8, usize, usize), &String)> = branches
        .iter()
        .filter_map(|name| match_score(&query, &name.to_lowercase()).map(|score| (score, name)))
        .collect();
    scored.sort_by(|(a, a_name), (b, b_name)| a.cmp(b).then_with(|| a_name.cmp(b_name)));
    scored.into_iter().map(|(_, name)| name.clone()).collect()
}

/// `(tier, offset, spread)`, lower is better; `None` when `text` does not match.
fn match_score(query: &str, text: &str) -> Option<(u8, usize, usize)> {
    if text == query {
        return Some((0, 0, 0));
    }
    let boundary = |ch: Option<char>| ch.is_none_or(|ch| !ch.is_ascii_alphanumeric());
    let mut best: Option<(u8, usize, usize)> = None;
    for (found, _) in text.match_indices(query) {
        let before = text[..found].chars().next_back();
        let after = text[found + query.len()..].chars().next();
        let starts = found == 0 || boundary(before);
        let tier = if starts && boundary(after) {
            1
        } else if found == 0 {
            2
        } else if starts {
            3
        } else {
            4
        };
        if best.is_none_or(|best| (tier, found) < (best.0, best.1)) {
            best = Some((tier, found, 0));
        }
    }
    best.or_else(|| subsequence_score(query, text))
}

/// The query letters in order inside one whitespace-free run of `text`.
fn subsequence_score(query: &str, text: &str) -> Option<(u8, usize, usize)> {
    let query: Vec<char> = query.chars().collect();
    let (mut next, mut first, mut last) = (0, None, 0);
    for (index, ch) in text.char_indices() {
        if next == query.len() {
            break;
        }
        if ch.is_whitespace() {
            (next, first) = (0, None);
            continue;
        }
        if ch == query[next] {
            first.get_or_insert(index);
            last = index;
            next += 1;
        }
    }
    let first = first.filter(|_| next == query.len())?;
    Some((5, first, last - first + 1))
}

/// A pick of the current branch does nothing.
pub(crate) fn should_switch(current: Option<&str>, picked: &str) -> bool {
    current != Some(picked)
}

/// The stash decision: a switch that fails on uncommitted changes offers "Stash & Switch";
/// any other failure only shows its message.
pub(crate) fn offers_stash(error: &str) -> bool {
    error.to_lowercase().contains("uncommitted")
}

/// `branchSwitcher.stashRestored`, the success toast of a restore.
const STASH_RESTORED: &str = "Stashed changes restored";

/// An error toast of the switcher.
fn failure(message: String) -> ExplorerNoticeEvent {
    ExplorerNoticeEvent { ok: false, message }
}

/// What a failed switch emits: nothing when it offers "Stash & Switch", else an error notice.
pub(crate) fn switch_failure_notice(error: &str) -> Option<ExplorerNoticeEvent> {
    (!offers_stash(error)).then(|| failure(error.to_string()))
}

/// What a restore emits: Electron's success note, or the error.
pub(crate) fn restore_notice(result: Result<(), String>) -> ExplorerNoticeEvent {
    match result {
        Ok(()) => ExplorerNoticeEvent {
            ok: true,
            message: STASH_RESTORED.to_string(),
        },
        Err(message) => failure(message),
    }
}

/// The Paseo stash to offer for restore after a switch to `branch`: the first one listed.
pub(crate) fn restore_index(entries: &[StashEntry], branch: &str) -> Option<u32> {
    entries
        .iter()
        .find(|entry| entry.branch.as_deref() == Some(branch))
        .map(|entry| entry.index)
}

/// The row Up or Down moves the highlight to, wrapping at both ends (`getNextActiveIndex`).
/// With no highlight, Down starts at the first row and Up at the last.
pub(crate) fn next_highlight(current: Option<usize>, count: usize, down: bool) -> Option<usize> {
    if count == 0 {
        return None;
    }
    let Some(current) = current else {
        return Some(if down { 0 } else { count - 1 });
    };
    let current = current % count;
    Some(if down {
        (current + 1) % count
    } else {
        (current + count - 1) % count
    })
}

/// The highlight when the list opens, the query changes, or the branches load
/// (`resolveInitialActiveIndex`, rows under the search): the current branch while the query is
/// empty, else the first row.
pub(crate) fn initial_highlight(
    visible: &[String],
    query: &str,
    current: Option<&str>,
) -> Option<usize> {
    if visible.is_empty() {
        return None;
    }
    if !query.trim().is_empty() {
        return Some(0);
    }
    let selected = current.and_then(|current| visible.iter().position(|name| name == current));
    Some(selected.unwrap_or(0))
}

/// The row Enter picks: the highlighted one, or the first when there is no highlight.
pub(crate) fn enter_row(count: usize, highlight: Option<usize>) -> Option<usize> {
    (count > 0).then(|| highlight.filter(|&index| index < count).unwrap_or(0))
}

/// The loaded list is still fresh, so opening the switcher asks the daemon nothing.
pub(crate) fn cache_fresh(loaded_at: Option<Instant>, now: Instant) -> bool {
    loaded_at.is_some_and(|loaded_at| now.saturating_duration_since(loaded_at) < BRANCH_CACHE)
}

// ----- view -----

/// The switcher tells the Explorer that the checkout changed, so it asks for the status again.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct BranchSwitched;

enum Confirm {
    /// The switch to `target` failed on uncommitted changes.
    Stash { target: String },
    /// The new branch has the Paseo stash `index`.
    Restore { index: u32 },
}

enum ListLoad {
    Loading,
    Failed(String),
    Ready,
}

struct Search {
    query: Entity<InputState>,
    /// The row that Enter picks; Up and Down move it.
    highlight: Option<usize>,
    scroll: ScrollHandle,
    _subscriptions: Vec<Subscription>,
}

pub(crate) struct BranchSwitcher {
    client: Entity<Client>,
    cwd: String,
    /// `None` for a detached HEAD: the switcher draws nothing, as in Electron.
    branch: Option<String>,
    /// `Some` while the list is open.
    search: Option<Search>,
    branches: Vec<String>,
    list: ListLoad,
    loaded_at: Option<Instant>,
    /// Bumped when `cwd` changes, so an answer for the old folder is dropped.
    generation: u64,
    busy: bool,
    confirm: Option<Confirm>,
}

impl EventEmitter<BranchSwitched> for BranchSwitcher {}
impl EventEmitter<ExplorerNoticeEvent> for BranchSwitcher {}

impl BranchSwitcher {
    pub(crate) fn new(client: Entity<Client>, cwd: String) -> BranchSwitcher {
        BranchSwitcher {
            client,
            cwd,
            branch: None,
            search: None,
            branches: Vec::new(),
            list: ListLoad::Loading,
            loaded_at: None,
            generation: 0,
            busy: false,
            confirm: None,
        }
    }

    pub(crate) fn set_branch(&mut self, branch: Option<String>, cx: &mut Context<Self>) {
        if self.branch != branch {
            self.branch = branch;
            cx.notify();
        }
    }

    pub(crate) fn set_cwd(&mut self, cwd: String, cx: &mut Context<Self>) {
        self.cwd = cwd;
        self.generation += 1;
        self.branch = None;
        self.search = None;
        self.branches.clear();
        self.list = ListLoad::Loading;
        self.loaded_at = None;
        self.busy = false;
        self.confirm = None;
        cx.notify();
    }

    fn open(&mut self, window: &mut Window, cx: &mut Context<Self>) {
        if self.branch.is_none() || self.busy {
            return;
        }
        if self.search.is_some() {
            self.close(cx);
            return;
        }
        self.confirm = None;
        let query = cx.new(|cx| InputState::new(window, cx).placeholder("Filter branches..."));
        let weak = cx.weak_entity();
        let subscriptions = vec![
            cx.subscribe_in(
                &query,
                window,
                |this: &mut BranchSwitcher, _, event: &InputEvent, _, cx| match event {
                    InputEvent::Change => this.reset_highlight(cx),
                    InputEvent::PressEnter { .. } => {
                        let mut visible = this.visible_branches(cx);
                        let highlight = this.search.as_ref().and_then(|search| search.highlight);
                        if let Some(index) = enter_row(visible.len(), highlight) {
                            this.pick(visible.swap_remove(index), cx);
                        }
                    }
                    _ => {}
                },
            ),
            // The input keeps Escape, Up, and Down for itself, so the switcher reads them first.
            cx.intercept_keystrokes(move |event, window, cx| {
                let key = event.keystroke.key.as_str();
                if !matches!(key, "escape" | "up" | "down") {
                    return;
                }
                let handled = weak
                    .update(cx, |this, cx| this.list_key(key, window, cx))
                    .unwrap_or(false);
                if handled {
                    cx.stop_propagation();
                }
            }),
        ];
        query.update(cx, |query, cx| query.focus(window, cx));
        self.search = Some(Search {
            query,
            highlight: None,
            scroll: ScrollHandle::new(),
            _subscriptions: subscriptions,
        });
        self.reset_highlight(cx);
        if !cache_fresh(self.loaded_at, Instant::now()) {
            self.load(cx);
        }
        cx.notify();
    }

    /// Escape, Up, and Down while the filter has focus. Returns whether it took the key.
    fn list_key(&mut self, key: &str, window: &Window, cx: &mut Context<Self>) -> bool {
        let Some(search) = &self.search else {
            return false;
        };
        if !search.query.focus_handle(cx).is_focused(window) {
            return false;
        }
        if key == "escape" {
            self.close(cx);
            return true;
        }
        let current = search.highlight;
        let count = self.visible_branches(cx).len();
        self.set_highlight(next_highlight(current, count, key == "down"), cx);
        true
    }

    fn reset_highlight(&mut self, cx: &mut Context<Self>) {
        let Some(search) = &self.search else {
            return;
        };
        let query = search.query.read(cx).value().to_string();
        let visible = rank_branches(&self.branches, &query);
        self.set_highlight(
            initial_highlight(&visible, &query, self.branch.as_deref()),
            cx,
        );
    }

    fn set_highlight(&mut self, highlight: Option<usize>, cx: &mut Context<Self>) {
        let Some(search) = &mut self.search else {
            return;
        };
        search.highlight = highlight;
        if let Some(index) = highlight {
            search.scroll.scroll_to_item(index);
        }
        cx.notify();
    }

    fn close(&mut self, cx: &mut Context<Self>) {
        self.search = None;
        cx.notify();
    }

    fn load(&mut self, cx: &mut Context<Self>) {
        if !self.client.read(cx).is_connected() {
            // Electron runs the query only while connected; the list stays empty.
            return;
        }
        if self.branches.is_empty() {
            self.list = ListLoad::Loading;
        }
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let task = self.client.update(cx, |client, cx| {
            client.branch_suggestions(&cwd, None, Some(BRANCH_LIMIT), cx)
        });
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                match result {
                    Ok(branches) => {
                        this.branches = branches.into_iter().map(|branch| branch.name).collect();
                        this.loaded_at = Some(Instant::now());
                        this.list = ListLoad::Ready;
                        this.reset_highlight(cx);
                    }
                    Err(error) => {
                        log::warn!("branch_suggestions {}: {error:#}", this.cwd);
                        this.list = ListLoad::Failed(format!("{error:#}"));
                    }
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn visible_branches(&self, cx: &App) -> Vec<String> {
        let query = self
            .search
            .as_ref()
            .map(|search| search.query.read(cx).value().to_string())
            .unwrap_or_default();
        rank_branches(&self.branches, &query)
    }

    fn pick(&mut self, name: String, cx: &mut Context<Self>) {
        self.close(cx);
        if self.busy || !should_switch(self.branch.as_deref(), &name) {
            return;
        }
        self.busy = true;
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.switch_branch(&cwd, &name, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.busy = false;
                match result {
                    Ok(()) => {
                        cx.emit(BranchSwitched);
                        this.offer_restore(name, cx);
                    }
                    Err(error) => match switch_failure_notice(&format!("{error:#}")) {
                        Some(notice) => cx.emit(notice),
                        None => this.confirm = Some(Confirm::Stash { target: name }),
                    },
                }
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    /// Stashes the changes under the current branch name, then switches.
    fn stash_and_switch(&mut self, target: String, cx: &mut Context<Self>) {
        self.confirm = None;
        self.busy = true;
        cx.notify();
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let current = self.branch.clone();
        let client = self.client.clone();
        let stash = client.update(cx, |client, cx| {
            client.stash_save(&cwd, current.as_deref(), cx)
        });
        cx.spawn(async move |this, cx| {
            let failed = |this: &WeakEntity<BranchSwitcher>, cx: &mut AsyncApp, message: String| {
                this.update(cx, |this, cx| {
                    if this.generation == generation {
                        this.busy = false;
                        cx.emit(failure(message));
                        cx.notify();
                    }
                })
                .ok();
            };
            if let Err(error) = stash.await {
                return failed(&this, cx, format!("{error:#}"));
            }
            let Ok(switch) = this.update(cx, |_, cx| {
                cx.emit(BranchSwitched);
                client.update(cx, |client, cx| client.switch_branch(&cwd, &target, cx))
            }) else {
                return;
            };
            if let Err(error) = switch.await {
                return failed(&this, cx, format!("{error:#}"));
            }
            this.update(cx, |this, cx| {
                if this.generation == generation {
                    this.busy = false;
                    cx.emit(BranchSwitched);
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    /// Offers the Paseo stash of `branch`, if it has one. A failed list is ignored, as in
    /// Electron: the next switch asks again.
    fn offer_restore(&mut self, branch: String, cx: &mut Context<Self>) {
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.stash_list(&cwd, true, cx));
        cx.spawn(async move |this, cx| {
            let entries = match task.await {
                Ok(entries) => entries,
                Err(error) => {
                    log::debug!("stash_list: {error:#}");
                    return;
                }
            };
            this.update(cx, |this, cx| {
                if this.generation == generation
                    && let Some(index) = restore_index(&entries, &branch)
                {
                    this.confirm = Some(Confirm::Restore { index });
                    cx.notify();
                }
            })
            .ok();
        })
        .detach();
    }

    fn restore(&mut self, index: u32, cx: &mut Context<Self>) {
        self.confirm = None;
        self.busy = true;
        cx.notify();
        let generation = self.generation;
        let cwd = self.cwd.clone();
        let task = self
            .client
            .update(cx, |client, cx| client.stash_pop(&cwd, index, cx));
        cx.spawn(async move |this, cx| {
            let result = task.await;
            this.update(cx, |this, cx| {
                if this.generation != generation {
                    return;
                }
                this.busy = false;
                cx.emit(restore_notice(result.map_err(|error| format!("{error:#}"))));
                cx.emit(BranchSwitched);
                cx.notify();
            })
            .ok();
        })
        .detach();
    }

    fn confirm_yes(&mut self, cx: &mut Context<Self>) {
        match self.confirm.take() {
            Some(Confirm::Stash { target }) => self.stash_and_switch(target, cx),
            Some(Confirm::Restore { index }) => self.restore(index, cx),
            None => cx.notify(),
        }
    }

    fn confirm_no(&mut self, cx: &mut Context<Self>) {
        self.confirm = None;
        cx.notify();
    }

    fn trigger(&self, branch: &str, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let open = self.search.is_some();
        let text_size = ui_font_size(FONT_SM, cx);
        let view = cx.weak_entity();
        h_flex()
            .id("changes-branch")
            .role(Role::Button)
            .aria_label(format!("Current branch: {branch}. Press to switch branch."))
            .aria_expanded(open)
            .group("changes-branch")
            .min_w_0()
            .flex_shrink(1.)
            .items_center()
            .gap(px(SPACE_1))
            .p(px(SPACE_1))
            .rounded(px(RADIUS_MD))
            .cursor_pointer()
            .when(open, |el| el.bg(colors.interaction_highlight))
            .hover(move |style| style.bg(colors.interaction_highlight))
            .on_click(cx.listener(|this, _, window, cx| this.open(window, cx)))
            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                view.update(cx, |this, cx| this.open(window, cx)).ok();
            })
            .child(
                div()
                    .min_w_0()
                    .truncate()
                    .text_size(text_size)
                    .line_height(text_size * TEXT_LINE_FACTOR)
                    .text_color(if open {
                        colors.foreground
                    } else {
                        colors.foreground_muted
                    })
                    .group_hover("changes-branch", |style| {
                        style.text_color(colors.foreground)
                    })
                    .child(branch.to_string()),
            )
            .child(
                div().flex_shrink_0().child(
                    icon(IconName::ChevronDown)
                        .with_size(px(ICON_XS))
                        .text_color(colors.foreground_extra_muted),
                ),
            )
            .into_any_element()
    }

    fn list(&self, search: &Search, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let text_size = ui_font_size(FONT_BASE, cx);
        let small = ui_font_size(FONT_SM, cx);
        let current = self.branch.clone();
        let highlight = search.highlight;
        let visible = self.visible_branches(cx);
        let status = match &self.list {
            ListLoad::Failed(message) => Some(message.clone()),
            _ if visible.is_empty() => Some("No branches found.".to_string()),
            _ => None,
        };
        let rows: Vec<AnyElement> = visible
            .into_iter()
            .enumerate()
            .map(|(index, name)| {
                let selected = current.as_deref() == Some(name.as_str());
                let highlighted = highlight == Some(index);
                let view = cx.weak_entity();
                let ax_name = name.clone();
                let click_name = name.clone();
                h_flex()
                    .id(SharedString::from(format!("branch-switcher-item-{index}")))
                    .role(Role::MenuItem)
                    .aria_label(name.clone())
                    .aria_selected(highlighted)
                    .min_h(px(ROW_HEIGHT))
                    .mx(px(SPACE_1))
                    .px(px(SPACE_2))
                    .items_center()
                    .gap(px(SPACE_2))
                    .rounded(px(RADIUS_MD))
                    .cursor_pointer()
                    .when(highlighted, |row| row.bg(colors.surface1))
                    .hover(move |style| style.bg(colors.surface2))
                    .on_click(cx.listener(move |this, _, _, cx| this.pick(click_name.clone(), cx)))
                    .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
                        view.update(cx, |this, cx| this.pick(ax_name.clone(), cx))
                            .ok();
                    })
                    .child(
                        icon(IconName::GitBranch)
                            .with_size(px(ICON_SM))
                            .text_color(colors.foreground_muted),
                    )
                    .child(
                        div()
                            .flex_1()
                            .min_w_0()
                            .truncate()
                            .text_size(text_size)
                            .line_height(text_size * TEXT_LINE_FACTOR)
                            .text_color(colors.foreground)
                            .child(name),
                    )
                    .when(selected, |row| {
                        row.child(
                            icon(IconName::Check)
                                .with_size(px(ICON_SM))
                                .text_color(colors.foreground),
                        )
                    })
                    .into_any_element()
            })
            .collect();
        popup(
            v_flex()
                .id("branch-switcher")
                .role(Role::Dialog)
                .aria_label("Switch branch")
                .min_w(px(LIST_MIN_WIDTH))
                .max_h(px(LIST_MAX_HEIGHT))
                .py(px(SPACE_1))
                .gap(px(SPACE_1))
                .bg(colors.popover)
                .border_1()
                .border_color(colors.border_accent)
                .rounded(px(RADIUS_LG))
                .shadow(vec![shadow(ShadowSize::Md, cx)])
                .occlude()
                .on_mouse_down_out(cx.listener(|this, _, _, cx| this.close(cx)))
                .child(
                    div().px(px(SPACE_2)).child(
                        Input::new(&search.query)
                            .small()
                            .aria_label("Filter branches"),
                    ),
                )
                .child(
                    v_flex()
                        .id("branch-switcher-list")
                        .role(Role::Menu)
                        .aria_label("Branches")
                        .flex_1()
                        .min_h_0()
                        .overflow_y_scroll()
                        .track_scroll(&search.scroll)
                        .children(rows),
                )
                .children(status.map(|status| {
                    div()
                        .px(px(SPACE_3))
                        .py(px(SPACE_2))
                        .text_size(small)
                        .line_height(small * TEXT_LINE_FACTOR)
                        .text_color(colors.foreground_muted)
                        .child(status)
                })),
        )
    }

    fn dialog(&self, confirm: &Confirm, cx: &mut Context<Self>) -> AnyElement {
        let colors = tokens(cx);
        let (title, message, yes, no) = match confirm {
            Confirm::Stash { .. } => (
                "Uncommitted changes",
                "You have uncommitted changes. Stash them before switching branches?".to_string(),
                "Stash & Switch",
                "Cancel",
            ),
            Confirm::Restore { .. } => (
                "Restore stashed changes?",
                "This branch has stashed changes from a previous session. Would you like to \
                 restore them?"
                    .to_string(),
                "Restore",
                "Later",
            ),
        };
        let title_size = ui_font_size(FONT_BASE, cx);
        let small = ui_font_size(FONT_SM, cx);
        popup(
            v_flex()
                .id("branch-switcher-confirm")
                .role(Role::AlertDialog)
                .aria_label(title)
                .w(px(DIALOG_WIDTH))
                .p(px(SPACE_3))
                .gap(px(SPACE_2))
                .bg(colors.popover)
                .border_1()
                .border_color(colors.border_accent)
                .rounded(px(RADIUS_LG))
                .shadow(vec![shadow(ShadowSize::Md, cx)])
                .occlude()
                .child(
                    div()
                        .text_size(title_size)
                        .line_height(title_size * TEXT_LINE_FACTOR)
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(colors.foreground)
                        .child(title),
                )
                .child(
                    div()
                        .text_size(small)
                        .line_height(small * TEXT_LINE_FACTOR)
                        .text_color(colors.foreground_muted)
                        .child(message),
                )
                .child(
                    h_flex()
                        .justify_end()
                        .gap(px(SPACE_2))
                        .child(dialog_button(
                            "branch-switcher-no",
                            no,
                            false,
                            cx,
                            |this, cx| this.confirm_no(cx),
                        ))
                        .child(dialog_button(
                            "branch-switcher-yes",
                            yes,
                            true,
                            cx,
                            |this, cx| this.confirm_yes(cx),
                        )),
                ),
        )
    }
}

/// Places `content` under the label, bottom-start, above the rest of the window.
fn popup(content: impl IntoElement) -> AnyElement {
    div()
        .absolute()
        .top_full()
        .left_0()
        .mt(px(SPACE_1))
        .child(
            deferred(
                anchored()
                    .anchor(Anchor::TopLeft)
                    .snap_to_window_with_margin(px(8.))
                    .child(content),
            )
            .with_priority(gpui_kit::base::POPUP_PRIORITY),
        )
        .into_any_element()
}

fn dialog_button(
    id: &'static str,
    label: &'static str,
    primary: bool,
    cx: &mut Context<BranchSwitcher>,
    run: fn(&mut BranchSwitcher, &mut Context<BranchSwitcher>),
) -> AnyElement {
    let colors = tokens(cx);
    let text_size = ui_font_size(FONT_SM, cx);
    let view = cx.weak_entity();
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .px(px(SPACE_3))
        .py(px(SPACE_1))
        .rounded(px(RADIUS_MD))
        .border_1()
        .border_color(colors.border_accent)
        .cursor_pointer()
        .text_size(text_size)
        .line_height(text_size * TEXT_LINE_FACTOR)
        .when(primary, |el| {
            el.bg(colors.accent)
                .border_color(colors.accent)
                .text_color(colors.accent_foreground)
        })
        .when(!primary, |el| {
            el.text_color(colors.foreground)
                .hover(move |style| style.bg(colors.surface2))
        })
        .on_click(cx.listener(move |this, _, _, cx| run(this, cx)))
        .on_a11y_action(AccessibleAction::Click, move |_, _, cx| {
            view.update(cx, run).ok();
        })
        .child(label)
        .into_any_element()
}

impl Render for BranchSwitcher {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let Some(branch) = self.branch.clone() else {
            return div().id("changes-branch").into_any_element();
        };
        let trigger = self.trigger(&branch, cx);
        let overlay = if let Some(confirm) = &self.confirm {
            Some(self.dialog(confirm, cx))
        } else {
            self.search.as_ref().map(|search| self.list(search, cx))
        };
        div()
            .relative()
            .min_w_0()
            .flex_shrink(1.)
            .child(trigger)
            .children(overlay)
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    use super::{
        BRANCH_CACHE, cache_fresh, enter_row, initial_highlight, next_highlight, offers_stash,
        rank_branches, restore_index, restore_notice, should_switch, switch_failure_notice,
    };
    use crate::explorer::ExplorerNoticeEvent;
    use bachuc_client::StashEntry;
    use std::time::{Duration, Instant};

    fn names(list: &[&str]) -> Vec<String> {
        list.iter().map(|name| name.to_string()).collect()
    }

    fn stash(index: u32, branch: Option<&str>) -> StashEntry {
        StashEntry {
            index,
            message: format!("paseo-stash {index}"),
            branch: branch.map(str::to_string),
            is_paseo: true,
        }
    }

    #[test]
    fn branch_stash_offered_only_for_uncommitted_changes() {
        assert!(offers_stash(
            "Cannot switch branches: you have uncommitted changes"
        ));
        assert!(offers_stash("UNCOMMITTED changes in the worktree"));
        assert!(!offers_stash("Branch not found: feat/x"));
        assert!(!offers_stash(""));
    }

    #[test]
    fn branch_switch_error_and_restore_success_map_to_notices() {
        assert_eq!(
            switch_failure_notice("Branch not found: feat/x"),
            Some(ExplorerNoticeEvent {
                ok: false,
                message: "Branch not found: feat/x".to_string(),
            })
        );
        assert_eq!(switch_failure_notice("you have uncommitted changes"), None);
        assert_eq!(
            restore_notice(Ok(())),
            ExplorerNoticeEvent {
                ok: true,
                message: "Stashed changes restored".to_string(),
            }
        );
        assert_eq!(
            restore_notice(Err("conflict in a.txt".to_string())),
            ExplorerNoticeEvent {
                ok: false,
                message: "conflict in a.txt".to_string(),
            }
        );
    }

    #[test]
    fn branch_restore_picks_the_first_stash_of_the_target() {
        let entries = vec![
            stash(0, Some("main")),
            stash(1, None),
            stash(2, Some("feat/a")),
            stash(3, Some("feat/a")),
        ];
        assert_eq!(restore_index(&entries, "feat/a"), Some(2));
        assert_eq!(restore_index(&entries, "main"), Some(0));
        assert_eq!(restore_index(&entries, "feat/b"), None);
        assert_eq!(restore_index(&[], "main"), None);
    }

    #[test]
    fn branch_pick_of_current_does_nothing() {
        assert!(!should_switch(Some("main"), "main"));
        assert!(should_switch(Some("main"), "feat/a"));
        assert!(should_switch(None, "main"));
    }

    #[test]
    fn branch_list_stays_fresh_for_15_seconds() {
        let now = Instant::now();
        assert!(!cache_fresh(None, now));
        assert!(cache_fresh(Some(now), now));
        assert!(cache_fresh(Some(now), now + Duration::from_secs(14)));
        assert!(!cache_fresh(Some(now), now + BRANCH_CACHE));
    }

    #[test]
    fn branch_highlight_moves_and_wraps() {
        assert_eq!(next_highlight(None, 3, true), Some(0));
        assert_eq!(next_highlight(None, 3, false), Some(2));
        assert_eq!(next_highlight(Some(0), 3, true), Some(1));
        assert_eq!(next_highlight(Some(2), 3, true), Some(0));
        assert_eq!(next_highlight(Some(0), 3, false), Some(2));
        assert_eq!(next_highlight(Some(5), 3, true), Some(0));
        assert_eq!(next_highlight(Some(1), 0, true), None);
    }

    #[test]
    fn branch_highlight_starts_on_current_branch_or_first_match() {
        let list = names(&["main", "feat/a", "feat/b"]);
        assert_eq!(initial_highlight(&list, "", Some("feat/a")), Some(1));
        assert_eq!(initial_highlight(&list, "", Some("gone")), Some(0));
        assert_eq!(initial_highlight(&list, "feat", Some("feat/b")), Some(0));
        assert_eq!(initial_highlight(&[], "", Some("main")), None);
    }

    #[test]
    fn branch_down_down_enter_picks_the_third_row() {
        let list = names(&["main", "feat/a", "feat/b", "fix/c"]);
        let mut highlight = initial_highlight(&list, "", Some("main"));
        highlight = next_highlight(highlight, list.len(), true);
        highlight = next_highlight(highlight, list.len(), true);
        assert_eq!(enter_row(list.len(), highlight), Some(2));
        assert_eq!(enter_row(list.len(), None), Some(0));
        assert_eq!(enter_row(2, Some(3)), Some(0));
        assert_eq!(enter_row(0, Some(0)), None);
    }

    #[test]
    fn branch_empty_query_keeps_daemon_order() {
        let list = names(&["zeta", "main", "alpha"]);
        assert_eq!(rank_branches(&list, "  "), list);
    }

    #[test]
    fn branch_rank_orders_by_match_tier() {
        let list = names(&[
            "feat/remain",
            "maintenance",
            "fix/main-menu",
            "main",
            "m-a-i-n",
            "docs",
        ]);
        assert_eq!(
            rank_branches(&list, "Main"),
            names(&[
                "main",
                "fix/main-menu",
                "maintenance",
                "feat/remain",
                "m-a-i-n"
            ]),
        );
    }

    #[test]
    fn branch_rank_matches_letters_in_order_within_one_word() {
        let list = names(&["feat/gpui-branch", "fix/git", "gb x"]);
        assert_eq!(rank_branches(&list, "gpbr"), names(&["feat/gpui-branch"]));
    }
}
