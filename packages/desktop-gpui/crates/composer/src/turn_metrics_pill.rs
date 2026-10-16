//! `packages/app/src/composer/turn-metrics-pill.tsx`, with the rules of `timeline/turn-time.ts`
//! and the tones of `components/turn-metrics-text.tsx`: TTFT and the output rate of the running
//! turn, live, or of the last turn once it ends.

use crate::pill_styles::docked_pill;
use bachuc_client::TurnMetrics;
use bachuc_theme::tokens;
use gpui_kit::*;
use serde_json::Value;

/// `TURN_METRIC_SEPARATOR`: a middle dot with a space on each side.
pub const TURN_METRIC_SEPARATOR: &str = " \u{b7} ";
/// The live tick (`LIVE_TICK_MS`).
pub const LIVE_TICK_MS: u64 = 1000;
/// `ESTIMATED_CHARS_PER_TOKEN` and `MIN_LIVE_RATE_WINDOW_MS`.
const CHARS_PER_TOKEN: f64 = 4.;
const MIN_LIVE_RATE_WINDOW_MS: i64 = 1000;
/// `TTFT_GOOD_BELOW_MS`, `TTFT_FAIR_BELOW_MS`, `TPS_GOOD_FROM`, and `TPS_FAIR_FROM`.
const TTFT_GOOD_BELOW_MS: f64 = 5000.;
const TTFT_FAIR_BELOW_MS: f64 = 15000.;
const TPS_GOOD_FROM: f64 = 80.;
const TPS_FAIR_FROM: f64 = 40.;

/// `TurnMetricTone`.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Tone {
    Good,
    Fair,
    Poor,
}

/// `TurnMetricPart`.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MetricPart {
    pub text: String,
    pub tone: Tone,
}

/// `rateTtft`: lower is better.
pub fn rate_ttft(ms: f64) -> Tone {
    if ms < TTFT_GOOD_BELOW_MS {
        Tone::Good
    } else if ms < TTFT_FAIR_BELOW_MS {
        Tone::Fair
    } else {
        Tone::Poor
    }
}

/// `rateTokensPerSecond`: higher is better.
pub fn rate_tokens_per_second(rate: f64) -> Tone {
    if rate >= TPS_GOOD_FROM {
        Tone::Good
    } else if rate >= TPS_FAIR_FROM {
        Tone::Fair
    } else {
        Tone::Poor
    }
}

/// `formatTtft`: whole ms under 1 s, else seconds with one decimal.
pub fn format_ttft(ms: f64) -> String {
    if ms < 1000. {
        format!("{}ms", ms.round())
    } else {
        format!("{:.1}s", ms / 1000.)
    }
}

/// `formatTokensPerSecond`: one decimal under 10, else a whole number.
pub fn format_rate(rate: f64) -> String {
    if rate < 10. {
        format!("{rate:.1}")
    } else {
        format!("{}", rate.round())
    }
}

/// `buildTurnMetricParts`: "TTFT 1.2s" and "45 tok/s"; `estimated` puts "~" before the rate.
pub fn metric_parts(metrics: &TurnMetrics, estimated: bool) -> Vec<MetricPart> {
    let mut parts = Vec::new();
    if let Some(ttft) = metrics.ttft_ms {
        parts.push(MetricPart {
            text: format!("TTFT {}", format_ttft(ttft)),
            tone: rate_ttft(ttft),
        });
    }
    if let Some(rate) = metrics.output_tokens_per_second {
        let prefix = if estimated { "~" } else { "" };
        parts.push(MetricPart {
            text: format!("{prefix}{} tok/s", format_rate(rate)),
            tone: rate_tokens_per_second(rate),
        });
    }
    parts
}

/// `ComposerTurnMetrics`.
#[derive(Clone, Debug, PartialEq)]
pub enum ComposerTurnMetrics {
    Waiting {
        elapsed_ms: i64,
    },
    Streaming {
        metrics: TurnMetrics,
        first_output_at: i64,
    },
    Completed(TurnMetrics),
}

/// `buildComposerTurnMetricParts`. While waiting, the counting TTFT takes the tone it would
/// have if the first token came now.
pub fn composer_parts(value: Option<&ComposerTurnMetrics>) -> Vec<MetricPart> {
    match value {
        None => Vec::new(),
        Some(ComposerTurnMetrics::Waiting { elapsed_ms }) => vec![MetricPart {
            text: format!("TTFT {}s\u{2026}", elapsed_ms / 1000),
            tone: rate_ttft(*elapsed_ms as f64),
        }],
        Some(ComposerTurnMetrics::Streaming { metrics, .. }) => metric_parts(metrics, true),
        Some(ComposerTurnMetrics::Completed(metrics)) => metric_parts(metrics, false),
    }
}

/// The 1 s tick runs only while a turn is active.
pub fn live_tick(turn_active: bool) -> bool {
    turn_active
}

