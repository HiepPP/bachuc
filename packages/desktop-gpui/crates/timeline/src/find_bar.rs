//! The find bar of the agent pane, as `PaneFind` in `packages/app/src/pane-find/index.tsx` with
//! the chat find of `agent-stream/chat-find`: it searches the user and assistant messages of the
//! loaded rows, highlights each match, and steps through them with Enter and Shift+Enter.

use std::ops::Range;

use bachuc_icons::IconName;
use bachuc_theme::metrics::{FONT_SM, RADIUS_LG, RADIUS_MD, SPACE_1, SPACE_1_5, SPACE_2, SPACE_3};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size};
use gpui_kit::component::h_flex;
use gpui_kit::component::input::{Input, InputEvent, InputState};
use gpui_kit::component::text::RangeHighlight;
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

use crate::find::{match_ranges, status, step};
use crate::grouping::{entry_key, item_type};
use crate::tool_view::icon_button;
use crate::{RowKind, TimelineView};

/// `FIND_WIDGET_WIDTH` and `COLLAPSED_FIELD_HEIGHT`.
const WIDGET_WIDTH: f32 = 340.;
const FIELD_HEIGHT: f32 = 28.;
/// The other matches show at this share of the current match's color.
const OTHER_MATCH_ALPHA: f32 = 0.35;

/// Where one match is: an entry, the markdown segment of an assistant entry (None for a user
/// message), and the byte range in its rendered text.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FindMatch {
    pub key: u64,
    pub segment: Option<usize>,
    pub range: Range<usize>,
}

pub(crate) struct FindState {
    pub input: Entity<InputState>,
    pub query: String,
    pub matches: Vec<FindMatch>,
    pub current: usize,
    /// The timeline version the matches were found in; a step after a change searches again.
    pub version: Option<u64>,
    _subscriptions: Vec<Subscription>,
}

impl TimelineView {
    /// Opens the find bar, or focuses it and selects its text when it is open.
    pub fn find(&mut self, window: &mut Window, cx: &mut Context<TimelineView>) {
        if self.finder.is_none() {
            let input = cx.new(|cx| InputState::new(window, cx).placeholder("Find in pane"));
            let weak = cx.weak_entity();
            let subscriptions =
                vec![
                    cx.subscribe_in(&input, window, |this, input, event: &InputEvent, _, cx| {
                        match event {
                            InputEvent::Change => {
                                let query = input.read(cx).value().to_string();
                                this.set_query(query, cx);
                            }
                            InputEvent::PressEnter { shift, .. } => this.step_find(!*shift, cx),
                            _ => {}
                        }
                    }),
                    // The input keeps Escape for itself, so the pane reads it first, while the
                    // input has focus.
                    cx.intercept_keystrokes(move |event, window, cx| {
                        if event.keystroke.key == "escape" {
                            let closed = weak
                                .update(cx, |this, cx| {
                                    let focused = this.finder.as_ref().is_some_and(|finder| {
                                        finder.input.focus_handle(cx).is_focused(window)
                                    });
                                    if focused {
                                        this.close_find(cx);
                                    }
                                    focused
                                })
                                .unwrap_or(false);
                            if closed {
                                cx.stop_propagation();
                            }
                        }
                    }),
                ];
            self.finder = Some(FindState {
                input,
                query: String::new(),
                matches: Vec::new(),
                current: 0,
                version: None,
                _subscriptions: subscriptions,
            });
        }
        if let Some(finder) = &self.finder {
            finder.input.update(cx, |input, cx| {
                input.focus(window, cx);
                input.select_all(window, cx);
            });
        }
        cx.notify();
    }

    pub(crate) fn close_find(&mut self, cx: &mut Context<Self>) {
        if self.finder.take().is_some() {
            self.clear_highlights(cx);
            cx.notify();
        }
    }

    fn set_query(&mut self, query: String, cx: &mut Context<Self>) {
        let Some(finder) = &mut self.finder else {
            return;
        };
        if finder.query == query {
            return;
        }
        finder.query = query;
        finder.current = 0;
        self.search(cx);
        self.reveal_current(cx);
    }

