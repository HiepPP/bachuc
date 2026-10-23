use crate::metrics::{
    Decision, calibration_bins, class_precision_recall, ece, gate_decision, percentile, sweep,
};
use crate::{Event, ResultLine, read_jsonl};
use serde_json::Value;
use std::collections::{HashMap, HashSet};
use std::fmt::Write as _;
use std::fs;
use std::path::Path;

const PRODUCTION_T: f64 = 0.9;
const SWEEP: [f64; 11] = [
    0.50, 0.55, 0.60, 0.65, 0.70, 0.75, 0.80, 0.85, 0.90, 0.95, 0.99,
];
const BUCKETS: [&str; 5] = ["<150", "150-400", "400-1000", ">1000", "unknown"];

struct Label {
    label: Decision,
    read_only: bool,
}

/// One labeled event with a complete, error-free answer pair from one backend.
struct Row {
    id: String,
    command: String,
    tier0: Option<Decision>,
    label: Decision,
    read_only: bool,
    p_true: f64,
    action: Decision,
    action_p: f64,
}

impl Row {
    fn model(&self, t: f64) -> Decision {
        gate_decision(self.p_true, self.action, self.action_p, t)
    }

    fn full(&self, t: f64) -> Decision {
        self.tier0.unwrap_or_else(|| self.model(t))
    }

    fn regex(&self) -> Decision {
        self.tier0.unwrap_or(Decision::Escalate)
    }
}

struct Backend {
    name: &'static str,
    lines: Vec<ResultLine>,
    rows: Vec<Row>,
    skipped_results: usize,
    labels_without_result: usize,
    errored: usize,
}

impl Backend {
    fn model_rows(&self) -> Vec<&Row> {
        self.rows.iter().filter(|r| r.tier0.is_none()).collect()
    }

    fn full_pairs(&self, t: f64) -> Vec<(Decision, Decision)> {
        self.rows.iter().map(|r| (r.full(t), r.label)).collect()
    }

    fn regex_pairs(&self) -> Vec<(Decision, Decision)> {
        self.rows.iter().map(|r| (r.regex(), r.label)).collect()
    }
}

fn pct(part: usize, whole: usize) -> String {
    if whole == 0 {
        "n/a".to_string()
    } else {
        format!("{:.1}%", part as f64 * 100.0 / whole as f64)
    }
}

fn num(value: Option<f64>) -> String {
    value.map_or("n/a".to_string(), |v| format!("{v:.3}"))
}

fn ms(value: Option<f64>) -> String {
    value.map_or("n/a".to_string(), |v| format!("{v:.0}"))
}

fn load_labels(path: &Path) -> Result<HashMap<String, Label>, String> {
    let mut labels = HashMap::new();
    for line in read_jsonl::<Value>(path)? {
        let id = line["id"].as_str();
        let label = line["label"].as_str().and_then(Decision::parse);
        let read_only = line["read_only"].as_bool();
        if let (Some(id), Some(label), Some(read_only)) = (id, label, read_only) {
            labels.insert(id.to_string(), Label { label, read_only });
        }
    }
    Ok(labels)
}

fn build_backend(
    name: &'static str,
    all: &[ResultLine],
    labels: &HashMap<String, Label>,
    events: &HashMap<String, Event>,
) -> Backend {
    let mine: Vec<&ResultLine> = all.iter().filter(|line| line.backend == name).collect();
    let skipped_results = mine
        .iter()
        .map(|line| line.id.as_str())
        .filter(|id| !labels.contains_key(*id))
        .collect::<HashSet<_>>()
        .len();
    let lines: Vec<ResultLine> = mine
        .into_iter()
        .filter(|line| labels.contains_key(&line.id))
        .cloned()
        .collect();
    let errored = lines.iter().filter(|line| line.error.is_some()).count();

    // The last error-free answer per event and question wins, so a retried line replaces an old one.
    let mut read_only = HashMap::new();
    let mut action = HashMap::new();
    for line in lines.iter().filter(|line| line.error.is_none()) {
        if let Some(answer) = &line.answers.read_only {
            read_only.insert(line.id.clone(), answer.p_true);
        }
        if let Some(answer) = &line.answers.action {
            action.insert(line.id.clone(), answer.clone());
        }
    }
    let mut rows = Vec::new();
    for (id, label) in labels {
        let (Some(&p_true), Some(answer)) = (read_only.get(id), action.get(id)) else {
            continue;
        };
        let Some(choice) = Decision::parse(&answer.choice) else {
            continue;
        };
        let Some(&action_p) = answer.probabilities.get(&answer.choice) else {
            continue;
        };
        let event = events.get(id);
        rows.push(Row {
            id: id.clone(),
            command: event
                .and_then(|e| e.state["command"].as_str())
                .unwrap_or_default()
                .to_string(),
            tier0: event
                .and_then(|e| e.meta["tier0"].as_str())
                .and_then(Decision::parse),
            label: label.label,
            read_only: label.read_only,
            p_true,
            action: choice,
            action_p,
        });
    }
    rows.sort_by(|a, b| a.id.cmp(&b.id));
    let labels_without_result = labels.len() - rows.len();
    Backend {
        name,
        lines,
        rows,
        skipped_results,
        labels_without_result,
        errored,
    }
}

