//! The `prompt-note` renderer (`client/note-view.tsx`): a right-aligned muted line under a
//! user prompt, with a bold label. A view asks the shared cache once and waits for its answer.

use crate::notes::{Ask, Note, Outcome, cache_key, is_cache_only};
use crate::shared::Shared;
use bachuc_plugin_host::TimelineRenderInput;
use bachuc_theme::metrics::{FONT_SM, SPACE_1_5, SPACE_3};
use bachuc_theme::{tokens, ui_font_size, zoomed};
use gpui_kit::*;
use std::rc::Rc;
use std::time::{Duration, Instant};

/// `lineHeight: 17` of `note-view.tsx:33`. No metric has 17.
const NOTE_LINE_HEIGHT: f32 = 17.;
/// A failed lookup waits this long before a render tries it again, so a window that redraws
/// every frame does not send a request per frame.
const RETRY_AFTER: Duration = Duration::from_secs(2);

#[derive(Clone, Copy, PartialEq, Eq)]
enum Status {
    Idle,
    Waiting,
    Done,
}

pub(crate) struct NoteView {
    shared: Rc<Shared>,
    text: String,
    timestamp: Option<String>,
    cache_only: bool,
    note: Option<Note>,
    status: Status,
    failed_at: Option<Instant>,
}

/// The renderer's view factory. Data that does not decode renders nothing.
pub(crate) fn build(shared: &Rc<Shared>, input: TimelineRenderInput, cx: &mut App) -> AnyView {
    let text = input
        .item
        .data
        .get("text")
        .and_then(|text| text.as_str())
        .unwrap_or_default()
        .to_string();
    let shared = shared.clone();
    cx.new(|cx| NoteView::new(shared, text, input.timestamp, cx))
        .into()
}

impl NoteView {
    fn new(
        shared: Rc<Shared>,
        text: String,
        timestamp: Option<String>,
        cx: &mut Context<Self>,
    ) -> Self {
        let cache_only = is_cache_only(timestamp.as_deref(), shared.current.borrow().active_since);
        let mut view = Self {
            shared,
            text,
            timestamp,
            cache_only,
            note: None,
            status: Status::Idle,
            failed_at: None,
        };
        view.ask(cx);
        view
    }

    /// Asks the cache when the view has no answer. `cache_only` follows `active_since`, so a
    /// view re-asks when switching translation back on moves it.
    fn ask(&mut self, cx: &mut Context<Self>) {
        let cache_only = is_cache_only(
            self.timestamp.as_deref(),
            self.shared.current.borrow().active_since,
        );
        if cache_only != self.cache_only {
            self.cache_only = cache_only;
            self.status = Status::Idle;
            self.note = None;
            self.failed_at = None;
        }
        if self.status != Status::Idle || self.text.trim().is_empty() {
            return;
        }
        if self
            .failed_at
            .is_some_and(|failed| failed.elapsed() < RETRY_AFTER)
        {
            return;
        }
        let waiter = cx.weak_entity();
        let ask = self
            .shared
            .notes
            .borrow_mut()
            .ask(&self.text, self.cache_only, waiter);
        match ask {
            Ask::Ready(note) => {
                self.note = note;
                self.status = Status::Done;
            }
            Ask::Wait => self.status = Status::Waiting,
            Ask::Start { key, request } => {
                self.status = Status::Waiting;
                self.shared.start_lookup(key, request, cx);
            }
        }
    }

    /// A lookup this view waited for ended. An answer for another cache mode is stale.
    pub(crate) fn settled(&mut self, key: &str, outcome: &Outcome, cx: &mut Context<Self>) {
        if self.status != Status::Waiting || key != cache_key(&self.text, self.cache_only) {
            return;
        }
        match outcome {
            Outcome::Settled(note) => {
                self.note = note.clone();
                self.status = Status::Done;
                cx.notify();
            }
            // Nothing shows; a later render tries again.
            Outcome::Failed => {
                self.status = Status::Idle;
                self.failed_at = Some(Instant::now());
            }
        }
    }
}

impl Render for NoteView {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.ask(cx);
        let Some(note) = &self.note else {
            return div().into_any_element();
        };
        let label = format!("{} ", note.label.text());
        let runs = vec![(
            0..label.len(),
            HighlightStyle {
                font_weight: Some(FontWeight::SEMIBOLD),
                ..Default::default()
            },
        )];
        let text = SharedString::from(format!("{label}{}", note.text));
        div()
            .w_full()
            .flex()
            .flex_col()
            .items_end()
            .px(zoomed(SPACE_3, cx))
            .pb(zoomed(SPACE_1_5, cx))
            .child(
                div()
                    .max_w_full()
                    .text_size(ui_font_size(FONT_SM, cx))
                    .line_height(zoomed(NOTE_LINE_HEIGHT, cx))
                    .text_color(tokens(cx).foreground_muted)
                    .child(StyledText::new(text).with_highlights(runs)),
            )
            .into_any_element()
    }
}
