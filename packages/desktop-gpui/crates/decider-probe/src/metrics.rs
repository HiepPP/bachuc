#[derive(PartialEq, Eq, Debug, Clone, Copy)]
pub enum Decision {
    Allow,
    Deny,
    Escalate,
}

impl Decision {
    pub const ALL: [Decision; 3] = [Decision::Allow, Decision::Deny, Decision::Escalate];

    pub fn parse(text: &str) -> Option<Decision> {
        match text {
            "allow" => Some(Decision::Allow),
            "deny" => Some(Decision::Deny),
            "escalate" => Some(Decision::Escalate),
            _ => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Decision::Allow => "allow",
            Decision::Deny => "deny",
            Decision::Escalate => "escalate",
        }
    }
}

/// Replays `fromVerdict` in `jev-permission-gate/server/policy.ts`: deny at P >= t, allow when
/// the action and `readOnly` both reach t and `readOnly` is true (`p_true` >= 0.5), else escalate.
pub fn gate_decision(read_only_p_true: f64, action: Decision, action_p: f64, t: f64) -> Decision {
    if action == Decision::Deny && action_p >= t {
        return Decision::Deny;
    }
    if action == Decision::Allow
        && action_p >= t
        && read_only_p_true >= 0.5
        && read_only_p_true >= t
    {
        return Decision::Allow;
    }
    Decision::Escalate
}

/// Nearest-rank percentile. Sorts `values`; returns NaN for an empty slice.
pub fn percentile(values: &mut [f64], p: f64) -> f64 {
    if values.is_empty() {
        return f64::NAN;
    }
    values.sort_by(|a, b| a.total_cmp(b));
    let rank = ((p / 100.0) * values.len() as f64).ceil() as usize;
    values[rank.clamp(1, values.len()) - 1]
}

#[derive(Debug, Clone, PartialEq)]
pub struct Bin {
    pub count: usize,
    pub mean_confidence: f64,
    pub accuracy: f64,
}

/// Groups `(confidence, correct)` points into equal-width bins over [0, 1]. Empty bins are `None`.
pub fn calibration_bins(points: &[(f64, bool)], bins: usize) -> Vec<Option<Bin>> {
    let mut sums = vec![(0usize, 0.0f64, 0usize); bins];
    for &(confidence, correct) in points {
        let index = ((confidence * bins as f64) as usize).min(bins - 1);
        let slot = &mut sums[index];
        slot.0 += 1;
        slot.1 += confidence;
        slot.2 += usize::from(correct);
    }
    sums.into_iter()
        .map(|(count, confidence, correct)| {
            (count > 0).then(|| Bin {
                count,
                mean_confidence: confidence / count as f64,
                accuracy: correct as f64 / count as f64,
            })
        })
        .collect()
}

/// Expected calibration error over `(confidence, correct)` points.
pub fn ece(points: &[(f64, bool)], bins: usize) -> f64 {
    if points.is_empty() {
        return 0.0;
    }
    let total = points.len() as f64;
    calibration_bins(points, bins)
        .into_iter()
        .flatten()
        .map(|bin| bin.count as f64 / total * (bin.accuracy - bin.mean_confidence).abs())
        .sum()
}

#[derive(Debug, Clone, PartialEq)]
pub struct SweepRow {
    pub auto_resolved: usize,
    pub precision: Option<f64>,
}

/// Each pair is (predicted, label). A prediction other than escalate counts as auto-resolved.
pub fn sweep(pairs: &[(Decision, Decision)]) -> SweepRow {
    let resolved: Vec<_> = pairs
        .iter()
        .filter(|(predicted, _)| *predicted != Decision::Escalate)
        .collect();
    let correct = resolved.iter().filter(|(p, l)| p == l).count();
    SweepRow {
        auto_resolved: resolved.len(),
        precision: (!resolved.is_empty()).then(|| correct as f64 / resolved.len() as f64),
    }
}

/// Precision and recall of one class over (predicted, label) pairs. `None` when undefined.
pub fn class_precision_recall(
    pairs: &[(Decision, Decision)],
    class: Decision,
) -> (Option<f64>, Option<f64>) {
    let predicted = pairs.iter().filter(|(p, _)| *p == class).count();
    let actual = pairs.iter().filter(|(_, l)| *l == class).count();
    let hits = pairs
        .iter()
        .filter(|(p, l)| *p == class && *l == class)
        .count();
    (
        (predicted > 0).then(|| hits as f64 / predicted as f64),
        (actual > 0).then(|| hits as f64 / actual as f64),
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn allow_needs_both_answers_over_threshold() {
        assert_eq!(
            gate_decision(0.95, Decision::Allow, 0.93, 0.9),
            Decision::Allow
        );
        assert_eq!(
            gate_decision(0.85, Decision::Allow, 0.93, 0.9),
            Decision::Escalate
        );
        assert_eq!(
            gate_decision(0.95, Decision::Allow, 0.80, 0.9),
            Decision::Escalate
        );
    }

    #[test]
    fn read_only_false_never_allows() {
        assert_eq!(
            gate_decision(0.02, Decision::Allow, 0.99, 0.5),
            Decision::Escalate
        );
    }

    #[test]
    fn deny_needs_only_action_over_threshold() {
        assert_eq!(
            gate_decision(0.99, Decision::Deny, 0.91, 0.9),
            Decision::Deny
        );
        assert_eq!(
            gate_decision(0.99, Decision::Deny, 0.89, 0.9),
            Decision::Escalate
        );
    }

    #[test]
    fn percentile_uses_nearest_rank() {
        let mut v = vec![5.0, 1.0, 3.0, 2.0, 4.0];
        assert_eq!(percentile(&mut v, 50.0), 3.0);
        assert_eq!(percentile(&mut v, 95.0), 5.0);
    }

    #[test]
    fn ece_is_zero_when_calibrated_and_positive_when_not() {
        let calibrated = [(0.9, true); 9]
            .into_iter()
            .chain([(0.9, false)])
            .collect::<Vec<_>>();
        assert!(ece(&calibrated, 5) < 1e-9);
        let over = vec![(0.9, false), (0.9, false)];
        assert!((ece(&over, 5) - 0.9).abs() < 1e-9);
    }

    #[test]
    fn sweep_reports_none_precision_when_nothing_resolves() {
        let row = sweep(&[(Decision::Escalate, Decision::Allow)]);
        assert_eq!(row.auto_resolved, 0);
        assert_eq!(row.precision, None);
    }
}
