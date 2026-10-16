//! The app hosts of plugin views (ADR-0045), after Electron's `plugins/surface-screen.tsx`: the
//! surface page and overlay with their header, and the cache that builds each plugin view once
//! per contribution and input. A view builds again only when its plugin contributes again.

use crate::panes::chrome_button;
use bachuc_icons::{IconName, icon};
use bachuc_plugin_host::{
    ContributionIndex, HeaderSubtitle, HeaderSubtitleInput, Surface, SurfaceInput,
    SurfacePresentation, ViewFactory,
};
use bachuc_theme::metrics::{
    FONT_BASE, HEADER_HEIGHT, ICON_SM, RADIUS_LG, RADIUS_MD, SPACE_2, SPACE_3, SPACE_4,
    WEIGHT_MEDIUM,
};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::{BTreeMap, HashMap};
use std::rc::Rc;

/// The overlay card: 92 percent wide up to 1180, 88 percent high.
const OVERLAY_WIDTH: f32 = 0.92;
const OVERLAY_MAX_WIDTH: f32 = 1180.;
const OVERLAY_HEIGHT: f32 = 0.88;
/// The backdrop's `rgba(0, 0, 0, 0.35)`.
const BACKDROP_ALPHA: f32 = 0.35;
/// `HeaderIconBadge`.
const ICON_BADGE: f32 = 24.;

pub const SURFACE_UNAVAILABLE: &str = "This plugin surface is unavailable.";

/// A surface the shell shows, as a page or an overlay.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct OpenSurface {
    pub plugin_id: String,
    pub surface_id: String,
    pub presentation: SurfacePresentation,
    pub params: BTreeMap<String, String>,
}

type SurfaceKey = (String, String, BTreeMap<String, String>);
type SubtitleKey = (&'static str, &'static str, HeaderSubtitleInput);

/// Plugin views built once per contribution and input.
#[derive(Default)]
pub struct PluginViews {
    surfaces: HashMap<SurfaceKey, (ViewFactory<SurfaceInput>, AnyView)>,
    subtitle: Option<(SubtitleKey, ViewFactory<HeaderSubtitleInput>, AnyView)>,
}

impl PluginViews {
    /// The view of an open surface for its params.
    pub fn surface(
        &mut self,
        surface: &Surface,
        open: &OpenSurface,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        let key = (
            open.plugin_id.clone(),
            open.surface_id.clone(),
            open.params.clone(),
        );
        if let Some((factory, view)) = self.surfaces.get(&key)
            && Rc::ptr_eq(factory, &surface.view)
        {
            return view.clone();
        }
        let input = SurfaceInput {
            params: open.params.clone(),
        };
        let view = (surface.view)(input, window, cx);
        self.surfaces
            .insert(key, (surface.view.clone(), view.clone()));
        view
    }

    /// The view of the header subtitle for this input.
    pub fn subtitle(
        &mut self,
        plugin_id: &'static str,
        subtitle: &HeaderSubtitle,
        input: HeaderSubtitleInput,
        window: &mut Window,
        cx: &mut App,
    ) -> AnyView {
        let key = (plugin_id, subtitle.id, input.clone());
        if let Some((cached, factory, view)) = &self.subtitle
            && *cached == key
            && Rc::ptr_eq(factory, &subtitle.view)
        {
            return view.clone();
        }
        let view = (subtitle.view)(input, window, cx);
        self.subtitle = Some((key, subtitle.view.clone(), view.clone()));
        view
    }

    /// Drops the views whose contribution left the index or was contributed again.
    pub fn prune(&mut self, index: &ContributionIndex) {
        self.surfaces
            .retain(|(plugin_id, surface_id, _), (factory, _)| {
                index
                    .surface(plugin_id, surface_id)
                    .is_some_and(|surface| Rc::ptr_eq(factory, &surface.value.view))
            });
        let keep = self
            .subtitle
            .as_ref()
            .is_some_and(|((plugin_id, id, _), factory, _)| {
                index.header_subtitles().iter().any(|entry| {
                    entry.plugin_id == *plugin_id
                        && entry.value.id == *id
                        && Rc::ptr_eq(factory, &entry.value.view)
                })
            });
        if !keep {
            self.subtitle = None;
        }
    }
}

