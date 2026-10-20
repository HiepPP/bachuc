//! The one frame ticker of the GPUI app and the clock every running mark reads (ADR-0052).
//!
//! GPUI marks the ancestor views of a notified view dirty, so each frame of a mark re-renders the
//! window tree that holds it, and the frame count is the CPU cost. One ticker at
//! [`FRAME_INTERVAL`] notifies every painted mark on the same frame, and a frame whose quantized
//! output did not change asks for nothing. The ticker sleeps when no mark was painted lately.
//!
//! A [`Motion`] is a small view that redraws itself on the ticker. Any other view calls
//! [`request_tick`] from its render or paint when it draws a moving frame.

use std::cell::{Cell, RefCell};
use std::sync::OnceLock;
use std::time::{Duration, Instant};

use gpui_kit::{AnyElement, App, Context, EntityId, IntoElement, Render, Task, WeakEntity, Window};

/// How often a painted mark asks for the next frame. Every frame re-renders the window tree that
/// holds the mark, so this rate is the CPU cost (ADR-0039, ADR-0052). 66 ms is 15 fps.
pub const FRAME_INTERVAL: Duration = Duration::from_millis(66);
/// A mark that was not painted for this long stops asking for frames.
pub const SLEEP_AFTER: Duration = Duration::from_millis(200);

/// Milliseconds on the one shared motion clock.
pub fn motion_ms() -> u128 {
    static START: OnceLock<Instant> = OnceLock::new();
    START.get_or_init(Instant::now).elapsed().as_millis()
}

thread_local! {
    /// Every live mark, and the one task that ticks them all. GPUI runs every view on the main
    /// thread. Weak handles, so a dropped mark leaves the registry at the next tick.
    static TICKER: RefCell<Ticker> = RefCell::new(Ticker::default());
}

#[derive(Default)]
struct Ticker {
    motions: Vec<WeakEntity<Motion>>,
    /// Plain views with the time of their last request. A view is notified once, then leaves.
    views: Vec<(EntityId, Instant)>,
    task: Option<Task<()>>,
}

/// Asks the ticker to notify `view` once at the next tick. A view calls this from its render or
/// paint when it draws a moving frame. A view that stops calling it drops out of the ticker, so
/// a covered window or a scrolled-out view costs nothing. Reduced motion requests no tick.
pub fn request_tick(view: EntityId, cx: &mut App) {
    if !ticks(cx.reduce_motion()) {
        return;
    }
    let now = Instant::now();
    TICKER.with(|ticker| note_request(&mut ticker.borrow_mut().views, view, now));
    start(cx);
}

/// Whether a mark asks for frames: reduced motion asks for none.
fn ticks(reduce_motion: bool) -> bool {
    !reduce_motion
}

/// Records a request of `view` at `now`. A view that asked before keeps one entry.
fn note_request(views: &mut Vec<(EntityId, Instant)>, view: EntityId, now: Instant) {
    match views.iter_mut().find(|(id, _)| *id == view) {
        Some((_, requested)) => *requested = now,
        None => views.push((view, now)),
    }
}

/// Whether the ticker task runs on after a step: a requesting view was notified, or a painted
/// mark was seen. The task ends when neither is left, so the app reads idle.
fn keeps_ticking(notified_views: usize, painted_motion: bool) -> bool {
    notified_views > 0 || painted_motion
}

/// The time a mark draws: the shared clock, or 0 (the still state) with reduced motion.
fn draw_time(reduce_motion: bool, clock_ms: u128) -> u128 {
    if ticks(reduce_motion) { clock_ms } else { 0 }
}

/// Adds a mark to the ticker.
fn register(motion: WeakEntity<Motion>, cx: &mut App) {
    TICKER.with(|ticker| {
        let mut ticker = ticker.borrow_mut();
        let id = motion.entity_id();
        if !ticker.motions.iter().any(|other| other.entity_id() == id) {
            ticker.motions.push(motion);
        }
    });
    start(cx);
}

/// Starts the ticker task when it is not running.
fn start(cx: &mut App) {
    if TICKER.with(|ticker| ticker.borrow().task.is_some()) {
        return;
    }
    let task = cx.spawn(async move |cx| {
        loop {
            cx.background_executor().timer(FRAME_INTERVAL).await;
            let painted = cx.update(tick_all);
            if !painted {
                break;
            }
        }
        TICKER.with(|ticker| ticker.borrow_mut().task = None);
    });
    TICKER.with(|ticker| ticker.borrow_mut().task = Some(task));
}

