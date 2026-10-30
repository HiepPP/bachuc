//! The larger view of an image or diagram over the pane, as `attachment-lightbox.tsx`, with zoom
//! and pan. It opens at fit, never above 100%. At 100% a diagram shows at its thread size.
//!
//! A two-finger swipe or a drag pans; a pinch, Cmd+scroll, the toolbar, or Cmd+= / Cmd+- zooms;
//! a double-click or Cmd+0 fits again. A click outside the image or Escape closes the view.

use std::cell::Cell;
use std::path::PathBuf;
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

use bachuc_icons::IconName;
use bachuc_theme::metrics::{FONT_BASE, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2, SPACE_4};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size, zoomed};
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{h_flex, v_flex};
use gpui_kit::*;

use crate::TimelineView;
use crate::image_view::image_dimensions;
use crate::tool_view::icon_button;

const BACKDROP_ALPHA: f32 = 0.9;
const MIN_ZOOM: f32 = 0.25;
const MAX_ZOOM: f32 = 8.;
/// The zoom change of one toolbar or Cmd+= / Cmd+- step.
const ZOOM_STEP: f32 = 0.25;
/// Zoom change per pixel of Cmd+scroll.
const SCROLL_ZOOM_RATE: f32 = 0.005;
/// Image pixels per logical pixel at 100%. Diagrams render at 2x, so 100% is their thread size.
const PIXELS_PER_POINT: f32 = 2.;
/// How long a copy button shows its check and the "Copied" badge, as the turn copy button.
const COPIED_FOR: Duration = Duration::from_millis(1500);
const COPIED_SOURCE: &str = "Copied source";
const COPIED_IMAGE: &str = "Copied image";

/// The image in the larger view, with its zoom and pan.
pub(crate) struct Lightbox {
    image: Arc<Image>,
    /// The size at 100%. Zero when the image header has no size.
    natural: Size<Pixels>,
    /// The diagram source for Copy source. `None` for an attached image.
    source: Option<String>,
    /// `None` keeps the image at fit, so the fit follows the pane size.
    zoom: Option<f32>,
    /// The image center relative to the area center.
    offset: Point<Pixels>,
    /// The pointer position and offset when a drag started.
    drag: Option<(Point<Pixels>, Point<Pixels>)>,
    /// True after a drag moved the image. The release then does not close the view.
    dragged: bool,
    /// The size of the area the image draws in, measured at each paint.
    area: Rc<Cell<Size<Pixels>>>,
    /// Takes Cmd+= / Cmd+- / Cmd+0 before the app zoom bindings while the view is open.
    _keys: Option<Subscription>,
    /// The copy just done, as the badge text, until [`COPIED_FOR`] ends.
    copied: Option<(&'static str, Task<()>)>,
}

impl Lightbox {
    fn new(image: Arc<Image>, source: Option<String>) -> Lightbox {
        let (width, height) = image_dimensions(&image.bytes).unwrap_or((0, 0));
        Lightbox {
            image,
            natural: size(
                px(width as f32 / PIXELS_PER_POINT),
                px(height as f32 / PIXELS_PER_POINT),
            ),
            source,
            zoom: None,
            offset: Point::default(),
            drag: None,
            dragged: false,
            area: Rc::default(),
            _keys: None,
            copied: None,
        }
    }

    fn has_size(&self) -> bool {
        self.natural.width > px(0.) && self.natural.height > px(0.)
    }

    /// The zoom that fits the image in the area, at most 100%.
    fn fit_zoom(&self) -> f32 {
        let area = self.area.get();
        if !self.has_size() || area.width <= px(0.) || area.height <= px(0.) {
            return 1.;
        }
        (area.width / self.natural.width)
            .min(area.height / self.natural.height)
            .min(1.)
    }

    fn zoom(&self) -> f32 {
        self.zoom.unwrap_or_else(|| self.fit_zoom())
    }

    /// Sets the zoom within 25% to 800% and keeps the same image point in the center.
    fn zoom_to(&mut self, zoom: f32) {
        if !zoom.is_finite() || zoom <= 0. {
            return;
        }
        let zoom = zoom.clamp(MIN_ZOOM, MAX_ZOOM);
        self.offset *= zoom / self.zoom();
        self.zoom = Some(zoom);
        self.offset = self.shown_offset();
    }

    fn zoom_by(&mut self, factor: f32) {
        self.zoom_to(self.zoom() * factor);
    }

    fn step(&mut self, steps: f32) {
        self.zoom_to(self.zoom() + steps * ZOOM_STEP);
    }

