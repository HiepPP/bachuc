//! The What Next split, the Recap fold, and the Recap chips (`shared/section.ts`).

use crate::prompts::{heading, is_next_title, parse_prompts};
use fancy_regex::Regex as Fancy;
use regex::Regex;
use serde::{Deserialize, Serialize};
use std::sync::LazyLock;

static FENCE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^ {0,3}(`{3,}|~{3,})").unwrap());

/// The fence marker that opens or closes on `line`.
fn fence_marker(line: &str) -> Option<&str> {
    FENCE
        .captures(line)
        .map(|caps| caps.get(1).unwrap().as_str())
}

fn first_char(text: &str) -> char {
    text.chars().next().unwrap_or_default()
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct NextSplit {
    pub before: String,
    pub title: String,
    pub section: String,
}

/// Splits a reply at its last top-level "What Next" or "Next Steps" heading. `None` when the
/// section holds no eligible suggestion block, so the reply renders unchanged.
pub(crate) fn split_next_section(markdown: &str) -> Option<NextSplit> {
    let markdown = markdown.replace("\r\n", "\n");
    let lines: Vec<&str> = markdown.split('\n').collect();
    let mut fence: Option<char> = None;
    let mut start = None;
    for (index, line) in lines.iter().enumerate() {
        if let Some(opening) = fence_marker(line) {
            match fence {
                None => fence = Some(first_char(opening)),
                Some(marker) if first_char(opening) == marker => fence = None,
                Some(_) => {}
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if heading(line).is_some_and(|title| is_next_title(title.trim())) {
            start = Some(index);
        }
    }
    let start = start?;
    let section = lines[start..].join("\n");
    if parse_prompts(&section).is_empty() {
        return None;
    }
    Some(NextSplit {
        before: lines[..start].join("\n").trim_end().to_string(),
        title: heading(lines[start]).unwrap().trim().to_string(),
        section,
    })
}

/// Text lines of the section outside the heading and fences, for the panel intro.
pub(crate) fn section_intro(section: &str) -> String {
    let mut kept = Vec::new();
    let mut fence: Option<char> = None;
    for line in section.split('\n').skip(1) {
        if let Some(opening) = fence_marker(line) {
            match fence {
                None => fence = Some(first_char(opening)),
                Some(marker) if first_char(opening) == marker => fence = None,
                Some(_) => {}
            }
            continue;
        }
        if fence.is_none() {
            kept.push(line);
        }
    }
    kept.join("\n").trim().to_string()
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
pub(crate) struct RecapSection {
    pub label: String,
    pub text: String,
}

/// `not_yet` and `need` exist only in the five-field Recap; `sections` only in the compact one,
/// with the labels as the reply wrote them.
#[derive(Clone, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Recap {
    pub branch: String,
    pub did: String,
    pub commit: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub not_yet: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub need: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub sections: Option<Vec<RecapSection>>,
}

const FULL: [&str; 5] = ["branch", "commit/push", "did", "not yet", "need from you"];
/// Only free-text fields hold a list: nested in the item, or as the block after the line.
const FREE: [&str; 3] = ["did", "not yet", "need from you"];

static LEGACY: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"^\s*(?:[-*]\s+)?(?:\*\*)?(Branch|Did|Commit/push):(?:\*\*)?\s*(.+?)\s*$").unwrap()
});
static INDENTED: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^\s+\S").unwrap());
static LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:\*\*)?(Branch|Commit/push|Did|Not yet|Need from you):(?:\*\*)?\s*(.*?)\s*$",
    )
    .unwrap()
});
static BULLET: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^ {0,3}[-*+]\s+(.*)$").unwrap());
static LIST: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"^ {0,3}(?:[-*+]|[0-9]{1,9}[.)])\s+\S").unwrap());
static NOTHING: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^nothing\.?$").unwrap());
static COMPACT_LABEL: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(
        r"(?i)^(?:\*\*)?(Did|Đã làm|Open|Còn lại|Need from you|Cần bạn):(?:\*\*)?\s*(.*?)\s*$",
    )
    .unwrap()
});
static BRANCH_STATE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^(.+?)\s+·\s+(.+)$").unwrap());
static UNCOMMITTED: LazyLock<Fancy> = LazyLock::new(|| {
    Fancy::new(r"(?i)^(no changes|not committed|không có thay đổi|chưa commit)(?=$|[\s(,.;:])")
        .unwrap()
});
static RECAP_TITLE: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^recap$").unwrap());
// Each compact field accepts its English or Vietnamese label from the global rules.
const COMPACT_FIELDS: [[&str; 2]; 3] = [
    ["did", "đã làm"],
    ["open", "còn lại"],
    ["need from you", "cần bạn"],
];

/// `line.length - line.trimStart().length`, in characters.
fn indent_of(line: &str) -> usize {
    line.chars().take_while(|c| c.is_whitespace()).count()
}

fn skip_chars(line: &str, count: usize) -> &str {
    match line.char_indices().nth(count) {
        Some((at, _)) => &line[at..],
        None => "",
    }
}

fn group<'a>(caps: &regex::Captures<'a>, index: usize) -> &'a str {
    caps.get(index).map_or("", |m| m.as_str())
}

/// A field's value: its text, then its list lines with their common margin removed.
fn field_value(text: &str, rest: &[String]) -> String {
    let margin = rest.iter().map(|line| indent_of(line)).min().unwrap_or(0);
    std::iter::once(text.to_string())
        .chain(rest.iter().map(|line| skip_chars(line, margin).to_string()))
        .filter(|part| !part.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Lines with text, tabs as four spaces.
fn content_lines(body: &[&str]) -> Vec<String> {
    body.iter()
        .filter(|line| !line.trim().is_empty())
        .map(|line| line.replace('\t', "    "))
        .collect()
}

/// Branch, Did, Commit/push as lines or list items; only Did may hold indented lines.
fn legacy_recap(body: &[&str]) -> Option<Recap> {
    let labels = ["branch", "did", "commit/push"];
    let mut fields: Vec<String> = Vec::new();
    for line in body {
        if line.trim().is_empty() {
            continue;
        }
        if let Some(caps) = LEGACY.captures(line) {
            if labels.get(fields.len()) != Some(&caps[1].to_lowercase().as_str()) {
                return None;
            }
            fields.push(caps[2].to_string());
        } else if fields.len() == 2 && INDENTED.is_match(line) {
            fields[1].push('\n');
            fields[1].push_str(line);
        } else {
            return None;
        }
    }
    let [branch, did, commit]: [String; 3] = fields.try_into().ok()?;
    Some(Recap {
        branch,
        did,
        commit,
        ..Recap::default()
    })
}

struct Field {
    text: String,
    rest: Vec<String>,
}

/// The five fields in order, as one bullet list of five items or as labeled lines. A list item
/// may nest a list, and a labeled line may be followed by a top-level list, in free-text fields
/// only. An empty value needs such a list. Anything else is not a Recap.
fn full_recap(body: &[&str]) -> Option<Recap> {
    let lines = content_lines(body);
    let first = lines.first()?;
    let listed = BULLET.is_match(first);
    let base = indent_of(first);
    let mut fields: Vec<Field> = Vec::new();
    for line in &lines {
        let indent = indent_of(line);
        let holds = !fields.is_empty() && FREE.contains(&FULL[fields.len() - 1]);
        let nested = if listed {
            indent >= base + 2
        } else {
            LIST.is_match(line)
                || (indent >= 2 && fields.last().is_some_and(|f| !f.rest.is_empty()))
        };
        if nested {
            if !holds {
                return None;
            }
            fields.last_mut().unwrap().rest.push(line.clone());
            continue;
        }
        let subject = if listed {
            BULLET.captures(line).map_or("", |caps| group(&caps, 1))
        } else if indent <= 3 {
            line.trim()
        } else {
            ""
        };
        let caps = LABEL.captures(subject)?;
        if FULL.get(fields.len()) != Some(&caps[1].to_lowercase().as_str()) {
            return None;
        }
        fields.push(Field {
            text: group(&caps, 2).to_string(),
            rest: Vec::new(),
        });
    }
    if fields.len() != FULL.len()
        || fields
            .iter()
            .any(|f| f.text.is_empty() && f.rest.is_empty())
    {
        return None;
    }
    let mut values = fields.iter().map(|f| field_value(&f.text, &f.rest));
    let (branch, commit, did, not_yet, need) = (
        values.next()?,
        values.next()?,
        values.next()?,
        values.next()?,
        values.next()?,
    );
    Some(Recap {
        branch,
        did,
        commit,
        not_yet: Some(not_yet),
        need: Some(need),
        sections: None,
    })
}

struct CompactField {
    index: usize,
    label: String,
    text: String,
    rest: Vec<String>,
}

/// The global rules' Recap, as one bullet list: `<branch> · <commit state>`, then Did, then
/// the optional Open and Need from you, in that order. Those three may nest a list.
fn compact_recap(body: &[&str]) -> Option<Recap> {
    let lines = content_lines(body);
    let first = lines.first()?;
    let bullet = BULLET.captures(first)?;
    let head = BRANCH_STATE.captures(group(&bullet, 1))?;
    let base = indent_of(first);
    let mut fields: Vec<CompactField> = Vec::new();
    for line in &lines[1..] {
        if indent_of(line) >= base + 2 {
            fields.last_mut()?.rest.push(line.clone());
            continue;
        }
        let item = BULLET.captures(line).map_or("", |caps| group(&caps, 1));
        let caps = COMPACT_LABEL.captures(item)?;
        let label = caps[1].to_string();
        let lower = label.to_lowercase();
        let index = COMPACT_FIELDS
            .iter()
            .position(|names| names.contains(&lower.as_str()))?;
        let in_order = match fields.last() {
            Some(current) => index > current.index,
            None => index == 0,
        };
        if !in_order {
            return None;
        }
        fields.push(CompactField {
            index,
            label,
            text: group(&caps, 2).to_string(),
            rest: Vec::new(),
        });
    }
    // The head alone is the current Recap: the reply above already holds Did, Open, and
    // Need from you.
    if fields
        .iter()
        .any(|f| f.text.is_empty() && f.rest.is_empty())
    {
        return None;
    }
    let sections: Vec<RecapSection> = fields
        .iter()
        .map(|f| RecapSection {
            label: f.label.clone(),
            text: field_value(&f.text, &f.rest),
        })
        .collect();
    Some(Recap {
        branch: head[1].to_string(),
        commit: head[2].to_string(),
        did: sections.first().map(|s| s.text.clone()).unwrap_or_default(),
        sections: Some(sections),
        ..Recap::default()
    })
}

/// Folds only a complete, recognized Recap right before What Next; unknown prose stays.
pub(crate) fn split_recap(before: &str) -> (String, Option<Recap>) {
    let lines: Vec<&str> = before.split('\n').collect();
    let mut fence: Option<&str> = None;
    let mut start: Option<usize> = None;
    for (index, line) in lines.iter().enumerate() {
        if let Some(opening) = fence_marker(line) {
            match fence {
                None => fence = Some(opening),
                Some(open)
                    if first_char(opening) == first_char(open) && opening.len() >= open.len() =>
                {
                    fence = None
                }
                Some(_) => {}
            }
            continue;
        }
        if fence.is_some() {
            continue;
        }
        if let Some(title) = heading(line) {
            start = RECAP_TITLE.is_match(title.trim()).then_some(index);
        }
    }
    let Some(start) = start else {
        return (before.to_string(), None);
    };
    let body = &lines[start + 1..];
    match legacy_recap(body)
        .or_else(|| full_recap(body))
        .or_else(|| compact_recap(body))
    {
        Some(recap) => (
            lines[..start].join("\n").trim_end().to_string(),
            Some(recap),
        ),
        None => (before.to_string(), None),
    }
}

fn section(label: &str, text: &str) -> RecapSection {
    RecapSection {
        label: label.into(),
        text: text.into(),
    }
}

/// Labeled sections of a five-field Recap: Did always, Not yet and Need from you unless they
/// say `nothing`. A compact Recap shows the fields it wrote. `None` for the legacy Recap.
pub(crate) fn recap_sections(recap: &Recap) -> Option<Vec<RecapSection>> {
    if let Some(sections) = &recap.sections {
        return Some(sections.clone());
    }
    let (not_yet, need) = (recap.not_yet.as_ref()?, recap.need.as_ref()?);
    let mut sections = vec![section("Did", &recap.did)];
    for (label, text) in [("Not yet", not_yet), ("Need from you", need)] {
        if !NOTHING.is_match(text) {
            sections.push(section(label, text));
        }
    }
    Some(sections)
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct CommitChip {
    pub text: String,
    pub done: bool,
    /// The full value, for assistive tech.
    pub label: String,
}

static YES: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^yes\b").unwrap());
static YES_LEAD: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^yes\b[\s,;:.\x{2013}\x{2014}-]*").unwrap());
static NO_COMMIT: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"(?i)^(?:no|none)\.?$").unwrap());
static WORD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[\p{L}\p{N}]").unwrap());
static DONE: LazyLock<Regex> =
    LazyLock::new(|| Regex::new(r"(?i)^(?:committed|pushed)\b").unwrap());

/// The commit chip of a Recap.
pub(crate) fn commit_chip(value: &str) -> CommitChip {
    let said = value.replace('`', "");
    let said = said.trim();
    let yes = YES.is_match(said);
    // `yes` only confirms; the chip shows the detail after it.
    let detail = YES_LEAD.replace(said, "");
    // A compact Recap state for a turn without a commit; the chip drops any note after it.
    let uncommitted = UNCOMMITTED
        .captures(said)
        .ok()
        .flatten()
        .and_then(|caps| caps.get(1).map(|m| m.as_str().to_string()));
    let text = if let Some(state) = uncommitted {
        let mut chars = state.chars();
        let first = chars.next().map(|c| c.to_uppercase().collect::<String>());
        format!("{}{}", first.unwrap_or_default(), chars.as_str())
    } else if NO_COMMIT.is_match(said) {
        "No commit".into()
    } else if !yes {
        said.into()
    } else if WORD.is_match(&detail) {
        detail.into_owned()
    } else {
        "Committed".into()
    };
    CommitChip {
        text,
        done: yes || DONE.is_match(said),
        label: format!("Commit/push: {said}"),
    }
}

