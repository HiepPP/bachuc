//! The Chat outline: a rail of prompt bars on the left of the thread, as Electron's
//! `agent-stream/chat-outline` (`model.ts`, `hover-intent.ts`, `rail.web.tsx`). Hovering a bar
//! shows the prompt, nearby bars grow like a dock, the prompt being read is lit, and a click or
//! Enter jumps to the prompt.
//!
//! The rules are plain functions and a plain state machine (`HoverIntent`), tested in
//! `tests/outline.rs`. `OutlineRail` is its own view, so a hover or a new reading position
//! redraws the rail and not the timeline rows.

use std::cell::Cell;
use std::f32::consts::PI;
use std::rc::Rc;
use std::time::{Duration, Instant};

use bachuc_client::PromptEntry;
use bachuc_theme::metrics::{FONT_SM, RADIUS_LG, SPACE_1, SPACE_2, SPACE_3};
use bachuc_theme::{ShadowSize, shadow, tokens, ui_font_size, zoom, zoomed};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// Slots further than this from the pointer keep their resting size.
pub const MAGNIFY_RADIUS: f32 = 3.;
/// The rail below this panel width is hidden, in px at 100% zoom.
pub const MIN_PANEL_WIDTH: f32 = 918.;
/// The rail needs at least this many prompts.
pub const MIN_PROMPTS: usize = 2;
/// The pointer rests this long on a bar before the first preview shows.
pub const INITIAL_ACTIVATION_DELAY: Duration = Duration::from_millis(150);
/// A horizontal move this long, wider than it is tall, restarts the activation timer.
pub const HORIZONTAL_TRANSIT_PX: f32 = 4.;
/// A bar eases to a new size or color over this long (`rail.web.tsx`: `transitionDuration`).
pub const BAR_EASE: Duration = Duration::from_millis(140);

const RAIL_WIDTH: f32 = 36.;
const SLOT_HEIGHT: f32 = 8.;
const RESTING_PILL_WIDTH: f32 = 10.;
const ACTIVE_PILL_WIDTH: f32 = 18.;
const MAGNIFIED_PILL_WIDTH: f32 = 26.;
const RESTING_PILL_HEIGHT: f32 = 2.;
const MAGNIFIED_PILL_HEIGHT: f32 = 4.;
const PREVIEW_WIDTH: f32 = 260.;
const PREVIEW_HEIGHT: f32 = 48.;
const PREVIEW_GAP: f32 = 4.;
/// The rail spans the middle 80% of the panel height.
const RAIL_INSET: f32 = 0.1;

/// Dock-style falloff: 1 under the pointer, easing to 0 at [`MAGNIFY_RADIUS`]. The raised
/// cosine has no corner at either end, so sweeping the rail reads as one bulge.
pub fn prompt_tick_magnification(slot_distance: f32) -> f32 {
    let distance = slot_distance.abs();
    if !distance.is_finite() || distance >= MAGNIFY_RADIUS {
        return 0.;
    }
    (1. + (PI * distance / MAGNIFY_RADIUS).cos()) / 2.
}

/// The prompt whose turn the reader is inside: the last prompt at or before `anchor_seq`, the
/// timeline position under the top of the viewport. `prompts` holds seqs in ascending order.
pub fn resolve_active_prompt_seq(prompts: &[u64], anchor_seq: Option<u64>) -> Option<u64> {
    let anchor = anchor_seq?;
    prompts
        .iter()
        .take_while(|&&seq| seq <= anchor)
        .last()
        .copied()
}

/// The size in px at 100% zoom of a bar: `(width, height)`. A lit bar rests wider, and a
/// magnification of 1 gives the 26 x 4 bar. The bar grows to the right from a shared left edge.
pub fn pill_size(is_active: bool, magnification: f32) -> (f32, f32) {
    let resting = if is_active {
        ACTIVE_PILL_WIDTH
    } else {
        RESTING_PILL_WIDTH
    };
    let magnification = magnification.clamp(0., 1.);
    (
        resting + magnification * (MAGNIFIED_PILL_WIDTH - resting),
        RESTING_PILL_HEIGHT + magnification * (MAGNIFIED_PILL_HEIGHT - RESTING_PILL_HEIGHT),
    )
}

