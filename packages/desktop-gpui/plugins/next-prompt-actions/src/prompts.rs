//! Suggestion blocks and explicit Git actions (`shared/prompts.ts`). Each JS pattern is ported
//! with `regex`, or with `fancy-regex` when it uses look-around.

use crate::next_prompts::{NextPromptsV1, js_len, parse_next_prompts};
use fancy_regex::Regex as Fancy;
use regex::{Captures, Regex};
use std::sync::LazyLock;
use unicode_normalization::UnicodeNormalization as _;

macro_rules! plain {
    ($name:ident, $pattern:expr) => {
        static $name: LazyLock<Regex> = LazyLock::new(|| Regex::new($pattern).unwrap());
    };
}

macro_rules! fancy {
    ($name:ident, $pattern:expr) => {
        static $name: LazyLock<Fancy> = LazyLock::new(|| Fancy::new($pattern).unwrap());
    };
}

/// One suggestion block. `whys[i]` is the reason shown under `prompts[i]` and is never sent;
/// `threads[i]` marks a prompt started only in a new thread; `suggestions` exists only when at
/// least one prompt is marked.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct PromptBlock {
    pub block: String,
    pub prompts: Vec<String>,
    pub whys: Vec<String>,
    pub threads: Vec<bool>,
    pub suggestions: Option<Vec<bool>>,
    pub declaration: Option<NextPromptsV1>,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum GitKind {
    Commit,
    CommitPush,
    Push,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct GitAction {
    pub kind: GitKind,
    pub label: &'static str,
    pub prompt: String,
}

fn fancy_match(pattern: &Fancy, text: &str) -> bool {
    pattern.is_match(text).unwrap_or(false)
}

/// `text.replace(pattern, with)`: the first match only, with `with` taken literally.
fn replace_first(pattern: &Fancy, text: &str, with: &str) -> String {
    match pattern.find(text).ok().flatten() {
        Some(found) => format!("{}{with}{}", &text[..found.start()], &text[found.end()..]),
        None => text.to_string(),
    }
}

/// `text.split(pattern)` for a pattern without groups that never matches empty.
fn split_fancy<'a>(pattern: &Fancy, text: &'a str) -> Vec<&'a str> {
    let mut parts = Vec::new();
    let mut last = 0;
    for found in pattern.find_iter(text).flatten() {
        parts.push(&text[last..found.start()]);
        last = found.end();
    }
    parts.push(&text[last..]);
    parts
}

plain!(MARK, r"\p{M}");
fancy!(SKILL_ANY_CASE, r"(?i)^\s*[/$]commit(?=\s|$)");
fancy!(SKILL, r"^\s*[/$]commit(?=\s|$)");
fancy!(CLAUSE, r"[!?;\n]+|\.(?=\s|$)|\b(?:then|roi|sau do)\b");
plain!(
    COMMAND_LEAD,
    r"^(?:(?:please|hay|vui long|chi can|chi|but|nhung)\s+)+"
);
plain!(COMMAND_GIT, r"^git\s+");
fancy!(
    COMMAND_CREATE,
    r"^(?:create(?: a)?|tao(?: mot)?)\s+(?=commit\b)"
);
plain!(
    LABEL_LIST,
    r"^(?:edit|send|commit|push)(?:\s*(?:and|va|&|\+)\s*(?:edit|send|commit|push))*(?:\s+(?:buttons?|controls?|labels?))?$"
);
fancy!(
    LABELS,
    r"\b(?:buttons?|controls?|labels?|nut)(?:\s+(?:named|called|labelled|labeled|co nhan))?(?:\s*:\s*|\s+)(?:edit|send|commit|push)\b|\b(?:edit|send)(?=\s*(?:,|and\b|va\b|[&+]|$))"
);
plain!(TRANSITION, r",[ \t]*(?:then|roi|sau do|but|nhung)\b");
plain!(
    RESTRICTION,
    r"(^|[.!?;\n,])[ \t]*((?:without\s+(?:changing|editing|modifying)|(?:do not|don['’]t|never)\s+(?:change|edit|modify)|(?:khong|dung)\s+(?:sua|chinh sua|thay doi))\s+(?:unrelated\s+(?:files|changes|work)|other\s+(?:files|changes)|wip\s+khac|(?:cac\s+)?(?:file|thay doi)\s+(?:khac|khong lien quan))),([^.!?;\n]*)"
);
plain!(
    COORDINATED,
    r"\b(?:and|or|va|hoac|hay)\s+(?:git\s+)?(?:commit|push)\b|,\s*(?:and|or|va|hoac|hay)\b"
);
fancy!(
    TARGETED,
    r"^(?:git\s+)?(?:commit|push)\s+(?!(?:and|or|va|hoac|hay)\b|[&+])\S"
);
// JS `\w` is ASCII.
plain!(
    DENIED_COMMIT,
    r"\b(?:do not|don['’]t|never|not|no|without|avoid|skip|khong|chua|dung|cam)(?:[ \t,]+[A-Za-z0-9_]+)*?[ \t,]+commit(?:ting|s)?\b"
);
plain!(
    DENIED_PUSH,
    r"\b(?:do not|don['’]t|never|not|no|without|avoid|skip|khong|chua|dung|cam)(?:[ \t,]+[A-Za-z0-9_]+)*?[ \t,]+push(?:ing)?\b"
);
fancy!(
    IS_COMMIT,
    r"^commit(?=[:\s]|$)(?!\s+(?:messages?|buttons?|hash|sha|history|status|diff|log|is|was|has|da|already|done|completed)\b)"
);
fancy!(
    IS_PUSH,
    r"^push(?=[:\s]|$)(?!\s+(?:notifications?|buttons?|events?|messages?|behavior|is|was|has|da|already|done|completed)\b)"
);
// Excluding unrelated work narrows a positive commit request; it does not cancel it.
plain!(
    UNRELATED_SCOPE,
    r"^\s+(?:unrelated\b|other\s+(?:files|changes)\b|wip\s+khac\b|(?:cac\s+)?(?:file|thay doi)\s+(?:khac|khong lien quan)\b)"
);
plain!(NO_PUSH_FLAG, r"--no-push\b");
plain!(NO_PUSH_FLAG_ANY_CASE, r"(?i)--no-push\b");
plain!(AND, r"\b(?:and|va)\b|[&+]");

