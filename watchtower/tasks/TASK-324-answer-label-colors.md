# TASK-324 Answer label colors in the GPUI timeline

Group: B (timeline and docs files; starts after TASK-323 by ordering, no shared file)
Class: code

## Brief

Goal: In the GPUI answer, the report, diagnosis, and recap labels show as tint chips in theme colors, so Hiep can scan an answer by role.

Change: every answer label draws in the body color -> each label in the table below draws as a chip with its role `tint_*` background and `status_*` text.

Design: Claude Design board "Bachuc — answer label colors (GPUI)", option 3, at <https://claude.ai/design/p/45e89f96-80df-4809-9b5b-885e622fd75a?file=Answer+Labels.dc.html>.

Boundaries:

- Follows [ADR-0076](watchtower/decisions/ADR-0076-semantic-tint-chips-for-answer-labels.md), which supersedes ADR-0075. Use only `Tokens` fields; add no hex value and no new token.
- Label table (label string -> role; each role is a `status_*` text and its `tint_*` background):
  - `status_merged`: "What I did:", "What happened:", "Why it matters:", "Did:", "Những gì đã làm:", "Chuyện gì đã xảy ra:", "Vì sao quan trọng:", "Đã làm:".
  - `status_warning`: "Symptom:", "Unverified:", "Open:", "Triệu chứng:", "Chưa xác nhận:", "Còn lại:".
  - `status_danger`: "Cause:", "Not fixed:", "Need from you:", "Nguyên nhân:", "Chưa sửa:", "Cần bạn:".
  - `status_success`: "Fix:", "Cách sửa:".
- Only assistant text uses the labels: `markdown_style` in [crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) feeds only `render_markdown`, which draws the `assistant_message` row and the assistant notes between steps. User messages use `StyledText`. The file viewer, the diff view, the pull request view, and plugin views keep no labels.
- Trap: `markdown_style` caches the style in `self.markdown_style`, and the theme observer drops the cache. Build the labels inside `markdown_style` from `tokens(cx)`, so a theme change, including a plugin theme, gives new colors.
- Non-goal: the Electron app in `packages/app`, and the global rules file outside the repo.

How:

- Add [crates/timeline/src/answer_labels.rs](packages/desktop-gpui/crates/timeline/src/answer_labels.rs) with a `const` table of (label, role) and `pub(crate) fn answer_labels(t: &Tokens) -> Arc<[LeadLabel]>`. Import `LeadLabel` and `match_lead_label` from `gpui_kit::base::text`.
- Call `.with_lead_labels(answer_labels(&t))` in `markdown_style`.
- Add one line to the `## Rules` section of [docs/gpui.md](docs/gpui.md): answer label chips come from the table in `answer_labels.rs` and use only theme `status_*` and `tint_*` tokens (ADR-0076).

Files:

- [packages/desktop-gpui/crates/timeline/src/answer_labels.rs](packages/desktop-gpui/crates/timeline/src/answer_labels.rs) (new: table, function, tests)
- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (`mod answer_labels;`, call in `markdown_style`)
- [docs/gpui.md](docs/gpui.md) (one rule line)

Interfaces:

- Consumes: `pub struct LeadLabel { pub text: SharedString, pub color: Hsla, pub background: Hsla }` from TASK-323
- Consumes: `TextViewStyle::with_lead_labels(self, labels: Arc<[LeadLabel]>) -> Self` from TASK-323
- Consumes: `match_lead_label(chunks: impl Iterator<Item = &str> + Clone, labels: &[LeadLabel]) -> Option<&LeadLabel>` from TASK-323

Expected result:

- WHEN the tokens are `Tokens::light()` THE table SHALL map each of the 22 labels to the light values of its role tokens.
- WHEN the tokens are `Tokens::claude_dark()` THE table SHALL map each label to the dark values of its role tokens.
- WHEN the theme changes THE next answer render SHALL use the label colors of the new theme.
- WHEN an answer holds "Symptom:", "Cause:", "Fix:", and a recap list with "Đã làm:", "Còn lại:", and "Cần bạn:" THE GPUI app SHALL show each label as a chip in its role colors, and a heading, a table cell, or "Fixes:" SHALL stay plain.

Red tests:

File: [packages/desktop-gpui/crates/timeline/src/answer_labels.rs](packages/desktop-gpui/crates/timeline/src/answer_labels.rs)

- `every_label_maps_to_its_role_tokens_in_both_schemes`: 22 labels; each maps to its `status_*` and `tint_*` in `Tokens::light()` and `Tokens::claude_dark()`.
- `a_label_matches_only_whole_at_the_start_of_the_text`: "Fix: x", split runs, "Fix:" alone, Vietnamese labels match; "Fixes: x", "fix: x", "Fix:x", "The Fix: x", "Did:x" do not.
- `the_longest_matching_label_wins`.

Anti-goal: the `bachuc-timeline` tests keep their pass count (plus the new test) with 0 edited expected values; tripwire; read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline answer_labels` -> pass.
- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> pass; count is the old count plus 3 (anti-goal).
- `rg -n 'hex\(|0x[0-9a-fA-F]{6}|rgb' packages/desktop-gpui/crates/timeline/src/answer_labels.rs` -> no match.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-timeline` -> no new warning.
- `npm run format:files -- docs/gpui.md` -> no change left.
- Screen check on live GPUI (UI checker, per [docs/gpui.md](docs/gpui.md) Screen checks): show an answer with "Symptom:", "**Cause:**", "Fix:", an inline code span after "Fix:", and a recap list with "Đã làm:", "Còn lại:", "Cần bạn:" -> each label is a chip in its role colors; the rest of each line, the heading, the table cells, and "Fixes:" stay plain.
- Screen check: switch from light to Claude dark while the answer is open -> the labels change color without a restart.
