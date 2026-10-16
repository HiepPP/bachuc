//! `bachuc-plugin-watchtower-board`: the native client of the watchtower-board plugin
//! (`hiep-plugins/plugins/watchtower-board/index.client.tsx`, ADR-0045). It adds the
//! `watchtower` surface, the `Mod+G` shortcut, the New workspace `board` panel, the "Open
//! Watchtower board" command, the `tasks` attachment source, and the composer pill. The server
//! part stays in TypeScript; this crate calls it only through its plugin RPCs.

mod cell;
mod model;
mod page;
mod panel;
mod rail;
mod store;
mod task_list;

use crate::model::pill_label;
use crate::page::WatchtowerPage;
use crate::panel::BoardPanel;
use crate::store::WatchtowerStore;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    AttachmentPicker, AttachmentSource, CommandCenterItem, CommandContext, ComposerPill,
    ComposerTarget, Contributions, Handler, NativePlugin, NewWorkspacePanel, OpenSurfaceOptions,
    PillBehavior, PillPlacement, PillState, PluginContext, Shortcut, ShowOnDraft, Surface,
    SurfacePresentation,
};
use gpui_kit::{App, AppContext as _, Entity};
use std::rc::Rc;

const PLUGIN_ID: &str = "watchtower-board";
const SURFACE_ID: &str = "watchtower";
const PANEL_ID: &str = "board";
const TITLE: &str = "Watchtower";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(WatchtowerBoard)
}

struct WatchtowerBoard;

impl NativePlugin for WatchtowerBoard {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        // One store per activation; every contribution reads the same boards.
        let store = cx.new(|cx| WatchtowerStore::new(ctx.clone(), cx));
        cx.defer({
            let store = store.clone();
            move |cx| store.update(cx, |store, cx| store.load_active(cx))
        });

        let page_store = store.clone();
        let page_ctx = ctx.clone();
        contributions.add_surface(Surface {
            id: SURFACE_ID,
            title: Some(TITLE.into()),
            icon: Some(IconName::ListTodo),
            view: Rc::new(move |input, window, cx| {
                let (store, ctx) = (page_store.clone(), page_ctx.clone());
                cx.new(|cx| WatchtowerPage::new(store, ctx, &input.params, window, cx))
                    .into()
            }),
        });

        // Mod+G toggles the board: it opens on the active workspace, and the host closes it when
        // it is open.
        let open = open_board(&ctx, &store);
        contributions.add_shortcut(Shortcut {
            id: "toggle-board",
            combo: "Mod+G",
            surface: Some(SURFACE_ID),
            on_press: Rc::new({
                let open = open.clone();
                move |workspace_id, window, cx| {
                    if let Some(workspace_id) = workspace_id {
                        open(Some(workspace_id), window, cx);
                    }
                }
            }),
        });

        let panel_store = store.clone();
        contributions.add_new_workspace_panel(NewWorkspacePanel {
            id: PANEL_ID,
            title: TITLE.into(),
            icon: IconName::ListTodo,
            view: Rc::new(move |input, window, cx| {
                let store = panel_store.clone();
                cx.new(|cx| BoardPanel::new(store, input.project_id, &input.cwd, window, cx))
                    .into()
            }),
        });

        contributions.add_command_item(CommandCenterItem {
            id: "open-board",
            title: "Open Watchtower board".into(),
            icon: IconName::ListTodo,
            keywords: vec![],
            context: CommandContext::Workspace,
            visible: None,
            on_select: Rc::new({
                let open = open.clone();
                move |input, window, cx| {
                    if let Some(workspace_id) = input.workspace_id {
                        open(Some(workspace_id), window, cx);
                    }
                }
            }),
        });

        contributions.add_attachment_source(task_attachments());

        // One unscoped pill serves every agent composer and New workspace. Its label reads the
        // composer's own board; a press opens the overview over the current screen on that
        // workspace. A draft has no workspace yet, so the host shows the `board` panel instead.
        let on_press: Handler<ComposerTarget> = Rc::new(move |target, window, cx| {
            let workspace_id = target.workspace_id.filter(|_| target.draft.is_none());
            open(workspace_id, window, cx);
        });
        contributions.add_composer_pill(ComposerPill {
            id: "watchtower",
            placement: PillPlacement::Track,
            show_on_draft: ShowOnDraft::Yes,
            workspace_id: None,
            agent_id: None,
            state: Rc::new(move |target, cx| {
                Some(PillState {
                    title: "Open Watchtower".into(),
                    label: Some(pill_text(store.read(cx), target).into()),
                    icon: Some(IconName::ListTodo),
                    color: None,
                    disabled: false,
                    behavior: PillBehavior::Action {
                        on_press: on_press.clone(),
                        new_workspace_panel: Some(PANEL_ID),
                    },
                })
            }),
        });
    }
}

/// Preselects the workspace, when there is one, and opens the overview as an overlay.
fn open_board(ctx: &PluginContext, store: &Entity<WatchtowerStore>) -> Handler<Option<String>> {
    let (ctx, store) = (ctx.clone(), store.clone());
    Rc::new(move |workspace_id, _window, cx| {
        if let Some(workspace_id) = workspace_id {
            store.update(cx, |store, cx| store.preselect(workspace_id, cx));
        }
        let options = OpenSurfaceOptions {
            presentation: SurfacePresentation::Overlay,
            ..OpenSurfaceOptions::default()
        };
        ctx.open_surface(SURFACE_ID, options, cx);
    })
}

/// The pill label: "Watchtower" on a draft, else the composer's board. It only reads the store.
fn pill_text(store: &WatchtowerStore, target: &ComposerTarget) -> String {
    let workspace_id = target
        .workspace_id
        .as_deref()
        .filter(|_| target.draft.is_none());
    let Some(workspace_id) = workspace_id else {
        return TITLE.to_string();
    };
    let board = store.board(workspace_id);
    pill_label(board.ready(), &store.live(workspace_id))
}

/// `taskAttachments` of `shared/board.ts`.
fn task_attachments() -> AttachmentSource {
    AttachmentSource {
        id: "tasks",
        title: "Watchtower task".into(),
        icon: IconName::ListTodo,
        picker: Some(AttachmentPicker {
            title: "Attach Watchtower task".into(),
            search_placeholder: "Search recent workspaces / tasks, or paste a board search key"
                .into(),
            search_method: "watchtower.search",
        }),
        on_open: None,
    }
}

#[cfg(test)]
mod tests {
    use super::{PLUGIN_ID, plugin, task_attachments};
    use bachuc_plugin_host::AttachmentPicker;

    #[test]
    fn the_plugin_id_is_watchtower_board() {
        assert_eq!(plugin().id(), PLUGIN_ID);
    }

    #[test]
    fn the_attachment_source_searches_through_watchtower_search() {
        let source = task_attachments();
        assert_eq!(source.id, "tasks");
        assert_eq!(source.title.as_ref(), "Watchtower task");
        assert_eq!(
            source.picker,
            Some(AttachmentPicker {
                title: "Attach Watchtower task".into(),
                search_placeholder: "Search recent workspaces / tasks, or paste a board search key"
                    .into(),
                search_method: "watchtower.search",
            })
        );
        assert!(source.on_open.is_none());
    }
}
