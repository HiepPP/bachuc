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

## Follow-ups (2026-10-28, after the plan closed)

Owner asks in the same thread, built by the lead with no builder agents.

- New labels: "Changes:", "Evidence:", "Thay đổi:", "Bằng chứng:" (merged) and "Not cleaned:", "Chưa dọn dẹp:" (warning); the table has 28 labels. `./cargo.sh test -p bachuc-timeline --lib answer_labels` -> 3 passed.
- One-line Recap ([ADR-0077](watchtower/decisions/ADR-0077-recap-branch-line-only.md)): the global rules and the stop hook allow only the `<branch> · <commit state>` bullet; "Open:" and "Need from you:" move to the body. GPUI `compact_recap` folds the head alone into the chips (`./cargo.sh test -p bachuc-plugin-next-prompt-actions` -> 65 passed). The test case "no Did" changed from native rendering to a folded Recap, as ADR-0077 asks. Board `parseRecap` reads the branch line and takes Did from the bold opening, cut at 300 characters (`npx tsx --test tests/recaps.test.ts` -> 16 passed; typecheck and lint clean); board plugin reloaded on release and live.
- Spacing A1 (Claude Design "Bachuc — thread spacing (GPUI)"): `with_tight_lead_in` drops the paragraph gap below a top-level paragraph that ends with ":" before a list, so the label sits 4 px above its list; `LIST_SPACING_BOTTOM` goes from 16 to 24, so a list ends its group with a paragraph gap. Global rules B1 (a one-item group stays on the label line) and B3 (group labels come from the chip table). `./cargo.sh check --workspace` -> builds; `./cargo.sh test -p bachuc-timeline` -> all suites pass, lib 35.
- UNVERIFIED (unattended) on live: the check bundle window stopped drawing in the background, so the spacing was not seen on screen. `sidebar.json` was backed up and restored after the check.

## Release UI Check (2026-10-28)

Run by the lead with `cua_repl` on the release GPUI app, installed by `scripts/gpui-release.sh` from `53fe0a6f1` (Hiep away, owner asked). Theme claude-light, zoom 1.75. Agent "release chip spacing check" (Haiku, release) holds the sample answer.

- PASS, spacing A1: "Changes:" sits right above its list, and the list ends with a paragraph gap before "Cause:" (`/tmp/labels-shot/release-changes-cause.jpg`).
- PASS, chips: bold `**Cause:**` is a red chip; "Fix:" before an inline code span is a green chip and the code chip keeps its look; "Còn lại:" is a brown chip over its tight list (`/tmp/labels-shot/release-fix-open.jpg`).
- PASS, no chip: the "### Fix:" heading, the table cells "Cause:" and "Fix: in a cell", and "Fixes:" stay plain (`/tmp/labels-shot/release-heading-table.jpg`).
- PASS, one-item group: "Unverified: one item on the label line." is a chip followed by its text (`/tmp/labels-shot/release-unverified.jpg`).
- Not checked: the Claude dark theme (the owner works in light; `hiep-plugins/AGENTS.md`).
- Cleanup: the Explorer was shown again and this thread's tab was selected again; `layout.json` and `sidebar.json` match their backups.