static BOLD: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"\*\*(.+?)\*\*").unwrap());

/// Splits `**bold**` runs out of a one-line reason; odd indexes are bold.
pub(crate) fn bold_runs(text: &str) -> Vec<String> {
    let mut runs = Vec::new();
    let mut last = 0;
    for caps in BOLD.captures_iter(text) {
        let whole = caps.get(0).unwrap();
        runs.push(text[last..whole.start()].to_string());
        runs.push(caps[1].to_string());
        last = whole.end();
    }
    runs.push(text[last..].to_string());
    runs
}

#[cfg(test)]
mod tests {
    use super::{
        Recap, RecapSection, bold_runs, commit_chip, recap_sections, section_intro,
        split_next_section, split_recap,
    };
    use crate::selection::{Group, layout, reachable};

    fn strings(items: &[&str]) -> Vec<String> {
        items.iter().map(|s| s.to_string()).collect()
    }

    fn reply() -> String {
        [
            "Done.",
            "",
            "## Recap",
            "- Branch: `main`",
            "",
            "## What Next",
            "",
            "Run the check first.",
            "",
            "```text",
            "prompt: Run the tests.",
            "why: Pick this to **verify**, because it is fast.",
            "```",
        ]
        .join("\n")
    }

    fn unchanged(before: &str) -> (String, Option<Recap>) {
        (before.to_string(), None)
    }

