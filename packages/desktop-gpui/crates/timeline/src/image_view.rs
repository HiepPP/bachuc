//! Images in assistant messages, as `AssistantMarkdownImage` in
//! `packages/app/src/components/message.tsx`: a local path loads through the host's
//! `read_file` (`assistant-image/file-acquisition.ts`), retries as `assistant-image/retry.ts`
//! does, and a click opens a larger view, as `attachment-lightbox.tsx`.

use std::sync::Arc;
use std::time::Duration;

use bachuc_client::FileKind;
use bachuc_icons::IconName;
use bachuc_theme::metrics::{FONT_BASE, ICON_MD, SPACE_1, SPACE_2, SPACE_4, SPACE_6};
use bachuc_theme::{tokens, ui_font_size, zoom, zoomed};
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{Sizable as _, v_flex};
use gpui_kit::*;

use crate::TimelineView;
use crate::links::is_absolute_path;
use crate::tool_view::icon_button;

/// `ASSISTANT_IMAGE_RETRY_DELAYS_MS`.
const RETRY_DELAYS_MS: [u64; 3] = [100, 400, 1_200];
/// `ASSISTANT_IMAGE_MAX_WIDTH`, `ASSISTANT_IMAGE_MAX_HEIGHT`, and the frame of a loading or
/// failed image, `ASSISTANT_IMAGE_MIN_HEIGHT`.
const IMAGE_MAX_WIDTH: f32 = 480.;
const IMAGE_MAX_HEIGHT: f32 = 360.;
const IMAGE_STATE_HEIGHT: f32 = 160.;
/// `LIGHTBOX_FIT` and the backdrop alpha.
const LIGHTBOX_MAX_WIDTH: f32 = 960.;
const LIGHTBOX_MAX_HEIGHT: f32 = 640.;
const BACKDROP_ALPHA: f32 = 0.9;
const UNAVAILABLE: &str = "Image preview unavailable.";

pub(crate) enum ImageLoad {
    Loading,
    Ready {
        image: Arc<Image>,
        size: Size<Pixels>,
    },
    Failed(String),
}

/// Where the host reads an image: `(cwd, path)`, as `resolveFilePreviewReadTarget`. A home path
/// reads from `~`, a relative path from the agent folder, and an absolute path from the folder
/// when it is inside it, else from the file system root.
pub fn read_target(source: &str, cwd: &str) -> Option<(String, String)> {
    let path = source
        .strip_prefix("file://")
        .unwrap_or(source)
        .trim()
        .to_string();
    if path.is_empty() {
        return None;
    }
    if path == "~" || path.starts_with("~/") {
        return Some(("~".into(), path));
    }
    let root = cwd.trim_end_matches('/');
    if !is_absolute_path(&path) {
        return is_absolute_path(cwd).then(|| (cwd.to_string(), path));
    }
    if !root.is_empty() && (path == root || path.starts_with(&format!("{root}/"))) {
        return Some((cwd.to_string(), path));
    }
    Some(("/".into(), path))
}

/// `resolveAssistantImageDisplaySize`: at most 480 wide and 360 high, keeping the aspect ratio.
pub fn display_size(width: u32, height: u32) -> (f32, f32) {
    let ratio = width.max(1) as f32 / height.max(1) as f32;
    let shown = IMAGE_MAX_WIDTH.min(IMAGE_MAX_HEIGHT * ratio).round();
    (shown, (shown / ratio).round())
}

/// The pixel size of a PNG, GIF, or JPEG from its header.
pub fn image_dimensions(bytes: &[u8]) -> Option<(u32, u32)> {
    let be32 = |at: usize| {
        bytes
            .get(at..at + 4)
            .map(|b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    let be16 = |at: usize| {
        bytes
            .get(at..at + 2)
            .map(|b| u16::from_be_bytes([b[0], b[1]]) as u32)
    };
    if bytes.starts_with(b"\x89PNG") {
        return Some((be32(16)?, be32(20)?));
    }
    if bytes.starts_with(b"GIF8") {
        let le16 = |at: usize| {
            bytes
                .get(at..at + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]) as u32)
        };
        return Some((le16(6)?, le16(8)?));
    }
    if bytes.starts_with(&[0xFF, 0xD8]) {
        // Walk the segments to a start-of-frame marker, which holds the height and width.
        let mut at = 2;
        while at + 9 < bytes.len() {
            if bytes[at] != 0xFF {
                return None;
            }
            let marker = bytes[at + 1];
            let length = be16(at + 2)? as usize;
            let is_frame = matches!(marker, 0xC0..=0xCF) && !matches!(marker, 0xC4 | 0xC8 | 0xCC);
            if is_frame {
                return Some((be16(at + 7)?, be16(at + 5)?));
            }
            at += 2 + length;
        }
    }
    None
}

