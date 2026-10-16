//! `bachuc-plugin-workspace-spaces`: the native client of the workspace-spaces plugin
//! (`hiep-plugins/plugins/workspace-spaces/index.client.tsx`, ADR-0045). It adds the sidebar
//! project filter by Space, the "Move to" project menu, the numbered Spaces strip with its
//! Space menu, and the Spaces page. The server part stays in TypeScript; this crate calls it
//! only through `spaces.catalog` and the `spaces` settings.

mod model;
mod page;
mod space_menu;
mod state;
mod strip;

use crate::state::SpacesState;
use bachuc_icons::IconName;
use bachuc_plugin_host::{
    Contributions, MessageLevel, NativePlugin, PluginContext, ProjectFilter, ProjectMenu,
    ProjectMenuItem, SidebarProject, SidebarSection, Surface,
};
use gpui_kit::{App, AppContext as _, Entity};
use std::rc::Rc;

const PLUGIN_ID: &str = "workspace-spaces";
const SETTINGS_ID: &str = "spaces";
const MOVE_FAILED: &str = "Move failed";

pub fn plugin() -> Box<dyn NativePlugin> {
    Box::new(WorkspaceSpaces)
}

struct WorkspaceSpaces;

impl NativePlugin for WorkspaceSpaces {
    fn id(&self) -> &'static str {
        PLUGIN_ID
    }

    fn contribute(&self, ctx: PluginContext, contributions: &mut Contributions, cx: &mut App) {
        // One state per activation; only these closures hold it, so deactivation drops it with
        // its timer.
        let settings = ctx.settings(SETTINGS_ID, cx);
        let state = cx.new(|cx| SpacesState::new(ctx.clone(), settings, cx));

        let (visible, title, swipe, added) =
            (state.clone(), state.clone(), state.clone(), state.clone());
        contributions.add_project_filter(ProjectFilter {
            id: "spaces",
            is_visible: Rc::new(move |project, cx| visible.read(cx).is_visible(&project.view_key)),
            title: Some(Rc::new(move |cx| title.read(cx).title())),
            on_swipe: Some(Rc::new(move |direction, cx| {
                swipe.update(cx, |state, cx| state.swipe(direction, cx));
            })),
            on_project_added: Some(Rc::new(move |project, cx| {
                added.update(cx, |state, cx| state.add_project(project, cx))
            })),
        });

        let (menu_state, menu_ctx) = (state.clone(), ctx.clone());
        contributions.add_project_menu(ProjectMenu {
            id: "move",
            items: Rc::new(move |project, cx| menu_items(&menu_state, &menu_ctx, project, cx)),
        });

        let section_state = state.clone();
        contributions.add_sidebar_section(SidebarSection {
            id: "spaces",
            view: Rc::new(move |(), _window, cx| strip::build(section_state.clone(), cx)),
        });

        contributions.add_surface(Surface {
            id: "spaces",
            title: Some("Spaces".into()),
            icon: Some(IconName::Layers),
            view: Rc::new(move |_input, window, cx| {
                page::build(ctx.clone(), state.clone(), window, cx)
            }),
        });
    }
}

/// One "Move to" item with a child per Space, the project's own checked. Empty while the
/// Spaces load or a save runs.
fn menu_items(
    state: &Entity<SpacesState>,
    ctx: &PluginContext,
    project: &SidebarProject,
    cx: &App,
) -> Vec<ProjectMenuItem> {
    let choices = state.read(cx).move_choices(&project.view_key);
    if choices.is_empty() {
        return Vec::new();
    }
    let children = choices
        .into_iter()
        .map(|choice| {
            let (state, ctx) = (state.clone(), ctx.clone());
            let (key, target, checked) =
                (project.view_key.clone(), choice.id.clone(), choice.checked);
            ProjectMenuItem {
                id: choice.id.into(),
                title: choice.name.into(),
                disabled: false,
                checked,
                items: Vec::new(),
                on_select: Some(Rc::new(move |(), _window, cx| {
                    // Choosing the Space the project is in does nothing.
                    if checked {
                        return;
                    }
                    let moved = state.update(cx, |state, cx| {
                        state.move_view_to(key.clone(), target.clone(), cx)
                    });
                    let (state, ctx) = (state.clone(), ctx.clone());
                    cx.spawn(async move |cx| {
                        if !moved.await {
                            cx.update(|cx| {
                                let message = state.read(cx).error_or(MOVE_FAILED);
                                ctx.show_message(message.into(), MessageLevel::Error, cx);
                            });
                        }
                    })
                    .detach();
                })),
            }
        })
        .collect();
    vec![ProjectMenuItem {
        id: "move".into(),
        title: "Move to".into(),
        disabled: false,
        checked: false,
        items: children,
        on_select: None,
    }]
}

#[cfg(test)]
mod tests {
    use super::{PLUGIN_ID, plugin};

    #[test]
    fn the_plugin_id_is_workspace_spaces() {
        assert_eq!(plugin().id(), PLUGIN_ID);
        assert_eq!(plugin().id(), "workspace-spaces");
    }
}
