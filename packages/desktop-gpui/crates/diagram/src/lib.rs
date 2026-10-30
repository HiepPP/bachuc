//! Renders Mermaid diagrams and KaTeX math to PNGs that GPUI shows as images. Mermaid goes through
//! Pele on QuickJS and resvg on a background thread (ADR-0067). KaTeX renders in one hidden
//! WebView that returns PNG snapshots (ADR-0037). The WebView sits outside the window's visible
//! content, not `isHidden`, so WebKit still paints it.

mod cache;
mod page;
mod pele;
#[cfg(target_os = "macos")]
mod snapshot;
mod svg;

use std::collections::HashMap;
use std::sync::Arc;

use futures::StreamExt as _;
use futures::channel::mpsc::{UnboundedSender, unbounded};
use gpui_kit::component::ActiveTheme as _;
use gpui_kit::{Context, EventEmitter, Image, ImageFormat, Pixels, Size, Task, Window, px, size};
use serde_json::{Value, json};
use wry::dpi::{LogicalPosition, LogicalSize};
use wry::{Rect, WebViewBuilder};

use cache::RenderCache;
use pele::PeleEngine;
use svg::{PeleTheme, pele_theme};

/// Snapshot pixels per logical pixel.
const SCALE: f32 = 2.0;
/// Wide enough for a display formula, in CSS pixels.
const WEBVIEW_WIDTH: f64 = 1000.0;
const MIN_WEBVIEW_HEIGHT: f64 = 600.0;
/// Fitted widths round up to this many pixels, so a resize or a zoom does not rasterize per pixel.
const FIT_STEP: u32 = 32;
/// The largest fitted PNG, in pixels. A larger view shows the cached PNG scaled up.
const MAX_FIT_PIXELS: u64 = 32 * 1024 * 1024;
/// Fitted PNGs kept at one time, over all diagrams.
const MAX_FITTED: usize = 16;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
pub enum DiagramKind {
    Mermaid,
    Math,
}

impl DiagramKind {
    /// A short name for labels, such as accessibility labels.
    pub fn label(self) -> &'static str {
        match self {
            DiagramKind::Mermaid => "Mermaid diagram",
            DiagramKind::Math => "Math formula",
        }
    }
}

#[derive(Clone, Debug)]
pub enum DiagramState {
    Pending,
    /// A PNG at 2x scale. Show it at [`image_size`].
    Ready(Arc<Image>),
    /// The error text. Show the source as a code block instead.
    Failed(String),
}

/// Emitted when a pending render finishes, as `Ready` or `Failed`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiagramReady {
    pub kind: DiagramKind,
    pub source: String,
}

/// The logical size to show a `Ready` image at: its PNG size divided by the 2x snapshot scale.
/// A Mermaid PNG already holds the content font size, so this size follows it.
pub fn image_size(image: &Image) -> Size<Pixels> {
    let (width, height) = png_size(image.bytes());
    size(px(width as f32 / SCALE), px(height as f32 / SCALE))
}

/// A PNG's width and height in pixels, from its IHDR chunk at bytes 16 and 20.
fn png_size(bytes: &[u8]) -> (u32, u32) {
    let read = |at: usize| {
        bytes
            .get(at..at + 4)
            .map_or(0, |b| u32::from_be_bytes([b[0], b[1], b[2], b[3]]))
    };
    (read(16), read(20))
}

/// The width to rasterize a diagram at, to show it `width` device pixels wide, or `None` when
/// the cached PNG `png` pixels in size is close enough or the fitted PNG would be too large.
fn fit_width(width: f32, png: (u32, u32)) -> Option<u32> {
    if !width.is_finite() || width < 1.0 || png.0 == 0 {
        return None;
    }
    let fitted = (width.ceil() as u32).div_ceil(FIT_STEP) * FIT_STEP;
    let height = u64::from(fitted) * u64::from(png.1) / u64::from(png.0);
    if fitted.abs_diff(png.0) <= FIT_STEP || u64::from(fitted) * height > MAX_FIT_PIXELS {
        return None;
    }
    Some(fitted)
}