impl TimelineView {
    /// Starts loading each image of a message that has no load yet.
    pub(crate) fn load_image(&mut self, source: &str, cx: &mut Context<Self>) {
        if self.images.contains_key(source) {
            return;
        }
        self.images.insert(source.to_string(), ImageLoad::Loading);
        let Some((cwd, path)) = read_target(source, &self.cwd) else {
            self.images
                .insert(source.to_string(), ImageLoad::Failed(UNAVAILABLE.into()));
            return;
        };
        let client = self.client.clone();
        let source = source.to_string();
        cx.spawn(async move |this, cx| {
            let mut attempt = 0;
            let result = loop {
                let read = client.update(cx, |client, cx| client.read_file(&cwd, &path, cx));
                let outcome = read.await.map_err(|error| error.to_string());
                let decoded = outcome.and_then(|file| {
                    let bytes = file.bytes.filter(|_| file.kind == FileKind::Image);
                    let format = file
                        .mime_type
                        .as_deref()
                        .and_then(ImageFormat::from_mime_type);
                    match (bytes, format) {
                        (Some(bytes), Some(format)) => {
                            let size = image_dimensions(&bytes)
                                .map(|(w, h)| display_size(w, h))
                                .map(|(w, h)| size(px(w), px(h)))
                                .unwrap_or(size(px(IMAGE_MAX_WIDTH), px(IMAGE_STATE_HEIGHT)));
                            Ok(ImageLoad::Ready {
                                image: Arc::new(Image::from_bytes(format, bytes)),
                                size,
                            })
                        }
                        _ => Err(UNAVAILABLE.to_string()),
                    }
                });
                match decoded {
                    Ok(load) => break load,
                    Err(error) => match RETRY_DELAYS_MS.get(attempt) {
                        Some(&delay) => {
                            attempt += 1;
                            cx.background_executor()
                                .timer(Duration::from_millis(delay))
                                .await;
                        }
                        None => break ImageLoad::Failed(error),
                    },
                }
            };
            let _ = this.update(cx, |this, cx| {
                this.images.insert(source.clone(), result);
                this.image_changed(&source, cx);
            });
        })
        .detach();
    }

    fn retry_image(&mut self, source: &str, cx: &mut Context<Self>) {
        self.images.remove(source);
        self.load_image(source, cx);
        self.image_changed(source, cx);
    }

    /// Measures again the rows that show `source`.
    fn image_changed(&mut self, source: &str, cx: &mut Context<Self>) {
        let keys: Vec<u64> = self
            .messages
            .iter()
            .filter(|(_, message)| {
                message.segments.iter().any(|segment| {
                    matches!(segment, crate::markdown::Segment::Image { source: s, .. } if s == source)
                })
            })
            .map(|(key, _)| *key)
            .collect();
        self.rehash_keys(&keys);
        self.refresh(cx);
    }

    /// One image block: a spinner while it loads, the message with a retry button when it
    /// failed, else the image, which opens the larger view on click.
    pub(crate) fn render_image(
        &self,
        id: ElementId,
        alt: &str,
        source: &str,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let t = tokens(cx);
        let state = div()
            .w_full()
            .h(zoomed(IMAGE_STATE_HEIGHT, cx))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .gap(zoomed(SPACE_2, cx))
            .px(zoomed(SPACE_4, cx))
            .py(zoomed(SPACE_6, cx));
        match self.images.get(source) {
            None | Some(ImageLoad::Loading) => state
                .id(id)
                .role(Role::Image)
                .aria_label(format!("{alt}, loading"))
                .child(
                    Spinner::new()
                        .with_size(zoomed(ICON_MD, cx))
                        .color(t.foreground_muted),
                )
                .into_any_element(),
            Some(ImageLoad::Failed(message)) => {
                let retry_source = source.to_string();
                state
                    .id(id)
                    .role(Role::Group)
                    .aria_label(format!("{alt}, failed to load"))
                    .text_size(ui_font_size(FONT_BASE, cx))
                    .text_color(t.foreground_muted)
                    .child(message.clone())
                    .child(
                        icon_button(
                            "image-retry",
                            "Retry",
                            IconName::RotateCw,
                            t.foreground_muted,
                            &t,
                            cx,
                        )
                        .on_click(
                            cx.listener(move |this, _, _, cx| this.retry_image(&retry_source, cx)),
                        ),
                    )
                    .into_any_element()
            }
            Some(ImageLoad::Ready { image, size }) => {
                // The size is kept at 100%; the zoom applies when it draws.
                let zoom = zoom(cx);
                let image = image.clone();
                let open = image.clone();
                div()
                    .id(id)
                    .role(Role::Button)
                    .aria_label(format!("Open image attachment: {alt}"))
                    .mx(zoomed(-SPACE_1, cx))
                    .w(size.width * zoom)
                    .h(size.height * zoom)
                    .max_w_full()
                    .overflow_hidden()
                    .cursor_pointer()
                    .on_click(cx.listener(move |this, _, window, cx| {
                        this.lightbox = Some(open.clone());
                        // Escape closes the view, so the pane takes the keys.
                        this.focus.focus(window, cx);
                        cx.notify();
                    }))
                    .child(img(image).size_full().object_fit(ObjectFit::Contain))
                    .into_any_element()
            }
        }
    }

    /// The larger view over the pane: the image fit in 960 by 640 on a dark backdrop. A click
    /// anywhere or Escape closes it.
    pub(crate) fn render_lightbox(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let image = self.lightbox.clone()?;
        let t = tokens(cx);
        Some(
            div()
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
                .on_click(cx.listener(|this, _, _, cx| {
                    this.lightbox = None;
                    cx.notify();
                }))
                .child(
                    v_flex()
                        .size_full()
                        .max_w(zoomed(LIGHTBOX_MAX_WIDTH, cx))
                        .max_h(zoomed(LIGHTBOX_MAX_HEIGHT, cx))
                        .child(img(image).size_full().object_fit(ObjectFit::Contain)),
                )
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
                )
                .into_any_element(),
        )
    }
}
