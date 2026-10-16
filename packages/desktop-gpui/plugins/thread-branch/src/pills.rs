//! The composer pills of `client/pills.ts` and `client/draft-pill.ts`: the branch, PR, and
//! changes pills of an agent composer, and the branch pill of the New workspace draft.
//!
//! `PillBehavior` has no menu kind, so the two branch pills use a flush popover whose rows are
//! drawn like the Electron pill menu (`packages/app/src/plugins/buttons/view.tsx:174`,
//! `docs/menus.md`).

use crate::branch::{
    MenuAction, MenuEntry, branch_menu, branch_pill, changes_pill, draft_menu, draft_pill_label,
    pr_pill, sync_tone,
};
use crate::store::Branches;
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{
    ComposerPill, ComposerTarget, PillBehavior, PillPlacement, PillState, PopoverClose, ShowOnDraft,
};
use bachuc_theme::metrics::{FONT_BASE, ICON_SM, RADIUS_MD, SPACE_1, SPACE_2};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::*;
use std::rc::Rc;

/// `MENU_ROW_HEIGHT` of the composer's own menus (`agent_controls/mod.rs`), so these rows match
/// the other pill menus.
const MENU_ROW_HEIGHT: f32 = 32.;
/// The menu width of the composer's pill menus (`plugin_slots.rs`).
const MENU_MIN_WIDTH: f32 = 200.;
const DRAFT_TITLE: &str = "Git branch of the selected project";

#[derive(Clone, Copy)]
enum Kind {
    Branch,
    Pr,
    Changes,
}

type MenuContent = Rc<dyn Fn(ComposerTarget, PopoverClose, &mut Window, &mut App) -> AnyView>;

/// The four pills: three on agent composers, one on the New workspace draft.
pub(crate) fn pills(store: &Rc<Branches>) -> Vec<ComposerPill> {
    let mut pills: Vec<ComposerPill> = [
        ("thread-branch", Kind::Branch),
        ("thread-branch-pr", Kind::Pr),
        ("thread-branch-changes", Kind::Changes),
    ]
    .into_iter()
    .map(|(id, kind)| {
        let store = store.clone();
        ComposerPill {
            id,
            placement: PillPlacement::Track,
            show_on_draft: ShowOnDraft::No,
            workspace_id: None,
            agent_id: None,
            state: Rc::new(move |target, cx| agent_state(&store, kind, target, cx)),
        }
    })
    .collect();
    let store = store.clone();
    pills.push(ComposerPill {
        id: "thread-branch-draft",
        placement: PillPlacement::Track,
        show_on_draft: ShowOnDraft::Only,
        workspace_id: None,
        agent_id: None,
        state: Rc::new(move |target, cx| draft_state(&store, target, cx)),
    });
    pills
}

/// The state of one agent pill, `None` while the agent has no folder, no answer, or no reason
/// to show it.
fn agent_state(
    store: &Rc<Branches>,
    kind: Kind,
    target: &ComposerTarget,
    cx: &App,
) -> Option<PillState> {
    let cwd = store.agent_cwd(target, cx)?;
    let info = store.ask(&cwd, cx)?;
    let store = store.clone();
    match kind {
        Kind::Branch => {
            let pill = branch_pill(&info)?;
            Some(PillState {
                title: pill.title.into(),
                label: Some(pill.label.into()),
                icon: Some(IconName::GitBranch),
                color: sync_tone(pill.sync, &tokens(cx)),
                disabled: false,
                behavior: PillBehavior::Popover {
                    flush: true,
                    content: menu_content(store, false),
                },
            })
        }
        Kind::Pr => {
            let pill = pr_pill(&info)?;
            Some(PillState {
                title: pill.title.into(),
                label: Some(pill.label.into()),
                icon: Some(IconName::GitPullRequest),
                color: None,
                disabled: false,
                behavior: PillBehavior::Action {
                    on_press: Rc::new(move |target, _window, cx| store.open_pr(&target, cx)),
                    new_workspace_panel: None,
                },
            })
        }
        Kind::Changes => {
            let pill = changes_pill(&info)?;
            Some(PillState {
                title: pill.title.into(),
                label: Some(pill.label.into()),
                icon: Some(IconName::FileDiff),
                color: None,
                disabled: false,
                behavior: PillBehavior::Action {
                    on_press: Rc::new(move |target, _window, cx| store.refresh_agent(&target, cx)),
                    new_workspace_panel: None,
                },
            })
        }
    }
}

