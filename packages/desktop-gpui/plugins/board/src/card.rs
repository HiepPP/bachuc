//! `RunCard` and `RunCluster` of `client/page.tsx` (287-1107): the card, its subagent row
//! variant, and the cluster parts the Runs lists draw as separate items. Every measure is the
//! Electron value times the Board scale.

use crate::avatar::agent_avatar;
use crate::model::colors::{
    effort_color, project_initial, repo_divider, repo_label, repo_mark, repo_surface,
};
use crate::model::text::{
    card_label, card_meta, card_status, open_hint, remove_label, remove_name, run_duration,
    subagent_count, summary_label,
};
use crate::model::tree::{RunTree, cluster_summary, two_columns};
use crate::model::{OrbSettings, Run, RunStatus, SEPARATOR};
use bachuc_icons::{IconName, icon};
use bachuc_theme::Tokens;
use gpui_kit::component::spinner::Spinner;
use gpui_kit::component::{Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::rc::Rc;
use std::sync::Arc;
use std::time::Duration;

// Shape lock: cards and notices 8; controls and subagent rows 6.
pub(crate) const CARD_RADIUS: f32 = 8.;
pub(crate) const CONTROL_RADIUS: f32 = 6.;
/// Content of a finished card that was opened since; actions keep full contrast.
const SEEN_OPACITY: f32 = 0.5;
/// The cluster panel's side padding and row gap (`RunCluster`).
pub(crate) const PANEL_EDGE: f32 = 6.;
const PANEL_GAP: f32 = 2.;
/// The nested panel's left margin.
const NESTED_INDENT: f32 = 11.;
/// One pulse: 700 ms in, 700 ms out.
const PULSE: Duration = Duration::from_millis(1_400);
const PULSE_LOW: f32 = 0.35;
const RING_WIDTH: f32 = 2.;

/// A card's own action state: the in-flight flags and the inline errors.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub(crate) struct CardUi {
    pub starring: bool,
    pub star_error: bool,
    pub toggling_diffs: bool,
    pub diffs_error: bool,
    pub removing: bool,
    pub remove_error: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) enum CardAction {
    Open(String),
    Star { run_id: String, starred: bool },
    Diffs { agent_id: String, off: bool },
    Remove { run_id: String, agent_id: String },
    OpenProject(Box<Run>),
    OpenTask(Box<Run>),
    Toggle(String),
}

pub(crate) type OnAction = Rc<dyn Fn(CardAction, &mut Window, &mut App)>;

/// What every card of one frame shares.
pub(crate) struct Paint<'a> {
    pub t: Tokens,
    pub scale: f32,
    pub now: i64,
    pub hues: &'a BTreeMap<String, u16>,
    pub orb: OrbSettings,
    pub online: bool,
    /// The active host runs watchtower-board, so TASK chips link.
    pub watchtower: bool,
    pub collapsed: &'a HashSet<String>,
    pub cards: &'a HashMap<String, CardUi>,
    pub on: OnAction,
}

impl Paint<'_> {
    pub(crate) fn s(&self, value: f32) -> Pixels {
        px(value * self.scale)
    }

    fn hue(&self, run: &Run) -> Option<u16> {
        run.project_id
            .as_ref()
            .and_then(|id| self.hues.get(id).copied())
    }

    fn act(&self, action: CardAction) -> impl Fn(&ClickEvent, &mut Window, &mut App) + 'static {
        let on = self.on.clone();
        move |_, window, cx| {
            cx.stop_propagation();
            on(action.clone(), window, cx);
        }
    }

    /// A click and an AX press run the same action. GPUI's default AX press replays a click at
    /// the node centre, which hits whatever is drawn there, or nothing when the centre is clipped.
    fn press<E: StatefulInteractiveElement>(&self, el: E, action: CardAction) -> E {
        let on = self.on.clone();
        el.on_click(self.act(action.clone())).on_a11y_action(
            AccessibleAction::Click,
            move |_, window, cx| {
                // An AX action goes only to its own node, so the card's Open never runs; this
                // keeps the click's contract anyway.
                cx.stop_propagation();
                on(action.clone(), window, cx);
            },
        )
    }

    fn text(&self, size: f32, line: f32) -> Div {
        div().text_size(self.s(size)).line_height(self.s(line))
    }
}

/// How far the repo row's pencil and star move right into the card padding: the star's
/// `marginRight: -6` in `page.tsx`. GPUI takes it as a relative offset, never as a negative margin:
/// taffy sizes a `flex_none` row as 0 wide when a child has a negative main-axis margin (its
/// max-content code scales the negative contribution by the flex basis), so the pencil and star
/// drew past the card's clip.
pub(crate) fn actions_offset(scale: f32) -> f32 {
    6. * scale
}

/// The opacity of a pulse at `delta` of its loop: ease in and out from 1 to 0.35 and back.
pub(crate) fn pulse_opacity(delta: f32) -> f32 {
    let t = if delta < 0.5 {
        delta * 2.
    } else {
        (1. - delta) * 2.
    };
    let eased = if t < 0.5 {
        2. * t * t
    } else {
        1. - (-2. * t + 2.).powi(2) / 2.
    };
    1. - (1. - PULSE_LOW) * eased
}

/// Draws the eye to runs that wait for the owner.
pub(crate) fn pulse(id: impl Into<ElementId>, child: impl IntoElement) -> AnyElement {
    div()
        .flex_none()
        .child(child)
        .with_animation(id, Animation::new(PULSE).repeat(), |el, delta| {
            el.opacity(pulse_opacity(delta))
        })
        .into_any_element()
}

/// The 2 px keyboard focus ring in `accent`; inset where a parent clips its overflow.
pub(crate) fn ring(color: Hsla, inset: bool) -> Vec<BoxShadow> {
    vec![BoxShadow {
        color,
        offset: point(px(0.), px(0.)),
        blur_radius: px(0.),
        spread_radius: px(RING_WIDTH),
        inset,
    }]
}

pub(crate) fn tabular() -> FontFeatures {
    FontFeatures(Arc::new(vec![("tnum".into(), 1)]))
}

