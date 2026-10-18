//! `packages/app/src/composer/tracks.tsx`: the strip docked at the top of the composer card, its
//! pills, the panel a pill opens above itself, and the rows of that panel.

use crate::pill_styles::{STRIP_HEIGHT, SURFACE_PADDING_X, SURFACE_PADDING_Y, docked_pill};
use bachuc_sidebar::model::StatusBucket;
use bachuc_theme::metrics::{
    FONT_BASE, RADIUS_MD, SPACE_0_5, SPACE_1, SPACE_1_5, SPACE_2, SPACE_4,
};
use bachuc_theme::{tokens, ui_font_size, zoomed};
use gpui_kit::component::popover::{Popover, PopoverState};
use gpui_kit::component::{Selectable, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

/// `PANEL_MIN_WIDTH`, `PANEL_MAX_WIDTH`, `PANEL_MAX_HEIGHT`, and `PANEL_OFFSET`.
const PANEL_MIN_WIDTH: f32 = 280.;
const PANEL_MAX_WIDTH: f32 = 620.;
const PANEL_MAX_HEIGHT: f32 = 440.;
const PANEL_OFFSET: f32 = 12.;
/// `STATUS_INDICATOR_FILLED_DOT_SIZE`, and the ring around the running dot
/// (`STATUS_RING_GAP` and `STATUS_RING_STROKE`) without its halo.
const DOT: f32 = 6.;
const RING_STROKE: f32 = 1.5;
const RING: f32 = DOT + (1.5 + RING_STROKE) * 2.;
/// `STATUS_RING_TRACK_OPACITY`.
const RING_TRACK_OPACITY: f32 = 0.3;
/// The row rail (`styles.row`).
const ROW_MIN_HEIGHT: f32 = 32.;

/// One part of a track pill: a state mark and its text (`ComposerTrackPillSegment`).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct TrackSegment {
    /// `None` draws the text on its own.
    pub bucket: Option<StatusBucket>,
    pub text: String,
}

/// `ComposerTrackStrip`: the pills in one row over a 1 px divider. The strip bleeds through the
/// card padding, so the divider spans the whole card; `trailing` pins to the right edge.
#[derive(IntoElement)]
pub struct TrackStrip {
    leading: Vec<AnyElement>,
    trailing: Vec<AnyElement>,
}

impl TrackStrip {
    pub fn new(leading: Vec<AnyElement>, trailing: Vec<AnyElement>) -> Self {
        Self { leading, trailing }
    }
}

impl RenderOnce for TrackStrip {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let t = tokens(cx);
        let has_trailing = !self.trailing.is_empty();
        h_flex()
            .id("composer-track-strip")
            .role(Role::Group)
            .aria_label("Agent tracks")
            .items_center()
            .gap(zoomed(SPACE_0_5, cx))
            .min_h(zoomed(STRIP_HEIGHT, cx))
            .mt(zoomed(-SURFACE_PADDING_Y, cx))
            .mx(zoomed(-SURFACE_PADDING_X, cx))
            .px(zoomed(SURFACE_PADDING_X - SPACE_1_5, cx))
            .border_b_1()
            .border_color(t.border)
            .overflow_hidden()
            .children(self.leading)
            .when(has_trailing, |strip| strip.child(div().flex_grow(1.)))
            .children(self.trailing)
    }
}

/// The state mark of a segment: the ring for running, else the status dot.
fn track_mark(bucket: StatusBucket, cx: &App) -> AnyElement {
    let t = tokens(cx);
    let dot = |color: Hsla| {
        div()
            .flex_none()
            .size(zoomed(DOT, cx))
            .rounded_full()
            .bg(color)
    };
    match bucket {
        StatusBucket::Running => div()
            .flex_none()
            .size(zoomed(RING, cx))
            .flex()
            .items_center()
            .justify_center()
            .rounded_full()
            .border(zoomed(RING_STROKE, cx))
            .border_color(t.dot_running.opacity(RING_TRACK_OPACITY))
            .child(dot(t.dot_running))
            .into_any_element(),
        StatusBucket::NeedsInput => dot(t.dot_warning).into_any_element(),
        StatusBucket::Failed => dot(t.dot_danger).into_any_element(),
        StatusBucket::Attention => dot(t.dot_success).into_any_element(),
        StatusBucket::Done => dot(t.border).into_any_element(),
    }
}

