//! The plugin parts of the timeline view: plugin item views, the assistant selection toolbar,
//! and the passage reveal. The rules live in `plugins.rs`.

use std::time::Duration;

use bachuc_plugin_host::{AssistantSelection, Handler, TimelineRenderInput};
use bachuc_theme::metrics::{FONT_SM, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_2};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size, zoomed};
use gpui_kit::component::notification::Notification;
use gpui_kit::component::text::{RangeHighlight, TextViewState};
use gpui_kit::component::{WindowExt as _, h_flex, v_flex};
use gpui_kit::*;

use crate::grouping::{item_type, message_key};
use crate::plugins::{self, Part, Projection, RevealPlan};
use crate::{RowKind, TimelineView};

/// How long a revealed passage stays highlighted (`HIGHLIGHT_MS`).
const REVEAL_HIGHLIGHT: Duration = Duration::from_millis(2500);
/// How long a reveal waits for the history to load (`PENDING_TTL_MS`).
const REVEAL_WAIT: Duration = Duration::from_secs(10);
const NOT_LOADED: &str = "That message isn't loaded in this chat";
/// The toolbar size used before its first layout.
const TOOLBAR_ESTIMATE: (f32, f32) = (120., 30.);

/// Selected assistant text that the selection actions act on.
pub(crate) struct Selection {
    pub state: Entity<TextViewState>,
    pub payload: AssistantSelection,
}

/// A reveal that waits for the history to load.
pub(crate) struct PendingReveal {
    pub message_id: String,
    pub text: Option<String>,
    pub window: AnyWindowHandle,
    pub _timer: Task<()>,
}

impl TimelineView {
    /// Scrolls to the message whose id is `message_id` (its `messageId`, else
    /// `seq:<seqStart>`). Text found exactly once in the message's source is highlighted for
    /// 2.5 s. Before the history loads, the request waits up to 10 s; a message that is not in
    /// the loaded chat shows a notification.
    pub fn reveal_passage(
        &mut self,
        message_id: &str,
        text: Option<&str>,
        window: &mut Window,
        cx: &mut Context<TimelineView>,
    ) {
        self.pending_reveal = None;
        let found = self.reveal_now(message_id, text, cx);
        match plugins::reveal_plan(self.loaded, found) {
            RevealPlan::Reveal => {}
            RevealPlan::NotLoaded => window.push_notification(Notification::info(NOT_LOADED), cx),
            RevealPlan::Wait => {
                let timer = cx.spawn(async move |this, cx| {
                    cx.background_executor().timer(REVEAL_WAIT).await;
                    let _ = this.update(cx, |this, cx| {
                        if let Some(pending) = this.pending_reveal.take() {
                            let _ = pending.window.update(cx, |_, window, cx| {
                                window.push_notification(Notification::info(NOT_LOADED), cx)
                            });
                        }
                    });
                });
                self.pending_reveal = Some(PendingReveal {
                    message_id: message_id.to_string(),
                    text: text.map(str::to_string),
                    window: window.window_handle(),
                    _timer: timer,
                });
            }
        }
    }

    /// Runs a reveal that waited, once the history loaded.
    pub(crate) fn run_pending_reveal(&mut self, cx: &mut Context<Self>) {
        if !self.loaded {
            return;
        }
        let Some(PendingReveal {
            message_id,
            text,
            window,
            ..
        }) = self.pending_reveal.take()
        else {
            return;
        };
        // The rows of this sync are not in place yet, and a sync can run inside a window update.
        let view = cx.weak_entity();
        cx.defer(move |cx| {
            let _ = window.update(cx, |_, window, cx| {
                let _ = view.update(cx, |this, cx| {
                    this.reveal_passage(&message_id, text.as_deref(), window, cx)
                });
            });
        });
    }

    fn is_message(&self, index: usize, message_id: &str) -> bool {
        let entry = &self.entries[index];
        matches!(item_type(entry), "user_message" | "assistant_message")
            && message_key(entry) == message_id
    }