fn bucket(tokens: Option<u64>) -> &'static str {
    match tokens {
        None => "unknown",
        Some(t) if t < 150 => "<150",
        Some(t) if t <= 400 => "150-400",
        Some(t) if t <= 1000 => "400-1000",
        Some(_) => ">1000",
    }
}

fn stat(mut values: Vec<f64>, p: f64) -> Option<f64> {
    (!values.is_empty()).then(|| percentile(&mut values, p))
}

fn latency(out: &mut String, backends: &[Backend]) {
    out.push_str(
        "## 1. Latency\n\nMilliseconds per request, nearest-rank percentiles. \
         The warm-up request is not logged.\n\n",
    );
    for backend in backends {
        let _ = writeln!(out, "### {}\n", backend.name);
        out.push_str(
            "| Input tokens | n | client p50 | client p95 | server p50 | server p95 |\n\
             |---|---|---|---|---|---|\n",
        );
        for name in BUCKETS {
            let lines: Vec<&ResultLine> = backend
                .lines
                .iter()
                .filter(|l| l.error.is_none() && bucket(l.input_tokens) == name)
                .collect();
            if lines.is_empty() {
                continue;
            }
            let client = |p| stat(lines.iter().map(|l| l.client_ms).collect(), p);
            let server = |p| {
                stat(
                    lines
                        .iter()
                        .filter_map(|l| l.server_ms.map(|v| v as f64))
                        .collect(),
                    p,
                )
            };
            let _ = writeln!(
                out,
                "| {name} | {} | {} | {} | {} | {} |",
                lines.len(),
                ms(client(50.0)),
                ms(client(95.0)),
                ms(server(50.0)),
                ms(server(95.0))
            );
        }
        out.push('\n');
    }
}

fn accuracy(out: &mut String, backends: &[Backend]) {
    let _ = writeln!(out, "## 2. Accuracy at t = {PRODUCTION_T}\n");
    for backend in backends {
        let _ = writeln!(out, "### {}\n", backend.name);
        let views: [(&str, Vec<(Decision, Decision)>); 2] = [
            (
                "model only (tier0 null)",
                backend
                    .model_rows()
                    .iter()
                    .map(|r| (r.model(PRODUCTION_T), r.label))
                    .collect(),
            ),
            ("full gate (tier0 wins)", backend.full_pairs(PRODUCTION_T)),
        ];
        out.push_str(
            "| View | n | accuracy | allow P/R | deny P/R | escalate P/R |\n\
             |---|---|---|---|---|---|\n",
        );
        for (name, pairs) in &views {
            let right = pairs.iter().filter(|(p, l)| p == l).count();
            let cells: Vec<String> = Decision::ALL
                .iter()
                .map(|&class| {
                    let (p, r) = class_precision_recall(pairs, class);
                    format!("{} / {}", num(p), num(r))
                })
                .collect();
            let _ = writeln!(
                out,
                "| {name} | {} | {} | {} | {} | {} |",
                pairs.len(),
                pct(right, pairs.len()),
                cells[0],
                cells[1],
                cells[2]
            );
        }
        out.push('\n');
    }
}

fn calibration_table(out: &mut String, title: &str, points: &[(f64, bool)]) {
    let _ = writeln!(
        out,
        "{title}: ECE {:.3}, n = {}\n",
        ece(points, 5),
        points.len()
    );
    out.push_str("| Confidence bin | n | mean confidence | accuracy |\n|---|---|---|---|\n");
    for (index, bin) in calibration_bins(points, 5).into_iter().enumerate() {
        let range = format!("{:.1}-{:.1}", index as f64 * 0.2, (index + 1) as f64 * 0.2);
        match bin {
            Some(bin) => {
                let _ = writeln!(
                    out,
                    "| {range} | {} | {:.3} | {:.3} |",
                    bin.count, bin.mean_confidence, bin.accuracy
                );
            }
            None => {
                let _ = writeln!(out, "| {range} | 0 | n/a | n/a |");
            }
        }
    }
    out.push('\n');
}