    fn step_find(&mut self, forward: bool, cx: &mut Context<Self>) {
        if self
            .finder
            .as_ref()
            .is_some_and(|finder| finder.version != self.version)
        {
            self.search(cx);
        }
        let Some(finder) = &mut self.finder else {
            return;
        };
        finder.current = step(finder.current, finder.matches.len(), forward);
        self.paint_highlights(cx);
        self.reveal_current(cx);
    }

    /// Finds the query in every loaded user and assistant message, in timeline order.
    fn search(&mut self, cx: &mut Context<Self>) {
        let Some(finder) = &self.finder else {
            return;
        };
        let query = finder.query.trim().to_string();
        let mut matches = Vec::new();
        if !query.is_empty() {
            for entry in &self.entries {
                let key = entry_key(entry);
                match item_type(entry) {
                    "user_message" => {
                        let text = entry["item"]["text"].as_str().unwrap_or("");
                        matches.extend(match_ranges(text, &query).into_iter().map(|range| {
                            FindMatch {
                                key,
                                segment: None,
                                range,
                            }
                        }));
                    }
                    "assistant_message" => {
                        let Some(message) = self.messages.get(&key) else {
                            continue;
                        };
                        for (position, state) in message.states.iter().enumerate() {
                            let Some(state) = state else { continue };
                            let rendered = state.read(cx).rendered_text();
                            matches.extend(
                                match_ranges(rendered.as_str(), &query)
                                    .into_iter()
                                    .map(|range| FindMatch {
                                        key,
                                        segment: Some(position),
                                        range,
                                    }),
                            );
                        }
                    }
                    _ => {}
                }
            }
        }
        let version = self.version;
        if let Some(finder) = &mut self.finder {
            finder.current = finder.current.min(matches.len().saturating_sub(1));
            finder.matches = matches;
            finder.version = version;
        }
        self.paint_highlights(cx);
    }

    fn clear_highlights(&mut self, cx: &mut Context<Self>) {
        for message in self.messages.values() {
            for state in message.states.iter().flatten() {
                state.update(cx, |state, cx| state.clear_range_highlights(cx));
            }
        }
    }

    /// Paints every match, the current one stronger. User messages read the matches when they
    /// draw.
    fn paint_highlights(&mut self, cx: &mut Context<Self>) {
        self.clear_highlights(cx);
        let Some(finder) = &self.finder else {
            return;
        };
        let current = tokens(cx).status_warning;
        let mut grouped: Vec<((u64, usize), Vec<RangeHighlight>)> = Vec::new();
        for (index, found) in finder.matches.iter().enumerate() {
            let Some(segment) = found.segment else {
                continue;
            };
            let color = if index == finder.current {
                current
            } else {
                current.opacity(OTHER_MATCH_ALPHA)
            };
            let highlight = RangeHighlight::new(found.range.clone(), color);
            match grouped.last_mut() {
                Some((at, list)) if *at == (found.key, segment) => list.push(highlight),
                _ => grouped.push(((found.key, segment), vec![highlight])),
            }
        }
        for ((key, segment), highlights) in grouped {
            if let Some(Some(state)) = self.messages.get(&key).and_then(|m| m.states.get(segment)) {
                state.update(cx, |state, cx| {
                    let _ = state.set_range_highlights(highlights, cx);
                });
            }
        }
        cx.notify();
    }

    /// Scrolls to the row of the current match and the line inside it.
    fn reveal_current(&mut self, cx: &mut Context<Self>) {
        let Some(found) = self
            .finder
            .as_ref()
            .and_then(|finder| finder.matches.get(finder.current).cloned())
        else {
            return;
        };
        let row = self.rows.iter().position(|row| match &row.kind {
            RowKind::Entry(index) => self.grouper.key(*index) == found.key,
            RowKind::Step(group, position) => group.steps[*position]
                .note
                .iter()
                .any(|&note| self.grouper.key(note) == found.key),
            _ => false,
        });
        if let Some(row) = row {
            self.list.scroll_to_reveal_item(row);
        }
        if let Some(segment) = found.segment
            && let Some(Some(state)) = self
                .messages
                .get(&found.key)
                .and_then(|m| m.states.get(segment))
        {
            state.update(cx, |state, cx| {
                let _ = state.reveal_range(found.range.clone(), cx);
            });
        }
        cx.notify();
    }