/// Takes the views whose request is fresh at `now`, and drops the rest. A view whose last request
/// is `SLEEP_AFTER` old was not painted lately, so it is not notified.
fn take_due_views(views: &mut Vec<(EntityId, Instant)>, now: Instant) -> Vec<EntityId> {
    std::mem::take(views)
        .into_iter()
        .filter(|(_, requested)| now.saturating_duration_since(*requested) < SLEEP_AFTER)
        .map(|(id, _)| id)
        .collect()
}

/// One ticker step: notifies each painted mark that is due and each requesting view, and tells
/// whether any was painted lately, so the ticker sleeps when nothing moves on screen.
fn tick_all(cx: &mut App) -> bool {
    let now = Instant::now();
    let (motions, views) = TICKER.with(|ticker| {
        let mut ticker = ticker.borrow_mut();
        let views = take_due_views(&mut ticker.views, now);
        (std::mem::take(&mut ticker.motions), views)
    });
    let notified_views = views.len();
    for view in views {
        cx.notify(view);
    }
    let mut painted_motion = false;
    let mut live = Vec::with_capacity(motions.len());
    for motion in motions {
        if let Ok(seen) = motion.update(cx, |motion, cx| motion.tick(now, cx)) {
            painted_motion |= seen;
            live.push(motion);
        }
    }
    // A notify can render and register a new mark, so add to the registry rather than replace it.
    TICKER.with(|ticker| {
        let mut ticker = ticker.borrow_mut();
        for motion in live {
            let id = motion.entity_id();
            if !ticker.motions.iter().any(|other| other.entity_id() == id) {
                ticker.motions.push(motion);
            }
        }
    });
    keeps_ticking(notified_views, painted_motion)
}

/// A mark that redraws itself at `interval` while it is on screen.
pub struct Motion {
    interval: Duration,
    /// A small number for the drawn output: equal states draw the same pixels.
    state: Option<Box<dyn Fn(u128) -> u64>>,
    draw: Box<dyn Fn(u128, &mut Window, &mut App) -> AnyElement>,
    painted: Cell<Instant>,
    notified: Cell<Instant>,
    drawn_state: Cell<Option<u64>>,
}

impl Motion {
    pub fn new(
        interval: Duration,
        draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut Context<Motion>,
    ) -> Motion {
        Self::with_state_opt(interval, None, draw, cx)
    }

    /// Like `new`, with a quantized `state` of what the mark draws. A frame in the same state
    /// does not notify.
    pub fn with_state(
        interval: Duration,
        state: impl Fn(u128) -> u64 + 'static,
        draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut Context<Motion>,
    ) -> Motion {
        Self::with_state_opt(interval, Some(Box::new(state)), draw, cx)
    }

    fn with_state_opt(
        interval: Duration,
        state: Option<Box<dyn Fn(u128) -> u64>>,
        draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut Context<Motion>,
    ) -> Motion {
        let now = Instant::now();
        let motion = Motion {
            interval,
            state,
            draw: Box::new(draw),
            painted: Cell::new(now),
            notified: Cell::new(now),
            drawn_state: Cell::new(None),
        };
        register(cx.weak_entity(), cx);
        motion
    }

    pub fn interval(&self) -> Duration {
        self.interval
    }

    pub fn set_draw(
        &mut self,
        draw: impl Fn(u128, &mut Window, &mut App) -> AnyElement + 'static,
        cx: &mut Context<Motion>,
    ) {
        self.draw = Box::new(draw);
        cx.notify();
    }

    pub fn set_state(&mut self, state: impl Fn(u128) -> u64 + 'static) {
        self.state = Some(Box::new(state));
    }

    /// One ticker step. Tells whether this mark was painted lately. A mark that is not due, or
    /// whose quantized output did not change, does not notify.
    fn tick(&mut self, now: Instant, cx: &mut Context<Self>) -> bool {
        if !ticks(cx.reduce_motion()) || self.painted.get().elapsed() >= SLEEP_AFTER {
            return false;
        }
        let ms = motion_ms();
        let changed = self
            .state
            .as_ref()
            .is_none_or(|state| self.drawn_state.get() != Some(state(ms)));
        if frame_draws(
            now.duration_since(self.notified.get()),
            self.interval,
            changed,
        ) {
            self.notified.set(now);
            cx.notify();
        }
        true
    }
}

