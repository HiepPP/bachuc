//! Pane size to cell grid, and a pane position to a cell. All values are in logical pixels.

/// Space between the pane edge and the first cell, on every side.
pub const PADDING: f32 = 4.0;

/// Rows and columns that fit a pane of `width` x `height`. The result is at least 1 x 1.
pub fn grid_size(width: f32, height: f32, cell_width: f32, cell_height: f32) -> (u16, u16) {
    let fit = |space: f32, cell: f32| {
        if cell <= 0.0 {
            return 1;
        }
        ((space - 2.0 * PADDING).max(0.0) / cell)
            .floor()
            .clamp(1.0, f32::from(u16::MAX)) as u16
    };
    (fit(height, cell_height), fit(width, cell_width))
}

/// The cell under a position relative to the pane's top left corner, clamped to the grid. The
/// last value tells whether the position is in the right half of the cell, which decides which
/// side of the cell a selection edge sits on.
pub fn cell_at(
    x: f32,
    y: f32,
    cell_width: f32,
    cell_height: f32,
    rows: usize,
    cols: usize,
) -> (usize, usize, bool) {
    let column = (x - PADDING) / cell_width;
    let row = (y - PADDING) / cell_height;
    let clamp =
        |value: f32, limit: usize| (value.floor().max(0.0) as usize).min(limit.saturating_sub(1));
    let right = if column < 0.0 {
        false
    } else if column >= cols as f32 {
        true
    } else {
        column.fract() >= 0.5
    };
    (clamp(row, rows), clamp(column, cols), right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn the_grid_fills_the_pane_without_the_padding() {
        // 8 px of padding leaves 792 x 392; 8 px cells and 16 px lines give 99 x 24 cells.
        assert_eq!(grid_size(800.0, 400.0, 8.0, 16.0), (24, 99));
        assert_eq!(grid_size(808.0, 400.0, 8.0, 16.0), (24, 100));
    }

    #[test]
    fn a_wider_pane_gives_more_columns() {
        let (_, narrow) = grid_size(600.0, 400.0, 7.8, 16.0);
        let (_, wide) = grid_size(900.0, 400.0, 7.8, 16.0);
        assert!(wide > narrow);
    }

    #[test]
    fn a_tiny_or_empty_pane_keeps_one_cell() {
        assert_eq!(grid_size(0.0, 0.0, 8.0, 16.0), (1, 1));
        assert_eq!(grid_size(5.0, 5.0, 8.0, 16.0), (1, 1));
        assert_eq!(grid_size(100.0, 100.0, 0.0, 0.0), (1, 1));
    }

    #[test]
    fn a_position_maps_to_a_clamped_cell_and_side() {
        assert_eq!(cell_at(4.0, 4.0, 8.0, 16.0, 24, 80), (0, 0, false));
        assert_eq!(
            cell_at(4.0 + 8.0 * 3.0 + 5.0, 4.0 + 16.0 * 2.0, 8.0, 16.0, 24, 80),
            (2, 3, true)
        );
        assert_eq!(cell_at(-50.0, -50.0, 8.0, 16.0, 24, 80), (0, 0, false));
        assert_eq!(cell_at(9999.0, 9999.0, 8.0, 16.0, 24, 80), (23, 79, true));
    }
}