    fn fit(&mut self) {
        self.zoom = None;
        self.offset = Point::default();
        self.drag = None;
    }

    /// The offset limited so that an image larger than the area keeps its edges at the area
    /// edges, and an image that fits stays centered.
    fn shown_offset(&self) -> Point<Pixels> {
        let zoom = self.zoom();
        let area = self.area.get();
        let limit = |natural: Pixels, area: Pixels| {
            (f32::from(natural) * zoom - f32::from(area)).max(0.) / 2.
        };
        let (x, y) = (
            limit(self.natural.width, area.width),
            limit(self.natural.height, area.height),
        );
        point(
            px(f32::from(self.offset.x).clamp(-x, x)),
            px(f32::from(self.offset.y).clamp(-y, y)),
        )
    }

    fn can_pan(&self) -> bool {
        let zoom = self.zoom();
        let area = self.area.get();
        f32::from(self.natural.width) * zoom > f32::from(area.width)
            || f32::from(self.natural.height) * zoom > f32::from(area.height)
    }

    fn pan_by(&mut self, delta: Point<Pixels>) {
        self.offset = self.shown_offset() + delta;
        self.offset = self.shown_offset();
    }

    fn start_drag(&mut self, position: Point<Pixels>) {
        self.drag = Some((position, self.shown_offset()));
        self.dragged = false;
    }

    /// Pans by the pointer move since the drag started.
    fn drag_to(&mut self, position: Point<Pixels>) -> bool {
        let Some((start, offset)) = self.drag else {
            return false;
        };
        self.offset = offset + (position - start);
        self.offset = self.shown_offset();
        self.dragged = true;
        true
    }

    /// The file name for Save, with the extension of the image format.
    fn file_name(&self) -> String {
        let extension = match self.image.format {
            ImageFormat::Jpeg => "jpg",
            ImageFormat::Gif => "gif",
            ImageFormat::Webp => "webp",
            ImageFormat::Svg => "svg",
            ImageFormat::Bmp => "bmp",
            ImageFormat::Tiff => "tiff",
            _ => "png",
        };
        let stem = if self.source.is_some() {
            "diagram"
        } else {
            "image"
        };
        format!("{stem}.{extension}")
    }
}

impl TimelineView {
    /// Opens the larger view. `source` is the Mermaid or math source of a diagram.
    pub(crate) fn open_lightbox(
        &mut self,
        image: Arc<Image>,
        source: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let mut view = Lightbox::new(image, source);
        let timeline = cx.entity().downgrade();
        let handle = window.window_handle();
        view._keys = Some(cx.intercept_keystrokes(move |event, window, cx| {
            let keystroke = &event.keystroke;
            let modifiers = keystroke.modifiers;
            if window.window_handle() != handle
                || !modifiers.platform
                || modifiers.control
                || modifiers.alt
            {
                return;
            }
            let act: fn(&mut Lightbox) = match keystroke.key.as_str() {
                "=" | "+" => |view| view.step(1.),
                "-" => |view| view.step(-1.),
                "0" => Lightbox::fit,
                _ => return,
            };
            let handled = timeline
                .update(cx, |this, cx| {
                    let Some(view) = this.lightbox.as_mut() else {
                        return false;
                    };
                    act(view);
                    cx.notify();
                    true
                })
                .unwrap_or(false);
            if handled {
                cx.stop_propagation();
            }
        }));
        self.lightbox = Some(view);
        // Escape closes the view, so the pane takes the keys.
        self.focus.focus(window, cx);
        cx.notify();
    }

    /// Writes `item` to the clipboard and shows `badge` with a check on the button.
    fn copy_from_lightbox(
        &mut self,
        item: ClipboardItem,
        badge: &'static str,
        cx: &mut Context<Self>,
    ) {
        cx.write_to_clipboard(item);
        let Some(view) = self.lightbox.as_mut() else {
            return;
        };
        let timer = cx.spawn(async move |this, cx| {
            cx.background_executor().timer(COPIED_FOR).await;
            let _ = this.update(cx, |this, cx| {
                if let Some(view) = this.lightbox.as_mut() {
                    view.copied = None;
                    cx.notify();
                }
            });
        });
        view.copied = Some((badge, timer));
        cx.notify();
    }

    fn save_lightbox_image(&mut self, cx: &mut Context<Self>) {
        let Some(view) = self.lightbox.as_ref() else {
            return;
        };
        let image = view.image.clone();
        let directory = std::env::var_os("HOME")
            .map(|home| PathBuf::from(home).join("Downloads"))
            .unwrap_or_default();
        let path = cx.prompt_for_new_path(&directory, Some(&view.file_name()));
        cx.spawn(async move |_, _| {
            if let Ok(Ok(Some(path))) = path.await
                && let Err(error) = std::fs::write(&path, &image.bytes)
            {
                log::warn!("saving the image to {} failed: {error}", path.display());
            }
        })
        .detach();
    }