    /// Scrolls to the message and highlights a unique match. False when no row shows it.
    fn reveal_now(&mut self, message_id: &str, text: Option<&str>, cx: &mut Context<Self>) -> bool {
        let Some(row) = self.rows.iter().position(|row| match &row.kind {
            RowKind::Entry(index) => self.is_message(*index, message_id),
            RowKind::Step(group, position) => group.steps[*position]
                .note
                .iter()
                .any(|&note| self.is_message(note, message_id)),
            _ => false,
        }) else {
            return false;
        };
        // `scroll_to` stops tail follow and puts the message at the top. `scroll_to_reveal_item`
        // keeps tail follow on, so the next layout of a new view moved back to the end.
        self.list.scroll_to(ListOffset {
            item_ix: row,
            offset_in_item: px(0.),
        });
        if let Some(text) = text {
            let states: Vec<Entity<TextViewState>> = (0..self.entries.len())
                .filter(|&index| {
                    self.is_message(index, message_id) && self.grouper.is_assistant(index)
                })
                .filter_map(|index| self.messages.get(&self.grouper.key(index)))
                .flat_map(|message| message.states.iter().flatten().cloned())
                .collect();
            let rendered: Vec<_> = states
                .iter()
                .map(|state| state.read(cx).rendered_text())
                .collect();
            let sources: Vec<&str> = rendered.iter().map(|text| text.source()).collect();
            if let Some((segment, range)) = plugins::unique_match(&sources, text)
                && let Some(range) = rendered[segment].range_for_source(range)
            {
                let state = states[segment].clone();
                let color = tokens(cx).status_warning;
                state.update(cx, |state, cx| {
                    let _ =
                        state.set_range_highlights([RangeHighlight::new(range.clone(), color)], cx);
                    let _ = state.reveal_range(range, cx);
                });
                self.reveal_highlight = Some(cx.spawn(async move |_, cx| {
                    cx.background_executor().timer(REVEAL_HIGHLIGHT).await;
                    let _ = state.update(cx, |state, cx| state.clear_range_highlights(cx));
                }));
            }
        }
        cx.notify();
        true
    }

    /// Builds the views of the plugin items a row draws that have none yet, or whose data
    /// changed.
    pub(crate) fn ensure_plugin_views(
        &mut self,
        row: usize,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self.host.is_none() {
            return;
        }
        let indices: Vec<usize> = match self.rows.get(row).map(|row| &row.kind) {
            Some(RowKind::Entry(index)) => vec![*index],
            Some(RowKind::Step(group, position)) => {
                let step = &group.steps[*position];
                step.note.iter().chain(&step.items).copied().collect()
            }
            _ => return,
        };
        for index in indices {
            let entry = &self.entries[index];
            let timestamp = || entry["timestamp"].as_str().map(str::to_string);
            if let Some(projection) = self.projector.get(index).cloned() {
                let timestamp = timestamp();
                for item in &projection.items {
                    self.ensure_view(
                        &item.id,
                        projection.plugin_id,
                        &item.item,
                        item.data_hash,
                        timestamp.clone(),
                        window,
                        cx,
                    );
                }
            } else if item_type(entry) == "plugin"
                && let Some((plugin_id, id, item)) = plugins::daemon_item(&entry["item"])
            {
                let timestamp = timestamp();
                let hash = entry["seqEnd"].as_u64().unwrap_or(0);
                self.ensure_view(&id, &plugin_id, &item, hash, timestamp, window, cx);
            }
        }
    }

    #[allow(clippy::too_many_arguments)]
    fn ensure_view(
        &mut self,
        id: &str,
        plugin_id: &str,
        item: &bachuc_plugin_host::PluginTimelineItem,
        hash: u64,
        timestamp: Option<String>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        if self
            .plugin_views
            .get(id)
            .is_some_and(|(built, _)| *built == hash)
        {
            return;
        }
        let Some(host) = self.host.clone() else {
            return;
        };
        let factory =
            plugins::renderer_for_item(host.read(cx).index(), plugin_id, &item.kind, item.version)
                .map(|renderer| renderer.value.view.clone());
        let Some(factory) = factory else {
            self.plugin_views.remove(id);
            return;
        };
        let input = TimelineRenderInput {
            agent_id: self.agent_id.clone(),
            item: item.clone(),
            timestamp,
        };
        let view = factory(input, window, cx);
        self.plugin_views.insert(id.to_string(), (hash, view));
    }

    /// A plugin item's view. None when no renderer of its plugin matches: the item draws
    /// nothing.
    pub(crate) fn render_plugin_item(&self, id: &str) -> Option<AnyElement> {
        let (_, view) = self.plugin_views.get(id)?;
        Some(div().w_full().child(view.clone()).into_any_element())
    }