/// An ISO time in Unix ms.
pub fn parse_time_ms(text: &str) -> Option<i64> {
    chrono::DateTime::parse_from_rfc3339(text)
        .ok()
        .map(|time| time.timestamp_millis())
}

fn item_type(entry: &Value) -> &str {
    entry["item"]["type"].as_str().unwrap_or_default()
}

/// The entries of the active turn: scans back from the end and stops at the first entry of
/// another turn, or, for entries without a turn id, at the user message.
pub fn active_turn_items<'a>(entries: &'a [Value], turn_id: &str) -> &'a [Value] {
    let mut start = entries.len();
    while start > 0 {
        let entry = &entries[start - 1];
        let in_turn = match entry["turnId"].as_str() {
            Some(id) => id == turn_id,
            None => item_type(entry) != "user_message",
        };
        if !in_turn {
            break;
        }
        start -= 1;
    }
    &entries[start..]
}

/// `isModelOutput`.
fn is_model_output(entry: &Value) -> bool {
    match item_type(entry) {
        "tool_call" => true,
        "assistant_message" | "thought" => entry["item"]["text"]
            .as_str()
            .is_some_and(|text| !text.is_empty()),
        _ => false,
    }
}

/// `deriveComposerTurnMetrics` for an active turn. `seen_first_output` is the earliest first
/// output time already seen for this turn: a merged entry carries its latest chunk time, so
/// the first-output time would drift as text streams.
pub fn live_metrics(
    turn_items: &[Value],
    started_at_ms: i64,
    now_ms: i64,
    seen_first_output: Option<i64>,
) -> ComposerTurnMetrics {
    let Some(first) = turn_items.iter().find(|entry| is_model_output(entry)) else {
        return ComposerTurnMetrics::Waiting {
            elapsed_ms: (now_ms - started_at_ms).max(0),
        };
    };
    let first_at = entry_time(first).unwrap_or(now_ms);
    let first_at = seen_first_output.map_or(first_at, |seen| seen.min(first_at));
    let mut metrics = TurnMetrics {
        ttft_ms: Some((first_at - started_at_ms).max(0) as f64),
        output_tokens: None,
        output_tokens_per_second: None,
    };
    let window_ms = now_ms - first_at;
    if window_ms >= MIN_LIVE_RATE_WINDOW_MS {
        // `text.length` counts UTF-16 units.
        let chars: usize = turn_items
            .iter()
            .filter(|entry| matches!(item_type(entry), "assistant_message" | "thought"))
            .map(|entry| {
                entry["item"]["text"]
                    .as_str()
                    .map_or(0, |text| text.encode_utf16().count())
            })
            .sum();
        metrics.output_tokens_per_second =
            Some(chars as f64 / CHARS_PER_TOKEN / (window_ms as f64 / 1000.));
    }
    ComposerTurnMetrics::Streaming {
        metrics,
        first_output_at: first_at,
    }
}

fn entry_time(entry: &Value) -> Option<i64> {
    parse_time_ms(entry["timestamp"].as_str()?)
}

/// The pill; `None` without parts.
pub fn turn_metrics_pill(parts: &[MetricPart], cx: &App) -> Option<AnyElement> {
    if parts.is_empty() {
        return None;
    }
    let t = tokens(cx);
    let label = parts
        .iter()
        .map(|part| part.text.as_str())
        .collect::<Vec<_>>()
        .join(TURN_METRIC_SEPARATOR);
    let mut runs = Vec::new();
    for (index, part) in parts.iter().enumerate() {
        if index > 0 {
            runs.push(div().child(TURN_METRIC_SEPARATOR).into_any_element());
        }
        let color = match part.tone {
            Tone::Good => t.status_success,
            Tone::Fair => t.status_warning,
            Tone::Poor => t.status_danger,
        };
        runs.push(
            div()
                .text_color(color)
                .child(part.text.clone())
                .into_any_element(),
        );
    }
    Some(
        docked_pill("composer-turn-metrics-pill", false, cx)
            .role(Role::Status)
            .aria_label(label)
            .whitespace_nowrap()
            .gap_0()
            .children(runs)
            .into_any_element(),
    )
}

#[cfg(test)]
mod tests {
    // Named imports: `use super::*` would bring in GPUI's `test` macro over `#[test]`.
    use super::{
        ComposerTurnMetrics, MetricPart, TURN_METRIC_SEPARATOR, Tone, active_turn_items,
        composer_parts, live_metrics, live_tick, metric_parts, parse_time_ms,
        rate_tokens_per_second, rate_ttft,
    };
    use bachuc_client::TurnMetrics;
    use serde_json::{Value, json};

    fn metrics(ttft: Option<f64>, rate: Option<f64>) -> TurnMetrics {
        TurnMetrics {
            ttft_ms: ttft,
            output_tokens: None,
            output_tokens_per_second: rate,
        }
    }

    fn texts(parts: &[MetricPart]) -> Vec<&str> {
        parts.iter().map(|part| part.text.as_str()).collect()
    }

