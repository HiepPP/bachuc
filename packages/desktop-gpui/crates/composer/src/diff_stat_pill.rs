//! `packages/app/src/composer/diff-stat-pill.tsx`: the workspace `+N -M` pill at the right end
//! of the track strip. A press opens the Changes tab.

use crate::pill_styles::docked_pill;
use bachuc_client::DiffStat;
use bachuc_theme::metrics::SPACE_1;
use bachuc_theme::tokens;
use gpui_kit::*;

/// `workspace.git.diff.openChangesTab`.
pub const OPEN_CHANGES_LABEL: &str = "Open Changes tab";

/// `hasDiffStatChanges`: the stat to draw, or `None` when there is none or it is all zero.
pub fn visible_diff_stat(stat: Option<DiffStat>) -> Option<DiffStat> {
    stat.filter(|stat| stat.additions > 0 || stat.deletions > 0)
}

/// `formatDiffCount`: compact notation with one fraction digit, lower case (`1.2k`).
pub fn format_diff_count(value: u32) -> String {
    let compact = |scaled: f64, suffix: &str| {
        let text = format!("{scaled:.1}");
        format!("{}{suffix}", text.strip_suffix(".0").unwrap_or(&text))
    };
    match value {
        0..=999 => value.to_string(),
        1_000..=999_949 => compact(f64::from(value) / 1_000., "k"),
        _ => compact(f64::from(value) / 1_000_000., "m"),
    }
}

/// The two texts of the pill, `+N` and `-M`, or `None` to hide it.
pub fn diff_row(stat: Option<DiffStat>) -> Option<(String, String)> {
    let stat = visible_diff_stat(stat)?;
    Some((
        format!("+{}", format_diff_count(stat.additions)),
        format!("-{}", format_diff_count(stat.deletions)),
    ))
}

/// The pill; `None` without changes.
pub fn diff_stat_pill(
    stat: Option<DiffStat>,
    on_press: impl Fn(&mut Window, &mut App) + 'static,
    cx: &App,
) -> Option<AnyElement> {
    let (additions, deletions) = diff_row(stat)?;
    let t = tokens(cx);
    Some(
        docked_pill("composer-diff-stat-pill", false, cx)
            .role(Role::Button)
            .aria_label(OPEN_CHANGES_LABEL)
            .gap(px(SPACE_1))
            .cursor_pointer()
            .on_click(move |_, window, cx| on_press(window, cx))
            .child(div().text_color(t.status_success).child(additions))
            .child(div().text_color(t.status_danger).child(deletions))
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{diff_row, format_diff_count};
    use bachuc_client::DiffStat;

    #[test]
    fn the_diff_row_shows_counts_and_hides_without_changes() {
        assert_eq!(diff_row(None), None);
        assert_eq!(
            diff_row(Some(DiffStat {
                additions: 0,
                deletions: 0
            })),
            None
        );
        assert_eq!(
            diff_row(Some(DiffStat {
                additions: 12,
                deletions: 3
            })),
            Some(("+12".into(), "-3".into()))
        );
        assert_eq!(
            diff_row(Some(DiffStat {
                additions: 0,
                deletions: 7
            })),
            Some(("+0".into(), "-7".into()))
        );
        assert_eq!(
            diff_row(Some(DiffStat {
                additions: 742,
                deletions: 0
            })),
            Some(("+742".into(), "-0".into()))
        );
    }

    #[test]
    fn diff_counts_use_compact_notation() {
        assert_eq!(format_diff_count(999), "999");
        assert_eq!(format_diff_count(1_000), "1k");
        assert_eq!(format_diff_count(1_234), "1.2k");
        assert_eq!(format_diff_count(12_345), "12.3k");
        assert_eq!(format_diff_count(1_500_000), "1.5m");
    }
}
