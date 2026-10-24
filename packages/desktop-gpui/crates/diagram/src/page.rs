//! The hidden page for KaTeX: its HTML, the init script with `renderMath`, and the files it loads
//! through the `bachuc-diagram://` protocol.

use std::borrow::Cow;
use std::path::{Component, Path, PathBuf};

use wry::http::{Request, Response, header::CONTENT_TYPE};

pub(crate) const PROTOCOL: &str = "bachuc-diagram";
pub(crate) const URL: &str = "bachuc-diagram://localhost/index.html";

const INDEX: &str = r#"<!doctype html>
<html><head><meta charset="utf-8">
<link rel="stylesheet" href="katex/katex.min.css">
<style>
html, body { margin: 0; background: transparent; }
body { font-family: -apple-system, sans-serif; }
#d { display: inline-block; padding: 6px; }
.katex-display { margin: 0; }
</style>
<script src="katex/katex.min.js"></script>
</head><body><div id="d"></div>
<script>window.ipc.postMessage(JSON.stringify({ type: "ready" }));</script>
</body></html>"#;

/// Each call renders one source into `#d`, then posts the bounds of the result, or the error.
pub(crate) const INIT_SCRIPT: &str = r##"(() => {
  const post = (message) => window.ipc.postMessage(JSON.stringify(message));
  // No requestAnimationFrame: WebKit stops it while the window is occluded, which would stall
  // the queue. getBoundingClientRect forces the layout it needs.
  let shown = null;
  window.reportBounds = (id) => {
    const r = shown.getBoundingClientRect();
    post({ type: "rendered", id, x: r.x, y: r.y, w: r.width, h: r.height });
  };
  const show = async (id, dark, fill) => {
    const d = document.getElementById("d");
    d.innerHTML = "";
    d.style.color = dark ? "#e6e6e6" : "#1f1f1f";
    try {
      shown = await fill(d);
      // Layout starts the KaTeX font loads, so lay out before waiting for the fonts.
      shown.getBoundingClientRect();
      await document.fonts.ready;
      reportBounds(id);
    } catch (e) {
      d.innerHTML = "";
      post({ type: "failed", id, error: String((e && e.message) || e) });
    }
  };
  window.renderMath = (id, src, dark) => show(id, dark, async (d) => {
    katex.render(src, d, { displayMode: true, throwOnError: true });
    return d;
  });
})();"##;

fn repo_file(relative: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../../..")
        .join(relative)
}

/// `BACHUC_KATEX_DIR` overrides the KaTeX `dist` folder.
fn katex_dir() -> PathBuf {
    std::env::var_os("BACHUC_KATEX_DIR")
        .map(PathBuf::from)
        .unwrap_or_else(|| repo_file("node_modules/katex/dist"))
}

fn file_path(path: &str) -> Option<PathBuf> {
    let rest = Path::new(path.strip_prefix("/katex/")?);
    // Serve files inside the KaTeX folder only.
    if !rest.components().all(|c| matches!(c, Component::Normal(_))) {
        return None;
    }
    Some(katex_dir().join(rest))
}

fn content_type(path: &str) -> &'static str {
    match path.rsplit('.').next() {
        Some("html") => "text/html; charset=utf-8",
        Some("js") => "text/javascript; charset=utf-8",
        Some("css") => "text/css; charset=utf-8",
        Some("woff2") => "font/woff2",
        Some("woff") => "font/woff",
        Some("ttf") => "font/ttf",
        _ => "application/octet-stream",
    }
}

pub(crate) fn respond(request: Request<Vec<u8>>) -> Response<Cow<'static, [u8]>> {
    let path = request.uri().path();
    let body = if path == "/index.html" {
        Some(INDEX.as_bytes().to_vec())
    } else {
        file_path(path).and_then(|file| std::fs::read(file).ok())
    };
    let builder = Response::builder().header(CONTENT_TYPE, content_type(path));
    match body {
        Some(body) => builder.status(200).body(Cow::Owned(body)),
        None => builder.status(404).body(Cow::Borrowed(&[][..])),
    }
    .expect("static response")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn serves_only_the_page_and_its_files() {
        assert_eq!(
            file_path("/katex/fonts/KaTeX_Main-Regular.woff2"),
            Some(katex_dir().join("fonts/KaTeX_Main-Regular.woff2"))
        );
        assert_eq!(file_path("/katex/../../secret"), None);
        assert_eq!(file_path("/other.js"), None);
    }
}