    fn entry(kind: &str, text: &str, time: &str, turn: Option<&str>) -> Value {
        let mut entry = json!({
            "item": {"type": kind, "text": text},
            "timestamp": format!("2026-10-16T10:00:{time}Z"),
        });
        if let Some(turn) = turn {
            entry["turnId"] = json!(turn);
        }
        entry
    }

    const START: &str = "2026-10-16T10:00:00Z";

    #[test]
    fn metrics_format_ttft_and_rate() {
        let parts = metric_parts(&metrics(Some(1200.), Some(45.)), false);
        assert_eq!(texts(&parts), ["TTFT 1.2s", "45 tok/s"]);
        assert_eq!(
            texts(&metric_parts(&metrics(Some(840.4), Some(7.26)), true)),
            ["TTFT 840ms", "~7.3 tok/s"]
        );
        assert!(metric_parts(&metrics(None, None), false).is_empty());
        assert_eq!(TURN_METRIC_SEPARATOR, " \u{b7} ");
    }

    #[test]
    fn tones_follow_the_thresholds() {
        assert_eq!(rate_ttft(4999.), Tone::Good);
        assert_eq!(rate_ttft(5000.), Tone::Fair);
        assert_eq!(rate_ttft(14999.), Tone::Fair);
        assert_eq!(rate_ttft(15000.), Tone::Poor);
        assert_eq!(rate_tokens_per_second(80.), Tone::Good);
        assert_eq!(rate_tokens_per_second(79.9), Tone::Fair);
        assert_eq!(rate_tokens_per_second(40.), Tone::Fair);
        assert_eq!(rate_tokens_per_second(39.9), Tone::Poor);
    }

    #[test]
    fn a_turn_with_no_output_counts_the_wait_with_an_ellipsis() {
        let started = parse_time_ms(START).unwrap();
        let items = [entry("user_message", "hi", "00", Some("t1"))];
        let value = live_metrics(&items, started, started + 3400, None);
        assert_eq!(value, ComposerTurnMetrics::Waiting { elapsed_ms: 3400 });
        let parts = composer_parts(Some(&value));
        assert_eq!(texts(&parts), ["TTFT 3s\u{2026}"]);
        assert_eq!(parts[0].tone, Tone::Good);
    }

    #[test]
    fn the_live_rate_waits_for_a_one_second_window() {
        let started = parse_time_ms(START).unwrap();
        let first = started + 1000;
        // 400 characters is 100 estimated tokens.
        let items = [
            entry("user_message", "hi", "00", Some("t1")),
            entry("assistant_message", &"a".repeat(400), "01", Some("t1")),
        ];
        let early = live_metrics(&items, started, first + 999, None);
        assert_eq!(texts(&composer_parts(Some(&early))), ["TTFT 1.0s"]);
        let later = live_metrics(&items, started, first + 2000, None);
        assert_eq!(
            later,
            ComposerTurnMetrics::Streaming {
                metrics: metrics(Some(1000.), Some(50.)),
                first_output_at: first,
            }
        );
        assert_eq!(
            texts(&composer_parts(Some(&later))),
            ["TTFT 1.0s", "~50 tok/s"]
        );
    }

    #[test]
    fn an_earlier_first_output_is_kept() {
        let started = parse_time_ms(START).unwrap();
        // The merged entry now carries the 05 s chunk time; 02 s was seen first.
        let items = [entry("thought", "thinking", "05", Some("t1"))];
        let seen = started + 2000;
        let value = live_metrics(&items, started, started + 6000, Some(seen));
        let ComposerTurnMetrics::Streaming {
            metrics,
            first_output_at,
        } = value
        else {
            panic!("streaming")
        };
        assert_eq!(first_output_at, seen);
        assert_eq!(metrics.ttft_ms, Some(2000.));
    }

    #[test]
    fn the_scan_stops_at_the_active_turn_start() {
        let entries = [
            entry("assistant_message", "old", "00", Some("t0")),
            entry("user_message", "hi", "01", Some("t1")),
            entry("tool_call", "", "02", Some("t1")),
        ];
        assert_eq!(active_turn_items(&entries, "t1").len(), 2);
        let untagged = [
            entry("assistant_message", "old", "00", None),
            entry("user_message", "hi", "01", None),
            entry("assistant_message", "new", "02", None),
        ];
        assert_eq!(active_turn_items(&untagged, "t9").len(), 1);
    }

    #[test]
    fn the_tick_is_off_without_an_active_turn() {
        assert!(!live_tick(false));
        assert!(live_tick(true));
    }

    #[test]
    fn a_finished_turn_shows_its_metrics_without_the_estimate_mark() {
        let value = ComposerTurnMetrics::Completed(metrics(Some(1200.), Some(45.)));
        assert_eq!(
            texts(&composer_parts(Some(&value))),
            ["TTFT 1.2s", "45 tok/s"]
        );
        assert!(composer_parts(None).is_empty());
    }
}
