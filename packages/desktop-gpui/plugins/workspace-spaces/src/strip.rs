//! The numbered Space tabs above the sidebar footer (`client/spaces-strip.tsx`). A tab click
//! selects its Space, a secondary click selects it and opens its menu, and "+" creates a Space.

use crate::space_menu::{MenuClosed, SpaceMenu};
use crate::state::SpacesState;
use bachuc_theme::metrics::{FONT_SM, RADIUS_MD, SPACE_1, SPACE_1_5, SPACE_2};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::cell::RefCell;
use std::collections::HashMap;
use std::rc::Rc;

/// `tab`: the minimum width and the height are 26, the padding is 7.
const TAB_SIZE: f32 = 26.;
const TAB_PADDING: f32 = 7.;
/// The container's `paddingHorizontal` and `paddingVertical`.
const STRIP_PADDING_X: f32 = 10.;
const STRIP_PADDING_Y: f32 = 6.;
const DISABLED: f32 = 0.5;

pub(crate) fn build(state: Entity<SpacesState>, cx: &mut App) -> AnyView {
    cx.new(|cx| Strip::new(state, cx)).into()
}

struct Strip {
    state: Entity<SpacesState>,
    menu: Option<Entity<SpaceMenu>>,
    /// The window bounds of each tab, measured on paint, for the menu anchor.
    tab_bounds: Rc<RefCell<HashMap<String, Bounds<Pixels>>>>,
    _observe: Subscription,
    _menu: Option<Subscription>,
}

impl Strip {
    fn new(state: Entity<SpacesState>, cx: &mut Context<Self>) -> Self {
        let observe = cx.observe(&state, |this, _, cx| {
            this.drop_stale_menu(cx);
            cx.notify();
        });
        Self {
            state,
            menu: None,
            tab_bounds: Rc::new(RefCell::new(HashMap::new())),
            _observe: observe,
            _menu: None,
        }
    }

    /// The menu of a Space that was removed closes with it.
    fn drop_stale_menu(&mut self, cx: &mut Context<Self>) {
        let gone = self.menu.as_ref().is_some_and(|menu| {
            let id = menu.read(cx).space_id();
            self.state
                .read(cx)
                .spaces()
                .is_none_or(|spaces| !spaces.has(id))
        });
        if gone {
            self.menu = None;
            self._menu = None;
        }
    }

    fn open_menu(&mut self, id: &str, window: &mut Window, cx: &mut Context<Self>) {
        self.state.update(cx, |state, cx| state.select(id, cx));
        let Some(anchor) = self.tab_bounds.borrow().get(id).copied() else {
            return;
        };
        let (state, space_id) = (self.state.clone(), id.to_string());
        let menu = cx.new(|cx| SpaceMenu::new(state, space_id, anchor, window, cx));
        self._menu = Some(cx.subscribe(&menu, |this, _, _: &MenuClosed, cx| {
            this.menu = None;
            this._menu = None;
            cx.notify();
        }));
        self.menu = Some(menu);
        cx.notify();
    }
}