/// A focusable button with a role, a label, and the keyboard ring.
pub(crate) fn button(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    accent: Hsla,
    inset: bool,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .focusable()
        .focus_visible(move |style| style.shadow(ring(accent, inset)))
        .cursor_pointer()
}

/// The card parts that take focus, in Tab order.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum FocusPart {
    Open,
    Repo,
    Diffs,
    Star,
    Task,
    Remove,
}

const FOCUS_PARTS: [FocusPart; 6] = [
    FocusPart::Open,
    FocusPart::Repo,
    FocusPart::Diffs,
    FocusPart::Star,
    FocusPart::Task,
    FocusPart::Remove,
];

/// The ring's distance outside the part edge and its corner radius. Electron draws a 2 px outline
/// at offset 1, or at -2 inside a surface that clips its overflow (`focus.ts`), so the inset ring
/// covers the part's outer 2 px.
pub(crate) fn ring_box(radius: f32, inset: bool) -> (f32, f32) {
    let outset = if inset { 0. } else { 1. + RING_WIDTH };
    (outset, if radius > 0. { radius + outset } else { 0. })
}

/// `actionsVisible = ... || actionFocused` (`page.tsx:386-394`): any focus on the star or
/// Remove, from a click too, shows a subagent row's actions.
pub(crate) fn subagent_actions_shown(pinned: bool, focused: Option<FocusPart>) -> bool {
    pinned || matches!(focused, Some(FocusPart::Star | FocusPart::Remove))
}

/// One keyed focus handle per focusable part of a rendered card, so the render knows which part
/// has focus. GPUI drops a handle when its card is not rendered.
struct Parts {
    handles: [FocusHandle; 6],
    focused: Option<FocusPart>,
    keyboard: bool,
}

impl Parts {
    fn new(agent_id: &str, keyboard: bool, window: &mut Window, cx: &mut App) -> Self {
        let key = SharedString::from(format!("board-part-{agent_id}"));
        // The handle carries the tab stop: `.tab_stop` on the element is ignored for a tracked handle.
        let handles = std::array::from_fn(|ix| {
            window
                .use_keyed_state(ElementId::named_usize(key.clone(), ix), cx, |_, cx| {
                    cx.focus_handle().tab_stop(true)
                })
                .read(cx)
                .clone()
        });
        let focused = FOCUS_PARTS
            .into_iter()
            .find(|&part| handles[part as usize].is_focused(window));
        Self {
            handles,
            focused,
            keyboard,
        }
    }

    fn handle(&self, part: FocusPart) -> &FocusHandle {
        &self.handles[part as usize]
    }

    /// The ring is for keyboard focus only; a click focuses the part without drawing it.
    fn ring(&self, part: FocusPart) -> bool {
        self.keyboard && self.focused == Some(part)
    }
}

/// A focusable card part with a role and a label; `focus_ring` draws its ring.
fn part(
    id: impl Into<ElementId>,
    label: impl Into<SharedString>,
    focus: &FocusHandle,
) -> Stateful<Div> {
    div()
        .id(id)
        .role(Role::Button)
        .aria_label(label)
        .track_focus(focus)
        .cursor_pointer()
}

/// The 2 px keyboard focus ring as a border with no fill, laid over the part. A spread `BoxShadow`
/// does not work here: GPUI paints its whole quad under the part, so a transparent part showed a
/// solid box.
fn focus_ring(color: Hsla, radius: f32, inset: bool) -> Div {
    let (outset, radius) = ring_box(radius, inset);
    div()
        .absolute()
        .top(px(-outset))
        .left(px(-outset))
        .right(px(-outset))
        .bottom(px(-outset))
        .border_2()
        .border_color(color)
        .rounded(px(radius))
}

pub(crate) fn spinner(color: Hsla, size: Pixels) -> AnyElement {
    div()
        .flex_none()
        .size(size)
        .flex()
        .items_center()
        .justify_center()
        .child(Spinner::new().with_size(size).color(color))
        .into_any_element()
}

/// The avatar of a running card that does not need input: the thinking orb when it is on,
/// else the running spinner over the avatar.
pub(crate) fn running_avatar(
    agent_id: &str,
    size: Pixels,
    orb: &OrbSettings,
    _window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    if orb.enabled {
        return crate::orb_view::orb_avatar(agent_id, size, orb, cx);
    }
    let accent = bachuc_theme::tokens(cx).accent;
    div()
        .relative()
        .flex_none()
        .size(size)
        .child(agent_avatar(agent_id, size))
        .child(
            div()
                .absolute()
                .inset_0()
                .flex()
                .items_center()
                .justify_center()
                .child(spinner(accent, size * 0.5)),
        )
        .into_any_element()
}

/// The status color: attention, running, then the outcome.
fn tone(run: &Run, t: &Tokens) -> Hsla {
    if run.needs_input {
        t.status_warning
    } else {
        match run.status {
            RunStatus::Running => t.accent,
            RunStatus::Failed => t.status_danger,
            RunStatus::Completed => t.status_success,
            RunStatus::Cancelled | RunStatus::Unknown => t.status_warning,
        }
    }
}

/// Only attention and non-success outcomes take a status color.
fn label_color(run: &Run, t: &Tokens) -> Hsla {
    if run.needs_input || (!run.running() && run.status != RunStatus::Completed) {
        tone(run, t)
    } else {
        t.foreground_muted
    }
}

/// A one-line text whose first `label.len()` bytes take their own color and weight.
fn meta_text(text: String, label: Option<(usize, Hsla, FontWeight)>) -> StyledText {
    let highlights = label
        .map(|(len, color, weight)| {
            (
                0..len,
                HighlightStyle {
                    color: Some(color),
                    font_weight: Some(weight),
                    ..Default::default()
                },
            )
        })
        .into_iter()
        .collect::<Vec<_>>();
    StyledText::new(text).with_highlights(highlights)
}

/// The parts of a card that the seen rule looks at.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum CardPart {
    Repo,
    /// The avatar and the title column.
    Main,
    Tags,
    /// The subagent elbow.
    Elbow,
    Star,
    Diffs,
    Remove,
}

