//! `WorkspaceTabIcon` (`screens/workspace/workspace-tab-presentation.tsx`): the state badge on
//! the bottom-right corner of an agent or subagent icon, shared by subagent rows and tabs.
//!
//! The badge is its own view, created once per row or tab key, so the running ring keeps its
//! clock and only the badge redraws on each turn.

use crate::model::StatusBucket;
use crate::ring::{FRAME, Knockout, RunningRing};
use bachuc_icons::{IconName, icon};
use bachuc_theme::{Tokens, tokens, zoomed};
use gpui_kit::component::Sizable as _;
use gpui_kit::*;

/// `DEFAULT_STATUS_DOT_OFFSET`, `STATUS_ALERT_OFFSET`, `STATUS_INDICATOR_DOT_SIZE`, and
/// `STATUS_INDICATOR_ALERT_SIZE`.
const DOT_OFFSET: f32 = -2.;
const ALERT_OFFSET: f32 = -3.;
const DOT: f32 = 8.;
const ALERT: f32 = 12.;
/// `getStatusRingOffset(DEFAULT_STATUS_DOT_OFFSET, STATUS_INDICATOR_DOT_SIZE)`: the frame grows
/// around the dot's centre, so the ring takes the dot's corner offset back by half the growth.
const RING_OFFSET: f32 = DOT_OFFSET - (FRAME - DOT) / 2.;

/// What a bucket draws on the icon corner.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BadgeKind {
    None,
    Ring,
    Dot,
    Alert,
}

/// The bucket-to-badge map of `WorkspaceTabIcon`. Pure, so the rule has a test.
pub fn badge_kind(bucket: StatusBucket) -> BadgeKind {
    match bucket {
        StatusBucket::Running => BadgeKind::Ring,
        StatusBucket::Failed | StatusBucket::Attention => BadgeKind::Dot,
        StatusBucket::NeedsInput => BadgeKind::Alert,
        StatusBucket::Done => BadgeKind::None,
    }
}

/// The surface the badge knocks out of. The dot's 1 px border uses it too, so the dot reads as a
/// hole in the icon like the ring's frame does.
fn fill(knockout: Knockout, t: &Tokens) -> Hsla {
    match knockout {
        Knockout::None | Knockout::Surface0 => t.surface0,
        Knockout::Sidebar => t.surface_sidebar,
        Knockout::Surface1 => t.surface1,
        Knockout::Surface2 => t.surface2,
    }
}

/// The badge of one row or tab. It sits in a `relative()` box the size of the icon. The ring is
/// created on the first running bucket and dropped when the agent stops, so a resting badge
/// costs no timer.
pub struct StatusBadge {
    bucket: StatusBucket,
    knockout: Knockout,
    ring: Option<Entity<RunningRing>>,
}

impl StatusBadge {
    pub fn new(bucket: StatusBucket, knockout: Knockout, cx: &mut Context<Self>) -> Self {
        let ring = (badge_kind(bucket) == BadgeKind::Ring)
            .then(|| cx.new(|cx| RunningRing::new(knockout, cx)));
        Self {
            bucket,
            knockout,
            ring,
        }
    }

    pub fn bucket(&self) -> StatusBucket {
        self.bucket
    }

    /// Follows the agent's bucket. The ring is created when the agent starts and dropped when it
    /// stops; a bucket it keeps does nothing.
    pub fn set_bucket(&mut self, bucket: StatusBucket, cx: &mut Context<Self>) {
        if self.bucket == bucket {
            return;
        }
        self.bucket = bucket;
        match badge_kind(bucket) {
            BadgeKind::Ring if self.ring.is_none() => {
                self.ring = Some(cx.new(|cx| RunningRing::new(self.knockout, cx)));
            }
            BadgeKind::Ring => {}
            _ => self.ring = None,
        }
        cx.notify();
    }
}

impl Render for StatusBadge {
    fn render(&mut self, _: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        let kind = badge_kind(self.bucket);
        if kind == BadgeKind::None {
            return div().into_any_element();
        }
        // Every mark sits in the ring's frame, which takes `RING_OFFSET` from the icon corner.
        // The dot and the alert keep their own Electron offsets inside that frame.
        div()
            .absolute()
            .right(zoomed(RING_OFFSET, cx))
            .bottom(zoomed(RING_OFFSET, cx))
            .size(zoomed(FRAME, cx))
            .children(match kind {
                BadgeKind::Ring => self.ring.clone().map(IntoElement::into_any_element),
                BadgeKind::Dot => {
                    let t = tokens(cx);
                    let color = if self.bucket == StatusBucket::Failed {
                        t.dot_danger
                    } else {
                        t.dot_success
                    };
                    Some(
                        div()
                            .absolute()
                            .right(zoomed(DOT_OFFSET - RING_OFFSET, cx))
                            .bottom(zoomed(DOT_OFFSET - RING_OFFSET, cx))
                            .size(zoomed(DOT, cx))
                            .rounded_full()
                            .bg(color)
                            .border(zoomed(1., cx))
                            .border_color(fill(self.knockout, &t))
                            .into_any_element(),
                    )
                }
                BadgeKind::Alert => Some(
                    div()
                        .absolute()
                        .right(zoomed(ALERT_OFFSET - RING_OFFSET, cx))
                        .bottom(zoomed(ALERT_OFFSET - RING_OFFSET, cx))
                        .child(
                            icon(IconName::CircleAlert)
                                .with_size(zoomed(ALERT, cx))
                                .text_color(tokens(cx).dot_warning),
                        )
                        .into_any_element(),
                ),
                BadgeKind::None => None,
            })
            .into_any_element()
    }
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{BadgeKind, badge_kind};
    use crate::model::StatusBucket;

    #[test]
    fn every_bucket_draws_its_badge() {
        assert_eq!(badge_kind(StatusBucket::Running), BadgeKind::Ring);
        assert_eq!(badge_kind(StatusBucket::Failed), BadgeKind::Dot);
        assert_eq!(badge_kind(StatusBucket::Attention), BadgeKind::Dot);
        assert_eq!(badge_kind(StatusBucket::NeedsInput), BadgeKind::Alert);
        assert_eq!(badge_kind(StatusBucket::Done), BadgeKind::None);
    }
}