/// CSS `ease-out`, `cubic-bezier(0, 0, 0.58, 1)`: 0 at `t = 0`, 1 at `t = 1`, never decreasing.
pub fn ease_out(t: f32) -> f32 {
    if !t.is_finite() {
        return if t > 0. { 1. } else { 0. };
    }
    if t <= 0. || t >= 1. {
        return t.clamp(0., 1.);
    }
    let x = t;
    // Solve x(s) = x for the curve parameter by bisection; x(s) rises with s.
    let (mut low, mut high) = (0_f32, 1_f32);
    for _ in 0..24 {
        let s = (low + high) / 2.;
        let at = 3. * (1. - s) * s * s * 0.58 + s * s * s;
        if at < x {
            low = s;
        } else {
            high = s;
        }
    }
    let s = (low + high) / 2.;
    3. * (1. - s) * s * s + s * s * s
}

fn lerp(from: f32, to: f32, t: f32) -> f32 {
    from + (to - from) * t
}

/// How a bar looks: size in px at 100% zoom, and color.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarLook {
    pub width: f32,
    pub height: f32,
    pub color: Rgba,
}

impl BarLook {
    /// The look `t` of the way from `self` to `to`, per channel (`t` is not clamped).
    pub fn lerp(self, to: Self, t: f32) -> Self {
        Self {
            width: lerp(self.width, to.width, t),
            height: lerp(self.height, to.height, t),
            color: Rgba {
                r: lerp(self.color.r, to.color.r, t),
                g: lerp(self.color.g, to.color.g, t),
                b: lerp(self.color.b, to.color.b, t),
                a: lerp(self.color.a, to.color.a, t),
            },
        }
    }
}

/// A bar easing toward a target look over [`BAR_EASE`] (Electron's 140 ms `ease-out` on width,
/// height, and background color). `elapsed` is the time since the last [`BarEase::retarget`].
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct BarEase {
    from: BarLook,
    to: BarLook,
    generation: u64,
}

impl BarEase {
    /// A bar at rest at `look`.
    pub fn new(look: BarLook) -> Self {
        Self {
            from: look,
            to: look,
            generation: 0,
        }
    }

    pub fn from(&self) -> BarLook {
        self.from
    }

    pub fn to(&self) -> BarLook {
        self.to
    }

    /// Counts the targets taken; a new value starts a new animation in the view.
    pub fn generation(&self) -> u64 {
        self.generation
    }

    /// The drawn look `elapsed` after the last retarget.
    pub fn look_at(&self, elapsed: Duration) -> BarLook {
        self.from.lerp(self.to, ease_out(progress(elapsed)))
    }

    /// Whether the ease has ended: the bar draws `to`.
    pub fn settled(&self, elapsed: Duration) -> bool {
        elapsed >= BAR_EASE
    }

    /// Aims at `target`. A new target starts from the drawn look, not from rest, so a change in
    /// mid-ease does not jump. Returns whether the target changed.
    pub fn retarget(&mut self, target: BarLook, elapsed: Duration) -> bool {
        if target == self.to {
            return false;
        }
        self.from = self.look_at(elapsed);
        self.to = target;
        self.generation += 1;
        true
    }
}

fn progress(elapsed: Duration) -> f32 {
    elapsed.as_secs_f32() / BAR_EASE.as_secs_f32()
}

/// Whether the rail shows: enough prompts and a panel at least [`MIN_PANEL_WIDTH`] wide at
/// `zoom`.
pub fn rail_visible(panel_width: f32, zoom: f32, prompt_count: usize) -> bool {
    prompt_count >= MIN_PROMPTS && panel_width >= MIN_PANEL_WIDTH * zoom
}