fn command_text(part: &str) -> String {
    let text = COMMAND_LEAD.replace(part.trim(), "");
    let text = COMMAND_GIT.replace(&text, "");
    replace_first(&COMMAND_CREATE, &text, "")
}

fn is_commit(command: &str) -> bool {
    fancy_match(&IS_COMMIT, command)
}

fn is_push(command: &str) -> bool {
    fancy_match(&IS_PUSH, command)
}

fn denied(pattern: &Regex, text: &str, scope_exception: Option<&Regex>) -> bool {
    pattern.find_iter(text).any(|found| {
        scope_exception.is_none_or(|exception| !exception.is_match(&text[found.end()..]))
    })
}

struct Command {
    text: String,
    labels: bool,
}

/// Recognizes explicit Git actions conservatively; mentions and unclear wording keep Send.
pub(crate) fn git_action(text: &str) -> Option<GitAction> {
    let stripped: String = text.nfd().collect();
    let normalized = MARK
        .replace_all(&stripped, "")
        .replace(['đ', 'Đ'], "d")
        .to_lowercase();
    let skill = fancy_match(&SKILL_ANY_CASE, text);
    let leading = replace_first(&SKILL, &normalized, "commit");
    // Require actual label names; mentioning button tests or fixes must not hide a Git command.
    let mut commands = Vec::new();
    for clause in split_fancy(&CLAUSE, &leading) {
        let labels = fancy_match(&LABELS, clause);
        for part in clause.split(',') {
            if labels && LABEL_LIST.is_match(part.trim()) {
                continue;
            }
            commands.push(Command {
                text: command_text(part),
                labels,
            });
        }
    }
    // Commas continue a prohibition's list; an explicit transition starts a new instruction.
    let transitions = TRANSITION.replace_all(&normalized, ". ");
    // An unrelated-edit restriction can precede a targeted Git command, not a denial list.
    let denial = RESTRICTION.replace_all(&transitions, |caps: &Captures| {
        let (whole, boundary, restriction, rest) = (&caps[0], &caps[1], &caps[2], &caps[3]);
        let without = restriction.starts_with("without ");
        if !without && COORDINATED.is_match(rest) {
            return whole.to_string();
        }
        if without || fancy_match(&TARGETED, rest.trim()) {
            format!("{boundary}{restriction}. {rest}")
        } else {
            whole.to_string()
        }
    });
    let commit = !denied(&DENIED_COMMIT, &denial, Some(&UNRELATED_SCOPE))
        && (skill || commands.iter().any(|command| is_commit(&command.text)));
    let no_push = NO_PUSH_FLAG.is_match(&normalized) || denied(&DENIED_PUSH, &denial, None);
    let push = !no_push
        && (commands.iter().any(|command| is_push(&command.text))
            || (commit
                && commands.iter().any(|command| {
                    is_commit(&command.text)
                        && AND.split(&command.text).skip(1).any(|part| {
                            is_push(&command_text(part))
                                && (!command.labels || !LABEL_LIST.is_match(part.trim()))
                        })
                })));
    if !commit {
        return push.then(|| GitAction {
            kind: GitKind::Push,
            label: "Push",
            prompt: text.to_string(),
        });
    }
    let command = if push { "/commit" } else { "/commit --no-push" };
    let prompt = if skill {
        let with = if push || NO_PUSH_FLAG_ANY_CASE.is_match(text) {
            "/commit"
        } else {
            command
        };
        replace_first(&SKILL_ANY_CASE, text, with)
    } else {
        format!("{command}\n{text}")
    };
    Some(GitAction {
        kind: if push {
            GitKind::CommitPush
        } else {
            GitKind::Commit
        },
        label: if push { "Commit & Push" } else { "Commit" },
        prompt: format!("{prompt}\nCommit only the described changes. Preserve unrelated work."),
    })
}