/// `dim = seen && !active ? 0.5 : 1` (`page.tsx:565`): only the repo, the avatar and title,
/// the model tags, and the elbow (at 0.6) dim; the actions keep full contrast.
pub(crate) fn part_opacity(part: CardPart, seen: bool, active: bool) -> f32 {
    let dim = if seen && !active { SEEN_OPACITY } else { 1. };
    match part {
        CardPart::Repo | CardPart::Main | CardPart::Tags => dim,
        CardPart::Elbow => 0.6 * dim,
        CardPart::Star | CardPart::Diffs | CardPart::Remove => 1.,
    }
}

/// `active = hovered || ... || focus?.keyboard` (`page.tsx:563-565`): a hover, or focus on one of
/// the card's parts that came from the keyboard.
pub(crate) fn card_active(hovered: bool, focused: bool, keyboard: bool) -> bool {
    hovered || (focused && keyboard)
}

/// Applies `part_opacity`. The card's keyboard focus is active at rest, and a hover
/// (`group_hover`) is always active; only opacity changes, never `display`.
fn dim<E: Styled + InteractiveElement>(
    el: E,
    part: CardPart,
    seen: bool,
    focused: bool,
    group: &SharedString,
) -> E {
    let (rest, active) = (
        part_opacity(part, seen, focused),
        part_opacity(part, seen, true),
    );
    let el = if rest < 1. { el.opacity(rest) } else { el };
    if rest == active {
        el
    } else {
        el.group_hover(group.clone(), move |style| style.opacity(active))
    }
}

/// One card, as `RunCard`.
pub(crate) struct Card<'r> {
    pub run: &'r Run,
    pub seen: bool,
    /// Inside a cluster: no own border.
    pub embedded: bool,
    /// The compact two-line row inside a parent's panel.
    pub subagent: bool,
    /// A subagent omits the project its parent already names.
    pub inherited_project: bool,
    /// Conversations Remove hides, this one included.
    pub remove_count: usize,
    /// Remove shows: the host is online and nothing in the subtree runs.
    pub can_remove: bool,
    /// Keyboard focus is on one of the card's parts; `run_card` sets it.
    pub focused: bool,
}

pub(crate) fn run_card(p: &Paint, c: Card, window: &mut Window, cx: &mut App) -> AnyElement {
    let run = c.run;
    let t = &p.t;
    let ui = p.cards.get(&run.id).cloned().unwrap_or_default();
    let running = run.running();
    let group: SharedString = format!("board-card-{}", run.id).into();
    // One handle per rendered card, tracked on the card root, so focus on any part is inside
    // it. It is no tab stop, and GPUI drops it when the card leaves the list. The parts are tab
    // stops at index 0, so Tab follows their paint order, which is Electron's DOM order.
    let focus = window
        .use_keyed_state(
            SharedString::from(format!("board-focus-{}", run.agent_id)),
            cx,
            |_, cx| cx.focus_handle(),
        )
        .read(cx)
        .clone();
    let keyboard = window.last_input_was_keyboard();
    let f = Parts::new(&run.agent_id, keyboard, window, cx);
    let c = Card {
        focused: card_active(
            false,
            keyboard && focus.contains_focused(window, cx),
            keyboard,
        ),
        ..c
    };

    let star = star_button(p, &c, &ui, &f, &group);
    let remove = (c.can_remove && !running).then(|| remove_button(p, &c, &ui, &f, &group));

    let body = if c.subagent {
        subagent_body(p, &c, &ui, &f, &group, star, remove)
    } else {
        card_body(p, &c, &ui, &f, &group, star, remove, window, cx)
    };
    let errors = [
        (ui.star_error, "Could not update star. Please retry."),
        (ui.diffs_error, "Could not change edit diffs. Please retry."),
        (ui.remove_error, "Could not remove. Please retry."),
    ]
    .into_iter()
    .filter(|(shown, _)| *shown)
    .map(|(_, message)| {
        p.text(12., 16.)
            .id(SharedString::from(format!(
                "board-error-{}-{message}",
                run.id
            )))
            .role(Role::Alert)
            .aria_label(message)
            .text_color(t.status_danger)
            .child(message)
    });

    let (background, border) = repo_surface(if c.seen { None } else { p.hue(run) }, t);
    let radius = if c.subagent {
        p.s(CONTROL_RADIUS)
    } else if c.embedded {
        px(0.)
    } else {
        p.s(CARD_RADIUS)
    };
    let open = part(
        SharedString::from(format!("board-open-{}", run.id)),
        format!("Open conversation {}", run.title),
        f.handle(FocusPart::Open),
    )
    .aria_description(open_hint(run, p.now))
    .absolute()
    .inset_0()
    .rounded(radius)
    .group_hover(group.clone(), |style| style.bg(t.surface2.opacity(0.55)))
    .active(|style| style.bg(t.surface2))
    .on_click(p.act(CardAction::Open(run.agent_id.clone())));

    div()
        .id(SharedString::from(format!("board-card-{}", run.id)))
        .group(group.clone())
        .track_focus(&focus)
        .role(Role::Group)
        .aria_label(card_label(run, c.seen, c.subagent))
        .relative()
        .w_full()
        .min_w_0()
        .rounded(radius)
        .overflow_hidden()
        .when(!c.embedded && !c.subagent, |card| {
            card.border_1().border_color(border).bg(background)
        })
        .child(open)
        .child(
            v_flex()
                .relative()
                .px(p.s(if c.subagent { 8. } else { 12. }))
                .py(p.s(if c.subagent { 10. } else { 16. }))
                .gap(p.s(6.))
                .child(body)
                .children(errors),
        )
        .when(run.needs_input, |card| {
            card.child(pulse(
                SharedString::from(format!("board-attention-{}", run.id)),
                div()
                    .absolute()
                    .inset_0()
                    .border_2()
                    .border_color(t.status_warning)
                    .rounded(if c.subagent {
                        p.s(CONTROL_RADIUS)
                    } else if c.embedded {
                        px(0.)
                    } else {
                        p.s(CARD_RADIUS - 1.)
                    }),
            ))
        })
        // The open target covers the card, whose clip would hide an outer ring, so it draws inset,
        // last, over the attention border.
        .when(f.ring(FocusPart::Open), |card| {
            card.child(focus_ring(t.accent, f32::from(radius), true))
        })
        .into_any_element()
}

