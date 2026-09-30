# Plan Context

## Shared Context

- The installed desktop and daemon are Paseo 0.9.1.
- The existing directory plugin is enabled and running from this checkout.
- Preserve the existing design images and unrelated changes. Do not commit or push.
- Legacy suggestions remain individually sendable. Bulk sends need an exact declared combination.
- The user authorized updating the global policy only after plugin verification.

## Decisions

- Use a versioned JSON fence named `next-prompts`.
- Exclusive groups allow at most one selection. Declared combinations are exact sets, not pairwise inference.
- Missing metadata never grants bulk-send permission. Git actions still require an individual manual send.
- Metadata describes declared intent. It does not prove semantic compatibility or parallel safety.

## References

- [Selected design](../docs/designs/recap-next-multiple-2026-09-25/02-select-exclusive.png)
- [Plugin](../plugins/next-prompt-actions/)
