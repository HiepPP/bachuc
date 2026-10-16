//! Tab widths for one tab bar, a port of `computeWorkspaceTabLayout` in
//! `packages/app/src/screens/workspace/workspace-tab-layout.ts` with the chip geometry of
//! `workspace-desktop-tabs-row.tsx` (lines 102-111).

pub const TAB_MIN_WIDTH: f32 = 96.;
pub const TAB_MAX_WIDTH: f32 = 160.;
pub const TAB_CHIP_PADDING: f32 = 8.;
pub const TAB_CHIP_GAP: f32 = 4.;
pub const TAB_ROW_PADDING: f32 = 4.;
pub const TAB_ICON_WIDTH: f32 = 14.;
pub const TAB_CONTENT_GAP: f32 = 4.;
pub const TAB_MODIFIED_DOT: f32 = 8.;
pub const TAB_DROP_INDICATOR_WIDTH: f32 = 4.;
const TAB_LABEL_LAYOUT_ALLOWANCE: f32 = 4.;
/// The inline new-tab button: a `CONTROL_XS` square in a `SPACE_1` padded slot.
pub const INLINE_NEW_TAB_RESERVED: f32 = 36.;
/// The small icon button frame of the toolbar (`SMALL_ICON_BUTTON_SIZE`).
pub const TOOLBAR_BUTTON: f32 = 20.;
/// `PANE_SPLIT_ACTIONS_HORIZONTAL_PADDING` and `PANE_SPLIT_ACTIONS_OUTER_MARGIN`.
pub const SPLIT_ACTIONS_PADDING: f32 = 2.;
pub const SPLIT_ACTIONS_MARGIN: f32 = 3.;
pub const SPLIT_ACTIONS_RESERVED: f32 =
    TOOLBAR_BUTTON + SPLIT_ACTIONS_PADDING * 2. + SPLIT_ACTIONS_MARGIN;

/// The chip width a label of `label_width` asks for before the row clamps it.
pub fn label_allowance(label_width: f32, modified: bool) -> f32 {
    let dot = if modified {
        TAB_CONTENT_GAP + TAB_MODIFIED_DOT
    } else {
        0.
    };
    label_width + TAB_LABEL_LAYOUT_ALLOWANCE + dot
}

#[derive(Clone, Debug, PartialEq)]
pub struct TabLayout {
    pub widths: Vec<f32>,
    /// The tabs do not fit at the min width: they take the min width and the row scrolls, and
    /// the new-tab button moves to the toolbar.
    pub scrolls: bool,
}

/// Widths for tabs whose labels need `label_widths` (from [`label_allowance`]) in a row of
/// `row_width`. Tabs take their natural width up to the max, shrink toward the min in proportion
/// to their room above it, then stop at the min and scroll.
pub fn compute(row_width: f32, label_widths: &[f32]) -> TabLayout {
    let count = label_widths.len();
    if count == 0 {
        return TabLayout {
            widths: Vec::new(),
            scrolls: false,
        };
    }
    let available = (row_width - INLINE_NEW_TAB_RESERVED - SPLIT_ACTIONS_RESERVED).max(0.);
    let overhead = TAB_ROW_PADDING * 2. + (count - 1) as f32 * TAB_CHIP_GAP;
    let available = (available - overhead).max(0.);
    let chrome = TAB_ICON_WIDTH + TAB_CONTENT_GAP + TAB_CHIP_PADDING * 2.;
    let natural: Vec<f32> = label_widths
        .iter()
        .map(|label| (chrome + label).clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH))
        .collect();
    let natural_total: f32 = natural.iter().sum();
    let min_total = TAB_MIN_WIDTH * count as f32;
    let scrolls = available < min_total;
    let widths = if scrolls {
        vec![TAB_MIN_WIDTH; count]
    } else if natural_total > available {
        let ratio = (natural_total - available) / (natural_total - min_total);
        natural
            .iter()
            .map(|width| width - (width - TAB_MIN_WIDTH) * ratio)
            .collect()
    } else {
        natural
    };
    TabLayout {
        widths: widths
            .into_iter()
            .map(|width| width.clamp(TAB_MIN_WIDTH, TAB_MAX_WIDTH).round())
            .collect(),
        scrolls,
    }
}

/// The insertion slot for a drop at `x`, measured from the row's left edge with the scroll
/// offset added: the slot before the first tab whose center lies right of `x`.
pub fn drop_slot(widths: &[f32], x: f32) -> usize {
    let mut left = TAB_ROW_PADDING;
    for (index, width) in widths.iter().enumerate() {
        let slot = width + TAB_CHIP_GAP;
        if x < left + slot / 2. {
            return index;
        }
        left += slot;
    }
    widths.len()
}
