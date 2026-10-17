//! `bachuc-plugin-plugins-nav`: the native client of the plugins-nav plugin
//! (`hiep-plugins/plugins/plugins-nav/index.client.tsx`, ADR-0045). It adds the `Plugins` sidebar
//! row, whose press opens the Plugins page of the active host.

use bachuc_icons::IconName;
use bachuc_plugin_host::{
    Contributions, Handler, NativePlugin, PluginContext, SidebarAction, SidebarItem, Surface,
    SurfaceInput,
};
use gpui_kit::{App, AppContext as _, Context, IntoElement, Render, Window, div};
use std::rc::Rc;

const PLUGIN_ID: &str = "plugins-nav";
const ITEM_ID: &str = "plugins";
const SURFACE_ID: &str = "plugins";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(PluginsNav)
}

struct PluginsNav;

impl NativePlugin for PluginsNav {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, _cx: &mut App) {
        // The sidebar pins the row to the active host, so the press opens that host's page.
        contributions.add_surface(surface());
        contributions.add_sidebar_item(sidebar_item(Rc::new(move |_workspace, _window, cx| {
            ctx.open_plugins_page(cx)
        })));
    }
}

/// The row. Electron registers the surface with the row; its press opens the Plugins page and
/// needs no workspace.
fn sidebar_item(on_press: Handler<Option<String>>) -> SidebarItem {
    SidebarItem {
        id: ITEM_ID,
        title: "Plugins".into(),
        icon: IconName::Blocks,
        surface: SURFACE_ID,
        action: Some(SidebarAction {
            on_press,
            requires_workspace: false,
            new_workspace_panel: None,
        }),
    }
}

/// An empty surface: the row exists only for its press (`addSurface("plugins", () => null)`).
fn surface() -> Surface {
    Surface {
        id: SURFACE_ID,
        title: None,
        icon: None,
        view: Rc::new(|_input: SurfaceInput, _window, cx| cx.new(|_| EmptySurface).into()),
    }
}

struct EmptySurface;

impl Render for EmptySurface {
    fn render(&mut self, _window: &mut Window, _cx: &mut Context<Self>) -> impl IntoElement {
        div()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use bachuc_plugin_host::ContributionIndex;

    #[test]
    fn sidebar_item_and_surface_ids() {
        let mut contributions = Contributions::default();
        contributions.add_surface(surface());
        contributions.add_sidebar_item(sidebar_item(Rc::new(|_, _, _| {})));
        let index = ContributionIndex::build([(PLUGIN_ID, &contributions)]);

        let [item] = index.sidebar_items() else {
            panic!("expected one sidebar item");
        };
        assert_eq!(item.plugin_id, PLUGIN_ID);
        assert_eq!(item.value.id, "plugins");
        assert_eq!(item.value.title.as_ref(), "Plugins");
        assert_eq!(item.value.icon, IconName::Blocks);
        assert_eq!(item.value.surface, "plugins");
        let action = item.value.action.as_ref().unwrap();
        assert!(!action.requires_workspace);
        assert_eq!(action.new_workspace_panel, None);

        let surface = index.surface(PLUGIN_ID, "plugins").unwrap();
        assert_eq!(surface.value.id, item.value.surface);
        assert!(surface.value.title.is_none() && surface.value.icon.is_none());
    }
}