/// Whether a prompt index belongs to the timeline on screen: any index while the timeline
/// epoch is unknown, else the same epoch.
pub fn accept_prompt_index_epoch(timeline_epoch: Option<&str>, index_epoch: &str) -> bool {
    timeline_epoch.is_none_or(|epoch| epoch == index_epoch)
}

/// The seq the reader is at: the anchor of the row at `top`, else of the closest row above it
/// that has one (a footer has none). `len` is the row count; `top` past the end is the last row.
pub fn reading_anchor(
    top: usize,
    len: usize,
    anchor_of: impl Fn(usize) -> Option<u64>,
) -> Option<u64> {
    if len == 0 {
        return None;
    }
    (0..=top.min(len - 1)).rev().find_map(anchor_of)
}

/// What a click on a prompt does next.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum JumpPlan {
    /// The prompt is loaded: scroll to it.
    Scroll,
    /// The prompt is older than the loaded window: load the page before it and plan again.
    LoadOlder,
    /// The prompt is not loaded and no older page is left: drop the jump.
    Abandon,
}

pub fn plan_prompt_jump(is_loaded: bool, has_older: bool) -> JumpPlan {
    if is_loaded {
        JumpPlan::Scroll
    } else if has_older {
        JumpPlan::LoadOlder
    } else {
        JumpPlan::Abandon
    }
}

/// What the view does after a pointer event of [`HoverIntent`].
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum IntentEffect {
    Idle,
    /// Start the [`INITIAL_ACTIVATION_DELAY`] timer, replacing a running one. When it ends the
    /// view calls [`HoverIntent::fire`].
    Schedule,
    /// Show the preview of this bar.
    Activate(usize),
    /// Hide the preview and cancel the timer.
    Deactivate,
}

/// Hover intent of the rail (`hover-intent.ts`): the first preview waits for the pointer to
/// rest on a bar, and once one shows, every bar the pointer reaches shows at once. A move that is
/// mostly horizontal, such as the pointer leaving the rail toward the thread, restarts the wait.
#[derive(Debug, Default)]
pub struct HoverIntent {
    armed: bool,
    candidate: Option<usize>,
    anchor: Option<(f32, f32)>,
    active: bool,
}

impl HoverIntent {
    /// The pointer entered the rail.
    pub fn enter(&mut self, point: (f32, f32)) {
        self.anchor = Some(point);
    }

    /// The pointer reached bar `index`.
    pub fn point_at(&mut self, index: usize) -> IntentEffect {
        self.candidate = Some(index);
        if self.active {
            return IntentEffect::Activate(index);
        }
        if self.armed {
            return IntentEffect::Idle;
        }
        self.schedule()
    }

    /// The pointer moved on the rail.
    pub fn pointer_moved(&mut self, point: (f32, f32)) -> IntentEffect {
        let Some(anchor) = self.anchor.filter(|_| !self.active) else {
            return IntentEffect::Idle;
        };
        let delta_x = point.0 - anchor.0;
        let delta_y = point.1 - anchor.1;
        if delta_y.abs() >= delta_x.abs() {
            self.anchor = Some(point);
            return IntentEffect::Idle;
        }
        if delta_x.abs() < HORIZONTAL_TRANSIT_PX {
            return IntentEffect::Idle;
        }
        self.anchor = Some(point);
        self.schedule()
    }

    /// The activation timer ended.
    pub fn fire(&mut self) -> IntentEffect {
        self.armed = false;
        self.active = true;
        match self.candidate {
            Some(index) => IntentEffect::Activate(index),
            None => IntentEffect::Idle,
        }
    }

    /// The pointer left the rail.
    pub fn leave(&mut self) -> IntentEffect {
        *self = Self::default();
        IntentEffect::Deactivate
    }