plain!(CLOSING, r"^ {0,3}(`{3,}|~{3,})\s*$");
plain!(OPENING, r"^ {0,3}(`{3,}|~{3,})([^`]*)$");
plain!(HEADING, r"^ {0,3}#{1,6}\s+(.+?)\s*#*\s*$");
plain!(NEXT_TITLE, r"(?i)^(?:what(?:['’]s)? next|next steps)$");
plain!(ELIGIBLE, r"(?i)^(?:text|txt|plaintext|next-prompts)?\s*$");
plain!(STRUCTURED, r"(?i)^next-prompts$");
plain!(PROMPT_LINE, r"(?i)^prompt:");
plain!(PROMPT_MARK, r"(?i)^prompt:[ \t]?");
plain!(WHY_LINE, r"(?i)^why:");
plain!(THREAD_LINE, r"(?i)^thread:\s*new\s*$");
plain!(
    SUGGESTION_LINE,
    r"(?i)^suggestion:[ \t]*(?:true|false)[ \t]*$"
);
plain!(TRUE, r"(?i)true");

/// Whether a heading title names a next-step section.
pub(crate) fn is_next_title(title: &str) -> bool {
    NEXT_TITLE.is_match(title)
}

/// The heading title of a line, as `HEADING` captures it.
pub(crate) fn heading(line: &str) -> Option<&str> {
    HEADING
        .captures(line)
        .map(|caps| caps.get(1).unwrap().as_str())
}

struct Fence {
    marker: char,
    length: usize,
    eligible: bool,
    structured: bool,
    lines: Vec<String>,
}

fn suggestions_if_any(suggestions: Vec<bool>) -> Option<Vec<bool>> {
    suggestions.iter().any(|&s| s).then_some(suggestions)
}

/// The prompts of a `text` block, or `None` when the block is not a valid suggestion list.
fn text_block(block: String) -> Option<PromptBlock> {
    let mut prompts = Vec::new();
    let mut whys: Vec<String> = Vec::new();
    let mut threads: Vec<bool> = Vec::new();
    let mut suggestions: Vec<bool> = Vec::new();
    // The first `suggestion:` line of a prompt decides; later ones are dropped, not sent.
    let mut flagged: Vec<bool> = Vec::new();
    let mut current: Option<Vec<&str>> = None;
    let mut valid = true;
    for line in block.split('\n') {
        if PROMPT_LINE.is_match(line) {
            if let Some(lines) = current.take() {
                prompts.push(lines.join("\n").trim_end_matches('\n').to_string());
            }
            current = Some(vec![&line[PROMPT_MARK.find(line).unwrap().end()..]]);
            whys.push(String::new());
            threads.push(false);
            suggestions.push(false);
            flagged.push(false);
        } else if current.is_some() && WHY_LINE.is_match(line) && whys.last().unwrap().is_empty() {
            let mark = WHY_LINE.find(line).unwrap().end();
            *whys.last_mut().unwrap() = line[mark..].trim().to_string();
        } else if current.is_some() && THREAD_LINE.is_match(line) && !threads.last().unwrap() {
            *threads.last_mut().unwrap() = true;
        } else if current.is_some() && SUGGESTION_LINE.is_match(line) {
            let last = flagged.len() - 1;
            if !flagged[last] {
                suggestions[last] = TRUE.is_match(line);
            }
            flagged[last] = true;
        } else if let Some(lines) = current.as_mut() {
            lines.push(line);
        } else if !line.trim().is_empty() {
            valid = false;
        }
    }
    if let Some(lines) = current {
        prompts.push(lines.join("\n").trim_end_matches('\n').to_string());
    }
    let fits = |p: &String| !p.trim().is_empty() && js_len(p) <= 16000;
    (valid && !prompts.is_empty() && prompts.iter().all(fits)).then(|| PromptBlock {
        block,
        prompts,
        whys,
        threads,
        suggestions: suggestions_if_any(suggestions),
        declaration: None,
    })
}

