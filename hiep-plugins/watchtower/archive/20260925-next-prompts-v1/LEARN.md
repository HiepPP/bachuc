# Learn 20260925-next-prompts-v1

## Summary

Discrepancy: 3 found. TASK-001 shipped as planned. TASK-002 shipped much more than its spec: a v3 single panel, a fix for Paseo history rows, read-only earlier replies, and a fix for scroll jank.

## Per TASK

- TASK-001: match. The plan and shipped result agree: strict `next-prompts` v1 parsing, selection rules, controls, and the global policy update after the gates passed. Note: a real Send into an agent was not tested in this TASK. It was confirmed later, during TASK-002.
- TASK-002: the plan was the selected v2 image (`02-select-exclusive.png`): a compact Recap strip, separate choice groups, and a shared action bar. What shipped: a v3 panel that folds the Recap and the What Next heading into the prompt panel. It follows [TASTE.md](../../../docs/designs/recap-next-panel-2026-09-25/TASTE.md): one accent and Phosphor icons. It also handles Paseo history rows, keeps earlier replies read-only, and fixes a scan race that caused scroll jank. Mistake: the spec assumed one `assistant-message` per reply. Paseo 0.9.1 renders each Markdown block of a reply as its own history row, so fixtures built on a guessed DOM passed while the desktop failed. Fix: capture the live host DOM before building a DOM adapter, and add a TASK for each new scope (redesign, row splitting, jank).
- TASK-002 verify: `cua_repl` stayed blocked with `Computer Use was not approved to use Paseo`. Desktop evidence came from user screenshots, user reports, and short-lived diagnostic RPCs that were removed afterward. Final acceptance was reported by the user, not observed by the agent.

## Plan-Level

- Scope creep: three user-directed redesigns (v1, v2, v3), read-only earlier replies, and virtualization fixes all went into one TASK. The plan title, CONTEXT, and selected-design link stayed on v2, so the plan did not describe what was shipped.
- Dependency gap: nothing in the plan covered host rendering facts (history rows, TanStack virtualization, block-height estimates) before UI work began.

## Lessons

- Before a DOM adapter, capture a real host DOM sample and keep it as a test fixture.
- For UI changes, approve a standalone HTML/CSS artifact first, then port it.
- Reproduce scroll issues with the host's real virtualizer library and count scroll corrections; guessing wasted two rounds.
- An async scan must not clean up nodes created after it listed the page.
- When the user changes the design or scope, add a TASK or update the spec before building.
