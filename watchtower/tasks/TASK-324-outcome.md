# TASK-324 Outcome

## Outcome

Status: DONE

Changed:

- New `crates/timeline/src/answer_labels.rs`: 22 labels in four roles; each role is a `status_*` text and `tint_*` background pair from `Tokens`.
- `markdown_style` in `crates/timeline/src/lib.rs` calls `.with_lead_labels(answer_labels::answer_labels(&t))`, so only assistant text gets chips, and a theme change rebuilds them.
- One rule line in `docs/gpui.md` `## Rules` (ADR-0076).

Contract:

- Consumes `LeadLabel`, `with_lead_labels`, and `match_lead_label` from TASK-323.

Verified:

- `./cargo.sh test -p bachuc-timeline answer_labels` -> 3 passed.
- `./cargo.sh test -p bachuc-timeline` -> all suites pass; lib 35 (32 + 3).
- `rg -n 'hex\(|0x[0-9a-fA-F]{6}|rgb' crates/timeline/src/answer_labels.rs` -> no match.
- `./cargo.sh clippy -p bachuc-timeline` -> no warning in the changed files; the 3 timeline warnings are Known Failures (`plugin_view.rs:185`, `lib.rs:377`, `lib.rs:3575`).
- `npm run format:files -- docs/gpui.md` -> no change left.

## UI Checks

Run 2026-10-28 by the lead with `cua_repl` (AX actions and background keys; check bundle "Bachuc GPUI LabelCheck" on live 6768, light theme). Agents "label chip check" and "label chip check 2" (Haiku, live) hold the sample answers.

- PASS, recap list items: "Đã làm:" draws as a purple `status_merged` chip, "Còn lại:" as a brown `status_warning` chip, "Cần bạn:" as a red `status_danger` chip. The rest of each line and the "Recap" heading stay plain. Evidence: `/tmp/labels-shot/recap-light.jpg`, `/tmp/labels-shot/recap-narrow-light.jpg` (narrow pane: the chips wrap with their line).
- UNVERIFIED (unattended): the bold `**Symptom:**`/`**Cause:**` chips, "Fix:" with an inline code span, the "### Fix:" heading, the table cells, the "Fixes:" line, and the Claude dark theme. The background window stopped drawing after it was covered, and scroll events did not move the timeline, so later screenshots repeated the old frame.
- Not cleaned: the check app wrote `layout.json` (`explorerOpen` false); the lead set it back to true. `sidebar.json` and `prompt-history.json` were not backed up before the check, and the daily GPUI app wrote them at the same time, so the lead cannot tell or undo any change from the check app.
