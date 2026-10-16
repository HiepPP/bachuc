//! `packages/app/src/composer/dock/index.tsx` (`ComposerDock`): the content and the composer of
//! a pane. The bottom dock puts the composer under the content; the centered dock (the New
//! workspace screen) centers both in an 820 px column.

use bachuc_theme::metrics::{HEADER_HEIGHT, MAX_CONTENT_WIDTH, SPACE_6};
use gpui_kit::component::v_flex;
use gpui_kit::*;

/// The `ComposerDock` layout. `content` scrolls in the centered dock.
pub fn composer_dock(centered: bool, content: AnyElement, composer: AnyElement) -> AnyElement {
    if centered {
        return v_flex()
            .size_full()
            .items_center()
            .justify_center()
            .pb(px(HEADER_HEIGHT + SPACE_6))
            .child(
                v_flex()
                    .flex_shrink(1.)
                    .w_full()
                    .max_w(px(MAX_CONTENT_WIDTH))
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