/// The trigger of a track pill: its segments, twice as far apart as a mark from its text.
#[derive(IntoElement)]
struct TrackPillTrigger {
    id: ElementId,
    segments: Vec<TrackSegment>,
    label: String,
    open: bool,
}

impl Selectable for TrackPillTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl RenderOnce for TrackPillTrigger {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        docked_pill(self.id, self.open, cx)
            .role(Role::Button)
            .aria_label(self.label)
            .aria_expanded(self.open)
            .cursor_pointer()
            .child(
                h_flex()
                    .min_w_0()
                    .flex_shrink(1.)
                    .items_center()
                    .gap(zoomed(SPACE_4, cx))
                    .children(self.segments.into_iter().map(|segment| {
                        h_flex()
                            .min_w_0()
                            .flex_shrink(1.)
                            .items_center()
                            .gap(zoomed(SPACE_2, cx))
                            .children(segment.bucket.map(|bucket| track_mark(bucket, cx)))
                            .child(div().truncate().child(segment.text))
                    })),
            )
    }
}

type PanelContent =
    Rc<dyn Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> AnyElement>;

/// `ComposerTrackPill`: a docked pill that opens its panel above itself.
pub fn track_pill(
    id: impl Into<ElementId>,
    segments: Vec<TrackSegment>,
    label: String,
    panel_title: &'static str,
    content: impl Fn(&mut PopoverState, &mut Window, &mut Context<PopoverState>) -> AnyElement + 'static,
) -> AnyElement {
    TrackPill {
        id: id.into(),
        segments,
        label,
        panel_title,
        content: Rc::new(content),
    }
    .into_any_element()
}

/// A `RenderOnce` wrapper, so the panel offset and sizes read the zoom at render time.
#[derive(IntoElement)]
struct TrackPill {
    id: ElementId,
    segments: Vec<TrackSegment>,
    label: String,
    panel_title: &'static str,
    content: PanelContent,
}

impl RenderOnce for TrackPill {
    fn render(self, _: &mut Window, cx: &mut App) -> impl IntoElement {
        let Self {
            id,
            segments,
            label,
            panel_title,
            content,
        } = self;
        let (min_w, max_w, max_h) = (
            zoomed(PANEL_MIN_WIDTH, cx),
            zoomed(PANEL_MAX_WIDTH, cx),
            zoomed(PANEL_MAX_HEIGHT, cx),
        );
        Popover::new(id.clone())
            .anchor(Anchor::BottomLeft)
            .offset(zoomed(PANEL_OFFSET, cx))
            .p_1()
            .trigger(TrackPillTrigger {
                id,
                segments,
                label,
                open: false,
            })
            .content(move |state, window, cx| {
                v_flex()
                    .id("composer-track-panel")
                    .role(Role::Menu)
                    .aria_label(panel_title)
                    .min_w(min_w)
                    .max_w(max_w)
                    .max_h(max_h)
                    .overflow_y_scroll()
                    .child(content(state, window, cx))
            })
    }
}

/// `ComposerTrackRow`: one line of a track panel. A pressable row fills with `surface2` on
/// hover; a read-only row does not.
pub fn track_row(
    id: impl Into<ElementId>,
    label: String,
    pressable: bool,
    cx: &App,
) -> Stateful<Div> {
    let t = tokens(cx);
    h_flex()
        .id(id)
        .role(if pressable {
            Role::MenuItem
        } else {
            Role::ListItem
        })
        .aria_label(label)
        .items_center()
        .gap(zoomed(SPACE_2, cx))
        .min_h(zoomed(ROW_MIN_HEIGHT, cx))
        .mx(zoomed(SPACE_1, cx))
        .px(zoomed(SPACE_2, cx))
        .py(zoomed(SPACE_1, cx))
        .rounded(zoomed(RADIUS_MD, cx))
        .text_size(ui_font_size(FONT_BASE, cx))
        .when(pressable, |row| {
            row.cursor_pointer().hover(move |row| row.bg(t.surface2))
        })
}

/// `MenuSeparator` under the panel actions.
pub fn track_separator(cx: &App) -> Div {
    div()
        .h(px(1.))
        .my(zoomed(SPACE_1, cx))
        .bg(tokens(cx).border)
}