enum Event {
    Ready,
    Rendered {
        id: u64,
        rect: (f64, f64, f64, f64),
    },
    Failed {
        id: u64,
        error: String,
    },
    Snapshot {
        id: u64,
        result: Result<Vec<u8>, String>,
    },
    Mermaid {
        id: u64,
        source: String,
        result: Result<pele::Rendered, String>,
    },
    Fitted {
        generation: u64,
        source: String,
        width: u32,
        result: Result<Vec<u8>, String>,
    },
}

fn parse_ipc(body: &str) -> Option<Event> {
    let message: Value = serde_json::from_str(body).ok()?;
    let id = message["id"].as_u64();
    match message["type"].as_str()? {
        "ready" => Some(Event::Ready),
        "rendered" => {
            let n = |key: &str| message[key].as_f64();
            Some(Event::Rendered {
                id: id?,
                rect: (n("x")?, n("y")?, n("w")?, n("h")?),
            })
        }
        "failed" => Some(Event::Failed {
            id: id?,
            error: message["error"].as_str().unwrap_or("render failed").into(),
        }),
        _ => None,
    }
}

/// One hidden WebView shared by all diagrams. Create it once per window with
/// `cx.new(|cx| DiagramRenderer::new(window, cx))`.
pub struct DiagramRenderer {
    webview: wry::WebView,
    events: UnboundedSender<Event>,
    cache: RenderCache,
    pele: PeleEngine,
    /// The theme the cached Mermaid PNGs use.
    theme: PeleTheme,
    /// The resolved SVG and the cached PNG size of each ready Mermaid diagram, by source.
    svgs: HashMap<String, (Arc<str>, (u32, u32))>,
    /// Mermaid PNGs rasterized for the width they show at, by source and width. `None` while
    /// one rasterizes.
    fitted: HashMap<(String, u32), Option<Arc<Image>>>,
    /// Goes up when the cache clears, so a fitted PNG of the old theme is dropped.
    generation: u64,
    ready: bool,
    dark: bool,
    height: f64,
    _events: Task<()>,
}

impl EventEmitter<DiagramReady> for DiagramRenderer {}

impl DiagramRenderer {
    pub fn new(window: &mut Window, cx: &mut Context<DiagramRenderer>) -> DiagramRenderer {
        use raw_window_handle::HasWindowHandle as _;

        let (events, mut receiver) = unbounded();
        let ipc = events.clone();
        let handle = window.window_handle().expect("window handle");
        let webview = WebViewBuilder::new()
            .with_transparent(true)
            .with_bounds(hidden_bounds(MIN_WEBVIEW_HEIGHT))
            .with_initialization_script(page::INIT_SCRIPT)
            .with_custom_protocol(page::PROTOCOL.into(), |_, request| page::respond(request))
            .with_ipc_handler(move |request| {
                if let Some(event) = parse_ipc(request.body()) {
                    let _ = ipc.unbounded_send(event);
                }
            })
            .with_url(page::URL)
            .build_as_child(&handle)
            .expect("diagram webview");

        let task = cx.spawn(async move |this, cx| {
            while let Some(event) = receiver.next().await {
                if this
                    .update(cx, |this, cx| this.on_event(event, cx))
                    .is_err()
                {
                    break;
                }
            }
        });

        DiagramRenderer {
            webview,
            events,
            cache: RenderCache::default(),
            pele: PeleEngine::spawn(),
            theme: current_theme(cx),
            svgs: HashMap::new(),
            fitted: HashMap::new(),
            generation: 0,
            ready: false,
            dark: cx.theme().is_dark(),
            height: MIN_WEBVIEW_HEIGHT,
            _events: task,
        }
    }

    /// Returns the cached result for `source`, or `Pending` and queues one render. Renders run
    /// one at a time; [`DiagramReady`] follows when a pending render finishes.
    pub fn render(
        &mut self,
        kind: DiagramKind,
        source: &str,
        cx: &mut Context<DiagramRenderer>,
    ) -> DiagramState {
        let dark = cx.theme().is_dark();
        // KaTeX takes only the dark flag. Mermaid takes every theme color, custom themes too, and
        // the content size.
        let theme = current_theme(cx);
        if dark != self.dark || theme != self.theme {
            self.dark = dark;
            self.theme = theme;
            self.cache.clear();
            self.svgs.clear();
            self.fitted.clear();
            self.generation += 1;
        }
        let state = self.cache.request(kind, source);
        if matches!(state, DiagramState::Pending) {
            self.start_next(cx);
        }
        state
    }

