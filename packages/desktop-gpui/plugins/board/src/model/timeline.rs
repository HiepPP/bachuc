//! The `board.timeline` wire types and the chart model of `shared/timeline.ts`: thread rows,
//! fold breaks, hour ticks, the load strip, and the footer, plus the bar text and colors of
//! `client/timeline.tsx`.

use super::SEPARATOR;
use super::colors::repo_label;
use super::run::RunStatus;
use super::text::time_value;
use crate::orb::js_round;
use anyhow::Context as _;
use bachuc_theme::Tokens;
use gpui_kit::{Hsla, hsla};
use serde::Deserialize;
use serde_json::Value;
use std::collections::{HashMap, HashSet};

pub(crate) const TIMELINE_POLL_MS: i64 = 15_000;
pub(crate) const TIMELINE_WINDOW_MS: f64 = 24. * 60. * 60. * 1000.;
const MINUTE: f64 = 60_000.;
const HOUR: f64 = 60. * MINUTE;
/// An idle gap longer than max(2 h, 4 x the average thread) draws as a fixed-width break.
pub(crate) const BREAK_WIDTH: f64 = 48.;
const MIN_BREAK_MINUTES: f64 = 120.;
const PAD_X: f64 = 4.;
const MIN_BAR_WIDTH: f64 = 4.;
/// A tick label is about 36 px wide. Keep every nth hour so neighbouring labels never touch.
const MIN_TICK_GAP: f64 = 44.;
/// A duration label fits inside a bar from this width; narrower bars show only their TASK chip.
pub(crate) const LABEL_MIN_WIDTH: f64 = 44.;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct WireThread {
    agent_id: String,
    #[serde(default)]
    parent_agent_id: Option<String>,
    title: String,
    project: String,
    project_key: String,
    #[serde(default)]
    project_id: Option<String>,
    status: RunStatus,
    started_at: String,
    #[serde(default)]
    ended_at: Option<String>,
    #[serde(default)]
    needs_input: bool,
    #[serde(default)]
    task_id: Option<String>,
    #[serde(default)]
    workspace_id: Option<String>,
}

/// One thread of `board.timeline` (`timelineThreadSchema`), with its times in milliseconds.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TimelineThread {
    pub agent_id: String,
    pub parent_agent_id: Option<String>,
    pub title: String,
    pub project: String,
    pub project_key: String,
    pub project_id: Option<String>,
    pub status: RunStatus,
    pub started_at: f64,
    /// `None` when absent or not a time, as `Date.parse` gives `NaN`.
    pub ended_at: Option<f64>,
    pub needs_input: bool,
    pub task_id: Option<String>,
    pub workspace_id: Option<String>,
}

/// A `board.timeline` answer.
#[derive(Clone, Debug, Default, PartialEq)]
pub(crate) struct TimelineAnswer {
    pub threads: Vec<TimelineThread>,
    pub missing_start: u64,
    pub older_dropped: bool,
}

/// Decodes a `board.timeline` answer. Each thread decodes on its own: a bad row is skipped and
/// logged.
pub(crate) fn decode_timeline(value: &Value) -> anyhow::Result<TimelineAnswer> {
    let threads = value
        .get("threads")
        .and_then(Value::as_array)
        .context("board.timeline answer has no threads")?
        .iter()
        .filter_map(|raw| match decode_thread(raw) {
            Ok(thread) => Some(thread),
            Err(error) => {
                log::warn!("board: skipped a timeline thread that does not decode: {error:#}");
                None
            }
        })
        .collect();
    Ok(TimelineAnswer {
        threads,
        missing_start: value
            .get("missingStart")
            .and_then(Value::as_u64)
            .context("board.timeline answer has no missingStart")?,
        older_dropped: value
            .get("olderDropped")
            .and_then(Value::as_bool)
            .context("board.timeline answer has no olderDropped")?,
    })
}