fn calibration(out: &mut String, backends: &[Backend]) {
    out.push_str(
        "## 3. Calibration\n\nConfidence is the probability of the answer given: \
         `max(p, 1 - p)` for `readOnly`, `probabilities[choice]` for `action`.\n\n",
    );
    for backend in backends {
        let _ = writeln!(out, "### {}\n", backend.name);
        let read_only: Vec<(f64, bool)> = backend
            .rows
            .iter()
            .map(|r| {
                (
                    r.p_true.max(1.0 - r.p_true),
                    (r.p_true >= 0.5) == r.read_only,
                )
            })
            .collect();
        let action: Vec<(f64, bool)> = backend
            .rows
            .iter()
            .map(|r| (r.action_p, r.action == r.label))
            .collect();
        calibration_table(out, "readOnly", &read_only);
        calibration_table(out, "action", &action);
    }
}

/// Lowest threshold in the sweep whose precision reaches 0.95.
fn lowest_t(precisions: &[(f64, Option<f64>)]) -> String {
    precisions
        .iter()
        .find(|(_, p)| p.is_some_and(|p| p >= 0.95))
        .map_or("none".to_string(), |(t, _)| format!("{t:.2}"))
}

fn threshold_sweep(out: &mut String, backends: &[Backend]) {
    out.push_str(
        "## 4. Threshold sweep\n\nAuto-resolve is the share of events that get allow or deny. \
         `n/a` means no event was auto-resolved.\n\n",
    );
    for backend in backends {
        let _ = writeln!(out, "### {}\n", backend.name);
        out.push_str(
            "| t | model auto-resolve | model precision | full auto-resolve | full precision |\n\
             |---|---|---|---|---|\n",
        );
        let model_rows = backend.model_rows();
        let mut model_precision = Vec::new();
        let mut full_precision = Vec::new();
        for t in SWEEP {
            let model_pairs: Vec<_> = model_rows.iter().map(|r| (r.model(t), r.label)).collect();
            let model = sweep(&model_pairs);
            let full = sweep(&backend.full_pairs(t));
            model_precision.push((t, model.precision));
            full_precision.push((t, full.precision));
            let _ = writeln!(
                out,
                "| {t:.2} | {} | {} | {} | {} |",
                pct(model.auto_resolved, model_rows.len()),
                num(model.precision),
                pct(full.auto_resolved, backend.rows.len()),
                num(full.precision)
            );
        }
        let _ = writeln!(
            out,
            "\nLowest t with precision >= 0.95: model only {}, full gate {}.\n",
            lowest_t(&model_precision),
            lowest_t(&full_precision)
        );
    }
}

fn cell(text: &str) -> String {
    let flat: String = text
        .chars()
        .map(|c| if c == '\n' || c == '|' { ' ' } else { c })
        .collect();
    let short: String = flat.chars().take(90).collect();
    format!("`{}`", short.replace('`', "'"))
}

fn confident_wrong(out: &mut String, backends: &[Backend]) {
    out.push_str(
        "## 5. Most confident wrong answers\n\nThe 10 `action` answers that disagree with \
         the label, by action probability.\n\n",
    );
    for backend in backends {
        let _ = writeln!(out, "### {}\n", backend.name);
        let mut wrong: Vec<&Row> = backend
            .rows
            .iter()
            .filter(|r| r.action != r.label)
            .collect();
        wrong.sort_by(|a, b| b.action_p.total_cmp(&a.action_p));
        if wrong.is_empty() {
            out.push_str("No wrong action answers.\n\n");
            continue;
        }
        out.push_str("| id | label | answer | P | command |\n|---|---|---|---|---|\n");
        for row in wrong.into_iter().take(10) {
            let _ = writeln!(
                out,
                "| {} | {} | {} | {:.3} | {} |",
                row.id,
                row.label.as_str(),
                row.action.as_str(),
                row.action_p,
                cell(&row.command)
            );
        }
        out.push('\n');
    }
}

struct Verdict {
    passes: Option<bool>,
    detail: String,
}

fn verdict_cell(verdict: &Verdict) -> String {
    let word = match verdict.passes {
        Some(true) => "PASS",
        Some(false) => "FAIL",
        None => "no data",
    };
    format!("{word} ({})", verdict.detail)
}