    fn schedule(&mut self) -> IntentEffect {
        if self.candidate.is_none() {
            return IntentEffect::Idle;
        }
        self.armed = true;
        IntentEffect::Schedule
    }
}

/// What the rail asks of the timeline.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum OutlineEvent {
    /// The reader picked the prompt with this seq.
    Jump(u64),
}

/// The ease of one bar, kept by prompt seq so it survives a new prompt index.
struct BarState {
    seq: u64,
    /// `None` until the bar is first drawn, and again after the rail hides: the bar then
    /// shows its target at once.
    ease: Option<BarEase>,
    /// When the ease last retargeted.
    started: Instant,
}

/// The rail view. The timeline feeds it the prompt index and the reading position.
pub struct OutlineRail {
    /// The timeline's bounds at its last paint, shared with the timeline.
    bounds: Rc<Cell<Bounds<Pixels>>>,
    prompts: Vec<PromptEntry>,
    handles: Vec<FocusHandle>,
    /// One per prompt, in the same order.
    bars: Vec<BarState>,
    anchor: Option<u64>,
    active: Option<u64>,
    hovered: Option<usize>,
    intent: HoverIntent,
    timer: Option<Task<()>>,
}

impl EventEmitter<OutlineEvent> for OutlineRail {}

impl OutlineRail {
    pub fn new(bounds: Rc<Cell<Bounds<Pixels>>>) -> Self {
        Self {
            bounds,
            prompts: Vec::new(),
            handles: Vec::new(),
            bars: Vec::new(),
            anchor: None,
            active: None,
            hovered: None,
            intent: HoverIntent::default(),
            timer: None,
        }
    }

    /// Replaces the prompt index.
    pub fn set_prompts(&mut self, prompts: Vec<PromptEntry>, cx: &mut Context<Self>) {
        if self.prompts == prompts {
            return;
        }
        self.prompts = prompts;
        let mut old = std::mem::take(&mut self.bars);
        self.bars = self
            .prompts
            .iter()
            .map(|prompt| {
                old.iter()
                    .position(|bar| bar.seq == prompt.seq)
                    .map(|at| old.swap_remove(at))
                    .unwrap_or_else(|| BarState {
                        seq: prompt.seq,
                        ease: None,
                        started: Instant::now(),
                    })
            })
            .collect();
        self.handles.truncate(self.prompts.len());
        while self.handles.len() < self.prompts.len() {
            self.handles.push(cx.focus_handle().tab_stop(true));
        }
        if self
            .hovered
            .is_some_and(|index| index >= self.prompts.len())
        {
            self.hovered = None;
        }
        self.active = self.resolve_active();
        cx.notify();
    }

    /// Sets the seq at the top of the viewport; redraws only when the lit prompt changes.
    pub fn set_anchor(&mut self, anchor: Option<u64>, cx: &mut Context<Self>) {
        self.anchor = anchor;
        let active = self.resolve_active();
        if active != self.active {
            self.active = active;
            cx.notify();
        }
    }

    fn resolve_active(&self) -> Option<u64> {
        let seqs: Vec<u64> = self.prompts.iter().map(|prompt| prompt.seq).collect();
        resolve_active_prompt_seq(&seqs, self.anchor)
    }

    fn apply(&mut self, effect: IntentEffect, cx: &mut Context<Self>) {
        match effect {
            IntentEffect::Idle => {}
            IntentEffect::Schedule => {
                self.timer = Some(cx.spawn(async move |this, cx| {
                    cx.background_executor()
                        .timer(INITIAL_ACTIVATION_DELAY)
                        .await;
                    let _ = this.update(cx, |this, cx| {
                        let effect = this.intent.fire();
                        this.apply(effect, cx);
                    });
                }));
            }
            IntentEffect::Activate(index) => {
                if self.hovered != Some(index) {
                    self.hovered = Some(index);
                    cx.notify();
                }
            }
            IntentEffect::Deactivate => {
                self.timer = None;
                if self.hovered.take().is_some() {
                    cx.notify();
                }
            }
        }
    }

