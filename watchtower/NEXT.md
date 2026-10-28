# NEXT

## Current Active Plan

- Title: GPUI answer label colors from theme tokens
- Slug: 20261028-gpui-answer-label-colors
- Status: DONE
- Updated: 2026-10-28

## Tracker

| Order | TASK                                                 | Group | Status | Spec                                                                                                 | Deps     | Context | Notes                                       |
| ----- | ---------------------------------------------------- | ----- | ------ | ---------------------------------------------------------------------------------------------------- | -------- | ------- | ------------------------------------------- |
| 1     | TASK-323 Lead label colors in the vendored text view | A     | DONE   | [watchtower/tasks/TASK-323-lead-label-hook.md](watchtower/tasks/TASK-323-lead-label-hook.md)         | -        | -       | 3b6976092. ADR-0076.                        |
| 2     | TASK-324 Answer label colors in the GPUI timeline    | B     | DONE   | [watchtower/tasks/TASK-324-answer-label-colors.md](watchtower/tasks/TASK-324-answer-label-colors.md) | TASK-323 | -       | b69edf27a. implement: manual checks pending |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p gpui-base text::` and `./cargo.sh test -p bachuc-timeline` -> pass.
- Screen check on live GPUI (human or UI checker): an answer with the report, diagnosis, and recap labels shows each label in its ADR-0076 chip in light and Claude dark.
- After the checks, quit the GPUI app and run `cd packages/desktop-gpui && ./cargo.sh clean --profile dev` when no other cargo or rustc process runs.

## Review Focus

- A label while the message streams and fades in -> the label ends in its role color, with no flash of the body color after the fade; owner TASK-323.
- A paragraph that starts with a label and holds an inline code chip (inline flow path) -> the label is colored and the line wraps as it does today; owner TASK-323.
- A heading or table cell that starts with "Fix:" -> no color; owner TASK-323.
- Text such as "Fixes:", "fix:", or "Fix:x" -> no color; owner TASK-323.
- A theme change while an answer is open -> new label colors without a restart; owner TASK-324.

## Handoff

- Next action: Hiep opens the agents "label chip check" and "label chip check 2" on live in light and Claude dark and checks the UNVERIFIED lines in [watchtower/tasks/TASK-324-outcome.md](watchtower/tasks/TASK-324-outcome.md), then reviews `feat/gpui-answer-label-colors` and merges it.

## Archive

- None.