impl Render for Strip {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let t = tokens(cx);
        let size = ui_font_size(FONT_SM, cx);
        let (tabs, selected, busy, error, load_error) = {
            let state = self.state.read(cx);
            let tabs: Vec<(String, String)> = state
                .snapshot()
                .map(|snap| {
                    snap.state
                        .spaces
                        .iter()
                        .map(|space| (space.id.clone(), space.name.clone()))
                        .collect()
                })
                .unwrap_or_default();
            (
                tabs,
                state.selected().map(str::to_string),
                state.busy(),
                state.error().to_string(),
                state.load_error().to_string(),
            )
        };
        if tabs.is_empty() {
            // Nothing shows until the Spaces load; a failed load offers a retry.
            if load_error.is_empty() {
                return div().into_any_element();
            }
            let text = format!("Spaces unavailable: {load_error}. Tap to retry.");
            return div()
                .id("space-retry")
                .role(Role::Button)
                .aria_label(text.clone())
                .p(px(SPACE_2))
                .cursor_pointer()
                .text_size(size)
                .text_color(t.foreground_muted)
                .on_click(cx.listener(|this, _, _, cx| {
                    this.state.update(cx, |state, cx| state.reload(cx));
                }))
                .child(text)
                .into_any_element();
        }
        let tab_style = |active: bool| {
            move |tab: Stateful<Div>| {
                tab.relative()
                    .flex()
                    .items_center()
                    .justify_center()
                    .min_w(px(TAB_SIZE))
                    .h(px(TAB_SIZE))
                    .px(px(TAB_PADDING))
                    .rounded(px(RADIUS_MD))
                    .border_1()
                    .border_color(if active { t.accent } else { t.border })
                    .when(active, |tab| tab.bg(t.accent))
            }
        };
        let tab_row = h_flex()
            .flex_wrap()
            .items_center()
            .gap(px(SPACE_1))
            .children(tabs.iter().enumerate().map(|(index, (id, name))| {
                let active = selected.as_deref() == Some(id.as_str());
                let (select_id, menu_id, bounds_id) = (id.clone(), id.clone(), id.clone());
                let known = self.tab_bounds.clone();
                let measure = canvas(
                    move |bounds, _, _| {
                        let mut known = known.borrow_mut();
                        if known.get(&bounds_id) != Some(&bounds) {
                            known.insert(bounds_id.clone(), bounds);
                        }
                    },
                    |_, _, _, _| {},
                )
                .absolute()
                .size_full();
                tab_style(active)(div().id(SharedString::from(format!("space-tab-{id}"))))
                    .role(Role::Tab)
                    .aria_label(name.clone())
                    .aria_selected(active)
                    .aria_description("Right-click for Rename and Remove")
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, _, cx| {
                        this.state
                            .update(cx, |state, cx| state.select(&select_id, cx));
                    }))
                    .on_mouse_down(
                        MouseButton::Right,
                        cx.listener(move |this, _, window, cx| {
                            this.open_menu(&menu_id, window, cx)
                        }),
                    )
                    .child(measure)
                    .child(
                        div()
                            .text_size(size)
                            .text_color(if active {
                                t.accent_foreground
                            } else {
                                t.foreground
                            })
                            .child((index + 1).to_string()),
                    )
            }))
            .child(
                tab_style(false)(div().id("space-create"))
                    .role(Role::Button)
                    .aria_label("Create a Space")
                    .when(busy, |tab| tab.opacity(DISABLED))
                    .when(!busy, |tab| {
                        tab.cursor_pointer().on_click(cx.listener(|this, _, _, cx| {
                            this.state.update(cx, |state, cx| state.create(cx)).detach();
                        }))
                    })
                    .child(div().text_size(size).text_color(t.foreground).child("+")),
            );

        v_flex()
            .gap(px(SPACE_1_5))
            .px(px(STRIP_PADDING_X))
            .py(px(STRIP_PADDING_Y))
            .border_t_1()
            .border_color(t.border)
            .child(tab_row)
            .when(!error.is_empty(), |strip| {
                strip.child(
                    div()
                        .id("space-error")
                        .role(Role::Alert)
                        .aria_label(error.clone())
                        .text_size(size)
                        .text_color(t.status_danger)
                        .child(error.clone()),
                )
            })
            .children(self.menu.clone())
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{STRIP_PADDING_X, STRIP_PADDING_Y, TAB_PADDING, TAB_SIZE};

    #[test]
    fn tabs_are_26_px_in_a_39_px_strip() {
        // `spaces-strip.tsx`: tab `minWidth` and `height` 26 with the 1 px border inside,
        // padding 7, and a strip of a 1 px top border, padding 6, and one tab row.
        assert_eq!(TAB_SIZE, 26.);
        assert_eq!(TAB_PADDING, 7.);
        assert_eq!(STRIP_PADDING_X, 10.);
        assert_eq!(1. + 2. * STRIP_PADDING_Y + TAB_SIZE, 39.);
    }
}