fn star_button(p: &Paint, c: &Card, ui: &CardUi, f: &Parts, group: &SharedString) -> AnyElement {
    let (run, t) = (c.run, &p.t);
    let size = p.s(if c.subagent { 24. } else { 28. });
    let quiet = !(c.subagent || run.starred || ui.starring);
    let glyph = if run.starred {
        p.text(
            if c.subagent { 16. } else { 20. },
            if c.subagent { 20. } else { 24. },
        )
        .text_color(t.status_warning)
        .child("★")
        .into_any_element()
    } else {
        icon(IconName::Star)
            .with_size(p.s(if c.subagent { 13. } else { 16. }))
            .text_color(t.foreground_muted)
            .into_any_element()
    };
    let star = part(
        SharedString::from(format!("board-star-{}", run.id)),
        format!(
            "{} {}",
            if run.starred { "Unstar" } else { "Star" },
            run.title
        ),
        f.handle(FocusPart::Star),
    )
    .aria_selected(run.starred)
    .flex_none()
    .size(size)
    .flex()
    .items_center()
    .justify_center()
    .rounded(p.s(CONTROL_RADIUS))
    .when(!c.subagent, |star| {
        // Keeps the repo row one text line tall.
        star.my(-p.s(6.))
    })
    .when(ui.starring, |star| star.opacity(0.5))
    .when(quiet, |star| {
        star.opacity(0.35)
            .group_hover(group.clone(), |style| style.opacity(1.))
            .focus_visible(|style| style.opacity(1.))
    })
    .when(!ui.starring, |star| {
        p.press(
            star.hover(|style| style.bg(t.surface2)),
            CardAction::Star {
                run_id: run.id.clone(),
                starred: !run.starred,
            },
        )
    })
    .child(glyph)
    .when(f.ring(FocusPart::Star), |star| {
        star.child(focus_ring(t.accent, CONTROL_RADIUS * p.scale, c.subagent))
    });
    dim(star, CardPart::Star, c.seen, c.focused, group).into_any_element()
}

fn diffs_button(p: &Paint, c: &Card, ui: &CardUi, f: &Parts, group: &SharedString) -> AnyElement {
    let (run, t) = (c.run, &p.t);
    let off = run.diffs_off;
    let diffs = part(
        SharedString::from(format!("board-diffs-{}", run.id)),
        format!(
            "Turn edit diffs {} for {}",
            if off { "on" } else { "off" },
            run.title
        ),
        f.handle(FocusPart::Diffs),
    )
    .aria_selected(off)
    .flex_none()
    .size(p.s(28.))
    .my(-p.s(6.))
    .flex()
    .items_center()
    .justify_center()
    .rounded(p.s(CONTROL_RADIUS))
    // Off stays visible like a star; on shows with the star on hover.
    .map(|diffs| {
        if ui.toggling_diffs {
            diffs.opacity(0.5)
        } else if off {
            diffs
        } else {
            diffs
                .opacity(0.)
                .group_hover(group.clone(), |style| style.opacity(1.))
                .focus_visible(|style| style.opacity(1.))
        }
    })
    .when(!ui.toggling_diffs, |diffs| {
        p.press(
            diffs.hover(|style| style.bg(t.surface2)),
            CardAction::Diffs {
                agent_id: run.agent_id.clone(),
                off: !off,
            },
        )
    })
    .child(
        icon(if off {
            IconName::PencilOff
        } else {
            IconName::Pencil
        })
        .with_size(p.s(16.))
        .text_color(if off {
            t.foreground
        } else {
            t.foreground_muted
        }),
    )
    .when(f.ring(FocusPart::Diffs), |diffs| {
        diffs.child(focus_ring(t.accent, CONTROL_RADIUS * p.scale, false))
    });
    dim(diffs, CardPart::Diffs, c.seen, c.focused, group).into_any_element()
}

fn remove_button(p: &Paint, c: &Card, ui: &CardUi, f: &Parts, card: &SharedString) -> AnyElement {
    let (run, t) = (c.run, &p.t);
    let group: SharedString = format!("board-remove-{}", run.id).into();
    let base = part(
        group.clone(),
        remove_name(&run.title, c.remove_count),
        f.handle(FocusPart::Remove),
    )
    .group(group.clone())
    .flex_none()
    .rounded(p.s(CONTROL_RADIUS))
    .when(ui.removing, |remove| remove.opacity(0.5))
    .when(!ui.removing, |remove| {
        p.press(
            remove.hover(|style| style.bg(t.surface2)),
            CardAction::Remove {
                run_id: run.id.clone(),
                agent_id: run.agent_id.clone(),
            },
        )
    });
    let base = dim(base, CardPart::Remove, c.seen, c.focused, card)
        .when(f.ring(FocusPart::Remove), |remove| {
            remove.child(focus_ring(t.accent, CONTROL_RADIUS * p.scale, c.subagent))
        });
    if c.subagent {
        base.p(p.s(5.))
            .child(
                icon(IconName::X)
                    .with_size(p.s(14.))
                    .text_color(t.foreground_muted),
            )
            .into_any_element()
    } else {
        base.px(p.s(8.))
            .py(p.s(4.))
            .my(-p.s(4.))
            .mr(-p.s(8.))
            .child(
                p.text(12., 16.)
                    .font_weight(FontWeight::SEMIBOLD)
                    .text_color(t.foreground_muted)
                    .when(!ui.removing, |text| {
                        text.group_hover(group, |style| style.text_color(t.status_danger))
                    })
                    .child(remove_label(ui.removing, c.remove_count)),
            )
            .into_any_element()
    }
}

/// The project mark of the sidebar: a rounded square with the project's initial.
fn project_mark(name: &str, color: Hsla, size: Pixels, t: &Tokens) -> AnyElement {
    div()
        .flex_none()
        .size(size)
        .rounded(size * 0.3)
        .bg(color)
        .flex()
        .items_center()
        .justify_center()
        .child(
            div()
                .text_size(size * 0.55)
                .line_height(size * 0.7)
                .font_weight(FontWeight::BOLD)
                .text_color(t.surface1)
                .child(project_initial(name)),
        )
        .into_any_element()
}