    #[test]
    fn the_reply_splits_at_the_what_next_heading_and_keeps_the_text_before_it() {
        let split = split_next_section(&reply()).unwrap();
        assert_eq!(split.title, "What Next");
        assert_eq!(split.before, "Done.\n\n## Recap\n- Branch: `main`");
        assert_eq!(section_intro(&split.section), "Run the check first.");
    }

    #[test]
    fn a_reply_without_an_eligible_block_or_with_the_heading_inside_a_fence_is_left_alone() {
        assert_eq!(split_next_section("## What Next\n\nNothing to send."), None);
        assert_eq!(
            split_next_section("```md\n## What Next\n```\n\n```text\nprompt: x\n```"),
            None
        );
    }

    #[test]
    fn layout_numbers_separate_sets_and_keeps_exclusive_choices_first() {
        assert_eq!(
            layout(
                &strings(&["a", "b", "c", "d", "e"]),
                &[],
                &[strings(&["a", "c"]), strings(&["b", "d"])],
            ),
            vec![
                Group::Together {
                    set: 1,
                    ids: strings(&["a", "c"])
                },
                Group::Together {
                    set: 2,
                    ids: strings(&["b", "d"])
                },
                Group::Alone {
                    ids: strings(&["e"])
                },
            ]
        );
        assert_eq!(
            layout(
                &strings(&["x", "risks", "y"]),
                &[strings(&["x", "y"])],
                &[strings(&["x", "risks"])],
            ),
            vec![
                Group::Choice {
                    index: 0,
                    ids: strings(&["x", "y"])
                },
                Group::FollowUp {
                    ids: strings(&["risks"])
                },
            ]
        );
        assert!(reachable(
            &strings(&["a"]),
            "c",
            &[],
            &[strings(&["a", "b", "c"])]
        ));
        assert!(
            reachable(&strings(&["x"]), "y", &[strings(&["x", "y"])], &[]),
            "radio swap replaces the choice"
        );
    }