fn criteria(backend: &Backend) -> [Verdict; 3] {
    let n = backend.rows.len();
    let full = sweep(&backend.full_pairs(PRODUCTION_T));
    let regex = sweep(&backend.regex_pairs());
    let precision_ok = full.precision.is_some_and(|p| p >= 0.95);
    let one = Verdict {
        passes: (n > 0).then_some(precision_ok && full.auto_resolved as f64 >= n as f64 * 0.5),
        detail: format!(
            "auto {}, precision {}",
            pct(full.auto_resolved, n),
            num(full.precision)
        ),
    };
    let two = Verdict {
        passes: (n > 0).then_some(precision_ok && full.auto_resolved > regex.auto_resolved),
        detail: format!(
            "gate {} vs regex {}, precision {}",
            full.auto_resolved,
            regex.auto_resolved,
            num(full.precision)
        ),
    };
    let server = stat(
        backend
            .lines
            .iter()
            .filter(|l| l.error.is_none() && l.input_tokens.is_some_and(|t| t <= 400))
            .filter_map(|l| l.server_ms.map(|v| v as f64))
            .collect(),
        95.0,
    );
    let three = Verdict {
        passes: server.map(|v| v <= 2000.0),
        detail: format!("server p95 {} ms", ms(server)),
    };
    [one, two, three]
}

fn baseline(out: &mut String, backends: &[Backend]) {
    out.push_str("## 6. Regex baseline and go/no-go\n\n");
    let _ = writeln!(
        out,
        "Regex alone is `tier0` with every null event escalated. Both rows use the labeled \
         events of that backend, at t = {PRODUCTION_T}.\n"
    );
    out.push_str("| Backend events | Gate | auto-resolved | precision |\n|---|---|---|---|\n");
    for backend in backends {
        let n = backend.rows.len();
        let regex = sweep(&backend.regex_pairs());
        let full = sweep(&backend.full_pairs(PRODUCTION_T));
        for (gate, row) in [("regex alone", regex), ("full gate", full)] {
            let _ = writeln!(
                out,
                "| {} (n = {n}) | {gate} | {} ({}) | {} |",
                backend.name,
                row.auto_resolved,
                pct(row.auto_resolved, n),
                num(row.precision)
            );
        }
    }
    out.push_str("\n| Criterion | decider | jev |\n|---|---|---|\n");
    let names = [
        "1. Full gate auto-resolves >= 50% at precision >= 0.95",
        "2. Full gate auto-resolves more than regex alone, precision >= 0.95",
        "3. Server p95 <= 2000 ms for state <= 400 tokens",
    ];
    let verdicts: Vec<[Verdict; 3]> = backends.iter().map(criteria).collect();
    for (index, name) in names.iter().enumerate() {
        let cells: Vec<String> = verdicts.iter().map(|v| verdict_cell(&v[index])).collect();
        let _ = writeln!(out, "| {name} | {} |", cells.join(" | "));
    }
    out.push_str(
        "\nCriterion 3 applies to Decider only; Jev is hosted and has no server latency.\n",
    );
}

pub fn write_report(data: &Path) -> Result<(), String> {
    let labels = load_labels(&data.join("labels.jsonl"))?;
    let events: HashMap<String, Event> = read_jsonl::<Event>(&data.join("events.jsonl"))?
        .into_iter()
        .map(|event| (event.id.clone(), event))
        .collect();
    let mut results = read_jsonl::<ResultLine>(&data.join("results-decider.jsonl"))?;
    results.extend(read_jsonl::<ResultLine>(&data.join("results-jev.jsonl"))?);
    let backends = [
        build_backend("decider", &results, &labels, &events),
        build_backend("jev", &results, &labels, &events),
    ];

    let mut out = String::from("# Decider probe report\n\n");
    for backend in &backends {
        let summary = format!(
            "{}: {} scored events, skipped {} result ids without a label, \
             {} labels without a complete result, {} errored lines",
            backend.name,
            backend.rows.len(),
            backend.skipped_results,
            backend.labels_without_result,
            backend.errored
        );
        println!("{summary}");
        let _ = writeln!(out, "- {summary}");
    }
    out.push('\n');
    latency(&mut out, &backends);
    accuracy(&mut out, &backends);
    calibration(&mut out, &backends);
    threshold_sweep(&mut out, &backends);
    confident_wrong(&mut out, &backends);
    baseline(&mut out, &backends);

    let path = data.join("report.md");
    fs::write(&path, out).map_err(|error| format!("{}: {error}", path.display()))?;
    println!("wrote {}", path.display());
    Ok(())
}
