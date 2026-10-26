//! The width rule of a timeline row's content column.

use bachuc_theme::metrics::{MAX_CONTENT_WIDTH, SPACE_2, SPACE_3};
use bachuc_timeline::{column_right_inset, content_column_width, under_corner_pills};

#[test]
fn a_narrow_row_gives_text_the_row_less_its_padding() {
    // A 700 px window with a 320 px sidebar leaves the timeline about 380 px.
    assert_eq!(
        content_column_width(380., MAX_CONTENT_WIDTH, 1.),
        380. - 32. - 16.
    );
    assert_eq!(
        content_column_width(700., MAX_CONTENT_WIDTH, 1.),
        700. - 32. - 16.
    );
}

#[test]
fn a_wide_row_stops_the_column_at_the_content_width() {
    assert_eq!(
        content_column_width(1600., MAX_CONTENT_WIDTH, 1.),
        MAX_CONTENT_WIDTH - 16.
    );
    assert_eq!(
        content_column_width(MAX_CONTENT_WIDTH + 32., MAX_CONTENT_WIDTH, 1.),
        MAX_CONTENT_WIDTH - 16.
    );
}

#[test]
fn the_text_width_never_passes_the_row_or_goes_below_zero() {
    for row in [0., 10., 32., 47., 48., 200., 851., 852., 853., 5000.] {
        let text = content_column_width(row, MAX_CONTENT_WIDTH, 1.);
        assert!(text >= 0., "{row} gives {text}");
        assert!(text <= row, "{row} gives {text}");
        assert!(text <= MAX_CONTENT_WIDTH, "{row} gives {text}");
    }
    assert_eq!(content_column_width(20., MAX_CONTENT_WIDTH, 1.), 0.);
}

/// The right edge of the text in a `row` wide row once it has `inset` extra right padding.
fn text_right(row: f32, inset: f32) -> f32 {
    let column = (row - 32.).clamp(0., MAX_CONTENT_WIDTH);
    (row + column) / 2. - SPACE_2 - inset
}

#[test]
fn no_pills_leave_the_column_alone() {
    for row in [0., 380., 900., 1200., 1600.] {
        assert_eq!(
            column_right_inset(row, 0., MAX_CONTENT_WIDTH, 1.),
            0.,
            "{row}"
        );
    }
}

#[test]
fn a_wide_row_already_clears_the_pills() {
    assert_eq!(column_right_inset(1600., 170., MAX_CONTENT_WIDTH, 1.), 0.);
    assert_eq!(column_right_inset(1400., 170., MAX_CONTENT_WIDTH, 1.), 0.);
}

#[test]
fn the_text_stops_left_of_the_pills_plus_the_clearance() {
    // 900 wide: the 820 column leaves a 40 px margin, which 170 px of pills do not fit in.
    let inset = column_right_inset(900., 170., MAX_CONTENT_WIDTH, 1.);
    assert!(inset > 0.);
    let pills_left = 900. - SPACE_3 - 170.;
    assert_eq!(text_right(900., inset), pills_left - SPACE_3);
    // A wider row needs less.
    assert!(column_right_inset(1000., 170., MAX_CONTENT_WIDTH, 1.) < inset);
}

#[test]
fn a_row_too_narrow_keeps_a_readable_text_width() {
    for zoom in [1., 1.5, 2.] {
        for row in [0., 20., 100., 250., 300., 400., 600.] {
            let inset = column_right_inset(row, 170. * zoom, MAX_CONTENT_WIDTH, zoom);
            let text = content_column_width(row, MAX_CONTENT_WIDTH, zoom);
            assert!(inset >= 0.);
            assert!(
                text - inset >= text.min(320. * zoom),
                "{row} at {zoom} gives {inset}"
            );
        }
    }
    // Too narrow for a gutter: the pills cover the line ends instead.
    assert_eq!(column_right_inset(200., 170., MAX_CONTENT_WIDTH, 1.), 0.);
}

#[test]
fn the_column_and_its_paddings_scale_with_the_zoom() {
    // At 150% the 820 column is 1230, and each 8 px column padding is 12.
    assert_eq!(
        content_column_width(2000., MAX_CONTENT_WIDTH, 1.5),
        1.5 * MAX_CONTENT_WIDTH - 24.
    );
    assert_eq!(
        content_column_width(380., MAX_CONTENT_WIDTH, 1.5),
        380. - 48. - 24.
    );
    // The text edge keeps `SPACE_3` of clearance and `SPACE_3` of corner offset, both zoomed.
    let zoom = 1.5;
    let (row, pills) = (1400., 255.);
    let inset = column_right_inset(row, pills, MAX_CONTENT_WIDTH, zoom);
    assert!(inset > 0.);
    let column = (row - 32. * zoom).clamp(0., MAX_CONTENT_WIDTH * zoom);
    let text_right = (row + column) / 2. - SPACE_2 * zoom - inset;
    assert_eq!(text_right, row - SPACE_3 * 2. * zoom - pills);
}

#[test]
fn a_narrower_reading_width_caps_the_column() {
    // claude-look narrows the column to Claude desktop's 768.
    assert_eq!(content_column_width(1600., 768., 1.), 768. - 16.);
    assert_eq!(content_column_width(1600., 768., 1.5), 1.5 * 768. - 24.);
    assert_eq!(column_right_inset(1600., 170., 768., 1.), 0.);
}

#[test]
fn only_rows_that_start_above_the_pills_bottom_are_under_the_pills() {
    // The list starts 48 px down; the pills end 12 + 100 = 112 px down.
    let (top, bottom) = (48., 112.);
    assert!(under_corner_pills(&[], 0, top, bottom));
    assert!(under_corner_pills(&[Some(40.)], 1, top, bottom));
    assert!(!under_corner_pills(&[Some(40.), Some(30.)], 2, top, bottom));
    // The answer is the same whatever the scroll position: it reads only content heights.
    assert!(!under_corner_pills(&[Some(400.)], 1, top, bottom));
}

#[test]
fn no_pills_or_an_unpainted_row_above_puts_no_row_under_the_pills() {
    assert!(!under_corner_pills(&[], 0, 48., 0.));
    assert!(!under_corner_pills(&[None], 1, 48., 112.));
    // Heights not recorded yet for the rows above.
    assert!(!under_corner_pills(&[], 3, 48., 112.));
}
