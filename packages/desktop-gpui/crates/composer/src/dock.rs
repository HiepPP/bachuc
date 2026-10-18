//! `packages/app/src/composer/dock/index.tsx` (`ComposerDock`): the content and the composer of
//! a pane. The bottom dock puts the composer under the content; the centered dock (the New
//! workspace screen) centers both in an 820 px column.

use bachuc_theme::metrics::{HEADER_HEIGHT, SPACE_6};
use bachuc_theme::zoomed;
use gpui_kit::component::v_flex;
use gpui_kit::*;

/// The `ComposerDock` layout. `content` scrolls in the centered dock.
pub fn composer_dock(centered: bool, content: AnyElement, composer: AnyElement) -> AnyElement {
    ComposerDock {
        centered,
        content,
        composer,
    }
    .into_any_element()
}

/// A `RenderOnce` wrapper, so the layout sizes read the zoom at render time.
#[derive(IntoElement)]
struct ComposerDock {
    centered: bool,
    content: AnyElement,
    composer: AnyElement,
}

impl RenderOnce for ComposerDock {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        dock_layout(self.centered, self.content, self.composer, cx)
    }
}

fn dock_layout(centered: bool, content: AnyElement, composer: AnyElement, cx: &App) -> AnyElement {
    if centered {
        return v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .pb(zoomed(HEADER_HEIGHT + SPACE_6, cx))
            .child(
                v_flex()
                    .flex_shrink(1.)
                    .w_full()
                    .max_w(zoomed(bachuc_theme::reading_look(cx).content_width, cx))
                    .child(
                        div()
                            .id("composer-dock-setup")
                            .flex_grow_0()
                            .flex_shrink(1.)
                            .min_h_0()
                            .overflow_y_scroll()
                            .child(content),
                    )
                    .child(div().flex_shrink_0().child(composer)),
            )
            .into_any_element();
    }
    v_flex()
        .size_full()
        .overflow_hidden()
        .child(v_flex().flex_1().min_h_0().justify_end().child(content))
        .child(div().w_full().flex_shrink(1.).child(composer))
        .into_any_element()
}