    #[test]
    fn a_complete_recap_folds_without_losing_markdown_in_did() {
        assert_eq!(
            split_recap(
                "Result.\n\n## Recap\nBranch: `main`\nDid: Kept **formatting**.\nCommit/push: none"
            ),
            (
                "Result.".into(),
                Some(Recap {
                    branch: "`main`".into(),
                    did: "Kept **formatting**.".into(),
                    commit: "none".into(),
                    ..Recap::default()
                })
            )
        );
        assert_eq!(
            split_recap(
                "## Recap\n- **Branch:** `main`\n- **Did:** Checked it.\n  - Nested evidence.\n- **Commit/push:** none",
            )
            .1,
            Some(Recap {
                branch: "`main`".into(),
                did: "Checked it.\n  - Nested evidence.".into(),
                commit: "none".into(),
                ..Recap::default()
            })
        );
    }

    #[test]
    fn unknown_or_incomplete_recap_content_stays_untouched() {
        for before in [
            "## Recap\nBranch: main\nDid: done",
            "## Recap\nBranch: main\nDid: done\nCommit/push: none\nExtra prose must remain.",
            "## Recap\nBranch: main\nDid: done\nCommit/push: none\n## Evidence\nKeep this.",
            "````md\n## Recap\nBranch: main\nDid: done\n```\nCommit/push: none\n````",
        ] {
            assert_eq!(split_recap(before), unchanged(before), "{before}");
        }
    }

