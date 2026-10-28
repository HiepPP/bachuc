//! The labels an answer line can start with, drawn as tint chips by role (ADR-0076). Only theme
//! tokens color them, so a theme change, plugin themes included, recolors them.

use std::sync::Arc;

use bachuc_theme::Tokens;
use gpui_kit::base::text::LeadLabel;

#[derive(Clone, Copy)]
enum Role {
    Report,
    Warning,
    Danger,
    Success,
}

/// The labels of the global answer rules, with the Vietnamese forms agents write.
const LABELS: [(&str, Role); 28] = [
    ("What I did:", Role::Report),
    ("What happened:", Role::Report),
    ("Why it matters:", Role::Report),
    ("Did:", Role::Report),
    ("Những gì đã làm:", Role::Report),
    ("Chuyện gì đã xảy ra:", Role::Report),
    ("Vì sao quan trọng:", Role::Report),
    ("Đã làm:", Role::Report),
    ("Changes:", Role::Report),
    ("Evidence:", Role::Report),
    ("Thay đổi:", Role::Report),
    ("Bằng chứng:", Role::Report),
    ("Symptom:", Role::Warning),
    ("Unverified:", Role::Warning),
    ("Open:", Role::Warning),
    ("Triệu chứng:", Role::Warning),
    ("Chưa xác nhận:", Role::Warning),
    ("Còn lại:", Role::Warning),
    ("Not cleaned:", Role::Warning),
    ("Chưa dọn dẹp:", Role::Warning),
    ("Cause:", Role::Danger),
    ("Not fixed:", Role::Danger),
    ("Need from you:", Role::Danger),
    ("Nguyên nhân:", Role::Danger),
    ("Chưa sửa:", Role::Danger),
    ("Cần bạn:", Role::Danger),
    ("Fix:", Role::Success),
    ("Cách sửa:", Role::Success),
];

pub(crate) fn answer_labels(t: &Tokens) -> Arc<[LeadLabel]> {
    LABELS
        .iter()
        .map(|&(text, role)| {
            let (color, background) = match role {
                Role::Report => (t.status_merged, t.tint_merged),
                Role::Warning => (t.status_warning, t.tint_warning),
                Role::Danger => (t.status_danger, t.tint_danger),
                Role::Success => (t.status_success, t.tint_success),
            };
            LeadLabel {
                text: text.into(),
                color,
                background,
            }
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use gpui_kit::base::text::match_lead_label;

    use super::*;

    fn matched(chunks: &[&str]) -> Option<String> {
        let labels = answer_labels(&Tokens::light());
        match_lead_label(chunks.iter().copied(), &labels).map(|label| label.text.to_string())
    }

    #[test]
    fn a_label_matches_only_whole_at_the_start_of_the_text() {
        assert_eq!(matched(&["Fix: set the flag"]).as_deref(), Some("Fix:"));
        // `**Fix:** x` renders as separate text runs.
        assert_eq!(matched(&["Fi", "x:", " split"]).as_deref(), Some("Fix:"));
        assert_eq!(matched(&["Fix:"]).as_deref(), Some("Fix:"));
        assert_eq!(matched(&["Fix:", "", " later"]).as_deref(), Some("Fix:"));
        assert_eq!(
            matched(&["Chưa xác nhận: x"]).as_deref(),
            Some("Chưa xác nhận:")
        );
        assert_eq!(matched(&["Đã làm: x"]).as_deref(), Some("Đã làm:"));
        for text in [
            "Fixes: x",
            "fix: x",
            "Fix:x",
            "The Fix: x",
            "Fi",
            "",
            "Did:x",
        ] {
            assert_eq!(matched(&[text]), None, "{text}");
        }
        assert!(match_lead_label(["Fix: x"].into_iter(), &[]).is_none());
    }

    #[test]
    fn the_longest_matching_label_wins() {
        let label = |text: &str| LeadLabel {
            text: text.to_string().into(),
            color: gpui_kit::red(),
            background: gpui_kit::blue(),
        };
        // Both match: "Not" is followed by a space.
        let labels = [label("Not"), label("Not fixed:")];
        assert_eq!(
            match_lead_label(["Not fixed: x"].into_iter(), &labels).map(|l| l.text.as_ref()),
            Some("Not fixed:")
        );
    }

    fn chip_of(labels: &[LeadLabel], text: &str) -> (gpui_kit::Hsla, gpui_kit::Hsla) {
        let label = labels
            .iter()
            .find(|label| label.text.as_ref() == text)
            .unwrap();
        (label.color, label.background)
    }

    #[test]
    fn every_label_maps_to_its_role_tokens_in_both_schemes() {
        for t in [Tokens::light(), Tokens::claude_dark()] {
            let labels = answer_labels(&t);
            assert_eq!(labels.len(), 28);
            let roles = [
                (
                    &[
                        "What I did:",
                        "What happened:",
                        "Why it matters:",
                        "Did:",
                        "Những gì đã làm:",
                        "Chuyện gì đã xảy ra:",
                        "Vì sao quan trọng:",
                        "Đã làm:",
                        "Changes:",
                        "Evidence:",
                        "Thay đổi:",
                        "Bằng chứng:",
                    ][..],
                    (t.status_merged, t.tint_merged),
                ),
                (
                    &[
                        "Symptom:",
                        "Unverified:",
                        "Open:",
                        "Triệu chứng:",
                        "Chưa xác nhận:",
                        "Còn lại:",
                        "Not cleaned:",
                        "Chưa dọn dẹp:",
                    ][..],
                    (t.status_warning, t.tint_warning),
                ),
                (
                    &[
                        "Cause:",
                        "Not fixed:",
                        "Need from you:",
                        "Nguyên nhân:",
                        "Chưa sửa:",
                        "Cần bạn:",
                    ][..],
                    (t.status_danger, t.tint_danger),
                ),
                (
                    &["Fix:", "Cách sửa:"][..],
                    (t.status_success, t.tint_success),
                ),
            ];
            for (texts, chip) in roles {
                for text in texts {
                    assert_eq!(chip_of(&labels, text), chip, "{text}");
                }
            }
        }
    }
}