    /// The parts of a projected entry that draw, as `plugins::shown_parts`.
    pub(crate) fn shown_parts<'a>(&self, projection: &'a Projection, cx: &App) -> Vec<Part<'a>> {
        let index = self.host.as_ref().map(|host| host.read(cx).index());
        plugins::shown_parts(projection, |item| {
            index.is_some_and(|index| {
                plugins::renderer_for_item(
                    index,
                    projection.plugin_id,
                    &item.item.kind,
                    item.item.version,
                )
                .is_some()
            })
        })
    }

    /// An entry a transformer projected: its kept source and its plugin items, in order.
    pub(crate) fn render_projected(
        &self,
        index: usize,
        projection: &Projection,
        cx: &mut Context<Self>,
    ) -> AnyElement {
        let mut parts = Vec::new();
        for part in self.shown_parts(projection, cx) {
            match part {
                Part::Source => parts.push(self.render_source_entry(index, cx)),
                Part::Item(item) => parts.extend(self.render_plugin_item(&item.id)),
            }
        }
        v_flex().w_full().children(parts).into_any_element()
    }

    /// A daemon plugin item (`type: "plugin"`). A malformed one draws nothing.
    pub(crate) fn render_daemon_item(&self, index: usize) -> AnyElement {
        plugins::daemon_item(&self.entries[index]["item"])
            .and_then(|(_, id, _)| self.render_plugin_item(&id))
            .unwrap_or_else(|| div().into_any_element())
    }

    /// A text state of an assistant message changed: its selection may have.
    pub(crate) fn text_state_changed(
        &mut self,
        message_id: &str,
        state: Entity<TextViewState>,
        cx: &mut Context<Self>,
    ) {
        let has_actions = self
            .host
            .as_ref()
            .is_some_and(|host| !host.read(cx).index().selection_actions().is_empty());
        let payload = has_actions
            .then(|| {
                let text = state.read(cx);
                if text.is_selecting() {
                    return None;
                }
                let rendered = text.rendered_text();
                plugins::assistant_selection(
                    &self.agent_id,
                    self.workspace_id.clone(),
                    message_id.to_string(),
                    rendered.source(),
                    text.selected_source_range(),
                    text.selected_text(),
                )
            })
            .flatten();
        let current = self.selection.as_ref().filter(|shown| shown.state == state);
        match payload {
            Some(payload) => {
                if current.is_some_and(|shown| shown.payload == payload) {
                    return;
                }
                self.selection = Some(Selection { state, payload });
                cx.notify();
            }
            None if current.is_some() => {
                self.selection = None;
                cx.notify();
            }
            None => {}
        }
    }

    /// Clears the selection, then runs the action on it.
    fn run_selection_action(
        &mut self,
        handler: Handler<AssistantSelection>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let Some(selection) = self.selection.take() else {
            return;
        };
        selection
            .state
            .update(cx, |state, cx| state.clear_selection(cx));
        cx.notify();
        handler(selection.payload, window, cx);
    }

    /// The selection actions over the selected assistant text, as `AssistantSelectionActions`.
    pub(crate) fn render_selection_toolbar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let selection = self.selection.as_ref()?;
        let actions = self
            .host
            .as_ref()?
            .read(cx)
            .index()
            .selection_actions()
            .to_vec();
        if actions.is_empty() {
            return None;
        }
        let measured = self.toolbar_size.get();
        let size = if measured.width > px(0.) {
            measured
        } else {
            size(
                zoomed(TOOLBAR_ESTIMATE.0, cx),
                zoomed(TOOLBAR_ESTIMATE.1, cx),
            )
        };
        let state = selection.state.read(cx);
        let text = plugins::selection_anchor(state.bounds(), state.selection_edges());
        let origin = plugins::toolbar_origin(
            text,
            size,
            self.bounds.get(),
            zoomed(plugins::TOOLBAR_GAP, cx),
        )?;
        let t = tokens(cx);
        let font = ui_font_size(FONT_SM, cx);
        let measure = self.toolbar_size.clone();
        let buttons: Vec<AnyElement> = actions
            .into_iter()
            .map(|action| {
                let handler = action.value.on_select.clone();
                let title = action.value.title.clone();
                div()
                    .id(SharedString::from(format!(
                        "selection-action-{}-{}",
                        action.plugin_id, action.value.id
                    )))
                    .role(Role::Button)
                    .aria_label(title.clone())
                    .px(zoomed(SPACE_2, cx))
                    .py(zoomed(SPACE_1, cx))
                    .rounded(zoomed(RADIUS_MD, cx))
                    .cursor_pointer()
                    .text_size(font)
                    .text_color(t.foreground)
                    .child(title)
                    // A press anywhere clears text selections before a click lands, so the
                    // action runs on the press, with the selection read before it.
                    .on_mouse_down(
                        MouseButton::Left,
                        cx.listener(move |this, _, window, cx| {
                            cx.stop_propagation();
                            this.run_selection_action(handler.clone(), window, cx);
                        }),
                    )
                    .into_any_element()
            })
            .collect();
        Some(
            div()
                .absolute()
                .left(origin.x)
                .top(origin.y)
                .child(
                    h_flex()
                        .id("selection-actions")
                        .role(Role::Toolbar)
                        .aria_label("Selection actions")
                        .relative()
                        .p(zoomed(SPACE_1, cx))
                        .gap(zoomed(SPACE_1, cx))
                        .bg(t.popover)
                        .border_1()
                        .border_color(t.border_accent)
                        .rounded(zoomed(RADIUS_LG, cx))
                        .shadow(vec![shadow(ShadowSize::Md, cx)])
                        .child(
                            canvas(
                                move |bounds, window, _| {
                                    if measure.get() != bounds.size {
                                        measure.set(bounds.size);
                                        window.refresh();
                                    }
                                },
                                |_, _, _, _| {},
                            )
                            .absolute()
                            .size_full(),
                        )
                        .children(buttons),
                )
                .into_any_element(),
        )
    }
}