    /// The Mermaid diagram `source` rasterized to show `width` device pixels wide. `None` means
    /// show the image from [`Self::render`]: it fits, it is not Mermaid, or the fitted PNG still
    /// rasterizes. The GPU scales an image down without mipmaps, so a diagram shown smaller than
    /// its PNG drops pixels of its text and lines, and the text looks thin.
    pub fn fitted(
        &mut self,
        source: &str,
        width: f32,
        cx: &mut Context<DiagramRenderer>,
    ) -> Option<Arc<Image>> {
        let (svg, png) = self.svgs.get(source)?;
        let width = fit_width(width, *png)?;
        let key = (source.to_string(), width);
        if let Some(image) = self.fitted.get(&key) {
            return image.clone();
        }
        // One fitted PNG at a time per diagram. The next request after it asks for the latest width.
        if self
            .fitted
            .iter()
            .any(|((other, _), image)| other == source && image.is_none())
        {
            return None;
        }
        self.fitted.insert(key, None);
        let (svg, events, generation) = (svg.clone(), self.events.clone(), self.generation);
        let source = source.to_string();
        cx.background_executor()
            .spawn(async move {
                let result = svg::rasterize_to_width(&svg, width);
                let _ = events.unbounded_send(Event::Fitted {
                    generation,
                    source,
                    width,
                    result,
                });
            })
            .detach();
        None
    }

    fn start_next(&mut self, cx: &mut Context<Self>) {
        let Some(job) = self.cache.next_job(self.ready) else {
            return;
        };
        match job.kind {
            DiagramKind::Mermaid => self.start_mermaid(job.id, job.source, cx),
            DiagramKind::Math => {
                let script = format!(
                    "renderMath({}, {}, {})",
                    job.id,
                    json!(job.source),
                    self.dark
                );
                if let Err(error) = self.webview.evaluate_script(&script) {
                    self.finish(job.id, Err(error.to_string()), cx);
                }
            }
        }
    }

    /// Sends the source to the Pele thread. The main thread only waits for the reply as an event.
    fn start_mermaid(&mut self, id: u64, source: String, cx: &mut Context<Self>) {
        let reply = self.pele.render_png(source.clone(), self.theme.clone());
        let events = self.events.clone();
        cx.background_executor()
            .spawn(async move {
                let result = reply
                    .await
                    .unwrap_or_else(|_| Err("the Pele engine stopped".into()));
                let _ = events.unbounded_send(Event::Mermaid { id, source, result });
            })
            .detach();
    }

    fn on_event(&mut self, event: Event, cx: &mut Context<Self>) {
        match event {
            Event::Ready => {
                self.ready = true;
                self.start_next(cx);
            }
            Event::Rendered { id, rect } => self.snapshot(id, rect, cx),
            Event::Failed { id, error } => self.finish(id, Err(error), cx),
            Event::Snapshot { id, result } => {
                let image = result.map(|png| Arc::new(Image::from_bytes(ImageFormat::Png, png)));
                self.finish(id, image, cx);
            }
            Event::Mermaid { id, source, result } => {
                let image = result.map(|rendered| {
                    if self.cache.running(id) {
                        let png = png_size(&rendered.png);
                        self.svgs.insert(source, (rendered.svg.into(), png));
                    }
                    Arc::new(Image::from_bytes(ImageFormat::Png, rendered.png))
                });
                self.finish(id, image, cx);
            }
            Event::Fitted {
                generation,
                source,
                width,
                result,
            } => {
                let key = (source, width);
                if generation != self.generation || !self.fitted.contains_key(&key) {
                    return;
                }
                // On an error the entry stays `None`, so the diagram does not rasterize again and
                // shows the cached PNG.
                if let Ok(png) = result {
                    if self.fitted.len() > MAX_FITTED {
                        self.fitted.retain(|_, image| image.is_none());
                    }
                    let image = Arc::new(Image::from_bytes(ImageFormat::Png, png));
                    self.fitted.insert(key, Some(image));
                }
                cx.notify();
            }
        }
    }

