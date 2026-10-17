//! The workspace header's project chip, as `ProjectHeaderChip` of
//! `client/project-header-chip.tsx`: the project mark and name in the project's Board color.
//! A press opens the New workspace screen for the project.

use crate::model::colors::{chip_colors, project_initial};
use crate::orb::js_round;
use crate::store::BoardStore;
use bachuc_plugin_host::{HeaderSubtitleInput, NewWorkspaceRequest, PluginContext};
use gpui_kit::component::h_flex;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// Matches the size of the project name it replaces.
const FONT_SIZE: f32 = 14.;

/// The mark's side: `round(fontSize * 1.05)`.
pub(crate) fn mark_size(font_size: f32) -> f32 {
    js_round(f64::from(font_size * 1.05)) as f32
}

/// The New workspace request of a press; `None` without a root path, which disables the chip.
pub(crate) fn chip_request(input: &HeaderSubtitleInput) -> Option<NewWorkspaceRequest> {
    Some(NewWorkspaceRequest {
        cwd: input.project_root_path.clone()?,
        project_id: Some(input.project_id.clone()),
        name: Some(input.project_display_name.clone()),
    })
}

pub(crate) struct HeaderChip {
    input: HeaderSubtitleInput,
    ctx: PluginContext,
    store: WeakEntity<BoardStore>,
    _observe: Subscription,
}

impl HeaderChip {
    pub(crate) fn new(
        input: HeaderSubtitleInput,
        ctx: PluginContext,
        store: &Entity<BoardStore>,
        cx: &mut Context<Self>,
    ) -> Self {
        Self {
            input,
            ctx,
            store: store.downgrade(),
            // Redraws when a hue is assigned.
            _observe: cx.observe(store, |_, _, cx| cx.notify()),
        }
    }
}

impl Render for HeaderChip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = bachuc_theme::tokens(cx);
        let hue = self
            .store
            .upgrade()
            .and_then(|store| store.read(cx).hues().get(&self.input.project_id).copied());
        let colors = chip_colors(hue, &t);
        let mark = mark_size(FONT_SIZE);
        let name = self.input.project_display_name.clone();
        let request = chip_request(&self.input);
        let ctx = self.ctx.clone();
        div()
            .id("board-project-chip")
            .role(Role::Button)
            .aria_label(format!("New workspace in {name}"))
            .flex_shrink(1.)
            .min_w_0()
            .when_some(request, |chip, request| {
                chip.cursor_pointer()
                    .active(|style| style.opacity(0.7))
                    .on_click(move |_, _, cx| ctx.open_new_workspace(request.clone(), cx))
            })
            .child(
                h_flex()
                    .items_center()
                    .gap(px(6.))
                    .flex_shrink(1.)
                    .min_w_0()
                    .py(px(1.))
                    .pl(px(4.))
                    .pr(px(8.))
                    .rounded(px(6.))
                    .border_1()
                    .border_color(colors.border)
                    .bg(colors.background)
                    .child(
                        div()
                            .flex_none()
                            .size(px(mark))
                            .rounded(px(mark * 0.35))
                            .bg(colors.mark_background)
                            .flex()
                            .items_center()
                            .justify_center()
                            .text_size(px(FONT_SIZE * 0.75))
                            .line_height(px(mark))
                            .font_weight(FontWeight::BOLD)
                            .text_color(colors.mark_text)
                            .child(project_initial(&name)),
                    )
                    .child(
                        div()
                            .min_w_0()
                            .flex_shrink(1.)
                            .truncate()
                            .text_size(px(FONT_SIZE))
                            .font_weight(FontWeight::BOLD)
                            .text_color(t.foreground_muted)
                            .child(name),
                    ),
            )
    }
}

#[cfg(test)]
mod tests {
    // Not a glob: `gpui_kit::*` brings GPUI's own `test` attribute.
    use super::{HeaderSubtitleInput, NewWorkspaceRequest, chip_request, mark_size};

    fn input(root: Option<&str>) -> HeaderSubtitleInput {
        HeaderSubtitleInput {
            workspace_id: "w1".into(),
            project_id: "p1".into(),
            project_display_name: "bachuc".into(),
            project_root_path: root.map(str::to_string),
        }
    }

    #[test]
    fn the_mark_is_the_rounded_font_size_times_1_05() {
        assert_eq!(mark_size(14.), 15.);
        assert_eq!(mark_size(12.), 13.);
    }

    #[test]
    fn a_press_opens_the_project_and_needs_a_root_path() {
        assert_eq!(
            chip_request(&input(Some("/p/bachuc"))),
            Some(NewWorkspaceRequest {
                cwd: "/p/bachuc".into(),
                project_id: Some("p1".into()),
                name: Some("bachuc".into()),
            })
        );
        assert_eq!(chip_request(&input(None)), None);
    }
}
