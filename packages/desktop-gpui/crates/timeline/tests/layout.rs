//! The width rule of a timeline row's content column.

use bachuc_theme::metrics::MAX_CONTENT_WIDTH;
use bachuc_timeline::content_column_width;

#[test]
fn a_narrow_row_gives_text_the_row_less_its_padding() {
    // A 700 px window with a 320 px sidebar leaves the timeline about 380 px.
    assert_eq!(content_column_width(380.), 380. - 32. - 16.);
    assert_eq!(content_column_width(700.), 700. - 32. - 16.);
}

#[test]
fn a_wide_row_stops_the_column_at_the_content_width() {
    assert_eq!(content_column_width(1600.), MAX_CONTENT_WIDTH - 16.);
    assert_eq!(
        content_column_width(MAX_CONTENT_WIDTH + 32.),
        MAX_CONTENT_WIDTH - 16.
    );
}

#[test]
fn the_text_width_never_passes_the_row_or_goes_below_zero() {
    for row in [0., 10., 32., 47., 48., 200., 851., 852., 853., 5000.] {
        let text = content_column_width(row);
        assert!(text >= 0., "{row} gives {text}");
        assert!(text <= row, "{row} gives {text}");
        assert!(text <= MAX_CONTENT_WIDTH, "{row} gives {text}");
    }
    assert_eq!(content_column_width(20.), 0.);
}