fn structured_block(block: String) -> Option<PromptBlock> {
    let declaration = parse_next_prompts(&block)?;
    let prompts = &declaration.prompts;
    Some(PromptBlock {
        prompts: prompts.iter().map(|p| p.prompt.clone()).collect(),
        whys: prompts
            .iter()
            .map(|p| p.why.clone().unwrap_or_default())
            .collect(),
        threads: prompts.iter().map(|p| p.thread_new).collect(),
        suggestions: suggestions_if_any(
            prompts.iter().map(|p| p.suggestion == Some(true)).collect(),
        ),
        block,
        declaration: Some(declaration),
    })
}

/// Accepts only top-level fenced suggestions in an explicit next-step section.
pub(crate) fn parse_prompts(markdown: &str) -> Vec<PromptBlock> {
    let mut result = Vec::new();
    let mut section = false;
    let mut fence: Option<Fence> = None;
    let markdown = markdown.replace("\r\n", "\n");
    for line in markdown.split('\n') {
        if let Some(open) = fence.as_mut() {
            let closes = CLOSING.captures(line).is_some_and(|caps| {
                let marker = &caps[1];
                marker.starts_with(open.marker) && marker.len() >= open.length
            });
            if !closes {
                open.lines.push(line.to_string());
                continue;
            }
            let open = fence.take().unwrap();
            if open.eligible {
                let block = open.lines.join("\n").trim_end_matches('\n').to_string();
                let parsed = if open.structured {
                    structured_block(block)
                } else {
                    text_block(block)
                };
                result.extend(parsed);
            }
            continue;
        }
        if let Some(title) = heading(line) {
            section = is_next_title(title.trim());
        }
        if let Some(caps) = OPENING.captures(line) {
            let marker = &caps[1];
            let info = caps[2].trim();
            fence = Some(Fence {
                marker: marker.chars().next().unwrap(),
                length: marker.len(),
                eligible: section && ELIGIBLE.is_match(info),
                structured: STRUCTURED.is_match(info),
                lines: Vec::new(),
            });
        }
    }
    result
}

/// Several prompts sent or edited together become one numbered message.
pub(crate) fn join_prompts(prompts: &[String]) -> String {
    if prompts.len() == 1 {
        return prompts[0].clone();
    }
    prompts
        .iter()
        .enumerate()
        .map(|(index, prompt)| format!("{}. {}", index + 1, prompt.replace('\n', "\n   ")))
        .collect::<Vec<_>>()
        .join("\n")
}

#[cfg(test)]
mod tests {
    use super::{GitKind, PromptBlock, git_action, join_prompts, parse_prompts};
    use serde_json::{Value, json};

    fn kind(text: &str) -> Option<GitKind> {
        git_action(text).map(|action| action.kind)
    }

    #[test]
    fn explicit_git_actions_distinguish_commit_commit_and_push_and_push_only() {
        use GitKind::{Commit, CommitPush, Push};
        for (text, expected) in [
            ("Commit thay đổi. Không push.", Commit),
            ("Review, then COMMIT.", Commit),
            ("Hãy commit thay đổi trong plugin.", Commit),
            ("Please git commit the fix.", Commit),
            ("Create a commit for the fix.", Commit),
            ("Tạo commit cho thay đổi.", Commit),
            ("Commit and push the fix.", CommitPush),
            ("Commit và push thay đổi.", CommitPush),
            ("Commit & Push.", CommitPush),
            ("Commit xong rồi push origin/main.", CommitPush),
            ("Commit the fix, then push origin/main.", CommitPush),
            (
                "Commit and push the fix. Do not commit unrelated files.",
                CommitPush,
            ),
            ("Kiểm tra diff, commit phần sửa lỗi.", Commit),
            ("Run tests, commit the fix.", Commit),
            ("Commit the fix, push origin/main.", CommitPush),
            (
                "Commit the button fixes, then push origin/main.",
                CommitPush,
            ),
            ("Commit the button fixes and push origin/main.", CommitPush),
            ("Commit the button fixes, git push.", CommitPush),
            ("Commit the button fixes and git push.", CommitPush),
            ("Review the buttons, commit the fix.", Commit),
            ("Run button tests, commit.", Commit),
            ("Chạy test cho các nút, commit.", Commit),
            ("Commit the button fixes and push.", CommitPush),
            ("Do not edit, then commit the fix.", Commit),
            ("Do not edit, but commit the fix.", Commit),
            ("Do not commit, then push the existing commit.", Push),
            ("/commit\nCommit and push the fix.", CommitPush),
            ("$commit and push the fix.", CommitPush),
            ("Push commit 5324b19 lên origin/main.", Push),
            ("Đã commit xong. Hãy push nhánh main.", Push),
            ("Do not commit. Push the existing commit.", Push),
        ] {
            assert_eq!(kind(text), Some(expected), "{text}");
        }
    }

