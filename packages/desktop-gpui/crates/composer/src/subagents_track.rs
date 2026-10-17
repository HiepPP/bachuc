//! `packages/app/src/subagents/track.tsx` and `track-presentation.ts`: the subagents pill, one
//! segment per active state, and its panel with the bulk archive row and one row per child.
//! The state marks use the sidebar's bucket map, so the composer and the sidebar agree.

use crate::tracks::{TrackSegment, track_pill, track_row, track_separator};
use bachuc_client::{SubagentKind, SubagentRow};
use bachuc_icons::{IconName, icon, provider_fallback_icon, provider_logo};
use bachuc_sidebar::model::{StatusBucket, bucket_for_agent};
use bachuc_theme::metrics::{FONT_SM, SPACE_1};
use bachuc_theme::{tokens, ui_font_size};
use gpui_kit::component::popover::PopoverState;
use gpui_kit::component::tooltip::Tooltip;
use gpui_kit::component::{Sizable as _, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;
use std::rc::Rc;

/// `ROW_ICON_SIZE`: the leading and action glyphs share one size.
const ROW_ICON: f32 = 14.;
/// The pill reports every state but done, in the sidebar's status group order
/// (`ACTIVE_STATUS_BUCKET_ORDER`).
const ACTIVE_ORDER: [StatusBucket; 4] = [
    StatusBucket::NeedsInput,
    StatusBucket::Failed,
    StatusBucket::Attention,
    StatusBucket::Running,
];

/// The lifecycle status the bucket reads. A provider child reports its own words
/// (`providerSubagentLifecycleStatus`).
fn lifecycle_status(row: &SubagentRow) -> &str {
    match row.kind {
        SubagentKind::Paseo => &row.status,
        SubagentKind::Provider => match row.status.as_str() {
            "running" => "running",
            "failed" => "error",
            _ => "idle",
        },
    }
}

/// The row's state. A row has no permission count and no attention reason.
pub fn row_bucket(row: &SubagentRow) -> StatusBucket {
    bucket_for_agent(lifecycle_status(row), 0, row.requires_attention, None)
}

/// `resolveRowLabel`: trimmed, and blank or "new agent" is no label.
fn resolve_row_label(text: Option<&str>) -> Option<String> {
    let text = text?.trim();
    (!text.is_empty() && !text.eq_ignore_ascii_case("new agent")).then(|| text.to_string())
}

/// The description names a row in a fan-out; else the title.
pub fn row_label(row: &SubagentRow) -> Option<String> {
    resolve_row_label(row.description.as_deref())
        .or_else(|| resolve_row_label(row.title.as_deref()))
}

/// A provider child shows its own subtitle; a described row shows its title.
pub fn row_subtitle(row: &SubagentRow) -> Option<String> {
    if row.kind == SubagentKind::Provider {
        if let Some(subtitle) = resolve_row_label(row.subtitle.as_deref()) {
            return Some(subtitle);
        }
    }
    resolve_row_label(row.description.as_deref())
        .and_then(|_| resolve_row_label(row.title.as_deref()))
}

/// `isFinishedSubagent`.
pub fn is_finished(row: &SubagentRow) -> bool {
    match row.kind {
        SubagentKind::Paseo => matches!(row.status.as_str(), "idle" | "error"),
        SubagentKind::Provider => {
            matches!(row.status.as_str(), "completed" | "failed" | "canceled")
        }
    }
}

fn status_label(bucket: StatusBucket, count: usize) -> String {
    match bucket {
        StatusBucket::Running => format!("{count} working"),
        StatusBucket::Failed => format!("{count} failed"),
        StatusBucket::NeedsInput if count == 1 => "1 needs input".into(),
        StatusBucket::NeedsInput => format!("{count} need input"),
        StatusBucket::Attention | StatusBucket::Done => format!("{count} ready to review"),
    }
}

/// `buildSubagentPillPresentation`: one segment per active state with its count, or the total
/// when every child is done. Returns the segments and the spoken label.
pub fn pill_segments(rows: &[SubagentRow]) -> (Vec<TrackSegment>, String) {
    let buckets: Vec<StatusBucket> = rows.iter().map(row_bucket).collect();
    let segments: Vec<TrackSegment> = ACTIVE_ORDER
        .iter()
        .filter_map(|bucket| {
            let count = buckets
                .iter()
                .filter(|candidate| *candidate == bucket)
                .count();
            (count > 0).then(|| TrackSegment {
                bucket: Some(*bucket),
                text: status_label(*bucket, count),
            })
        })
        .collect();
    if segments.is_empty() {
        let label = if rows.len() == 1 {
            "1 subagent".to_string()
        } else {
            format!("{} subagents", rows.len())
        };
        return (
            vec![TrackSegment {
                bucket: None,
                text: label.clone(),
            }],
            label,
        );
    }
    let label = segments
        .iter()
        .map(|segment| segment.text.as_str())
        .collect::<Vec<_>>()
        .join(", ");
    (segments, label)
}

/// The bulk archive in flight: the ids it archives and how many left the list.
#[derive(Clone, Debug, Default, PartialEq, Eq)]
pub struct ArchiveFinished {
    pub ids: Vec<String>,
}

impl ArchiveFinished {
    /// `(completed, total)` while some child is still listed; `None` when it is done.
    pub fn progress(&self, rows: &[SubagentRow]) -> Option<(usize, usize)> {
        let left = self
            .ids
            .iter()
            .filter(|id| rows.iter().any(|row| &row.id == *id))
            .count();
        (left > 0).then(|| (self.ids.len() - left, self.ids.len()))
    }
}

/// The finished Paseo children the bulk archive takes. A provider child has no archive here.
pub fn finished_paseo_ids(rows: &[SubagentRow]) -> Vec<String> {
    rows.iter()
        .filter(|row| row.kind == SubagentKind::Paseo && is_finished(row))
        .map(|row| row.id.clone())
        .collect()
}

/// What the panel rows do. The composer owns each.
#[derive(Clone)]
pub struct SubagentActions {
    pub open: Rc<dyn Fn(&str, &mut App)>,
    pub archive: Rc<dyn Fn(&str, &mut App)>,
    pub archive_finished: Rc<dyn Fn(&mut App)>,
}

fn provider_icon(provider: &str, color: Hsla) -> AnyElement {
    match provider_logo(provider, color) {
        Some(image) => img(image).flex_none().size(px(ROW_ICON)).into_any_element(),
        None => icon(provider_fallback_icon(provider))
            .with_size(px(ROW_ICON))
            .text_color(color)
            .into_any_element(),
    }
}

/// The pill; `None` with no child and no bulk archive in flight.
pub fn subagents_pill(
    rows: Vec<SubagentRow>,
    progress: Option<(usize, usize)>,
    actions: SubagentActions,
) -> Option<AnyElement> {
    if rows.is_empty() && progress.is_none() {
        return None;
    }
    let (segments, label) = pill_segments(&rows);
    let finished = finished_paseo_ids(&rows).len();
    Some(track_pill(
        "subagents-track-header",
        segments,
        label,
        "Subagents",
        move |_, _, cx| {
            let t = tokens(cx);
            let small = ui_font_size(FONT_SM, cx);
            let archive_row = (finished > 0 || progress.is_some()).then(|| {
                let archive_finished = actions.archive_finished.clone();
                track_row(
                    "subagents-track-archive-finished",
                    "Archive finished subagents".into(),
                    progress.is_none(),
                    cx,
                )
                .group("subagents-archive-finished")
                .child(
                    icon(IconName::Archive)
                        .with_size(px(ROW_ICON))
                        .text_color(t.foreground_muted),
                )
                .child(
                    div()
                        .flex_1()
                        .min_w_0()
                        .truncate()
                        .text_color(t.foreground)
                        .child("Archive finished subagents"),
                )
                .when_some(progress, |row, (done, total)| {
                    row.child(
                        div()
                            .flex_none()
                            .text_size(small)
                            .text_color(t.foreground_muted)
                            .child(format!("{done}/{total}")),
                    )
                })
                .when(progress.is_none(), |row| {
                    // The progress lands on this row, so the panel stays open.
                    row.on_click(move |_, _, cx| archive_finished(cx))
                })
            });
            let list = rows.iter().map(|row| {
                let label = row_label(row).unwrap_or_else(|| "Loading...".into());
                let paseo = row.kind == SubagentKind::Paseo;
                let group: SharedString = format!("subagent-row-{}", row.id).into();
                let (open, archive) = (actions.open.clone(), actions.archive.clone());
                let (open_id, archive_id) = (row.id.clone(), row.id.clone());
                let archive_label = format!("Archive {label}");
                // A provider child opens nothing: the app has no provider-subagent tab.
                track_row(
                    ElementId::Name(format!("subagents-track-row-{}", row.id).into()),
                    label.clone(),
                    paseo,
                    cx,
                )
                .group(group.clone())
                .child(provider_icon(&row.provider, t.foreground_muted))
                .child(
                    div()
                        .flex_grow(1.)
                        .flex_shrink(1.)
                        .min_w_0()
                        .truncate()
                        .text_color(t.foreground)
                        .child(label),
                )
                .children(row_subtitle(row).map(|subtitle| {
                    div()
                        .flex_shrink(1.)
                        .min_w_0()
                        .truncate()
                        .text_size(small)
                        .text_color(t.foreground_muted)
                        .child(subtitle)
                }))
                .when(paseo, |line| {
                    let dismiss = cx.listener(move |state: &mut PopoverState, _, window, cx| {
                        open(&open_id, cx);
                        state.dismiss(window, cx);
                    });
                    line.on_click(dismiss).child(
                        // Shown on hover by opacity, never by `display` (CONTEXT).
                        div()
                            .id(ElementId::Name(
                                format!("subagents-track-archive-{}", row.id).into(),
                            ))
                            .role(Role::Button)
                            .aria_label(archive_label)
                            .flex_none()
                            .p(px(SPACE_1))
                            .cursor_pointer()
                            .opacity(0.)
                            .group_hover(group, |action| action.opacity(1.))
                            .tooltip(|window, cx| {
                                Tooltip::new("Archive subagent").build(window, cx)
                            })
                            .on_click(move |_, _, cx| {
                                cx.stop_propagation();
                                archive(&archive_id, cx);
                            })
                            .child(
                                icon(IconName::Archive)
                                    .with_size(px(ROW_ICON))
                                    .text_color(t.foreground_muted),
                            ),
                    )
                })
            });
            // A divider with nothing under it separates nothing (`ComposerTrackActions`).
            let divided = archive_row.is_some() && !rows.is_empty();
            v_flex()
                .children(archive_row)
                .when(divided, |panel| panel.child(track_separator(cx)))
                .children(list)
                .into_any_element()
        },
    ))
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{
        ArchiveFinished, finished_paseo_ids, is_finished, pill_segments, row_label, row_subtitle,
    };
    use crate::tracks::TrackSegment;
    use bachuc_client::{SubagentKind, SubagentRow};
    use bachuc_sidebar::model::StatusBucket;

    fn row(id: &str, kind: SubagentKind, status: &str, attention: bool) -> SubagentRow {
        SubagentRow {
            id: id.into(),
            kind,
            provider: "claude".into(),
            title: Some(format!("Title {id}")),
            description: None,
            subtitle: None,
            status: status.into(),
            requires_attention: attention,
            created_at: String::new(),
        }
    }

    #[test]
    fn segments_follow_the_bucket_order_with_their_counts() {
        let rows = vec![
            row("a", SubagentKind::Paseo, "running", false),
            row("b", SubagentKind::Paseo, "error", false),
            row("c", SubagentKind::Paseo, "running", false),
            row("d", SubagentKind::Paseo, "idle", true),
            row("e", SubagentKind::Provider, "failed", false),
        ];
        let (segments, label) = pill_segments(&rows);
        let texts: Vec<_> = segments
            .iter()
            .map(|s| (s.bucket, s.text.as_str()))
            .collect();
        assert_eq!(
            texts,
            [
                (Some(StatusBucket::Failed), "2 failed"),
                (Some(StatusBucket::Attention), "1 ready to review"),
                (Some(StatusBucket::Running), "2 working"),
            ]
        );
        assert_eq!(label, "2 failed, 1 ready to review, 2 working");
    }

    #[test]
    fn a_done_fan_out_names_its_size() {
        let rows = vec![
            row("a", SubagentKind::Paseo, "idle", false),
            row("b", SubagentKind::Provider, "completed", false),
        ];
        let (segments, label) = pill_segments(&rows);
        assert_eq!(
            segments,
            [TrackSegment {
                bucket: None,
                text: "2 subagents".into()
            }]
        );
        assert_eq!(label, "2 subagents");
        let (one, _) = pill_segments(&rows[..1]);
        assert_eq!(one[0].text, "1 subagent");
    }

    #[test]
    fn the_description_names_the_row_over_the_title() {
        let mut described = row("a", SubagentKind::Paseo, "running", false);
        described.description = Some("  Fix the build ".into());
        assert_eq!(row_label(&described).as_deref(), Some("Fix the build"));
        assert_eq!(row_subtitle(&described).as_deref(), Some("Title a"));
        let plain = row("b", SubagentKind::Paseo, "running", false);
        assert_eq!(row_label(&plain).as_deref(), Some("Title b"));
        assert_eq!(row_subtitle(&plain), None);
        let mut blank = row("c", SubagentKind::Paseo, "running", false);
        blank.title = Some("New Agent".into());
        assert_eq!(row_label(&blank), None);
        let mut provider = row("d", SubagentKind::Provider, "running", false);
        provider.subtitle = Some("opus".into());
        assert_eq!(row_subtitle(&provider).as_deref(), Some("opus"));
    }

    #[test]
    fn finished_children_and_the_bulk_archive_progress() {
        let rows = vec![
            row("a", SubagentKind::Paseo, "idle", false),
            row("b", SubagentKind::Paseo, "running", false),
            row("c", SubagentKind::Provider, "completed", false),
        ];
        assert!(is_finished(&rows[0]) && !is_finished(&rows[1]) && is_finished(&rows[2]));
        assert_eq!(finished_paseo_ids(&rows), ["a"]);
        let archive = ArchiveFinished {
            ids: vec!["a".into(), "x".into()],
        };
        assert_eq!(archive.progress(&rows), Some((1, 2)));
        assert_eq!(archive.progress(&rows[1..]), None);
    }
}
