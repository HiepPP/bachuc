# Plan Context

## Shared Context

- On 2026-10-27, Hiep compared the Cursor thread UI with the GPUI step list. The Cursor work log is not better than the GPUI step list. Only the Cursor answer reads clearer: Symptom, Cause, and Fix, with code excerpts in cards headed "lib.rs Ln 2331-2334".
- Citation fence format (Q-197 default): ` ```<lang> <path>:<start>-<end> ` or ` ```<lang> <path>:<line> `. TASK-317 keeps the meta, TASK-318 draws the card, and TASK-319 tells agents to write it.
- The step list stays as [ADR-0019](watchtower/decisions/ADR-0019-always-open-step-list-over-folding-group.md) set it. This plan does not change it.

## Decisions

- Drop the Cursor-style Log view spike; Hiep chose the answer direction on 2026-10-27.

## Open Decisions

- None.

## References

- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) `render_markdown`
- [packages/desktop-gpui/vendor/gpui-base/src/text/node.rs](packages/desktop-gpui/vendor/gpui-base/src/text/node.rs) `CodeBlock`
