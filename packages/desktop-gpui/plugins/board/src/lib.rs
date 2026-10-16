//! `bachuc-plugin-board`: the native client of the board plugin
//! (`hiep-plugins/plugins/board/index.client.tsx`, ADR-0045). This part adds the Board page
//! with its Runs view, the "Board" sidebar item, and the `Mod+D` shortcut. The server part
//! stays in TypeScript; this crate calls it only through the `board.*` RPCs and the host
//! settings.

mod avatar;
mod card;
mod model;
mod page;
mod runs;
mod store;

use crate::page::BoardPage;
use crate::store::BoardStore;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    Contributions, Handler, NativePlugin, PluginContext, Shortcut, SidebarItem, Surface,
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
        contributions.add_surface(Surface {
            id: SURFACE_ID,
            title: Some("Board".into()),
            icon: Some(IconName::Columns3),
            view: Rc::new(move |_input, _window, cx| {
                cx.new(|cx| BoardPage::new(&store, cx)).into()
            }),
        });
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