/// The surface header: the icon badge, the title, and the close button.
pub fn surface_header(
    title: SharedString,
    glyph: Option<IconName>,
    close: impl Fn(&mut Window, &mut App) + 'static,
    leading_inset: f32,
    cx: &App,
) -> Stateful<Div> {
    let t = tokens(cx);
    h_flex()
        .id("plugin-surface-header")
        .role(Role::Banner)
        .aria_label(format!("{title} header"))
        .relative()
        .flex_shrink_0()
        .h(px(HEADER_HEIGHT))
        .pl(px(leading_inset + SPACE_3))
        .pr(px(SPACE_3))
        .gap(px(SPACE_2))
        .items_center()
        .bg(t.surface0)
        .border_b_1()
        .border_color(t.border)
        .when_some(glyph, |row, glyph| {
            row.child(
                div()
                    .flex_none()
                    .size(px(ICON_BADGE))
                    .flex()
                    .items_center()
                    .justify_center()
                    .rounded(px(RADIUS_MD))
                    .bg(t.surface2)
                    .child(
                        icon(glyph)
                            .with_size(px(ICON_SM))
                            .text_color(t.foreground_muted),
                    ),
            )
        })
        .child(
            div()
                .id("plugin-surface-title")
                .role(Role::Heading)
                .aria_label(title.clone())
                .flex_1()
                .min_w_0()
                .truncate()
                .text_size(ui_font_size(FONT_BASE, cx))
                .font_weight(FontWeight(WEIGHT_MEDIUM))
                .text_color(t.foreground)
                .child(title),
        )
        .child(
            chrome_button("plugin-surface-close", "Close plugin", IconName::X, cx)
                .on_click(move |_, window, cx| close(window, cx)),
        )
}

/// The surface view, or the unavailable text in `statusDanger`.
pub fn surface_body(view: Option<AnyView>, cx: &App) -> AnyElement {
    let t = tokens(cx);
    let notice = surface_notice(view.is_some());
    match (view, notice) {
        (Some(view), _) => div()
            .flex_1()
            .min_h_0()
            .w_full()
            .child(view)
            .into_any_element(),
        (None, notice) => {
            let notice = notice.unwrap_or(SURFACE_UNAVAILABLE);
            div()
                .id("plugin-surface-unavailable")
                .role(Role::Alert)
                .aria_label(notice)
                .flex_1()
                .p(px(SPACE_4))
                .text_size(ui_font_size(FONT_BASE, cx))
                .text_color(t.status_danger)
                .child(notice)
                .into_any_element()
        }
    }
}

/// The text in place of a surface its plugin does not contribute.
pub fn surface_notice(contributed: bool) -> Option<&'static str> {
    (!contributed).then_some(SURFACE_UNAVAILABLE)
}

/// The page form: the header over the view, in place of the workspace.
pub fn surface_page(header: AnyElement, body: AnyElement, cx: &App) -> AnyElement {
    v_flex()
        .id("plugin-surface-page")
        .role(Role::Main)
        .aria_label("Plugin")
        .flex_1()
        .min_w_0()
        .h_full()
        .bg(tokens(cx).surface0)
        .child(header)
        .child(body)
        .into_any_element()
}

/// The overlay form: a dimmed backdrop that closes on a press, and the centered card.
pub fn surface_overlay(
    header: AnyElement,
    body: AnyElement,
    close: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> AnyElement {
    let t = tokens(cx);
    div()
        .id("plugin-surface-overlay")
        .absolute()
        .inset_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .id("plugin-surface-backdrop")
                .role(Role::Button)
                .aria_label("Dismiss overlay")
                .absolute()
                .inset_0()
                .bg(hsla(0., 0., 0., BACKDROP_ALPHA))
                .on_mouse_down(MouseButton::Left, move |_, window, cx| close(window, cx)),
        )
        .child(
            v_flex()
                .id("plugin-surface-card")
                .role(Role::Dialog)
                .aria_label("Plugin")
                .w(relative(OVERLAY_WIDTH))
                .max_w(px(OVERLAY_MAX_WIDTH))
                .h(relative(OVERLAY_HEIGHT))
                .bg(t.surface0)
                .border_1()
                .border_color(t.border)
                .rounded(px(RADIUS_LG))
                .overflow_hidden()
                .shadow(vec![shadow(ShadowSize::Lg, cx)])
                .occlude()
                .child(header)
                .child(body),
        )
        .into_any_element()
}

#[cfg(test)]
mod tests {
    use super::surface_notice;

    #[test]
    fn an_unknown_surface_shows_the_unavailable_text() {
        assert_eq!(
            surface_notice(false),
            Some("This plugin surface is unavailable.")
        );
        assert_eq!(surface_notice(true), None);
    }
}