    pub(crate) fn render_lightbox(
        &self,
        window: &Window,
        cx: &mut Context<Self>,
    ) -> Option<AnyElement> {
        let view = self.lightbox.as_ref()?;
        let t = tokens(cx);
        let zoom = view.zoom();
        // A diagram rasterizes again at the size it shows, so its text stays sharp at any zoom.
        let shown = view
            .source
            .as_ref()
            .filter(|_| view.has_size())
            .and_then(|source| {
                let width = f32::from(view.natural.width) * zoom * window.scale_factor();
                self.diagrams
                    .update(cx, |renderer, cx| renderer.fitted(source, width, cx))
            })
            .unwrap_or_else(|| view.image.clone());
        let offset = view.shown_offset();
        let cursor = match (view.drag.is_some(), view.can_pan()) {
            (true, _) => CursorStyle::ClosedHand,
            (false, true) => CursorStyle::OpenHand,
            _ => CursorStyle::Arrow,
        };
        let area = view.area.clone();
        let measure = canvas(
            move |bounds, window, cx| {
                // A new area changes the fit, so the next frame draws with it.
                if area.replace(bounds.size) != bounds.size {
                    window.defer(cx, |window, _| window.refresh());
                }
            },
            |_, _, _, _| {},
        )
        .absolute()
        .size_full();
        let image = if view.has_size() {
            div()
                .flex_none()
                .relative()
                .left(offset.x)
                .top(offset.y)
                .w(view.natural.width * zoom)
                .h(view.natural.height * zoom)
                .child(img(shown).size_full())
        } else {
            // A format without a readable size fits the area and does not zoom.
            div().size_full().child(
                img(view.image.clone())
                    .size_full()
                    .object_fit(ObjectFit::Contain),
            )
        };
        let area = div()
            .id("lightbox-image")
            .relative()
            .size_full()
            .flex()
            .items_center()
            .justify_center()
            .overflow_hidden()
            .cursor(cursor)
            .on_scroll_wheel(cx.listener(|this, event: &ScrollWheelEvent, _, cx| {
                let Some(view) = this.lightbox.as_mut() else {
                    return;
                };
                let delta = event.delta.pixel_delta(px(20.));
                if event.modifiers.platform {
                    view.zoom_by((f32::from(delta.y) * SCROLL_ZOOM_RATE).exp());
                } else {
                    view.pan_by(delta);
                }
                cx.stop_propagation();
                cx.notify();
            }))
            .on_pinch(cx.listener(|this, event: &PinchEvent, _, cx| {
                if let Some(view) = this.lightbox.as_mut() {
                    view.zoom_by(1. + event.delta);
                    cx.notify();
                }
            }))
            .on_mouse_down(
                MouseButton::Left,
                cx.listener(|this, event: &MouseDownEvent, _, cx| {
                    if let Some(view) = this.lightbox.as_mut() {
                        view.start_drag(event.position);
                        cx.notify();
                    }
                }),
            )
            .on_click(cx.listener(|this, event: &ClickEvent, _, cx| {
                // A click on the image keeps the view open.
                cx.stop_propagation();
                if event.click_count() == 2
                    && let Some(view) = this.lightbox.as_mut()
                {
                    view.fit();
                    cx.notify();
                }
            }))
            .child(measure)
            .child(image);
        let panel = div()
            .size_full()
            // A transparent image, such as a diagram, draws dark text for the thread background.
            // The panel keeps that text readable on the dark backdrop.
            .p(zoomed(SPACE_4, cx))
            .rounded(zoomed(RADIUS_MD, cx))
            .bg(t.surface0)
            .child(area);
        let lightbox = div()
            .id("lightbox")
            .role(Role::Dialog)
            .aria_label("Image")
            .absolute()
            .inset_0()
            .flex()
            .items_center()
            .justify_center()
            .p(zoomed(SPACE_4, cx))
            .bg(black().opacity(BACKDROP_ALPHA))
            .cursor_pointer()
            // The backdrop takes the moves and the release, so a drag may leave the image.
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                let Some(view) = this.lightbox.as_mut() else {
                    return;
                };
                // A release outside the window sends no mouse up; the next move ends the drag.
                if event.pressed_button != Some(MouseButton::Left) {
                    if view.drag.take().is_some() {
                        cx.notify();
                    }
                } else if view.drag_to(event.position) {
                    cx.notify();
                }
            }))
            // Capture phase: the image's click handler stops the release from bubbling here.
            .capture_any_mouse_up(cx.listener(|this, event: &MouseUpEvent, _, cx| {
                if event.button == MouseButton::Left
                    && let Some(view) = this.lightbox.as_mut()
                    && view.drag.take().is_some()
                {
                    cx.notify();
                }
            }))
            .on_click(cx.listener(|this, _, _, cx| {
                if this.lightbox.as_ref().is_some_and(|view| view.dragged) {
                    return;
                }
                this.lightbox = None;
                cx.notify();
            }))
            .child(panel)
            .child(self.render_lightbox_toolbar(view, zoom, cx))
            .child(
                div()
                    .absolute()
                    .top(zoomed(SPACE_2, cx))
                    .right(zoomed(SPACE_2, cx))
                    .child(
                        icon_button(
                            "lightbox-close",
                            "Dismiss image",
                            IconName::X,
                            t.foreground_muted,
                            &t,
                            cx,
                        )
                        .on_click(cx.listener(|this, _, _, cx| {
                            this.lightbox = None;
                            cx.notify();
                        })),
                    ),
            );
        // It paints deferred and occludes, so the pane's corner pills, drawn after the
        // timeline, stay under it and take no clicks.
        Some(
            deferred(lightbox.occlude())
                .with_priority(1)
                .into_any_element(),
        )
    }

    fn render_lightbox_toolbar(
        &self,
        view: &Lightbox,
        zoom: f32,
        cx: &mut Context<Self>,
    ) -> impl IntoElement {
        let t = tokens(cx);
        let text_button = |id: &'static str, tip: &'static str, text: &'static str| {
            let fill = t.interaction_highlight;
            div()
                .id(id)
                .role(Role::Button)
                .aria_label(tip)
                .px(zoomed(SPACE_2, cx))
                .py(zoomed(SPACE_1, cx))
                .rounded(zoomed(RADIUS_MD, cx))
                .cursor_pointer()
                .hover(move |button| button.bg(fill))
                .tooltip(move |window, cx| Tooltip::new(tip).build(window, cx))
                .child(text)
        };
        let tool_button = |id: &'static str, tip: &'static str, name: IconName| {
            icon_button(id, tip, name, t.foreground, &t, cx)
                .tooltip(move |window, cx| Tooltip::new(tip).build(window, cx))
        };
        let copied = view.copied.as_ref().map(|(badge, _)| *badge);
        // A copy button shows a check while its badge shows.
        let copy_icon = |badge: &'static str, name: IconName| {
            if copied == Some(badge) {
                IconName::Check
            } else {
                name
            }
        };
        let divider = || div().w(px(1.)).h(zoomed(SPACE_4, cx)).bg(t.border);
        let zoom_action = |act: fn(&mut Lightbox)| {
            cx.listener(move |this: &mut TimelineView, _: &ClickEvent, _, cx| {
                if let Some(view) = this.lightbox.as_mut() {
                    act(view);
                    cx.notify();
                }
            })
        };
        let copy_source = view.source.clone().map(|source| {
            tool_button(
                "lightbox-copy-source",
                "Copy source",
                copy_icon(COPIED_SOURCE, IconName::Code),
            )
            .on_click(cx.listener(move |this, _, _, cx| {
                this.copy_from_lightbox(
                    ClipboardItem::new_string(source.clone()),
                    COPIED_SOURCE,
                    cx,
                )
            }))
        });
        let image = view.image.clone();
        let toolbar = h_flex()
            .id("lightbox-toolbar")
            .role(Role::Toolbar)
            .aria_label("Image zoom")
            .gap(zoomed(SPACE_1, cx))
            .p(zoomed(SPACE_1, cx))
            .rounded(zoomed(RADIUS_LG, cx))
            .border_1()
            .border_color(t.border)
            .bg(t.surface1)
            .shadow(vec![shadow(ShadowSize::Md, cx)])
            .text_size(ui_font_size(FONT_BASE, cx))
            .text_color(t.foreground)
            .cursor_default()
            // The toolbar sits on the backdrop, so its clicks must not close the view.
            .on_click(|_, _, cx| cx.stop_propagation())
            .child(
                text_button(
                    "lightbox-zoom-out",
                    "Zoom out (\u{2318}\u{2212})",
                    "\u{2212}",
                )
                .on_click(zoom_action(|view| view.step(-1.))),
            )
            .child(
                div()
                    .min_w(zoomed(SPACE_4 * 3., cx))
                    .text_center()
                    .text_color(t.foreground_muted)
                    .child(format!("{}%", (zoom * 100.).round())),
            )
            .child(
                text_button("lightbox-zoom-in", "Zoom in (\u{2318}=)", "+")
                    .on_click(zoom_action(|view| view.step(1.))),
            )
            .child(divider())
            .child(
                text_button("lightbox-fit", "Fit to view (\u{2318}0)", "Fit")
                    .on_click(zoom_action(Lightbox::fit)),
            )
            .child(
                text_button("lightbox-actual-size", "Actual size", "100%")
                    .on_click(zoom_action(|view| view.zoom_to(1.))),
            )
            .child(divider())
            .children(copy_source)
            .child(
                tool_button(
                    "lightbox-copy-image",
                    "Copy image",
                    copy_icon(COPIED_IMAGE, IconName::Image),
                )
                .on_click(cx.listener(move |this, _, _, cx| {
                    this.copy_from_lightbox(ClipboardItem::new_image(&image), COPIED_IMAGE, cx)
                })),
            )
            .child(
                tool_button("lightbox-save", "Save image", IconName::Download)
                    .on_click(cx.listener(|this, _, _, cx| this.save_lightbox_image(cx))),
            );
        // The badge sits above the toolbar, so the toolbar does not move.
        let badge = copied.map(|badge| {
            div()
                .id("lightbox-copied")
                .absolute()
                .bottom_full()
                .mb(zoomed(SPACE_2, cx))
                .px(zoomed(SPACE_2, cx))
                .py(zoomed(SPACE_1, cx))
                .rounded(zoomed(RADIUS_MD, cx))
                .bg(t.foreground)
                .text_color(t.surface0)
                .text_size(ui_font_size(FONT_BASE, cx))
                .role(Role::Status)
                .aria_label(badge)
                .child(badge)
        });
        let toolbar = v_flex()
            .relative()
            .items_center()
            .children(badge)
            .child(toolbar);
        h_flex()
            .absolute()
            .left_0()
            .right_0()
            .bottom(zoomed(SPACE_4 * 2., cx))
            .justify_center()
            .child(toolbar)
    }
}