    fn pointer_entered(&mut self, point: Point<Pixels>) {
        self.intent.enter((f32::from(point.x), f32::from(point.y)));
    }

    fn pointer_moved(&mut self, point: Point<Pixels>, cx: &mut Context<Self>) {
        let effect = self
            .intent
            .pointer_moved((f32::from(point.x), f32::from(point.y)));
        self.apply(effect, cx);
    }

    fn pointer_left(&mut self, cx: &mut Context<Self>) {
        let effect = self.intent.leave();
        self.apply(effect, cx);
    }

    fn point_at(&mut self, index: usize, cx: &mut Context<Self>) {
        let effect = self.intent.point_at(index);
        self.apply(effect, cx);
    }

    fn jump(&mut self, seq: u64, window: &mut Window, cx: &mut Context<Self>) {
        // The bar stops lighting from focus once the jump starts.
        window.blur(cx);
        cx.emit(OutlineEvent::Jump(seq));
    }

    /// The look the bar at `index` heads to.
    fn target_look(
        &self,
        index: usize,
        attention: Option<usize>,
        reduce_motion: bool,
        cx: &Context<Self>,
    ) -> BarLook {
        let t = tokens(cx);
        let is_active = self.active == Some(self.prompts[index].seq);
        let magnification = match attention {
            Some(at) if !reduce_motion => prompt_tick_magnification(index as f32 - at as f32),
            _ => 0.,
        };
        let (width, height) = pill_size(is_active, magnification);
        let color = if attention == Some(index) {
            t.foreground
        } else if is_active {
            t.foreground_extra_muted
        } else {
            t.border_accent
        };
        BarLook {
            width,
            height,
            color: color.into(),
        }
    }

    /// Aims every bar at its target look. A bar whose target changed starts to ease from the
    /// look it draws now.
    fn retarget_bars(&mut self, attention: Option<usize>, reduce_motion: bool, cx: &Context<Self>) {
        let now = Instant::now();
        for index in 0..self.prompts.len() {
            let target = self.target_look(index, attention, reduce_motion, cx);
            let bar = &mut self.bars[index];
            match bar.ease.as_mut() {
                Some(ease) => {
                    if ease.retarget(target, now.saturating_duration_since(bar.started)) {
                        bar.started = now;
                    }
                }
                None => {
                    bar.ease = Some(BarEase::new(target));
                    bar.started = now;
                }
            }
        }
    }