    /// The find matches in one user message, for its text highlights.
    pub(crate) fn user_matches(&self, key: u64) -> Vec<(Range<usize>, bool)> {
        let Some(finder) = &self.finder else {
            return Vec::new();
        };
        finder
            .matches
            .iter()
            .enumerate()
            .filter(|(_, found)| found.key == key && found.segment.is_none())
            .map(|(index, found)| (found.range.clone(), index == finder.current))
            .collect()
    }

    /// The bar over the top right of the pane.
    pub(crate) fn render_find_bar(&self, cx: &mut Context<Self>) -> Option<AnyElement> {
        let finder = self.finder.as_ref()?;
        let t = tokens(cx);
        let count = finder.matches.len();
        let text = status(&finder.query, count, finder.current);
        let can_step = count > 0;
        let size = ui_font_size(FONT_SM, cx);
        let nav = |id: &'static str, label: &'static str, name: IconName| {
            icon_button(id, label, name, t.foreground_muted, &t).when(!can_step, |b| b.opacity(0.5))
        };
        Some(
            div()
                .absolute()
                .top(px(SPACE_2))
                .right(px(SPACE_3))
                .child(
                    h_flex()
                        .id("find")
                        .role(Role::Search)
                        .aria_label("Find")
                        .w(px(WIDGET_WIDTH))
                        .max_w_full()
                        .p(px(SPACE_1_5))
                        .gap(px(SPACE_1))
                        .items_center()
                        .bg(t.surface1)
                        .border_1()
                        .border_color(t.border)
                        .rounded(px(RADIUS_LG))
                        .shadow(vec![shadow(ShadowSize::Md, cx)])
                        .child(
                            h_flex()
                                .flex_1()
                                .min_w_0()
                                .h(px(FIELD_HEIGHT))
                                .gap(px(SPACE_2))
                                .items_center()
                                .px(px(SPACE_2))
                                .bg(t.surface2)
                                .rounded(px(RADIUS_MD))
                                .child(
                                    div().flex_1().min_w_0().child(
                                        Input::new(&finder.input)
                                            .appearance(false)
                                            .bordered(false)
                                            .focus_bordered(false)
                                            .px_0()
                                            .py_0()
                                            .text_size(size)
                                            .aria_label("Find in pane"),
                                    ),
                                )
                                .child(
                                    div()
                                        .id("find-status")
                                        .role(Role::Status)
                                        .aria_label(if text.is_empty() {
                                            "Find matches".to_string()
                                        } else {
                                            text.clone()
                                        })
                                        .flex_none()
                                        .text_size(size)
                                        .text_color(t.foreground_muted)
                                        .child(text),
                                ),
                        )
                        .child(
                            nav("find-previous", "Previous match", IconName::ArrowUp).when(
                                can_step,
                                |b| {
                                    b.on_click(
                                        cx.listener(|this, _, _, cx| this.step_find(false, cx)),
                                    )
                                },
                            ),
                        )
                        .child(
                            nav("find-next", "Next match", IconName::ArrowDown).when(
                                can_step,
                                |b| {
                                    b.on_click(
                                        cx.listener(|this, _, _, cx| this.step_find(true, cx)),
                                    )
                                },
                            ),
                        )
                        .child(
                            icon_button(
                                "find-close",
                                "Close Find",
                                IconName::X,
                                t.foreground_muted,
                                &t,
                            )
                            .on_click(cx.listener(|this, _, _, cx| this.close_find(cx))),
                        ),
                )
                .into_any_element(),
        )
    }
}