    #[test]
    fn mentions_negative_requests_and_existing_commits_do_not_imply_a_new_commit() {
        for text in [
            "Review commit 5324b19.",
            "Review commit and push behavior.",
            "Kiểm tra commit gần nhất.",
            "Không commit hoặc push.",
            "Do not commit or push.",
            "Don't commit and push.",
            "Never git commit changes.",
            "Avoid committing changes.",
            "Commit message needs review.",
            "Commit button needs review.",
            "Commit đã xong.",
            "Review the commitment.",
            "Review the Edit, Commit and Push buttons.",
            "Review nút Edit, Commit và Push.",
            "Review nút Commit, Push.",
            "Review buttons named Commit, Push.",
            "Review các nút có nhãn Commit, Push.",
            "Review commit abc123. Không sửa code, commit hoặc push.",
            "Review the change. Do not edit, commit or push.",
            "Review the change. Don't edit, commit, or push.",
            "Review the change. Never edit files, commit changes, or push them.",
            "Do not edit unrelated files, commit changes, or push them.",
            "Do not edit unrelated files, commit changes and push them.",
            "Do not edit unrelated files, commit & push.",
            "Không sửa WIP khác, commit hoặc push.",
            "Inspect src/commit.ts and push.ts.",
            "Push notifications need review.",
            "Push đã xong.",
            "Run tests.",
        ] {
            assert_eq!(git_action(text), None, "{text}");
        }
    }

    #[test]
    fn button_list_ordering_never_authorizes_push() {
        let names = ["Edit", "Send", "Push"];
        for first in names {
            for second in names.iter().filter(|name| **name != first) {
                let third = names
                    .iter()
                    .find(|name| **name != first && *name != second)
                    .unwrap();
                for text in [
                    format!("Commit sửa nút {first}, {second} và {third}."),
                    format!("Commit updates to {first}, {second} and {third}."),
                    format!("Commit changes to {first}, {second}, and {third} buttons."),
                ] {
                    let action = git_action(&text).unwrap();
                    assert_eq!(action.kind, GitKind::Commit, "{text}");
                    assert!(action.prompt.starts_with("/commit --no-push\n"), "{text}");
                }
            }
        }
        assert_eq!(kind("Commit sửa nút Edit và Push."), Some(GitKind::Commit));
    }

    #[test]
    fn push_mentions_outside_an_explicit_command_do_not_turn_a_commit_into_a_push() {
        for text in [
            "Commit the fix. Review commit and push behavior.",
            "Commit changes to commit and push buttons.",
            "Commit the push notification fix.",
            "Commit sửa nút Edit, Push và Send.",
            "Commit changes to Edit, Push, and Send buttons.",
            "Commit changes to buttons named Commit and Push.",
            "Commit changes to buttons called Commit, Push.",
            "Commit sửa các nút có nhãn Commit, Push.",
            "Commit sửa các nút:Commit, Push.",
        ] {
            let action = git_action(text).unwrap();
            assert_eq!(action.kind, GitKind::Commit, "{text}");
            assert!(action.prompt.starts_with("/commit --no-push\n"), "{text}");
        }
    }

    #[test]
    fn excluding_unrelated_files_preserves_a_positive_commit_request_not_a_blanket_prohibition() {
        for text in [
            "Commit plugin changes. Không commit WIP khác.",
            "Không commit WIP khác. Commit plugin changes.",
            "Commit the fix. Do not commit unrelated files.",
            "Commit the fix. Không commit thay đổi không liên quan.",
            "/commit only the fix. Do not commit unrelated files.",
            "Without changing unrelated files, commit the fix.",
            "Do not edit unrelated files, commit the fix.",
            "Không sửa WIP khác, commit bản sửa.",
        ] {
            let action = git_action(text).unwrap();
            assert_eq!(action.kind, GitKind::Commit, "{text}");
            assert!(action.prompt.starts_with("/commit --no-push"), "{text}");
        }
        for text in [
            "Không commit WIP khác.",
            "Do not commit unrelated files.",
            "Commit the fix. Không commit.",
            "Commit the fix. Do not commit any changes.",
            "Commit the fix. Do not commit unrelated files. Do not commit.",
        ] {
            assert_eq!(git_action(text), None, "{text}");
        }
    }

