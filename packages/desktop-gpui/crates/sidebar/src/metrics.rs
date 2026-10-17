//! Row heights of the sidebar chrome, from `sidebar-header-row.tsx` and `left-sidebar.tsx`.
//! GPUI's default line height is 1.618, which made each label row taller than its Electron
//! `minHeight`, so the labels pin `LABEL_LINE`.

/// The label line of the nav rows and the Add project button.
pub(crate) const LABEL_LINE: f32 = 20.;
/// `button.minHeight` of the compact header row: Search and the plugin nav rows.
pub(crate) const NAV_ROW_MIN: f32 = 28.;
/// `footerAddProjectButton.minHeight`.
pub(crate) const ADD_PROJECT_MIN: f32 = 32.;

#[cfg(test)]
mod tests {
    use super::{ADD_PROJECT_MIN, LABEL_LINE, NAV_ROW_MIN};
    use bachuc_theme::metrics::{SPACE_0_5, SPACE_1, SPACE_1_5, SPACE_3};

    /// A row's height: its `minHeight`, or the label line and vertical padding when taller.
    fn row_height(min: f32, padding_y: f32) -> f32 {
        min.max(LABEL_LINE + 2. * padding_y)
    }

    #[test]
    fn nav_rows_are_30_px_apart() {
        // A 28 px row (`paddingVertical` 4) and the group's 2 px gap.
        assert_eq!(row_height(NAV_ROW_MIN, SPACE_1) + SPACE_0_5, 30.);
    }

    #[test]
    fn the_footer_is_57_px_tall() {
        // A 1 px top border, `paddingVertical` 12, and the 32 px Add project button.
        assert_eq!(
            1. + 2. * SPACE_3 + row_height(ADD_PROJECT_MIN, SPACE_1_5),
            57.
        );
    }
}
