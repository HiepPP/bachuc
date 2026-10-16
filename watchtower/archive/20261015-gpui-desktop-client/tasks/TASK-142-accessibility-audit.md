# TASK-142 Accessibility audit with VoiceOver and the AX tree

Group: H (writes only the audit report)
Class: docs

## Brief

Goal: Check that the GPUI app is usable with VoiceOver and readable through the accessibility tree, and record every gap as a follow-up item.

Change: per-TASK labels with no full check -> one audited app with a repeatable AX checklist.

Boundaries:

- Follows [ADR-0034](watchtower/decisions/ADR-0034-rust-gpui-client-over-swift-and-electron.md) `Revisit If`: if VoiceOver cannot read the sidebar, the timeline, and the composer, the audit says so plainly.
- Do not fix UI code here. Each gap becomes a row in the report, with the crate, the element, and the expected role and label. The lead turns the gaps into TASKs.
- Read the AX tree with the `cua_repl` MCP tool `getAXState`. It is an agent tool, so the repeatable check is a checklist in the report that an agent walks, not a script. Do not use AppleScript, `osascript`, or System Events.
- The VoiceOver pass needs a human. Mark it as a manual check.

How:

- Walk each screen with `cua_repl` `getAXState`: sidebar, host switcher, tab bar, timeline rows and steps, permission buttons, composer, pills, terminal, and the new-agent dialog.
- Write the report: one row per element with the screen, the expected role and label, the seen role and label, and pass or gap.
- Add a short "How to re-run" section with the bundle command and the walk order.

Files:

- [packages/desktop-gpui/docs/a11y-audit.md](packages/desktop-gpui/docs/a11y-audit.md) (new report and checklist)

Interfaces:

- Consumes: the bundle `target/bundles/Bachuc GPUI.app` built by `bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop` from TASK-141

Expected result:

- The report lists every element above with pass or gap.
- Each gap names the crate and the missing role or label.

Anti-goal: no file outside [packages/desktop-gpui/docs](packages/desktop-gpui/docs) changes; tripwire; read with `git status --short -- packages/desktop-gpui` at completion.

## Verify

- `rg -c "^\| " packages/desktop-gpui/docs/a11y-audit.md` -> at least one row per element listed in How
- Agent check: follow "How to re-run" in the report with `cua_repl` on the bundled app on live -> each row marked pass is seen in the AX tree with its label
- Manual check (needs a human): turn on VoiceOver with Cmd+F5, then move through the sidebar, a timeline message, and the composer -> VoiceOver reads each one; record the result in the report
- Anti-goal: `git status --short -- packages/desktop-gpui` -> only `packages/desktop-gpui/docs/a11y-audit.md` changed by this TASK