#[cfg(test)]
mod tests {
    use super::{Lightbox, MAX_ZOOM, MIN_ZOOM};
    use gpui_kit::{Image, ImageFormat, Point, point, px, size};
    use std::sync::Arc;

    /// A 2000 by 1000 PNG header: 1000 by 500 at 100%.
    fn view() -> Lightbox {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&2000u32.to_be_bytes());
        png.extend_from_slice(&1000u32.to_be_bytes());
        let view = Lightbox::new(Arc::new(Image::from_bytes(ImageFormat::Png, png)), None);
        view.area.set(size(px(500.), px(500.)));
        view
    }

    #[test]
    fn opens_at_fit_and_never_above_actual_size() {
        let view = view();
        assert_eq!(view.zoom(), 0.5);
        view.area.set(size(px(4000.), px(4000.)));
        assert_eq!(view.zoom(), 1.);
    }

    #[test]
    fn zoom_stays_in_range_and_fit_resets() {
        let mut view = view();
        view.zoom_to(100.);
        assert_eq!(view.zoom(), MAX_ZOOM);
        view.zoom_to(0.01);
        assert_eq!(view.zoom(), MIN_ZOOM);
        view.zoom_to(f32::NAN);
        assert_eq!(view.zoom(), MIN_ZOOM);
        view.step(1.);
        assert_eq!(view.zoom(), MIN_ZOOM + 0.25);
        view.fit();
        assert_eq!((view.zoom(), view.offset), (0.5, Point::default()));
    }

    #[test]
    fn pan_stops_at_the_image_edges_and_a_fitted_image_stays_centered() {
        let mut view = view();
        view.pan_by(point(px(50.), px(50.)));
        assert_eq!(view.shown_offset(), Point::default());

        // At 100% the image is 1000 by 500 in a 500 by 500 area: 250 to spare on each side.
        view.zoom_to(1.);
        view.start_drag(point(px(0.), px(0.)));
        assert!(view.drag_to(point(px(400.), px(30.))));
        assert_eq!(view.shown_offset(), point(px(250.), px(0.)));
        assert!(view.dragged);
    }

    #[test]
    fn save_name_follows_the_kind_and_format() {
        let mut view = view();
        assert_eq!(view.file_name(), "image.png");
        view.source = Some("pie".into());
        assert_eq!(view.file_name(), "diagram.png");
    }
}
