# TASK-323 Outcome

## Outcome

Status: DONE

Changed:

- `LeadLabel`, `match_lead_label`, and `TextViewStyle::with_lead_labels` in `vendor/gpui-base/src/text/style.rs`; `PartialEq` and `from_colors` cover the new field.
- `Paragraph::render` in `node.rs` takes a `body` flag (true for the paragraph block, false for headings and table cells). A body paragraph with a label renders through the inline flow, and `inline_flow_items` layers the chip over the label marks (`with_lead_label`).
- `text_size_ranges` in `inline.rs` keys its ranges on the scale `Option`, so a body-size chip (`Some(1.)`) does not merge with plain text.
- Patch note "Lead label chips (TASK-323)" in `BACHUC-PATCH.md`.

Contract:

- `LeadLabel { text, color, background }`, `with_lead_labels(Arc<[LeadLabel]>)`, `match_lead_label(chunks, labels)`. An empty label list keeps today's output: `text_size_ranges` gives the same ranges for existing input, and no paragraph changes its render path.

Verified:

- `./cargo.sh check --workspace` -> builds.
- `./cargo.sh test -p bachuc-timeline` -> all suites pass; the lib suite has 35 tests (32 before plus 3 new), 0 edited expected values.
- `git diff -U0 -- packages/desktop-gpui/vendor/gpui-base/src/text | rg '^-\s+assert'` -> no match.
- `./cargo.sh clippy -p bachuc-timeline` -> no warning in the changed files.
- UNSATISFIABLE: `cargo test -p gpui-base` fails ("requires dev-dependencies and is not a member of the workspace"), as in TASK-317. Matcher tests run from the timeline crate instead; the render paths are covered by the TASK-324 screen check.

Anti-goal:

- Before: `bachuc-timeline` lib 32 tests; read from the filtered-out count of the first `answer_labels` run, 2026-10-28.
- Final: 35 tests, 0 edited expected values; 2026-10-28.
- Result: PASS.

Lessons:

- `gpui-base` tests cannot run in the workspace; a separate `--manifest-path` build fails too, because the crate sits inside the workspace folder. Keep vendor rules in public pure functions and test them from a workspace crate.