fn decode_thread(raw: &Value) -> anyhow::Result<TimelineThread> {
    let wire = WireThread::deserialize(raw)?;
    let started_at = time_value(Some(&wire.started_at)).context("startedAt is not a time")? as f64;
    Ok(TimelineThread {
        ended_at: time_value(wire.ended_at.as_deref()).map(|time| time as f64),
        agent_id: wire.agent_id,
        parent_agent_id: wire.parent_agent_id,
        title: wire.title,
        project: wire.project,
        project_key: wire.project_key,
        project_id: wire.project_id,
        status: wire.status,
        started_at,
        needs_input: wire.needs_input,
        task_id: wire.task_id,
        workspace_id: wire.workspace_id,
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum TimelineKind {
    Running,
    Needs,
    Done,
    Failed,
    Cancelled,
}

/// The legend, in order.
pub(crate) const LEGEND: [(TimelineKind, &str); 5] = [
    (TimelineKind::Running, "running"),
    (TimelineKind::Done, "done"),
    (TimelineKind::Needs, "needs you"),
    (TimelineKind::Failed, "failed"),
    (TimelineKind::Cancelled, "cancelled"),
];

/// `KIND_TEXT` of a bar's accessibility label.
pub(crate) fn kind_text(kind: TimelineKind) -> &'static str {
    match kind {
        TimelineKind::Running => "running",
        TimelineKind::Needs => "needs input",
        TimelineKind::Done => "completed",
        TimelineKind::Failed => "failed",
        TimelineKind::Cancelled => "cancelled",
    }
}

pub(crate) fn timeline_kind(status: RunStatus, needs_input: bool) -> TimelineKind {
    match status {
        RunStatus::Running if needs_input => TimelineKind::Needs,
        RunStatus::Running => TimelineKind::Running,
        RunStatus::Failed => TimelineKind::Failed,
        RunStatus::Cancelled => TimelineKind::Cancelled,
        RunStatus::Completed | RunStatus::Unknown => TimelineKind::Done,
    }
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TimelineBar {
    pub agent_id: String,
    pub title: String,
    pub kind: TimelineKind,
    pub task_id: Option<String>,
    pub workspace_id: Option<String>,
    pub start: f64,
    pub end: f64,
    pub x: f64,
    pub width: f64,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TimelineRow {
    pub agent_id: String,
    pub project: String,
    pub project_key: String,
    pub project_id: Option<String>,
    pub bar: TimelineBar,
    /// A thread's subagents share one row under it.
    pub subagents: Vec<TimelineBar>,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TimelineBreak {
    pub x: f64,
    pub width: f64,
    pub minutes: f64,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TimelineTick {
    pub x: f64,
    pub label: String,
}

/// Top-level threads running at once in one clock hour.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct LoadBucket {
    pub x: f64,
    pub width: f64,
    pub count: usize,
    pub running: bool,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct Peak {
    pub count: usize,
    pub label: String,
}

#[derive(Clone, Debug, PartialEq)]
pub(crate) struct TimelineModel {
    pub window_start: f64,
    pub now: f64,
    pub width: f64,
    pub now_x: f64,
    /// Rows per host, in host order.
    pub rows: Vec<(String, Vec<TimelineRow>)>,
    pub breaks: Vec<TimelineBreak>,
    pub ticks: Vec<TimelineTick>,
    pub load: Vec<LoadBucket>,
    pub peak: Option<Peak>,
    pub threads: usize,
    pub agent_minutes: f64,
    pub idle_minutes: f64,
}

impl TimelineModel {
    pub(crate) fn host_rows(&self, host_id: &str) -> &[TimelineRow] {
        self.rows
            .iter()
            .find(|(id, _)| id == host_id)
            .map_or(&[], |(_, rows)| rows.as_slice())
    }
}

pub(crate) struct TimelineHostInput<'a> {
    pub host_id: &'a str,
    pub threads: &'a [TimelineThread],
}

pub(crate) fn duration_text(milliseconds: f64) -> String {
    let seconds = js_round(milliseconds / 1000.).max(0.);
    if seconds < 60. {
        return format!("{seconds}s");
    }
    let minutes = js_round(seconds / 60.);
    if minutes < 60. {
        return format!("{minutes}m");
    }
    let hours = (minutes / 60.).floor();
    let rest = minutes % 60.;
    if rest != 0. {
        format!("{hours}h {rest}m")
    } else {
        format!("{hours}h")
    }
}

/// `HH:MM` in local time.
pub(crate) fn clock_text(time: f64) -> String {
    use chrono::TimeZone as _;
    chrono::Local
        .timestamp_millis_opt(time as i64)
        .single()
        .map_or_else(|| "--:--".into(), |date| date.format("%H:%M").to_string())
}

/// Notes under the host's header once its timeline has loaded.
pub(crate) fn timeline_notes(
    missing_start: u64,
    older_dropped: bool,
    threads: usize,
) -> Vec<String> {
    let mut notes = Vec::new();
    if threads == 0 {
        notes.push("Nothing ran in the last 24 hours.".to_string());
    }
    if older_dropped {
        notes.push("Older threads are not kept.".to_string());
    }
    match missing_start {
        0 => {}
        1 => notes.push("1 thread without a start time is left out.".to_string()),
        count => notes.push(format!(
            "{count} threads without a start time are left out."
        )),
    }
    notes
}

#[derive(Clone, Copy, Debug, PartialEq)]
struct Span {
    start: f64,
    end: f64,
}

fn span(thread: &TimelineThread, window_start: f64, now: f64) -> Span {
    let start = thread.started_at.max(window_start).min(now);
    let ended = if thread.status == RunStatus::Running {
        Some(now)
    } else {
        thread.ended_at
    };
    let end = ended.map_or(start, |ended| ended.max(start).min(now));
    Span { start, end }
}

/// The top-level thread each thread belongs to: its oldest ancestor on the same host.
fn roots(threads: &[TimelineThread]) -> HashMap<&str, &str> {
    let by_id: HashMap<&str, &TimelineThread> = threads
        .iter()
        .map(|thread| (thread.agent_id.as_str(), thread))
        .collect();
    threads
        .iter()
        .map(|thread| {
            let mut current = thread;
            let mut seen = HashSet::from([current.agent_id.as_str()]);
            while let Some(parent) = current
                .parent_agent_id
                .as_deref()
                .filter(|id| !id.is_empty())
                .and_then(|id| by_id.get(id))
            {
                if !seen.insert(parent.agent_id.as_str()) {
                    break;
                }
                current = parent;
            }
            (thread.agent_id.as_str(), current.agent_id.as_str())
        })
        .collect()
}

fn idle_gaps(spans: &[Span], window_start: f64, now: f64) -> Vec<Span> {
    // An empty window has nothing to fold toward.
    if spans.is_empty() {
        return Vec::new();
    }
    let average = spans.iter().map(|item| item.end - item.start).sum::<f64>() / spans.len() as f64;
    let threshold = (MIN_BREAK_MINUTES * MINUTE).max(4. * average);
    // The window start and now count as busy points, so a long quiet stretch at either end
    // folds.
    let mut points = vec![Span {
        start: window_start,
        end: window_start,
    }];
    points.extend_from_slice(spans);
    points.push(Span {
        start: now,
        end: now,
    });
    points.sort_by(|a, b| a.start.total_cmp(&b.start));
    let mut gaps = Vec::new();
    let mut busy_end = points[0].end;
    for point in &points[1..] {
        if point.start - busy_end > threshold {
            gaps.push(Span {
                start: busy_end,
                end: point.start,
            });
        }
        busy_end = busy_end.max(point.end);
    }
    gaps
}

/// Most intervals overlapping at one moment inside `[from, to)`.
fn concurrency(spans: &[Span], from: f64, to: f64) -> usize {
    let mut events: Vec<(f64, i32)> = spans
        .iter()
        .filter(|item| item.start < to && item.end >= from)
        .flat_map(|item| [(item.start.max(from), 1), (item.end.min(to), -1)])
        .collect();
    if events.is_empty() {
        return 0;
    }
    // An end and a start at the same moment do not overlap.
    events.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.cmp(&b.1)));
    let (mut count, mut peak) = (0, 1);
    for (_, delta) in events {
        count += delta;
        peak = peak.max(count);
    }
    peak as usize
}

/// `timelineModel`: the chart of every host on one shared axis `width` px wide.
pub(crate) fn timeline_model(
    hosts: &[TimelineHostInput<'_>],
    now: f64,
    width: f64,
) -> TimelineModel {
    let window_start = now - TIMELINE_WINDOW_MS;
    let mut top: Vec<(Span, bool)> = Vec::new();
    // Per host: each top-level thread with the threads under it.
    type Rows<'t> = Vec<(&'t TimelineThread, Vec<&'t TimelineThread>)>;
    let grouped: Vec<(&str, Rows)> = hosts
        .iter()
        .map(|host| {
            let root = roots(host.threads);
            let mut rows: Rows = host
                .threads
                .iter()
                .filter(|thread| {
                    root.get(thread.agent_id.as_str()) == Some(&thread.agent_id.as_str())
                })
                .map(|thread| (thread, Vec::new()))
                .collect();
            for thread in host.threads {
                let owner = root[thread.agent_id.as_str()];
                if owner != thread.agent_id
                    && let Some((_, children)) =
                        rows.iter_mut().find(|(row, _)| row.agent_id == owner)
                {
                    children.push(thread);
                }
            }
            for (thread, _) in &rows {
                top.push((
                    span(thread, window_start, now),
                    thread.status == RunStatus::Running,
                ));
            }
            (host.host_id, rows)
        })
        .collect();

    let top_spans: Vec<Span> = top.iter().map(|(span, _)| *span).collect();
    let gaps = idle_gaps(&top_spans, window_start, now);
    let hidden: f64 = gaps.iter().map(|gap| gap.end - gap.start).sum();
    let scaled = (now - window_start - hidden).max(1.);
    let usable = (width - PAD_X * 2. - gaps.len() as f64 * BREAK_WIDTH).max(1.);
    // Time before a gap scales normally; a gap takes BREAK_WIDTH whatever its length.
    let x = |time: f64| {
        let mut offset = PAD_X;
        let mut from = window_start;
        for gap in &gaps {
            if time <= gap.start {
                break;
            }
            offset += (gap.start - from) / scaled * usable;
            if time < gap.end {
                return offset + (time - gap.start) / (gap.end - gap.start) * BREAK_WIDTH;
            }
            offset += BREAK_WIDTH;
            from = gap.end;
        }
        offset + (time - from) / scaled * usable
    };
    let in_gap = |time: f64| gaps.iter().find(|gap| gap.start < time && time < gap.end);

    let bar = |thread: &TimelineThread| {
        let Span { start, end } = span(thread, window_start, now);
        let left = x(start);
        TimelineBar {
            agent_id: thread.agent_id.clone(),
            title: thread.title.clone(),
            kind: timeline_kind(thread.status, thread.needs_input),
            task_id: thread.task_id.clone(),
            workspace_id: thread.workspace_id.clone(),
            start,
            end,
            x: left,
            width: (x(end) - left).max(MIN_BAR_WIDTH),
        }
    };
    let rows = grouped
        .into_iter()
        .map(|(host_id, rows)| {
            let mut built: Vec<TimelineRow> = rows
                .into_iter()
                .map(|(thread, children)| {
                    let mut subagents: Vec<TimelineBar> = children.into_iter().map(&bar).collect();
                    subagents.sort_by(|a, b| a.start.total_cmp(&b.start));
                    TimelineRow {
                        agent_id: thread.agent_id.clone(),
                        project: thread.project.clone(),
                        project_key: thread.project_key.clone(),
                        project_id: thread.project_id.clone(),
                        bar: bar(thread),
                        subagents,
                    }
                })
                .collect();
            // Projects in order of their first thread, threads by start inside a project.
            let mut first_start: HashMap<String, f64> = HashMap::new();
            for row in &built {
                let first = first_start
                    .entry(row.project_key.clone())
                    .or_insert(f64::INFINITY);
                *first = first.min(row.bar.start);
            }
            built.sort_by(|a, b| {
                first_start[&a.project_key]
                    .total_cmp(&first_start[&b.project_key])
                    .then_with(|| a.project_key.cmp(&b.project_key))
                    .then_with(|| a.bar.start.total_cmp(&b.bar.start))
            });
            (host_id.to_string(), built)
        })
        .collect();

    let first_hour = (window_start / HOUR).floor() * HOUR;
    let mut hour_starts = Vec::new();
    let mut hour = first_hour;
    while hour < now {
        hour_starts.push(hour);
        hour += HOUR;
    }

    let mut ticks: Vec<TimelineTick> = Vec::new();
    for &hour in &hour_starts {
        if hour < window_start || in_gap(hour).is_some() {
            continue;
        }
        let tick_x = x(hour);
        if ticks
            .last()
            .is_some_and(|last| tick_x - last.x < MIN_TICK_GAP)
        {
            continue;
        }
        ticks.push(TimelineTick {
            x: tick_x,
            label: clock_text(hour),
        });
    }

    let mut load = Vec::new();
    let mut counts = Vec::new();
    for &hour in &hour_starts {
        let from = hour.max(window_start);
        let to = (hour + HOUR).min(now);
        let count = concurrency(&top_spans, from, to);
        counts.push(count);
        let left = x(in_gap(from).map_or(from, |gap| gap.end));
        let right = x(in_gap(to).map_or(to, |gap| gap.start));
        if count == 0 || right <= left {
            continue;
        }
        load.push(LoadBucket {
            x: left,
            width: right - left,
            count,
            running: top
                .iter()
                .any(|(span, running)| *running && span.start < to && span.end >= from),
        });
    }
    let most = counts.iter().copied().max().unwrap_or(0);
    let peak = (most > 0).then(|| {
        let first = counts.iter().position(|count| *count == most).unwrap_or(0);
        let mut last = first;
        while counts.get(last + 1) == Some(&most) {
            last += 1;
        }
        let to = (hour_starts[last] + HOUR).min(now);
        Peak {
            count: most,
            label: format!("{}–{}", clock_text(hour_starts[first]), clock_text(to)),
        }
    });

    TimelineModel {
        window_start,
        now,
        width,
        now_x: x(now),
        rows,
        breaks: gaps
            .iter()
            .map(|gap| TimelineBreak {
                x: x(gap.start),
                width: BREAK_WIDTH,
                minutes: js_round((gap.end - gap.start) / MINUTE),
                label: duration_text(gap.end - gap.start),
            })
            .collect(),
        ticks,
        load,
        peak,
        threads: top.len(),
        agent_minutes: js_round(
            top_spans
                .iter()
                .map(|item| item.end - item.start)
                .sum::<f64>()
                / MINUTE,
        ),
        idle_minutes: js_round(hidden / MINUTE),
    }
}

/// The footer: threads, agent time, the peak, and the folded idle time.
pub(crate) fn timeline_footer(model: &TimelineModel) -> String {
    let mut parts = vec![
        format!(
            "{} {}",
            model.threads,
            if model.threads == 1 {
                "thread"
            } else {
                "threads"
            }
        ),
        format!("{} agent time", duration_text(model.agent_minutes * MINUTE)),
    ];
    if let Some(peak) = &model.peak {
        parts.push(format!("peak {} at once, {}", peak.count, peak.label));
    }
    if model.idle_minutes != 0. {
        parts.push(format!(
            "idle {} folded",
            duration_text(model.idle_minutes * MINUTE)
        ));
    }
    parts.join(SEPARATOR)
}

/// The strip's accessibility label.
pub(crate) fn peak_label(peak: Option<&Peak>) -> String {
    peak.map_or_else(
        || "No agents ran".into(),
        |peak| format!("Peak {} agents at once, {}", peak.count, peak.label),
    )
}

/// The text inside a bar: its duration, with `!` while it needs input.
pub(crate) fn bar_text(bar: &TimelineBar) -> String {
    let duration = duration_text(bar.end - bar.start);
    if bar.kind == TimelineKind::Needs {
        format!("{duration} !")
    } else {
        duration
    }
}

pub(crate) fn bar_label(bar: &TimelineBar) -> String {
    format!(
        "{}, {}, {} to {}, {}",
        bar.title,
        kind_text(bar.kind),
        clock_text(bar.start),
        clock_text(bar.end),
        duration_text(bar.end - bar.start)
    )
}

pub(crate) fn subagent_text(count: usize) -> String {
    if count == 1 {
        "1 subagent".into()
    } else {
        format!("{count} subagents")
    }
}

/// The host header's meta: offline, the row count, unavailable, or loading.
pub(crate) fn host_meta(online: bool, loaded: bool, failed: bool, rows: usize) -> String {
    if !online {
        "Offline".into()
    } else if loaded {
        if rows == 1 {
            "1 thread".into()
        } else {
            format!("{rows} threads")
        }
    } else if failed {
        "Unavailable".into()
    } else {
        "Loading".into()
    }
}

/// The notes under the host header.
pub(crate) fn host_notes(
    online: bool,
    answer: Option<&TimelineAnswer>,
    failed: bool,
    rows: usize,
) -> Vec<String> {
    if !online {
        vec!["Host offline. Reconnect to see its timeline.".into()]
    } else if let Some(answer) = answer {
        timeline_notes(answer.missing_start, answer.older_dropped, rows)
    } else if failed {
        vec!["Timeline unavailable. Update or reload Board on this host.".into()]
    } else {
        Vec::new()
    }
}

/// The TASK chip text: the id without `TASK-`.
pub(crate) fn task_chip_text(task_id: &str) -> &str {
    task_id.strip_prefix("TASK-").unwrap_or(task_id)
}

/// A bar's fill, fill opacity, border, and text. Finished bars take the project's hue.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct BarColors {
    pub fill: Hsla,
    pub fill_opacity: f32,
    pub border: Hsla,
    pub text: Hsla,
}

pub(crate) fn bar_colors(kind: TimelineKind, hue: Option<u16>, t: &Tokens) -> BarColors {
    let colors = |fill, fill_opacity, border, text| BarColors {
        fill,
        fill_opacity,
        border,
        text,
    };
    match (kind, hue) {
        (TimelineKind::Running, _) => colors(t.foreground, 1., t.foreground, t.surface0),
        (TimelineKind::Needs, _) => colors(t.status_warning, 0.18, t.status_warning, t.foreground),
        (TimelineKind::Failed, _) => {
            colors(t.status_danger, 0.12, t.status_danger, t.status_danger)
        }
        (TimelineKind::Cancelled, _) => {
            colors(t.surface2, 1., t.foreground_muted, t.foreground_muted)
        }
        (TimelineKind::Done, None) => colors(t.surface2, 1., t.border, t.foreground),
        (TimelineKind::Done, Some(hue)) => colors(
            hsla(f32::from(hue) / 360., 0.38, 0.58, 1.),
            0.28,
            hsla(f32::from(hue) / 360., 0.30, 0.70, 1.),
            repo_label(Some(hue), t),
        ),
    }
}

/// The height of a load bucket: its share of the peak of the strip's drawable height.
pub(crate) fn load_height(count: usize, peak: Option<&Peak>, drawable: f32) -> f32 {
    count as f32 / peak.map_or(1, |peak| peak.count) as f32 * drawable
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone as _;
    use serde_json::json;

    /// Local times, so hour ticks and labels match the machine's clock like the app does.
    fn at(day: u32, hour: u32, minute: u32) -> f64 {
        chrono::Local
            .with_ymd_and_hms(2026, 10, day, hour, minute, 0)
            .single()
            .expect("a local time")
            .timestamp_millis() as f64
    }

    const WIDTH: f64 = 900.;

    fn now() -> f64 {
        at(12, 12, 0)
    }

    fn thread(agent_id: &str, start: f64, end: Option<f64>) -> TimelineThread {
        TimelineThread {
            agent_id: agent_id.into(),
            parent_agent_id: None,
            title: agent_id.into(),
            project: "bachuc".into(),
            project_key: "project:bachuc".into(),
            project_id: None,
            status: if end.is_none() {
                RunStatus::Running
            } else {
                RunStatus::Completed
            },
            started_at: start,
            ended_at: end,
            needs_input: false,
            task_id: None,
            workspace_id: None,
        }
    }

    fn child(mut thread: TimelineThread, parent: &str) -> TimelineThread {
        thread.parent_agent_id = Some(parent.into());
        thread
    }

    fn in_project(mut thread: TimelineThread, key: &str) -> TimelineThread {
        thread.project_key = key.into();
        thread.project = key.trim_start_matches("project:").into();
        thread
    }

    fn model(threads: &[TimelineThread]) -> TimelineModel {
        timeline_model(
            &[TimelineHostInput {
                host_id: "a",
                threads,
            }],
            now(),
            WIDTH,
        )
    }

    /// Half-hour threads every 90 minutes, quiet from 01:30 to 08:10, then a running thread.
    fn busy_day() -> Vec<TimelineThread> {
        let mut threads = Vec::new();
        let mut start = at(11, 13, 0);
        while start <= at(12, 1, 0) {
            threads.push(thread(
                &format!("t{}", threads.len()),
                start,
                Some(start + 30. * MINUTE),
            ));
            start += 90. * MINUTE;
        }
        threads.push(thread("morning-1", at(12, 8, 10), Some(at(12, 8, 40))));
        threads.push(thread("morning-2", at(12, 9, 40), Some(at(12, 10, 10))));
        threads.push(thread("now", at(12, 11, 10), None));
        threads
    }

    #[test]
    fn a_6h_40m_idle_gap_folds_into_one_48_px_break() {
        let model = model(&busy_day());
        assert_eq!(model.breaks.len(), 1);
        assert_eq!(model.breaks[0].width, BREAK_WIDTH);
        assert_eq!(model.breaks[0].label, "6h 40m");
        assert_eq!(model.idle_minutes, 400.);
        // No hour tick sits inside the folded gap.
        let gap = &model.breaks[0];
        assert!(
            model
                .ticks
                .iter()
                .all(|tick| tick.x <= gap.x || tick.x >= gap.x + BREAK_WIDTH)
        );
    }

    #[test]
    fn a_90_minute_gap_stays_at_scale() {
        let model = model(&[
            thread("a", at(11, 12, 30), Some(at(12, 9, 0))),
            thread("b", at(12, 10, 30), Some(at(12, 12, 0))),
        ]);
        assert!(model.breaks.is_empty());
    }

    #[test]
    fn a_running_thread_ends_at_the_now_line_and_its_subagents_share_one_row() {
        let mut first = child(
            thread("child-1", at(12, 11, 5), Some(at(12, 11, 20))),
            "parent",
        );
        first.task_id = Some("TASK-069".into());
        let model = model(&[
            thread("parent", at(12, 11, 0), None),
            first,
            child(thread("child-2", at(12, 11, 25), None), "parent"),
            child(
                thread("grandchild", at(12, 11, 30), Some(at(12, 11, 40))),
                "child-2",
            ),
        ]);
        let rows = model.host_rows("a");
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].bar.kind, TimelineKind::Running);
        assert!((rows[0].bar.x + rows[0].bar.width - model.now_x).abs() < 0.001);
        let ids: Vec<&str> = rows[0]
            .subagents
            .iter()
            .map(|bar| bar.agent_id.as_str())
            .collect();
        assert_eq!(ids, ["child-1", "child-2", "grandchild"]);
        assert_eq!(rows[0].subagents[0].task_id.as_deref(), Some("TASK-069"));
        assert_eq!(model.threads, 1);
    }

    #[test]
    fn rows_group_by_host_then_project_and_a_missing_parent_makes_a_top_level_row() {
        let a = [
            in_project(
                thread("late", at(12, 10, 0), Some(at(12, 11, 0))),
                "project:x",
            ),
            child(thread("orphan", at(12, 9, 0), Some(at(12, 9, 30))), "gone"),
            in_project(
                thread("early-x", at(12, 8, 0), Some(at(12, 8, 30))),
                "project:x",
            ),
        ];
        let b = [thread("other", at(12, 9, 0), Some(at(12, 10, 0)))];
        let model = timeline_model(
            &[
                TimelineHostInput {
                    host_id: "a",
                    threads: &a,
                },
                TimelineHostInput {
                    host_id: "b",
                    threads: &b,
                },
            ],
            now(),
            WIDTH,
        );
        let ids = |host: &str| -> Vec<String> {
            model
                .host_rows(host)
                .iter()
                .map(|row| row.agent_id.clone())
                .collect()
        };
        assert_eq!(ids("a"), ["early-x", "late", "orphan"]);
        assert_eq!(ids("b"), ["other"]);
    }

    #[test]
    fn the_load_strip_peaks_at_the_busiest_hour_and_labels_it() {
        let model = model(&[
            thread("a", at(12, 9, 0), Some(at(12, 10, 30))),
            thread("b", at(12, 9, 15), Some(at(12, 9, 45))),
            thread("c", at(12, 9, 30), Some(at(12, 9, 40))),
            // A subagent does not count toward agents at once.
            child(thread("sub", at(12, 9, 20), Some(at(12, 9, 50))), "b"),
            thread("d", at(12, 11, 0), None),
        ]);
        assert_eq!(
            model.peak,
            Some(Peak {
                count: 3,
                label: "09:00–10:00".into()
            })
        );
        assert_eq!(model.load.iter().map(|bucket| bucket.count).max(), Some(3));
        assert!(model.load.iter().any(|bucket| bucket.running));
        assert_eq!(model.threads, 4);
        assert_eq!(
            peak_label(model.peak.as_ref()),
            "Peak 3 agents at once, 09:00–10:00"
        );
        assert_eq!(load_height(3, model.peak.as_ref(), 30.), 30.);
        assert_eq!(load_height(1, model.peak.as_ref(), 30.), 10.);
    }

    #[test]
    fn no_threads_gives_an_empty_chart_and_the_nothing_ran_note() {
        let model = model(&[]);
        assert_eq!(model.threads, 0);
        assert_eq!(model.peak, None);
        assert!(model.load.is_empty());
        // The quiet day stays at scale instead of folding into one 24h break.
        assert!(model.breaks.is_empty());
        assert_eq!(model.idle_minutes, 0.);
        assert_eq!(peak_label(None), "No agents ran");
        assert_eq!(timeline_footer(&model), "0 threads · 0s agent time");
    }

    #[test]
    fn the_ticks_keep_their_gap_and_the_now_line_ends_the_axis() {
        let model = model(&busy_day());
        assert!(
            model
                .ticks
                .windows(2)
                .all(|pair| pair[1].x - pair[0].x >= MIN_TICK_GAP)
        );
        assert!(model.ticks.iter().all(|tick| tick.label.len() == 5));
        assert!((model.now_x - (WIDTH - PAD_X)).abs() < 1e-6);
        assert_eq!(
            timeline_footer(&model),
            format!(
                "{} threads · {} agent time · {} · idle 6h 40m folded",
                model.threads,
                duration_text(model.agent_minutes * MINUTE),
                format_args!(
                    "peak {} at once, {}",
                    model.peak.as_ref().unwrap().count,
                    model.peak.as_ref().unwrap().label
                )
            )
        );
    }

    #[test]
    fn durations_round_like_the_electron_text() {
        assert_eq!(duration_text(90_000.), "2m");
        assert_eq!(duration_text(3_600_000.), "1h");
        assert_eq!(duration_text(29_400.), "29s");
        assert_eq!(duration_text(29_500.), "30s");
        assert_eq!(duration_text(-5_000.), "0s");
        assert_eq!(duration_text(24_000_000.), "6h 40m");
    }

    #[test]
    fn the_clock_text_uses_local_time() {
        assert_eq!(clock_text(at(12, 9, 5)), "09:05");
        assert_eq!(clock_text(at(11, 23, 59)), "23:59");
    }

    #[test]
    fn the_notes_cover_no_threads_dropped_runs_and_missing_starts() {
        assert_eq!(
            timeline_notes(0, false, 0),
            ["Nothing ran in the last 24 hours."]
        );
        assert_eq!(timeline_notes(0, true, 2), ["Older threads are not kept."]);
        assert_eq!(
            timeline_notes(1, false, 3),
            ["1 thread without a start time is left out."]
        );
        assert_eq!(
            timeline_notes(2, false, 3),
            ["2 threads without a start time are left out."]
        );
    }

    #[test]
    fn the_host_header_says_offline_loading_unavailable_or_the_count() {
        assert_eq!(host_meta(false, true, false, 2), "Offline");
        assert_eq!(host_meta(true, false, false, 0), "Loading");
        assert_eq!(host_meta(true, false, true, 0), "Unavailable");
        assert_eq!(host_meta(true, true, true, 1), "1 thread");
        assert_eq!(
            host_notes(false, None, false, 0),
            ["Host offline. Reconnect to see its timeline."]
        );
        assert_eq!(
            host_notes(true, None, true, 0),
            ["Timeline unavailable. Update or reload Board on this host."]
        );
        assert!(host_notes(true, None, false, 0).is_empty());
    }

    #[test]
    fn the_bar_text_kind_and_colors_follow_the_electron_bar() {
        assert_eq!(timeline_kind(RunStatus::Running, true), TimelineKind::Needs);
        assert_eq!(timeline_kind(RunStatus::Unknown, false), TimelineKind::Done);
        let bar = TimelineBar {
            agent_id: "a".into(),
            title: "Fix login".into(),
            kind: TimelineKind::Needs,
            task_id: None,
            workspace_id: None,
            start: at(12, 9, 0),
            end: at(12, 9, 30),
            x: 0.,
            width: 10.,
        };
        assert_eq!(bar_text(&bar), "30m !");
        assert_eq!(
            bar_label(&bar),
            "Fix login, needs input, 09:00 to 09:30, 30m"
        );
        assert_eq!(subagent_text(1), "1 subagent");
        assert_eq!(subagent_text(3), "3 subagents");
        assert_eq!(task_chip_text("TASK-069"), "069");
        assert_eq!(task_chip_text("Q-1"), "Q-1");
        let t = Tokens::light();
        let done = bar_colors(TimelineKind::Done, Some(120), &t);
        assert_eq!(done.fill, hsla(120. / 360., 0.38, 0.58, 1.));
        assert_eq!(done.fill_opacity, 0.28);
        assert_eq!(done.text, repo_label(Some(120), &t));
        let plain = bar_colors(TimelineKind::Done, None, &t);
        assert_eq!((plain.fill, plain.border), (t.surface2, t.border));
        let running = bar_colors(TimelineKind::Running, Some(120), &t);
        assert_eq!((running.fill, running.text), (t.foreground, t.surface0));
    }

    #[test]
    fn a_real_timeline_payload_decodes_and_skips_a_bad_thread() {
        let answer = decode_timeline(&json!({
            "threads": [
                {
                    "agentId": "a1",
                    "parentAgentId": null,
                    "title": "Fix login",
                    "project": "bachuc",
                    "projectKey": "project:bachuc",
                    "projectId": "p1",
                    "status": "completed",
                    "startedAt": "2026-10-12T10:00:00.000Z",
                    "endedAt": "2026-10-12T11:00:00.000Z",
                    "taskId": "TASK-069",
                    "workspaceId": "w1"
                },
                {
                    "agentId": "a2",
                    "parentAgentId": "a1",
                    "title": "Explore",
                    "project": "bachuc",
                    "projectKey": "project:bachuc",
                    "status": "running",
                    "startedAt": "2026-10-12T11:30:00.000Z",
                    "endedAt": null,
                    "needsInput": true
                },
                { "agentId": "bad", "title": "No times" }
            ],
            "missingStart": 1,
            "olderDropped": false
        }))
        .expect("decodes");
        assert_eq!(answer.threads.len(), 2);
        assert_eq!(answer.missing_start, 1);
        assert!(!answer.older_dropped);
        let first = &answer.threads[0];
        assert_eq!(first.started_at, 1_791_799_200_000.);
        assert_eq!(first.ended_at, Some(1_791_802_800_000.));
        assert_eq!(first.task_id.as_deref(), Some("TASK-069"));
        let second = &answer.threads[1];
        assert_eq!(second.parent_agent_id.as_deref(), Some("a1"));
        assert_eq!(second.ended_at, None);
        assert!(second.needs_input);
        assert!(decode_timeline(&json!({"threads": []})).is_err());
    }
}
