//! `bachuc-plugin-board`: the native client of the board plugin
//! (`hiep-plugins/plugins/board/index.client.tsx`, ADR-0045). It adds the Board page with its
//! Runs, Timeline, and Recaps views, the "Board" sidebar item, the `Mod+D` shortcut, the
//! "Thinking orb" settings screen, the workspace header's project chip, and the thread header
//! pills. The server part stays in TypeScript; this crate calls it only through the `board.*`
//! RPCs and the host settings.

mod avatar;
mod card;
mod header_chip;
mod model;
mod orb;
mod orb_settings;
mod orb_view;
mod page;
mod pills;
mod recaps;
mod runs;
mod store;
mod timeline;

use crate::header_chip::HeaderChip;
use crate::model::settings::ORB_SETTINGS_ID;
use crate::orb_settings::OrbSettingsView;
use crate::page::BoardPage;
use crate::store::BoardStore;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    Contributions, Handler, HeaderSubtitle, NativePlugin, PluginContext, SettingsScreen, Shortcut,
    SidebarItem, Surface,
};
use gpui_kit::{App, AppContext as _};
use std::rc::Rc;

const PLUGIN_ID: &str = "board";
const SURFACE_ID: &str = "board";
/// Cmd+D on macOS, the only platform the GPUI app runs on.
const SHORTCUT_COMBO: &str = "Mod+D";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(Board)
}

struct Board;

impl NativePlugin for Board {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        // One store per activation; only these closures hold it, so deactivation drops it.
        let store = cx.new(|cx| BoardStore::new(ctx.clone(), cx));
        let page_store = store.clone();
        contributions.add_surface(Surface {
            id: SURFACE_ID,
            title: Some("Board".into()),
            icon: Some(IconName::Columns3),
            view: Rc::new(move |_input, _window, cx| {
                cx.new(|cx| BoardPage::new(&page_store, cx)).into()
            }),
        });
        let orb_settings = store.read(cx).orb_settings();
        contributions.add_settings_screen(SettingsScreen {
            id: ORB_SETTINGS_ID,
            title: "Thinking orb".into(),
            icon: IconName::Sparkles,
            view: Rc::new(move |(), _window, cx| {
                let settings = orb_settings.clone();
                cx.new(|cx| OrbSettingsView::new(settings, cx)).into()
            }),
        });
        let (chip_store, chip_ctx) = (store.clone(), ctx.clone());
        contributions.add_header_subtitle(HeaderSubtitle {
            id: "project",
            view: Rc::new(move |input, _window, cx| {
                let ctx = chip_ctx.clone();
                cx.new(|cx| HeaderChip::new(input, ctx, &chip_store, cx))
                    .into()
            }),
        });
        for pill in pills::header_pills(&store, &ctx) {
            contributions.add_composer_pill(pill);
        }
        contributions.add_sidebar_item(SidebarItem {
            id: "board",
            title: "Board".into(),
            icon: IconName::Columns3,
            surface: SURFACE_ID,
            action: None,
        });
        contributions.add_shortcut(board_shortcut(Rc::new(move |_workspace, _window, cx| {
            ctx.open_surface(SURFACE_ID, Default::default(), cx);
        })));
    }
}

fn board_shortcut(on_press: Handler<Option<String>>) -> Shortcut {
    Shortcut {
        id: "open-board",
        combo: SHORTCUT_COMBO,
        surface: Some(SURFACE_ID),
        on_press,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mod_d_opens_the_board_surface() {
        let shortcut = board_shortcut(Rc::new(|_, _, _| {}));
        assert_eq!(shortcut.id, "open-board");
        assert_eq!(shortcut.combo, "Mod+D");
        assert_eq!(shortcut.surface, Some("board"));
        // One plain chord: no Shift, Alt, or Ctrl, and no second key.
        let keys: Vec<&str> = shortcut.combo.split('+').collect();
        assert_eq!(keys, ["Mod", "D"]);
    }
}