fn model_tags(p: &Paint, run: &Run, compact: bool) -> Div {
    let t = &p.t;
    let tag = |name: IconName, label: String, color: Hsla| {
        h_flex()
            .items_center()
            .gap(p.s(4.))
            .min_w_0()
            .flex_shrink(1.)
            .when(!compact, |tag| {
                tag.px(p.s(7.))
                    .py(p.s(2.))
                    .border_1()
                    .border_color(if color == t.foreground {
                        t.border
                    } else {
                        color
                    })
                    .rounded(p.s(CONTROL_RADIUS))
                    .bg(t.surface2)
            })
            .child(
                icon(name)
                    .with_size(p.s(if compact { 12. } else { 14. }))
                    .text_color(color),
            )
            .child(
                p.text(
                    if compact { 11.5 } else { 12.5 },
                    if compact { 15. } else { 17. },
                )
                .min_w_0()
                .truncate()
                .font_weight(FontWeight::SEMIBOLD)
                .text_color(color)
                .child(label),
            )
    };
    h_flex()
        .items_center()
        .gap(p.s(if compact { 8. } else { 6. }))
        .min_w_0()
        .child(tag(
            IconName::Cpu,
            run.model.clone().unwrap_or_else(|| run.provider.clone()),
            t.foreground,
        ))
        .children(run.effort.clone().map(|effort| {
            let color = effort_color(&effort, t.surface2).unwrap_or(t.foreground_muted);
            tag(IconName::Brain, effort, color)
        }))
}

/// The Watchtower TASK a thread served; a link when Watchtower can open it.
fn task_chip(p: &Paint, run: &Run, f: &Parts, compact: bool) -> Option<AnyElement> {
    let task = run.task_id.clone()?;
    let t = &p.t;
    let chip = p
        .text(
            if compact { 9.5 } else { 10.5 },
            if compact { 13. } else { 15. },
        )
        .flex_none()
        .px(p.s(if compact { 4. } else { 6. }))
        .py(p.s(1.))
        .rounded(p.s(4.))
        .bg(t.foreground)
        .text_color(t.surface0)
        .font_weight(FontWeight::BOLD)
        .child(task.clone());
    let link = p.watchtower && run.workspace_id.is_some();
    Some(if link {
        div()
            .id(SharedString::from(format!("board-task-{}", run.id)))
            .role(Role::Link)
            .aria_label(format!("Open {task} in Watchtower"))
            .track_focus(f.handle(FocusPart::Task))
            .flex_none()
            .cursor_pointer()
            .active(|style| style.opacity(0.7))
            .on_click(p.act(CardAction::OpenTask(Box::new(run.clone()))))
            .child(chip)
            .when(f.ring(FocusPart::Task), |link| {
                link.child(focus_ring(t.accent, 4. * p.scale, false))
            })
            .into_any_element()
    } else {
        chip.into_any_element()
    })
}

#[allow(clippy::too_many_arguments)]
fn card_body(
    p: &Paint,
    c: &Card,
    ui: &CardUi,
    f: &Parts,
    group: &SharedString,
    star: AnyElement,
    remove: Option<AnyElement>,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let (run, t) = (c.run, &p.t);
    let running = run.running();
    let hue = if c.seen { None } else { p.hue(run) };
    let diffs = p.online.then(|| diffs_button(p, c, ui, f, group));

    let repo = part(
        SharedString::from(format!("board-repo-{}", run.id)),
        format!("New conversation in {}", run.project),
        f.handle(FocusPart::Repo),
    )
    .flex_shrink(1.)
    .min_w_0()
    .flex()
    .flex_row()
    .items_center()
    .gap(p.s(7.))
    .px(p.s(2.))
    .rounded(p.s(CONTROL_RADIUS))
    .active(|style| style.opacity(0.6))
    .hover(|style| style.underline())
    .on_click(p.act(CardAction::OpenProject(Box::new(run.clone()))))
    .child(project_mark(&run.project, repo_mark(hue, t), p.s(14.), t))
    .child(
        p.text(12., 16.)
            .min_w_0()
            .truncate()
            .font_weight(FontWeight::SEMIBOLD)
            .text_color(repo_label(hue, t))
            .child(run.project.clone()),
    )
    .when(f.ring(FocusPart::Repo), |repo| {
        repo.child(focus_ring(t.accent, CONTROL_RADIUS * p.scale, c.embedded))
    });
    let repo_row = h_flex()
        .items_center()
        .justify_between()
        .gap(p.s(8.))
        .mb(p.s(2.))
        .child(dim(
            div().flex_shrink(1.).min_w_0().child(repo),
            CardPart::Repo,
            c.seen,
            c.focused,
            group,
        ))
        .child(
            h_flex()
                .flex_none()
                .relative()
                .left(px(actions_offset(p.scale)))
                .items_center()
                .children(diffs)
                .child(star),
        );

    let thinking = running && !run.needs_input;
    let avatar = if thinking {
        running_avatar(&run.agent_id, p.s(36.), &p.orb, window, cx)
    } else {
        agent_avatar(&run.agent_id, p.s(36.))
    };
    let indicator = if run.needs_input {
        Some(pulse(
            SharedString::from(format!("board-needs-{}", run.id)),
            p.text(11.5, 15.)
                .px(p.s(7.))
                .py(p.s(1.))
                .rounded(p.s(CONTROL_RADIUS))
                .bg(t.status_warning)
                .text_color(t.surface1)
                .font_weight(FontWeight::BOLD)
                .child("Needs input"),
        ))
    } else if running {
        // The running avatar carries the spinner.
        None
    } else {
        Some(
            div()
                .flex_none()
                .size(p.s(6.))
                .rounded(p.s(3.))
                .bg(tone(run, t))
                .into_any_element(),
        )
    };
    let (status, rest) = card_meta(run, p.now);
    let meta = match status {
        Some(status) => meta_text(
            format!("{status}{rest}"),
            Some((status.len(), label_color(run, t), FontWeight::NORMAL)),
        ),
        None => meta_text(rest, None),
    };
    let main_row = dim(
        h_flex().items_start().gap(p.s(10.)).child(avatar).child(
            v_flex()
                .flex_1()
                .min_w_0()
                .gap(p.s(3.))
                .child(
                    p.text(14., 19.)
                        .line_clamp(2)
                        // `numberOfLines={2}` ends the cut on an ellipsis.
                        .text_ellipsis()
                        .font_weight(FontWeight::SEMIBOLD)
                        .text_color(t.foreground)
                        .child(run.title.clone()),
                )
                .child(
                    h_flex()
                        .min_w_0()
                        .items_center()
                        .gap(p.s(6.))
                        .children(indicator)
                        .child(
                            p.text(12., 16.)
                                .flex_1()
                                .min_w_0()
                                .truncate()
                                .font_features(tabular())
                                .text_color(t.foreground_muted)
                                .child(meta),
                        ),
                ),
        ),
        CardPart::Main,
        c.seen,
        c.focused,
        group,
    );

    let badges = h_flex()
        .items_center()
        .gap(p.s(8.))
        .pl(p.s(46.))
        .mt(p.s(2.))
        .min_h(p.s(23.))
        .children(task_chip(p, run, f, false))
        .child(dim(
            div()
                .flex_1()
                .min_w_0()
                .flex()
                .overflow_hidden()
                .child(model_tags(p, run, false)),
            CardPart::Tags,
            c.seen,
            c.focused,
            group,
        ))
        .children(remove);

    v_flex()
        .gap(p.s(6.))
        .child(repo_row)
        .child(main_row)
        .child(badges)
        .into_any_element()
}