    fn tick(
        &self,
        index: usize,
        attention: Option<usize>,
        reduce_motion: bool,
        cx: &Context<Self>,
    ) -> impl IntoElement {
        let t = tokens(cx);
        let prompt = &self.prompts[index];
        let seq = prompt.seq;
        let has_attention = attention == Some(index);
        let bar = &self.bars[index];
        let ease = bar.ease.expect("retarget_bars ran before tick");
        let pill = div().rounded_full();
        // Only a running ease draws frames; a settled bar is a plain element.
        let pill = if reduce_motion || ease.settled(bar.started.elapsed()) {
            let look = ease.to();
            pill.w(zoomed(look.width, cx))
                .h(zoomed(look.height, cx))
                .bg(Hsla::from(look.color))
                .into_any_element()
        } else {
            let (from, to, scale) = (ease.from(), ease.to(), zoom(cx));
            pill.with_animation(
                ("outline-pill", ease.generation()),
                Animation::new(BAR_EASE).with_easing(ease_out),
                move |pill, progress| {
                    let look = from.lerp(to, progress);
                    pill.w(px(look.width * scale))
                        .h(px(look.height * scale))
                        .bg(Hsla::from(look.color))
                },
            )
            .into_any_element()
        };
        let label = format!(
            "{} of {}: {}",
            index + 1,
            self.prompts.len(),
            prompt.preview
        );
        let weak = cx.weak_entity();
        div()
            .id(("outline-tick", seq))
            .role(Role::Tab)
            .aria_label(label)
            .track_focus(&self.handles[index])
            .relative()
            .w_full()
            .flex_basis(zoomed(SLOT_HEIGHT, cx))
            .flex_shrink_1()
            .min_h(px(0.))
            .flex()
            .flex_col()
            .justify_center()
            .pl(zoomed(SPACE_1, cx))
            .cursor_pointer()
            .on_hover(cx.listener(move |this, hovered: &bool, _, cx| {
                if *hovered {
                    this.point_at(index, cx);
                }
            }))
            .on_click(cx.listener(move |this, _, window, cx| this.jump(seq, window, cx)))
            .on_a11y_action(AccessibleAction::Click, move |_, window, cx| {
                weak.update(cx, |this, cx| this.jump(seq, window, cx)).ok();
            })
            .child(pill)
            .when(has_attention, |slot| {
                slot.child(
                    div()
                        .absolute()
                        .top_0()
                        .bottom_0()
                        .left(zoomed(RAIL_WIDTH + PREVIEW_GAP, cx))
                        .flex()
                        .items_center()
                        .child(
                            div()
                                .w(zoomed(PREVIEW_WIDTH, cx))
                                .h(zoomed(PREVIEW_HEIGHT, cx))
                                .flex()
                                .items_center()
                                .px(zoomed(SPACE_3, cx))
                                .overflow_hidden()
                                .rounded(zoomed(RADIUS_LG, cx))
                                .border_1()
                                .border_color(t.border)
                                .bg(t.surface2)
                                .shadow(vec![shadow(ShadowSize::Md, cx)])
                                .text_size(ui_font_size(FONT_SM, cx))
                                .text_color(t.foreground)
                                // A flex row lays its text out unwrapped; a full-width block
                                // wraps it and clamps it to Electron's two lines.
                                .child(
                                    div()
                                        .w_full()
                                        .min_w_0()
                                        .line_clamp(2)
                                        .text_ellipsis()
                                        .child(prompt.preview.clone()),
                                ),
                        ),
                )
            })
    }
}

impl Render for OutlineRail {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let panel_width = f32::from(self.bounds.get().size.width);
        if !rail_visible(panel_width, zoom(cx), self.prompts.len()) {
            // A rail that goes away takes its pointer with it: no leave event follows.
            self.intent = HoverIntent::default();
            self.timer = None;
            self.hovered = None;
            // Bars come back settled, not mid-ease.
            self.bars.iter_mut().for_each(|bar| bar.ease = None);
            return div().absolute().size_0().into_any_element();
        }
        // The pointer owns the magnified band while it is on the rail; keyboard focus drives the
        // same band and the same preview once the pointer leaves.
        let attention = self.hovered.or_else(|| {
            self.handles
                .iter()
                .position(|handle| handle.is_focused(window))
        });
        let reduce_motion = cx.reduce_motion();
        self.retarget_bars(attention, reduce_motion, &*cx);
        let ticks: Vec<_> = (0..self.prompts.len())
            .map(|index| self.tick(index, attention, reduce_motion, &*cx))
            .collect();
        div()
            .id("chat-outline-rail")
            .role(Role::TabList)
            .aria_label("Chat outline")
            .absolute()
            .left(zoomed(SPACE_2, cx))
            .top(relative(RAIL_INSET))
            .bottom(relative(RAIL_INSET))
            .w(zoomed(RAIL_WIDTH, cx))
            .flex()
            .flex_col()
            .items_center()
            .justify_center()
            .on_hover(cx.listener(|this, hovered: &bool, window, cx| {
                if *hovered {
                    this.pointer_entered(window.mouse_position());
                } else {
                    this.pointer_left(cx);
                }
            }))
            .on_mouse_move(cx.listener(|this, event: &MouseMoveEvent, _, cx| {
                this.pointer_moved(event.position, cx)
            }))
            .children(ticks)
            .into_any_element()
    }
}