    fn five(lines: &[&str]) -> (String, Option<Recap>) {
        let mut all = vec!["Result.", "", "## Recap"];
        all.extend_from_slice(lines);
        split_recap(&all.join("\n"))
    }

    const LIST: [&str; 5] = [
        "- Branch: `main`",
        "- Commit/push: no",
        "- Did: Rewrote the [section](/report). +12/-8.",
        "- Not yet: nothing",
        "- Need from you: Check the `chip`.",
    ];

    fn full(branch: &str, commit: &str, did: &str, not_yet: &str, need: &str) -> Recap {
        Recap {
            branch: branch.into(),
            commit: commit.into(),
            did: did.into(),
            not_yet: Some(not_yet.into()),
            need: Some(need.into()),
            sections: None,
        }
    }

    #[test]
    fn a_five_field_bullet_list_folds_with_nested_lists_in_free_text_fields() {
        assert_eq!(
            five(&LIST),
            (
                "Result.".into(),
                Some(full(
                    "`main`",
                    "no",
                    "Rewrote the [section](/report). +12/-8.",
                    "nothing",
                    "Check the `chip`."
                ))
            )
        );
        assert_eq!(
            five(&[
                "- **Branch:** main",
                "- **Commit/push:** yes, committed abc1234",
                "- **Did:** Ported it.",
                "    - Parser.",
                "        - Deeper.",
                "    - Panel.",
                "",
                "- **Not yet:**",
                "  - Runtime check.",
                "- **Need from you:** nothing",
            ])
            .1,
            Some(full(
                "main",
                "yes, committed abc1234",
                "Ported it.\n- Parser.\n    - Deeper.\n- Panel.",
                "- Runtime check.",
                "nothing"
            ))
        );
    }