impl Render for Motion {
    fn render(&mut self, window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        self.painted.set(Instant::now());
        // Reduced motion draws the still state, as `useReducedMotion` does, and asks for no tick.
        let reduce_motion = cx.reduce_motion();
        if ticks(reduce_motion) {
            // A covered window stops drawing, so this restarts a ticker that slept meanwhile.
            register(cx.weak_entity(), cx);
        }
        let time = draw_time(reduce_motion, motion_ms());
        if let Some(state) = &self.state {
            self.drawn_state.set(Some(state(time)));
        }
        (self.draw)(time, window, cx)
    }
}

/// Whether a ticker step draws: the mark is due, and its quantized output changed.
pub fn frame_draws(elapsed: Duration, interval: Duration, changed: bool) -> bool {
    elapsed >= interval && changed
}

#[cfg(test)]
mod tests {
    use super::*;

    fn id(n: u64) -> EntityId {
        EntityId::from(n)
    }

    fn ms(n: u64) -> Duration {
        Duration::from_millis(n)
    }

    #[test]
    fn two_views_requesting_in_one_interval_are_notified_in_one_pass() {
        let now = Instant::now();
        let mut views = Vec::new();
        note_request(&mut views, id(1), now);
        note_request(&mut views, id(2), now + ms(10));
        // A repeat request keeps one entry, so the view is notified once.
        note_request(&mut views, id(1), now + ms(20));
        let due = take_due_views(&mut views, now + FRAME_INTERVAL);
        assert_eq!(due, vec![id(1), id(2)]);
        assert!(keeps_ticking(due.len(), false));
    }

    #[test]
    fn a_view_not_painted_for_200_ms_is_not_notified() {
        let now = Instant::now();
        let mut views = vec![(id(1), now - ms(201)), (id(2), now - ms(199)), (id(3), now)];
        assert_eq!(take_due_views(&mut views, now), vec![id(2), id(3)]);
        assert!(views.is_empty(), "every request is taken or dropped");
    }

    #[test]
    fn the_ticker_ends_when_no_view_is_left() {
        let now = Instant::now();
        let mut views = Vec::new();
        note_request(&mut views, id(1), now);
        let first = take_due_views(&mut views, now + FRAME_INTERVAL);
        assert!(keeps_ticking(first.len(), false));
        // The view did not ask again, so the next pass finds nothing and the ticker sleeps.
        let second = take_due_views(&mut views, now + FRAME_INTERVAL * 2);
        assert!(!keeps_ticking(second.len(), false));
        // A painted mark keeps it awake with no requesting view.
        assert!(keeps_ticking(0, true));
    }

    #[test]
    fn a_dropped_view_is_not_notified_and_the_next_request_wakes_the_ticker_again() {
        let now = Instant::now();
        let mut views = Vec::new();
        note_request(&mut views, id(1), now);
        // The view is dropped and never asks again: its request is stale at SLEEP_AFTER.
        let late = now + SLEEP_AFTER;
        let due = take_due_views(&mut views, late);
        assert!(due.is_empty());
        assert!(!keeps_ticking(due.len(), false), "the ticker slept");
        // The next request is fresh again and the step after it keeps the ticker going.
        note_request(&mut views, id(2), late + ms(5));
        let due = take_due_views(&mut views, late + FRAME_INTERVAL);
        assert_eq!(due, vec![id(2)]);
        assert!(keeps_ticking(due.len(), false));
    }

    #[test]
    fn reduced_motion_requests_no_tick() {
        assert!(!ticks(true));
        assert!(ticks(false));
    }

    #[test]
    fn a_motion_draws_time_0_with_reduced_motion() {
        assert_eq!(draw_time(true, 12_345), 0);
        assert_eq!(draw_time(false, 12_345), 12_345);
    }

    #[test]
    fn a_frame_draws_when_due_and_changed() {
        assert!(frame_draws(FRAME_INTERVAL, FRAME_INTERVAL, true));
        assert!(!frame_draws(ms(10), FRAME_INTERVAL, true));
        assert!(!frame_draws(FRAME_INTERVAL, FRAME_INTERVAL, false));
    }
}