    fn snapshot(&mut self, id: u64, rect: (f64, f64, f64, f64), cx: &mut Context<Self>) {
        if !self.cache.running(id) {
            return;
        }
        let (x, y, width, height) = (rect.0.floor(), rect.1.floor(), rect.2.ceil(), rect.3.ceil());
        if width < 1.0 || height < 1.0 {
            self.finish(id, Err("the diagram is empty".into()), cx);
            return;
        }
        // A tall result needs a taller WebView first; the page then reports its bounds again.
        if y + height > self.height {
            self.height = y + height + 40.0;
            let resized = self.webview.set_bounds(hidden_bounds(self.height));
            let measured =
                resized.and_then(|_| self.webview.evaluate_script(&format!("reportBounds({id})")));
            if let Err(error) = measured {
                self.finish(id, Err(error.to_string()), cx);
            }
            return;
        }
        self.take_snapshot(id, (x, y, width, height));
    }

    #[cfg(target_os = "macos")]
    fn take_snapshot(&mut self, id: u64, rect: (f64, f64, f64, f64)) {
        use wry::WebViewExtMacOS as _;

        let events = self.events.clone();
        snapshot::take(&self.webview.webview(), rect, move |result| {
            let _ = events.unbounded_send(Event::Snapshot { id, result });
        });
    }

    #[cfg(not(target_os = "macos"))]
    fn take_snapshot(&mut self, id: u64, _rect: (f64, f64, f64, f64)) {
        let _ = self.events.unbounded_send(Event::Snapshot {
            id,
            result: Err("diagram snapshots need macOS".into()),
        });
    }

    fn finish(&mut self, id: u64, result: Result<Arc<Image>, String>, cx: &mut Context<Self>) {
        if let Some((kind, source)) = self.cache.finish(id, result) {
            cx.emit(DiagramReady { kind, source });
            cx.notify();
        }
        self.start_next(cx);
    }
}

fn current_theme(cx: &gpui_kit::App) -> PeleTheme {
    pele_theme(
        &bachuc_theme::tokens(cx),
        f32::from(bachuc_theme::content_font_size(cx)),
    )
}

/// Bounds left of the window, outside its visible content.
fn hidden_bounds(height: f64) -> Rect {
    Rect {
        position: LogicalPosition::new(-(WEBVIEW_WIDTH + 100.0), 0.0).into(),
        size: LogicalSize::new(WEBVIEW_WIDTH, height).into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn image_size_halves_the_png_size() {
        let mut png = vec![0x89, b'P', b'N', b'G', 0x0d, 0x0a, 0x1a, 0x0a, 0, 0, 0, 13];
        png.extend_from_slice(b"IHDR");
        png.extend_from_slice(&400u32.to_be_bytes());
        png.extend_from_slice(&120u32.to_be_bytes());
        let image = Image::from_bytes(ImageFormat::Png, png);
        assert_eq!(image_size(&image), size(px(200.), px(60.)));
    }

    #[test]
    fn fit_width_rounds_up_and_skips_a_png_that_already_fits() {
        assert_eq!(fit_width(963.0, (1926, 480)), Some(992));
        assert_eq!(fit_width(1900.0, (1926, 480)), None);
        assert_eq!(fit_width(3852.0, (1926, 480)), Some(3872));
        assert_eq!(fit_width(40_000.0, (1926, 480)), None);
        assert_eq!(fit_width(0.0, (1926, 480)), None);
    }

    #[test]
    fn page_messages_parse_into_events() {
        assert!(matches!(
            parse_ipc(r#"{"type":"rendered","id":3,"x":0,"y":4.5,"w":10,"h":20}"#),
            Some(Event::Rendered {
                id: 3,
                rect: (0.0, 4.5, 10.0, 20.0)
            })
        ));
        match parse_ipc(r#"{"type":"failed","id":7,"error":"Parse error on line 2"}"#) {
            Some(Event::Failed { id: 7, error }) => assert_eq!(error, "Parse error on line 2"),
            _ => panic!("expected Failed"),
        }
        assert!(matches!(
            parse_ipc(r#"{"type":"ready"}"#),
            Some(Event::Ready)
        ));
        assert!(parse_ipc("not json").is_none());
    }
}