    #[test]
    fn five_labeled_lines_fold_each_free_text_line_owning_the_list_that_follows_it() {
        assert_eq!(
            five(&[
                "Branch: main",
                "Commit/push: no",
                "Did: Rewrote the section.",
                "Not yet:",
                "",
                "- Item one.",
                "  - Detail.",
                "- Item two.",
                "",
                "Need from you: Two steps.",
                "1. Open a session.",
                "2. Check the chip.",
            ])
            .1,
            Some(full(
                "main",
                "no",
                "Rewrote the section.",
                "- Item one.\n  - Detail.\n- Item two.",
                "Two steps.\n1. Open a session.\n2. Check the chip."
            ))
        );
        assert_eq!(
            five(&[
                "Branch: main",
                "",
                "Commit/push: no",
                "",
                "Did: x",
                "Not yet: nothing",
                "Need from you: y",
            ])
            .1,
            Some(full("main", "no", "x", "nothing", "y"))
        );
    }

    #[test]
    fn any_other_five_field_shape_keeps_native_rendering() {
        let swap = |from: usize, to: &'static str| -> Vec<&'static str> {
            LIST.iter()
                .enumerate()
                .map(|(index, line)| if index == from { to } else { *line })
                .collect()
        };
        let joined = |parts: &[&[&'static str]]| parts.concat();
        let numbered: Vec<String> = LIST
            .iter()
            .enumerate()
            .map(|(index, line)| line.replacen('-', &format!("{}.", index + 1), 1))
            .collect();
        let numbered: Vec<&str> = numbered.iter().map(String::as_str).collect();
        let cases: Vec<(&str, Vec<&str>)> = vec![
            (
                "legacy order plus two fields",
                vec![LIST[0], LIST[2], LIST[1], LIST[3], LIST[4]],
            ),
            ("missing field", LIST[..4].to_vec()),
            ("extra field", joined(&[&LIST, &["- Tests: pending"]])),
            ("unknown label", swap(2, "- Summary: Rewrote it.")),
            ("missing label", swap(3, "- nothing")),
            (
                "duplicate label",
                vec![LIST[0], LIST[1], LIST[2], LIST[2], LIST[4]],
            ),
            (
                "unlabeled block after",
                joined(&[&LIST, &["", "Extra prose must remain."]]),
            ),
            (
                "unlabeled block between",
                joined(&[&LIST[..3], &["", "Extra prose.", ""], &LIST[3..]]),
            ),
            (
                "wrapped line at the margin",
                joined(&[&LIST[..3], &["continued here."], &LIST[3..]]),
            ),
            ("empty value without a list", swap(3, "- Not yet:")),
            (
                "list nested in Branch",
                joined(&[&LIST[..1], &["  - nested"], &LIST[1..]]),
            ),
            (
                "list nested in Commit/push",
                joined(&[&LIST[..2], &["  - nested"], &LIST[2..]]),
            ),
            (
                "list after a Branch line",
                vec![
                    "Branch: main",
                    "- nested",
                    "Commit/push: no",
                    "Did: x",
                    "Not yet: y",
                    "Need from you: z",
                ],
            ),
            (
                "list before the first line",
                vec![
                    "- stray",
                    "Branch: main",
                    "Commit/push: no",
                    "Did: x",
                    "Not yet: y",
                    "Need from you: z",
                ],
            ),
            (
                "lines then bullets",
                joined(&[&["Branch: main", "Commit/push: no"], &LIST[2..]]),
            ),
            ("numbered fields", numbered),
            (
                "fence in the Recap",
                joined(&[&LIST, &["```", "code", "```"]]),
            ),
        ];
        for (name, lines) in cases {
            let mut all = vec!["Result.", "", "## Recap"];
            all.extend(lines);
            let before = all.join("\n");
            assert_eq!(split_recap(&before), unchanged(&before), "{name}");
        }
        let quoted: Vec<String> = LIST.iter().map(|line| format!("> {line}")).collect();
        for before in [
            format!("> ## Recap\n{}", quoted.join("\n")),
            format!("````md\n## Recap\n{}\n````", LIST.join("\n")),
            format!("## Recap\n{}\n## Evidence\nKeep this.", LIST.join("\n")),
        ] {
            assert_eq!(split_recap(&before), unchanged(&before), "{before}");
        }
    }

    #[test]
    fn the_commit_chip_maps_each_value_and_keeps_the_full_value_in_its_label() {
        let rows: [(&str, &str, bool); 21] = [
            ("no", "No commit", false),
            ("No.", "No commit", false),
            ("none", "No commit", false),
            ("None.", "No commit", false),
            ("yes", "Committed", true),
            ("Yes!", "Committed", true),
            ("yes, committed abc1234", "committed abc1234", true),
            ("yes - pushed main", "pushed main", true),
            (
                "Yes \u{2014} committed `abc1234`",
                "committed abc1234",
                true,
            ),
            ("yes committed abc1234", "committed abc1234", true),
            ("committed abc1234", "committed abc1234", true),
            ("Pushed main", "Pushed main", true),
            ("pending review", "pending review", false),
            ("not committed", "Not committed", false),
            ("no changes", "No changes", false),
            (
                "no changes (files created and edited in working directory)",
                "No changes",
                false,
            ),
            ("chưa commit", "Chưa commit", false),
            ("không có thay đổi", "Không có thay đổi", false),
            (
                "committed abc1234, pushed main",
                "committed abc1234, pushed main",
                true,
            ),
            ("no changesets", "no changesets", false),
            ("yesterday", "yesterday", false),
        ];
        for (value, text, done) in rows {
            let chip = commit_chip(value);
            assert_eq!(chip.text, text, "{value}");
            assert_eq!(chip.done, done, "{value}");
            assert_eq!(
                chip.label,
                format!("Commit/push: {}", value.replace('`', "")),
                "{value}"
            );
        }
    }

    #[test]
    fn five_field_sections_omit_not_yet_and_need_from_you_that_say_nothing() {
        let recap = |not_yet: &str, need: &str| full("main", "no", "nothing", not_yet, need);
        let labels = |value: &Recap| -> Vec<String> {
            recap_sections(value)
                .unwrap()
                .into_iter()
                .map(|s| s.label)
                .collect()
        };
        for nothing in ["nothing", "Nothing.", "NOTHING"] {
            assert_eq!(
                labels(&recap(nothing, nothing)),
                ["Did"],
                "Did always shows"
            );
            assert_eq!(
                labels(&recap(nothing, "Check it.")),
                ["Did", "Need from you"]
            );
            assert_eq!(labels(&recap("One test.", nothing)), ["Did", "Not yet"]);
        }
        assert_eq!(
            recap_sections(&recap("- nothing", "Nothing so far.")).unwrap(),
            vec![
                RecapSection {
                    label: "Did".into(),
                    text: "nothing".into()
                },
                RecapSection {
                    label: "Not yet".into(),
                    text: "- nothing".into()
                },
                RecapSection {
                    label: "Need from you".into(),
                    text: "Nothing so far.".into()
                },
            ]
        );
        let swapped: Vec<String> = LIST
            .iter()
            .map(|line| line.replace("Not yet: nothing", "Not yet: Nothing."))
            .collect();
        let swapped: Vec<&str> = swapped.iter().map(String::as_str).collect();
        assert_eq!(labels(&five(&swapped).1.unwrap()), ["Did", "Need from you"]);
    }

    fn sections(pairs: &[(&str, &str)]) -> Vec<RecapSection> {
        pairs
            .iter()
            .map(|(label, text)| RecapSection {
                label: label.to_string(),
                text: text.to_string(),
            })
            .collect()
    }

    #[test]
    fn a_compact_recap_folds_with_the_fields_it_wrote() {
        let recap = five(&[
            "- `main` · `no changes` (files created and edited in working directory)",
            "- Did: Created notes.ts with 5 constants.",
        ])
        .1
        .unwrap();
        assert_eq!(
            recap,
            Recap {
                branch: "`main`".into(),
                commit: "`no changes` (files created and edited in working directory)".into(),
                did: "Created notes.ts with 5 constants.".into(),
                sections: Some(sections(&[("Did", "Created notes.ts with 5 constants.")])),
                ..Recap::default()
            }
        );
        assert_eq!(recap_sections(&recap), recap.sections);
        assert_eq!(
            five(&[
                "- `main` · not committed",
                "- **Did:** Added the toggle.",
                "- Open: The trace is not measured.",
                "- Need from you: Install the release.",
            ])
            .1
            .unwrap()
            .sections,
            Some(sections(&[
                ("Did", "Added the toggle."),
                ("Open", "The trace is not measured."),
                ("Need from you", "Install the release."),
            ]))
        );
    }

    #[test]
    fn a_vietnamese_compact_recap_keeps_its_labels_and_nested_lists() {
        assert_eq!(
            five(&[
                "- `main` · chưa commit",
                "- Đã làm: Thêm nút trên Board.",
                "- Còn lại:",
                "  - Chưa đo trace.",
                "  - Release chưa cập nhật.",
                "- Cần bạn: Cài bản release.",
            ])
            .1,
            Some(Recap {
                branch: "`main`".into(),
                commit: "chưa commit".into(),
                did: "Thêm nút trên Board.".into(),
                sections: Some(sections(&[
                    ("Đã làm", "Thêm nút trên Board."),
                    ("Còn lại", "- Chưa đo trace.\n- Release chưa cập nhật."),
                    ("Cần bạn", "Cài bản release."),
                ])),
                ..Recap::default()
            })
        );
        let labels: Vec<String> = five(&[
            "- main · không có thay đổi",
            "- Đã làm: Kiểm tra parser.",
            "- Cần bạn: Trả lời.",
        ])
        .1
        .unwrap()
        .sections
        .unwrap()
        .into_iter()
        .map(|s| s.label)
        .collect();
        assert_eq!(labels, ["Đã làm", "Cần bạn"]);
    }

    #[test]
    fn a_branch_only_recap_folds_into_the_chips() {
        assert_eq!(
            five(&["- `main` · not committed"]).1,
            Some(Recap {
                branch: "`main`".into(),
                commit: "not committed".into(),
                sections: Some(Vec::new()),
                ..Recap::default()
            })
        );
        assert_eq!(
            five(&["- main · không có thay đổi"]).1.map(|r| r.commit),
            Some("không có thay đổi".into())
        );
    }

    #[test]
    fn any_other_compact_shape_keeps_native_rendering() {
        let head = "- `main` · not committed";
        let cases: Vec<(&str, Vec<&str>)> = vec![
            ("Open before Did", vec![head, "- Open: x", "- Did: y"]),
            (
                "Need before Open",
                vec![head, "- Did: x", "- Need from you: y", "- Open: z"],
            ),
            (
                "duplicate Open",
                vec![head, "- Did: x", "- Open: y", "- Open: z"],
            ),
            ("unknown label", vec![head, "- Did: x", "- Tests: y"]),
            ("no middle dot", vec!["- `main`, not committed", "- Did: x"]),
            (
                "lines without bullets",
                vec!["`main` · not committed", "Did: x"],
            ),
            (
                "list nested in the head",
                vec![head, "  - nested", "- Did: x"],
            ),
            ("empty Did without a list", vec![head, "- Did:"]),
            (
                "prose after",
                vec![head, "- Did: x", "", "Extra prose must remain."],
            ),
        ];
        for (name, lines) in cases {
            let mut all = vec!["Result.", "", "## Recap"];
            all.extend(lines);
            let before = all.join("\n");
            assert_eq!(split_recap(&before), unchanged(&before), "{name}");
        }
    }

    #[test]
    fn the_legacy_recap_has_no_sections_and_no_five_field_keys() {
        let legacy = split_recap("## Recap\nBranch: `main`\nDid: Kept it.\nCommit/push: none")
            .1
            .unwrap();
        assert_eq!(
            legacy,
            Recap {
                branch: "`main`".into(),
                did: "Kept it.".into(),
                commit: "none".into(),
                ..Recap::default()
            }
        );
        assert_eq!(recap_sections(&legacy), None);
    }

    #[test]
    fn bold_runs_puts_bold_text_at_odd_indexes() {
        assert_eq!(
            bold_runs("Pick this to **verify**, because it is fast."),
            ["Pick this to ", "verify", ", because it is fast."]
        );
        assert_eq!(bold_runs("No bold here."), ["No bold here."]);
    }
}
