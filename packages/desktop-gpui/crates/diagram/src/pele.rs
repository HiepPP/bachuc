//! The Mermaid engine: Pele on QuickJS, then resvg, on one background thread (ADR-0067). Pele can
//! take longer than a frame on a large diagram, so nothing here runs on the GPUI main thread.

use std::sync::mpsc::{Sender, channel};

use futures::channel::oneshot;
use rquickjs::{CatchResultExt as _, Context, Runtime};

use crate::svg::{self, PeleTheme};

const PELE: &str = include_str!("../assets/pele/pele.global.js");

/// Copies Pele's `Y1` source limit, the character count that `pele.render` accepts.
#[cfg(test)]
const PELE_SOURCE_LIMIT: usize = 50_000;

/// A rendered diagram: the PNG at `theme.scale`, and the SVG with no `var()` left, so the diagram
/// can rasterize again at another size.
pub(crate) struct Rendered {
    pub png: Vec<u8>,
    pub svg: String,
}

type Reply = oneshot::Sender<Result<Rendered, String>>;

struct Job {
    source: String,
    theme: PeleTheme,
    reply: Reply,
}

/// A handle to the engine thread. The thread ends when the last handle drops.
#[derive(Clone)]
pub(crate) struct PeleEngine {
    jobs: Sender<Job>,
}

impl PeleEngine {
    pub(crate) fn spawn() -> PeleEngine {
        let (jobs, receiver) = channel::<Job>();
        std::thread::Builder::new()
            .name("bachuc-pele".into())
            .spawn(move || {
                // The runtime is not `Send`, so it is created here and never leaves this thread.
                let context = load_pele();
                svg::load_fonts();
                while let Ok(job) = receiver.recv() {
                    let result = match &context {
                        Ok(context) => render(context, &job.source, &job.theme),
                        Err(error) => Err(error.clone()),
                    };
                    // The caller may have dropped the receiver; the result is then not needed.
                    let _ = job.reply.send(result);
                }
            })
            .expect("pele thread");
        PeleEngine { jobs }
    }

    /// Renders Mermaid `source` to a PNG at `theme.scale`. The reply holds Pele's error text on failure.
    pub(crate) fn render_png(
        &self,
        source: String,
        theme: PeleTheme,
    ) -> oneshot::Receiver<Result<Rendered, String>> {
        let (reply, receiver) = oneshot::channel();
        if let Err(failed) = self.jobs.send(Job {
            source,
            theme,
            reply,
        }) {
            let _ = failed.0.reply.send(Err("the Pele engine stopped".into()));
        }
        receiver
    }
}

fn load_pele() -> Result<Context, String> {
    let runtime = Runtime::new().map_err(|error| error.to_string())?;
    let context = Context::full(&runtime).map_err(|error| error.to_string())?;
    context
        .with(|ctx| {
            ctx.eval::<(), _>(PELE)
                .catch(&ctx)
                .map_err(|e| e.to_string())
        })
        .map_err(|error| format!("Pele failed to load: {error}"))?;
    // The context keeps the runtime alive.
    Ok(context)
}

fn render(context: &Context, source: &str, theme: &PeleTheme) -> Result<Rendered, String> {
    let svg = context.with(|ctx| {
        ctx.globals()
            .set("__src", source)
            .catch(&ctx)
            .map_err(|e| e.to_string())?;
        // The script catches Pele's error and returns its `message`, as the old mermaid.js page
        // did. A caught QuickJS error prints as "Error: <message>", and the timeline shows only
        // the first line. `catch` still reads the text of any other JS error.
        ctx.eval::<(), _>(
            "try { globalThis.__svg = pele.render(__src).svg; globalThis.__err = null; } \
             catch (e) { globalThis.__svg = null; \
             globalThis.__err = String((e && e.message) || e); }",
        )
        .catch(&ctx)
        .map_err(|e| e.to_string())?;
        let read = |name: &str| {
            ctx.globals()
                .get::<_, Option<String>>(name)
                .catch(&ctx)
                .map_err(|e| e.to_string())
        };
        match (read("__svg")?, read("__err")?) {
            (Some(svg), _) => Ok(svg),
            (None, error) => Err(error.unwrap_or_else(|| "Pele returned no SVG".into())),
        }
    })?;
    let svg = svg::resolve_vars(&svg, theme);
    let png = svg::rasterize(&svg, theme.scale)?;
    Ok(Rendered { png, svg })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::svg::pele_theme;
    use futures::executor::block_on;

    fn theme() -> crate::svg::PeleTheme {
        pele_theme(&bachuc_theme::Tokens::light(), 16.0)
    }

    #[test]
    fn valid_flowchart_gives_png_bytes() {
        let engine = PeleEngine::spawn();
        let rendered = block_on(engine.render_png("flowchart LR\n  A --> B".into(), theme()))
            .expect("reply")
            .expect("png");
        assert_eq!(&rendered.png[1..4], b"PNG");
        assert!(!rendered.svg.contains("var("));
    }

    #[test]
    fn bad_source_gives_pele_error_text_and_the_engine_keeps_working() {
        let engine = PeleEngine::spawn();
        let error = block_on(engine.render_png("flowchart LR\n  A --> (".into(), theme()))
            .expect("reply")
            .err()
            .expect("parse error");
        assert!(!error.is_empty());
        assert!(!error.contains("Exception generated by QuickJS"), "{error}");
        // The timeline shows the first line, so it must match the mermaid.js text it replaced.
        assert_eq!(
            error.lines().next(),
            Some("Parse error on line 2:"),
            "{error}"
        );
        assert!(
            error.contains("Expecting 'NODE_STRING', got 'PS'"),
            "{error}"
        );
        // Read the source limit constant in pele.global.js and go one character over it.
        let huge = format!(
            "flowchart LR\n{}",
            "  A --> B\n".repeat(PELE_SOURCE_LIMIT / 10 + 1)
        );
        assert!(
            block_on(engine.render_png(huge, theme()))
                .expect("reply")
                .is_err()
        );
        assert!(
            block_on(engine.render_png("pie\n  \"a\" : 1".into(), theme()))
                .expect("reply")
                .is_ok()
        );
    }
}