    #[test]
    fn scoped_edit_restrictions_preserve_explicit_push_commands_and_later_no_push_constraints() {
        for text in [
            "Without changing unrelated files, commit and push the fix.",
            "Không sửa WIP khác, commit bản sửa rồi push.",
            "Commit changes to buttons named Commit and Push, then push.",
        ] {
            assert_eq!(kind(text), Some(GitKind::CommitPush), "{text}");
        }
        assert_eq!(
            kind("Without changing unrelated files, push the existing commit."),
            Some(GitKind::Push)
        );
        assert_eq!(
            kind("Without changing unrelated files, commit the fix. Do not push."),
            Some(GitKind::Commit)
        );
    }

    #[test]
    fn no_push_constraints_override_positive_push_wording_and_skill_defaults() {
        for suffix in [
            "Không push.",
            "Chưa push.",
            "Đừng push.",
            "Do not push.",
            "Don't push.",
            "Never push.",
            "Without pushing.",
            "--no-push",
            "Do not commit unrelated files or push.",
            "Do not commit unrelated files and push.",
            "Không commit WIP khác hay push.",
            "Không commit WIP khác hoặc push.",
            "Không commit WIP khác và push.",
            "Không được tự ý thực hiện push.",
            "Do not edit, push.",
        ] {
            let text = format!("Commit and push the fix. {suffix}");
            let action = git_action(&text).unwrap();
            assert_eq!(action.label, "Commit", "{text}");
            assert!(action.prompt.starts_with("/commit --no-push\n"), "{text}");
            assert!(
                action.prompt.contains(&text),
                "the original scope and constraints remain intact"
            );
        }
    }

    #[test]
    fn skill_commands_are_normalized_once_and_push_only_never_invokes_the_commit_skill() {
        for text in [
            "/commit only the fix",
            "$commit only the fix",
            "/commit --no-push only the fix",
        ] {
            let action = git_action(text).unwrap();
            assert!(
                action
                    .prompt
                    .starts_with("/commit --no-push only the fix\n"),
                "{text}"
            );
            assert_eq!(action.prompt.matches("--no-push").count(), 1);
            assert_eq!(action.prompt.matches("/commit").count(), 1);
            assert!(action.prompt.ends_with("Preserve unrelated work."));
        }
        let push = "Push commit 5324b19 lên origin/main. Giữ nguyên WIP.";
        let action = git_action(push).unwrap();
        assert_eq!(
            (action.kind, action.label, action.prompt.as_str()),
            (GitKind::Push, "Push", push)
        );
    }

    #[test]
    fn headings_multiline_vietnamese_multiple_suggestions_and_marker_removal() {
        assert_eq!(
            parse_prompts(
                "## What Next\n```text\nprompt: Kiểm tra UI.\nGiữ nguyên WIP.\nprompt: Báo kết quả.\n```",
            ),
            vec![PromptBlock {
                block: "prompt: Kiểm tra UI.\nGiữ nguyên WIP.\nprompt: Báo kết quả.".into(),
                prompts: vec![
                    "Kiểm tra UI.\nGiữ nguyên WIP.".into(),
                    "Báo kết quả.".into()
                ],
                whys: vec![String::new(), String::new()],
                threads: vec![false, false],
                suggestions: None,
                declaration: None,
            }]
        );
        assert_eq!(
            parse_prompts("## Next Steps\n~~~\nprompt: Test.\n~~~").len(),
            1
        );
    }

    #[test]
    fn a_why_line_under_a_prompt_is_its_reason_not_prompt_text() {
        let parsed = parse_prompts(
            "## What Next\n```\nprompt: Commit.\nwhy: Locks in the fix.\nprompt: Push.\n```",
        );
        assert_eq!(parsed[0].prompts, ["Commit.", "Push."]);
        assert_eq!(parsed[0].whys, ["Locks in the fix.", ""]);
        assert_eq!(
            parse_prompts("## What Next\n```\nwhy: Orphan.\nprompt: Test.\n```"),
            vec![]
        );
    }

    #[test]
    fn a_thread_new_line_marks_only_its_own_prompt_as_unrelated_work() {
        let parsed = parse_prompts(
            "## What Next\n```\nprompt: Commit and push the fix.\nprompt: Audit the logs.\nwhy: Separate issue.\nthread: new\n```",
        );
        assert_eq!(
            parsed[0].prompts,
            ["Commit and push the fix.", "Audit the logs."]
        );
        assert_eq!(parsed[0].whys, ["", "Separate issue."]);
        assert_eq!(parsed[0].threads, [false, true]);
    }