fn subagent_body(
    p: &Paint,
    c: &Card,
    ui: &CardUi,
    f: &Parts,
    group: &SharedString,
    star: AnyElement,
    remove: Option<AnyElement>,
) -> AnyElement {
    let (run, t) = (c.run, &p.t);
    let running = run.running();
    let tone = tone(run, t);
    let status = if run.needs_input {
        pulse(
            SharedString::from(format!("board-needs-{}", run.id)),
            icon(IconName::CircleAlert)
                .with_size(p.s(12.))
                .text_color(tone),
        )
    } else if running {
        spinner(t.foreground_muted, p.s(12.))
    } else {
        icon(match run.status {
            RunStatus::Completed => IconName::CircleCheck,
            RunStatus::Failed => IconName::CircleX,
            _ => IconName::CircleHelp,
        })
        .with_size(p.s(12.))
        .text_color(tone)
        .into_any_element()
    };
    let label = card_status(run);
    let duration = run_duration(run, p.now)
        .map(|duration| format!("{SEPARATOR}{duration}"))
        .unwrap_or_default();
    let project = if c.inherited_project {
        String::new()
    } else {
        format!("{SEPARATOR}{}", run.project)
    };
    let meta = meta_text(
        format!("{label}{duration}{project}"),
        Some((label.len(), label_color(run, t), FontWeight::MEDIUM)),
    );
    // Hidden actions swap by opacity, never by display or visibility: a hidden element registers no
    // tab stop, so Tab could not reach them. `in_focus` cannot show them: it tests whether the
    // container is inside the focused element, not whether it holds the focused star.
    let pinned = run.starred || ui.starring || ui.removing || ui.star_error || ui.remove_error;
    let actions = h_flex()
        .absolute()
        .right(-p.s(2.))
        .top(p.s(2.))
        .rounded(p.s(CONTROL_RADIUS))
        .bg(t.surface1)
        .when(!subagent_actions_shown(pinned, f.focused), |actions| {
            actions
                .opacity(0.)
                .group_hover(group.clone(), |style| style.opacity(1.))
        })
        .child(star)
        .children(remove);

    v_flex()
        .gap(p.s(6.))
        .child(
            h_flex()
                .relative()
                .items_center()
                .gap(p.s(8.))
                .min_h(p.s(32.))
                // The elbow marks the row as nested under the card above.
                .child(dim(
                    div().flex_none().child(
                        icon(IconName::CornerDownRight)
                            .with_size(p.s(14.))
                            .text_color(t.foreground_muted),
                    ),
                    CardPart::Elbow,
                    c.seen,
                    c.focused,
                    group,
                ))
                .child(dim(
                    div()
                        .flex_none()
                        .child(agent_avatar(&run.agent_id, p.s(24.))),
                    CardPart::Main,
                    c.seen,
                    c.focused,
                    group,
                ))
                .child(dim(
                    v_flex()
                        .flex_1()
                        .min_w_0()
                        .gap(p.s(1.))
                        .child(
                            p.text(13., 17.)
                                .truncate()
                                .font_weight(FontWeight::SEMIBOLD)
                                .text_color(t.foreground)
                                .child(run.title.clone()),
                        )
                        .child(
                            h_flex()
                                .items_center()
                                .gap(p.s(4.))
                                .min_w_0()
                                .child(status)
                                .child(
                                    p.text(11.5, 15.)
                                        .flex_1()
                                        .min_w_0()
                                        .truncate()
                                        .font_features(tabular())
                                        .text_color(t.foreground_muted)
                                        .child(meta),
                                ),
                        ),
                    CardPart::Main,
                    c.seen,
                    c.focused,
                    group,
                ))
                .child(actions),
        )
        .child(
            h_flex()
                .items_center()
                .gap(p.s(8.))
                .pl(p.s(54.))
                .mt(p.s(2.))
                .children(task_chip(p, run, f, true))
                .child(dim(
                    div()
                        .flex_1()
                        .min_w_0()
                        .flex()
                        .overflow_hidden()
                        .child(model_tags(p, run, true)),
                    CardPart::Tags,
                    c.seen,
                    c.focused,
                    group,
                )),
        )
        .into_any_element()
}