/// The New workspace pill: always shown on a draft, naming the selected project's branch or
/// why there is none.
fn draft_state(store: &Rc<Branches>, target: &ComposerTarget, cx: &App) -> Option<PillState> {
    let draft = target.draft.as_ref()?;
    store.track_draft(draft);
    let cwd = Branches::draft_cwd(target, cx);
    let info = cwd.as_deref().and_then(|cwd| store.ask(cwd, cx));
    Some(PillState {
        title: DRAFT_TITLE.into(),
        label: Some(draft_pill_label(cwd.as_deref(), info.as_ref()).into()),
        icon: Some(IconName::GitBranch),
        color: None,
        disabled: false,
        behavior: PillBehavior::Popover {
            flush: true,
            content: menu_content(store.clone(), true),
        },
    })
}

fn menu_content(store: Rc<Branches>, draft: bool) -> MenuContent {
    Rc::new(move |target, close, _window, cx| {
        let store = store.clone();
        cx.new(|_| BranchMenu {
            store,
            target,
            close,
            draft,
        })
        .into()
    })
}

/// The popover of a branch pill: the rows of `branch_menu` or `draft_menu`, read from the
/// store each time the popover draws.
struct BranchMenu {
    store: Rc<Branches>,
    target: ComposerTarget,
    close: PopoverClose,
    draft: bool,
}

impl BranchMenu {
    fn entries(&self, cx: &App) -> Vec<MenuEntry> {
        if self.draft {
            return draft_menu();
        }
        self.store
            .agent_cwd(&self.target, cx)
            .and_then(|cwd| self.store.info(&cwd))
            .map(|info| branch_menu(&info))
            .unwrap_or_default()
    }

    /// A press runs the action, then closes the popover.
    fn press(&mut self, action: MenuAction, window: &mut Window, cx: &mut Context<Self>) {
        self.store.run_menu(action, &self.target, cx);
        (self.close)(window, cx);
    }
}

impl Render for BranchMenu {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let entries = self.entries(cx);
        v_flex()
            .id("thread-branch-menu")
            .role(Role::Menu)
            .aria_label("Branch")
            .min_w(px(MENU_MIN_WIDTH))
            .p(px(SPACE_1))
            .children(entries.into_iter().enumerate().map(|(index, entry)| {
                let MenuEntry::Row(row) = entry else {
                    return div()
                        .h(px(1.))
                        .my(px(SPACE_1))
                        .bg(t.border)
                        .into_any_element();
                };
                let action = row.action;
                h_flex()
                    .id(("thread-branch-menu-row", index))
                    .role(Role::MenuItem)
                    .aria_label(row.title.clone())
                    .h(px(MENU_ROW_HEIGHT))
                    .px(px(SPACE_2))
                    .gap(px(SPACE_2))
                    .items_center()
                    .rounded(px(RADIUS_MD))
                    .cursor_pointer()
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(t.foreground)
                    .hover(move |row| row.bg(t.surface2))
                    .on_click(
                        cx.listener(move |this, _, window, cx| this.press(action, window, cx)),
                    )
                    .child(
                        icon(row.icon)
                            .with_size(px(ICON_SM))
                            .text_color(t.foreground_muted),
                    )
                    .child(div().flex_1().truncate().child(row.title))
                    .into_any_element()
            }))
    }
}