    #[test]
    fn a_suggestion_line_marks_only_its_own_prompt_and_is_never_prompt_text() {
        let parsed = parse_prompts(
            &[
                "## What Next",
                "```text",
                "prompt: Verify the layout.",
                "suggestion: true",
                "prompt: Review spacing.",
                "Suggestion: FALSE",
                "prompt: Audit the logs.",
                "why: Separate issue.",
                "thread: new",
                "SUGGESTION: True",
                "prompt: Check docs.",
                "suggestion: false",
                "suggestion: true",
                "```",
            ]
            .join("\n"),
        );
        assert_eq!(
            parsed[0].prompts,
            [
                "Verify the layout.",
                "Review spacing.",
                "Audit the logs.",
                "Check docs."
            ]
        );
        assert_eq!(
            parsed[0].suggestions,
            Some(vec![true, false, true, false]),
            "first occurrence wins"
        );
        assert_eq!(parsed[0].whys, ["", "", "Separate issue.", ""]);
        assert_eq!(parsed[0].threads, [false, false, true, false]);
    }

    #[test]
    fn an_unrecognised_suggestion_value_is_not_a_flag_and_the_line_stays_prompt_text() {
        let parsed = parse_prompts(
            "## What Next\n```\nprompt: Verify.\nsuggestion: maybe\nprompt: Review.\nsuggestion: true\n```",
        );
        assert_eq!(parsed[0].prompts, ["Verify.\nsuggestion: maybe", "Review."]);
        assert_eq!(parsed[0].suggestions, Some(vec![false, true]));
        assert_eq!(
            parse_prompts("## What Next\n```\nsuggestion: true\nprompt: Test.\n```"),
            vec![]
        );
    }

    #[test]
    fn ignore_incomplete_streams_ordinary_code_quotes_empty_entries_and_other_sections() {
        for text in [
            "```\nprompt: Test\n```",
            "## Next Steps\n```js\nprompt: test\n```",
            "> ## Next Steps\n> ```\n> prompt: Test\n> ```",
            "## Next Steps\n```\nprompt: Test",
            "## Next Steps\n```\nprompt: \n```",
            "## Next Steps\n## Example\n```\nprompt: Test\n```",
            "## Next Steps\n```\nexample\nprompt: Test\n```",
        ] {
            assert_eq!(parse_prompts(text), vec![], "{text}");
        }
    }

    #[test]
    fn headings_inside_a_code_fence_cannot_enable_prompt_parsing() {
        assert_eq!(
            parse_prompts("````\n## Next Steps\n```\nprompt: Test\n```\n````"),
            vec![]
        );
    }

    #[test]
    fn join_prompts_numbers_several_prompts() {
        assert_eq!(join_prompts(&["One.".into()]), "One.");
        assert_eq!(
            join_prompts(&["Implement.\nKeep files.".into(), "List risks.".into()]),
            "1. Implement.\n   Keep files.\n2. List risks."
        );
    }

    // The `parsePrompts`-only cases of `tests/next-prompts.test.ts`.

    fn declaration() -> Value {
        json!({
            "version": 1,
            "prompts": [
                {"id": "implement", "prompt": "Implement the layout.\nPreserve other files.",
                 "why": "Apply the design."},
                {"id": "review", "prompt": "Review only. Do not change code."},
                {"id": "risks", "prompt": "List remaining risks."},
            ],
            "exclusiveGroups": [["implement", "review"]],
            "allowedCombinations": [["implement", "risks"], ["review", "risks"]],
        })
    }

    fn markdown(value: &Value) -> String {
        format!("## What's Next\n```next-prompts\n{value}\n```")
    }

    #[test]
    fn v1_defaults_a_missing_prompt_id_to_its_position() {
        let collides = json!({"version": 1, "prompts": [
            {"prompt": "Apply G1."}, {"id": "p1", "prompt": "Apply G4."}]});
        assert!(parse_prompts(&markdown(&collides)).is_empty());
        let value = json!({"version": 1, "prompts": [
            {"prompt": "Apply G1."}, {"prompt": "Apply G4."}]});
        let blocks = parse_prompts(&markdown(&value));
        let ids: Vec<_> = blocks[0]
            .declaration
            .as_ref()
            .unwrap()
            .prompts
            .iter()
            .map(|p| p.id.as_str())
            .collect();
        assert_eq!(ids, ["p1", "p2"]);
    }

    fn with(mut value: Value, key: &str, field: Value) -> Value {
        value[key] = field;
        value
    }

    #[test]
    fn v1_parses_exact_prompts_and_reasons_only_in_completed_next_step_fences() {
        let value = declaration();
        let parsed = parse_prompts(&markdown(&value));
        assert_eq!(
            parsed[0].prompts,
            [
                "Implement the layout.\nPreserve other files.",
                "Review only. Do not change code.",
                "List remaining risks."
            ]
        );
        assert_eq!(parsed[0].whys, ["Apply the design.", "", ""]);
        let text = markdown(&value);
        for text in [
            text[..text.len() - 3].to_string(),
            text.replace("next-prompts", "json"),
            text.replace("## What's Next", "## Example"),
            text.split('\n')
                .map(|line| format!("> {line}"))
                .collect::<Vec<_>>()
                .join("\n"),
        ] {
            assert_eq!(parse_prompts(&text), vec![], "{text}");
        }
    }