/// The row that collapses or expands a cluster's subagents.
fn toggle_row(p: &Paint, tree: &RunTree, nested: bool, seen: bool) -> AnyElement {
    let (run, t) = (&tree.run, &p.t);
    let expanded = !p.collapsed.contains(&run.agent_id);
    let group: SharedString = format!("board-toggle-{}", run.id).into();
    let (running, needs_input) = cluster_summary(tree);
    let summary = (!expanded)
        .then(|| summary_label(running, needs_input))
        .flatten()
        .map(|label| {
            let color = if needs_input > 0 {
                t.status_warning
            } else {
                t.accent
            };
            h_flex()
                .flex_none()
                .items_center()
                .gap(p.s(6.))
                .child(if needs_input > 0 {
                    pulse(
                        SharedString::from(format!("board-summary-{}", run.id)),
                        div().size(p.s(6.)).rounded(p.s(3.)).bg(color),
                    )
                } else {
                    spinner(color, p.s(12.))
                })
                .child(
                    p.text(12., 16.)
                        .font_weight(FontWeight::MEDIUM)
                        .text_color(if needs_input > 0 {
                            color
                        } else {
                            t.foreground_muted
                        })
                        .child(label),
                )
        });
    let hue = if seen { None } else { p.hue(run) };
    button(
        group.clone(),
        format!(
            "{} subagents of {}",
            if expanded { "Collapse" } else { "Expand" },
            run.title
        ),
        t.accent,
        true,
    )
    .tab_stop(true)
    .aria_expanded(expanded)
    .aria_description(if expanded {
        "Hides the subagent cards"
    } else {
        "Shows the subagent cards"
    })
    .group(group.clone())
    .flex()
    .flex_row()
    .items_center()
    .gap(p.s(8.))
    .min_h(px(32.))
    .py(p.s(6.))
    .px(p.s(if nested { 6. } else { 12. }))
    .map(|row| {
        if nested {
            row.rounded(p.s(CONTROL_RADIUS))
        } else {
            row.border_t_1().border_color(repo_divider(hue, t))
        }
    })
    .hover(|style| style.bg(t.surface2))
    .active(|style| style.opacity(0.7))
    .on_click(p.act(CardAction::Toggle(run.agent_id.clone())))
    .child(
        div().flex_none().w(p.s(12.)).flex().justify_center().child(
            icon(if expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
            .with_size(p.s(12.))
            .text_color(t.foreground_muted),
        ),
    )
    .child(
        p.text(12., 16.)
            .flex_grow(1.)
            .font_weight(FontWeight::SEMIBOLD)
            .font_features(tabular())
            .text_color(t.foreground_muted)
            .group_hover(group, |style| style.text_color(t.foreground))
            .child(subagent_count(tree.count)),
    )
    .children(summary)
    .into_any_element()
}

/// A top-level card: a lone card, or a cluster's head with its root card and toggle row.
/// An expanded head leaves the bottom of the card to its last child row.
pub(crate) fn column_card(
    p: &Paint,
    tree: &RunTree,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let seen = crate::model::tree::cluster_seen(tree);
    let can_remove = p.online && !tree.running;
    if tree.children.is_empty() {
        return run_card(
            p,
            Card {
                run: &tree.run,
                seen,
                embedded: false,
                subagent: false,
                inherited_project: false,
                remove_count: 1,
                can_remove,
                focused: false,
            },
            window,
            cx,
        );
    }
    let expanded = !p.collapsed.contains(&tree.run.agent_id);
    let (background, border) = repo_surface(if seen { None } else { p.hue(&tree.run) }, &p.t);
    v_flex()
        .w_full()
        .overflow_hidden()
        .bg(background)
        .border_color(border)
        .border_t_1()
        .border_l_1()
        .border_r_1()
        .rounded_t(p.s(CARD_RADIUS))
        .when(!expanded, |head| {
            head.border_b_1().rounded_b(p.s(CARD_RADIUS))
        })
        .child(run_card(
            p,
            Card {
                run: &tree.run,
                seen,
                embedded: true,
                subagent: false,
                inherited_project: false,
                remove_count: tree.count,
                can_remove,
                focused: false,
            },
            window,
            cx,
        ))
        .child(toggle_row(p, tree, false, seen))
        .into_any_element()
}

/// One row of an expanded cluster's panel: one or two subagent cells.
#[allow(clippy::too_many_arguments)]
pub(crate) fn children_row(
    p: &Paint,
    tree: &RunTree,
    cells: &[usize],
    first: bool,
    last: bool,
    panel_width: f32,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let seen = crate::model::tree::cluster_seen(tree);
    let (background, border) = repo_surface(if seen { None } else { p.hue(&tree.run) }, &p.t);
    let edge = p.s(PANEL_EDGE);
    let paired = two_columns(tree.children.len(), panel_width, p.scale);
    let cell_width = if paired {
        panel_width / 2.
    } else {
        panel_width
    };
    v_flex()
        .w_full()
        .bg(background)
        .border_color(border)
        .border_l_1()
        .border_r_1()
        .px(edge)
        .when(!first, |row| row.pt(p.s(PANEL_GAP)))
        .when(last, |row| {
            row.border_b_1().rounded_b(p.s(CARD_RADIUS)).pb(edge)
        })
        .child(
            h_flex()
                .w_full()
                .items_start()
                .children(cells.iter().enumerate().map(|(column, &child)| {
                    let child = &tree.children[child];
                    div()
                        .min_w_0()
                        .map(|cell| {
                            if paired {
                                cell.w(relative(0.5))
                            } else {
                                cell.w_full()
                            }
                        })
                        .when(paired && column == 1, |cell| cell.pl(edge / 2.))
                        .when(paired && column == 0, |cell| cell.pr(edge / 2.))
                        .child(nested_cluster(
                            p, child, &tree.run, seen, cell_width, window, cx,
                        ))
                })),
        )
        .into_any_element()
}

/// A subagent and its own subagents, drawn inside one cell.
fn nested_cluster(
    p: &Paint,
    tree: &RunTree,
    parent: &Run,
    seen: bool,
    width: f32,
    window: &mut Window,
    cx: &mut App,
) -> AnyElement {
    let card = Card {
        run: &tree.run,
        seen,
        embedded: !tree.children.is_empty(),
        subagent: true,
        inherited_project: tree.run.project_key == parent.project_key,
        remove_count: tree.count,
        can_remove: p.online && !tree.running,
        focused: false,
    };
    if tree.children.is_empty() {
        return run_card(p, card, window, cx);
    }
    let expanded = !p.collapsed.contains(&tree.run.agent_id);
    let edge = PANEL_EDGE * p.scale;
    let panel_width = width - NESTED_INDENT * p.scale - 1. - 2. * edge;
    let paired = two_columns(tree.children.len(), panel_width, p.scale);
    v_flex()
        .w_full()
        .child(run_card(p, card, window, cx))
        .child(toggle_row(p, tree, true, seen))
        .when(expanded, |cluster| {
            cluster.child(
                h_flex()
                    .flex_wrap()
                    .items_start()
                    .px(px(edge))
                    .ml(p.s(NESTED_INDENT))
                    .border_l_1()
                    .border_color(p.t.border)
                    .children(tree.children.iter().enumerate().map(|(index, child)| {
                        div()
                            .min_w_0()
                            // The row gap: under every cell but those of the first row.
                            .when(index >= if paired { 2 } else { 1 }, |cell| {
                                cell.pt(p.s(PANEL_GAP))
                            })
                            .map(|cell| {
                                if paired {
                                    cell.w(relative(0.5))
                                } else {
                                    cell.w_full()
                                }
                            })
                            .when(paired && index % 2 == 1, |cell| cell.pl(px(edge / 2.)))
                            .when(paired && index % 2 == 0, |cell| cell.pr(px(edge / 2.)))
                            .child(nested_cluster(
                                p,
                                child,
                                &tree.run,
                                seen,
                                if paired {
                                    panel_width / 2.
                                } else {
                                    panel_width
                                },
                                window,
                                cx,
                            ))
                    })),
            )
        })
        .into_any_element()
}

#[cfg(test)]
mod tests {
    // Not `super::*`: the gpui glob import brings a `test` attribute of its own.
    use super::{
        CardPart, FocusPart, actions_offset, card_active, part_opacity, pulse_opacity, ring_box,
        subagent_actions_shown,
    };

    #[test]
    fn a_pulse_fades_to_0_35_and_back_in_one_loop() {
        assert_eq!(pulse_opacity(0.), 1.);
        assert!((pulse_opacity(0.5) - 0.35).abs() < 1e-6);
        assert!((pulse_opacity(1.) - 1.).abs() < 1e-6);
        assert!(pulse_opacity(0.25) < 1. && pulse_opacity(0.25) > 0.35);
        assert!((pulse_opacity(0.25) - pulse_opacity(0.75)).abs() < 1e-6);
    }

    #[test]
    fn the_repo_actions_move_6_px_into_the_padding_and_scale_with_the_board() {
        assert_eq!(actions_offset(1.), 6.);
        assert!((actions_offset(1.6) - 9.6).abs() < 1e-6);
        // Inside the 12 px card padding at every scale, so the star is never clipped.
        assert!(actions_offset(1.6) < 12. * 1.6);
    }

    #[test]
    fn a_seen_card_dims_only_the_repo_main_row_tags_and_elbow() {
        assert_eq!(part_opacity(CardPart::Repo, true, false), 0.5);
        assert_eq!(part_opacity(CardPart::Main, true, false), 0.5);
        assert_eq!(part_opacity(CardPart::Tags, true, false), 0.5);
        assert!((part_opacity(CardPart::Elbow, true, false) - 0.3).abs() < 1e-6);
        for part in [CardPart::Star, CardPart::Diffs, CardPart::Remove] {
            assert_eq!(part_opacity(part, true, false), 1., "{part:?}");
        }
    }

    #[test]
    fn hover_and_unseen_cards_keep_full_contrast_but_the_elbow_stays_at_0_6() {
        for (seen, active) in [(true, true), (false, false), (false, true)] {
            for part in [
                CardPart::Repo,
                CardPart::Main,
                CardPart::Tags,
                CardPart::Star,
                CardPart::Diffs,
                CardPart::Remove,
            ] {
                assert_eq!(part_opacity(part, seen, active), 1., "{part:?}");
            }
            assert!((part_opacity(CardPart::Elbow, seen, active) - 0.6).abs() < 1e-6);
        }
    }

    #[test]
    fn card_active_on_hover_or_keyboard_focus_but_not_mouse_focus() {
        assert!(!card_active(false, false, false));
        assert!(!card_active(false, false, true));
        assert!(card_active(true, false, false));
        assert!(card_active(true, true, false));
        // A click focuses a part too, but only keyboard focus un-dims (`focus?.keyboard`).
        assert!(!card_active(false, true, false));
        assert!(card_active(false, true, true));
        // The seen card's dimmed parts show at full opacity while focus is from the keyboard.
        let active = card_active(false, true, true);
        for part in [CardPart::Repo, CardPart::Main, CardPart::Tags] {
            assert_eq!(part_opacity(part, true, active), 1., "{part:?}");
        }
        assert_eq!(
            part_opacity(CardPart::Main, true, card_active(false, true, false)),
            0.5
        );
    }

    #[test]
    fn the_focus_ring_sits_1_px_outside_the_part_or_inside_a_clipping_surface() {
        // Electron's 2 px outline at offset 1: the ring's outer edge is 3 px out.
        assert_eq!(ring_box(6., false), (3., 9.));
        // Offset -2: the ring covers the part's outer 2 px and keeps its radius.
        assert_eq!(ring_box(6., true), (0., 6.));
        // A square part keeps square corners.
        assert_eq!(ring_box(0., false), (3., 0.));
        assert_eq!(ring_box(0., true), (0., 0.));
    }

    #[test]
    fn focus_on_the_star_or_remove_shows_a_subagent_rows_actions() {
        assert!(!subagent_actions_shown(false, None));
        assert!(subagent_actions_shown(true, None));
        assert!(subagent_actions_shown(false, Some(FocusPart::Star)));
        assert!(subagent_actions_shown(false, Some(FocusPart::Remove)));
        for part in [
            FocusPart::Open,
            FocusPart::Repo,
            FocusPart::Diffs,
            FocusPart::Task,
        ] {
            assert!(!subagent_actions_shown(false, Some(part)), "{part:?}");
        }
    }
}