    #[test]
    fn v1_ignores_unknown_fields_instead_of_rejecting_the_block() {
        let mut value = declaration();
        value["extra"] = json!(true);
        value["prompts"] =
            json!([{"id": "commit", "prompt": "Commit the fix.", "parallel": false}]);
        value["exclusiveGroups"] = json!([]);
        value["allowedCombinations"] = json!([]);
        let parsed = parse_prompts(&markdown(&value));
        assert_eq!(parsed[0].prompts, ["Commit the fix."]);
    }

    #[test]
    fn v1_rejects_invalid_versions_references_contradictory_declarations_and_oversized_data() {
        let base = declaration;
        let invalid = [
            with(base(), "version", json!(2)),
            with(base(), "prompts", json!([])),
            with(base(), "prompts", json!([{"id": "x", "prompt": " "}])),
            with(
                base(),
                "prompts",
                json!([{"id": "same", "prompt": "Test."}, {"id": "same", "prompt": "Other."}]),
            ),
            with(base(), "exclusiveGroups", json!([["implement", "missing"]])),
            with(
                base(),
                "exclusiveGroups",
                json!([["implement", "review"], ["review", "risks"]]),
            ),
            with(
                base(),
                "allowedCombinations",
                json!([["implement", "review"]]),
            ),
            with(
                base(),
                "allowedCombinations",
                json!([["implement", "implement"]]),
            ),
            with(
                base(),
                "allowedCombinations",
                json!([["implement", "missing"]]),
            ),
            with(
                base(),
                "allowedCombinations",
                json!([["implement", "risks"], ["risks", "implement"]]),
            ),
            with(
                base(),
                "prompts",
                json!([{"id": "x", "prompt": "x".repeat(16001)}]),
            ),
        ];
        for value in invalid {
            assert_eq!(parse_prompts(&markdown(&value)), vec![], "{value}");
        }
        assert_eq!(
            parse_prompts("## What Next\n```next-prompts\n{broken}\n```"),
            vec![]
        );
    }

    #[test]
    fn v1_goal_and_thread_fields_parse_and_new_thread_prompts_never_join_relationships() {
        let mut value = declaration();
        value["goal"] = json!("done");
        value["prompts"]
            .as_array_mut()
            .unwrap()
            .push(json!({"id": "audit", "prompt": "Audit logs.", "thread": "new"}));
        let parsed = parse_prompts(&markdown(&value));
        assert!(parsed[0].declaration.as_ref().unwrap().goal_done);
        assert_eq!(parsed[0].threads, [false, false, false, true]);
        for invalid in [
            with(value.clone(), "goal", json!("open")),
            with(
                value.clone(),
                "prompts",
                json!([{"id": "x", "prompt": "Test.", "thread": "old"}]),
            ),
            with(
                value.clone(),
                "exclusiveGroups",
                json!([["implement", "audit"]]),
            ),
            with(
                value.clone(),
                "allowedCombinations",
                json!([["risks", "audit"]]),
            ),
        ] {
            assert_eq!(parse_prompts(&markdown(&invalid)), vec![], "{invalid}");
        }
    }

    #[test]
    fn v1_suggestion_flag_marks_prompts_for_display_and_never_rejects_a_block() {
        let value = json!({
            "version": 1,
            "prompts": [
                {"id": "implement", "prompt": "Implement the layout.", "suggestion": true},
                {"id": "review", "prompt": "Review only.", "suggestion": false},
                {"id": "risks", "prompt": "List remaining risks."},
                {"id": "audit", "prompt": "Audit logs.", "thread": "new", "suggestion": true},
                {"id": "text", "prompt": "Not a boolean.", "suggestion": "yes"},
                {"id": "number", "prompt": "Also not a boolean.", "suggestion": 1},
                {"id": "nil", "prompt": "Null.", "suggestion": null},
            ],
            "exclusiveGroups": [["implement", "review"]],
            "allowedCombinations": [["implement", "risks"]],
        });
        let parsed = parse_prompts(&markdown(&value));
        assert_eq!(
            parsed.len(),
            1,
            "a stray suggestion value must not hide the block"
        );
        assert_eq!(
            parsed[0].suggestions,
            Some(vec![true, false, false, true, false, false, false])
        );
        assert_eq!(parsed[0].prompts.len(), 7);
        let declaration = parsed[0].declaration.as_ref().unwrap();
        assert_eq!(declaration.exclusive_groups, [["implement", "review"]]);
        assert_eq!(declaration.allowed_combinations, [["implement", "risks"]]);
    }
}
