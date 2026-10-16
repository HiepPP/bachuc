# Learn 20261015-gpui-desktop-client

## Summary

Discrepancy: 7 found. All 11 TASKs shipped on `feat/bachuc-gpui`, and Plan Verify passed after TASK-144. The gaps were spec guesses about live protocol behavior, CPU checks that could not pass, and run process issues.

## Per TASK

- TASK-134: match. Builder note: one `sed -i` edit broke the edit-tool rule; the result was rebuilt and tested.
- TASK-135: plan said `git diff --exit-code` proves the schema export is stable -> the file was untracked, so the check could not fail. Mistake: an idempotence check for a new file. Fix: compare `shasum` of two runs.
- TASK-136: plan said restore mode `"full-snapshot"` gives the `0x04` JSON grid -> live sends `0x05` ANSI text in that mode, so the client subscribes with no restore mode (Q-059, [ADR-0042](watchtower/decisions/ADR-0042-json-grid-terminal-restore-over-ansi-snapshot.md)). Plan also said `agent.timeline.replacement` names a range -> it carries only `agentId` and `epoch` (Q-060). Mistake: protocol facts taken from the docs, not from live frames. Fix: record live fixtures before the spec names frame shapes.
- TASK-137: plan said the scroll check stays under 10% CPU -> a 60 fps scroll with no diagrams already used 9.6%. The lead changed the check to the diagram cost against a no-image control (Q-058). Mistake: a limit set from a stream baseline and used for a scroll. Fix: set a scroll limit against a no-content control run.
- TASK-138: match after a fix. The first build put a whole turn in one list item (74% CPU); one item per step fixed it. The final read was 9.5% at step 56, with a thin margin (Q-064).
- TASK-139: plan said `inspect` or the timeline shows the sent image -> the timeline `user_message` has no image field, and the mock ignores images, so a real Claude agent confirmed the image. A `cua_repl` `getApp` hang held the shared screen lock for about 3 hours.
- TASK-140: plan said open the terminal in `/tmp` -> `create_terminal` needs a daemon workspace folder, so checks used `/tmp/gpui-bench`. `htop` is not installed, so `top` replaced it.
- TASK-141: plan said 10% CPU with a stream tab -> 9.7% on a young turn passed, but 13.4% on a 15-minute turn failed. TASK-144 fixed it. Notifications are refused for the unsigned bundle, which the spec allowed as a gap.
- TASK-142: match. The builder's format run passed no file to the formatter (zsh word split), and the lead formatted the report. The VoiceOver pass is still open.
- TASK-143: match. The same format issue; the lead formatted the files. [ADR-0038](watchtower/decisions/ADR-0038-native-data-kinds-and-webview-panes-over-all-webview-or-rewrite.md) was accepted.
- TASK-144: match. The app read 7.5% CPU at 2 minutes and 8.9% at 15 minutes, down from 11.3% ([ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md)).

## Plan-Level

- Plan misdescribed reality: the CPU anti-goals measured a young stream only. Turn length drove the cost, so the first plan missed the long-turn case until TASK-141 measured it by chance. TASK-144 was added late.
- Process: Paseo `create_agent` failed for waves 1 to 5, because the release `jev-orchestrator` plugin kept over 500 leases across reloads. Builders ran as native subagents and stayed off the board.
- Process: three wave 3 builders shared one screen. The `/tmp/gpui-ui-lock` lock kept their UI checks apart, but one hung `cua_repl` call held it for about 3 hours.

## Open Items

- Q-069: after about 40 minutes, one GPUI app window drew only its left 350 of 1100 points, with full-width layout. A relaunch drew the full width. The cause is unknown. Source: [TASK-144 outcome](watchtower/archive/20261015-gpui-desktop-client/tasks/TASK-144-outcome.md).
- VoiceOver pass: not run; it needs a human. [ADR-0034](watchtower/decisions/ADR-0034-rust-gpui-client-over-swift-and-electron.md) `Revisit If` asks whether VoiceOver can read the sidebar, the timeline, and the composer.
- The six accessibility gaps in [packages/desktop-gpui/docs/a11y-audit.md](packages/desktop-gpui/docs/a11y-audit.md):
  - Host options and mode and model menu items have no role name or selected state (`bachuc-app`, `bachuc-composer`).
  - The diff toggle and the diff text sit inside the step row's own label, with no separate button (`bachuc-timeline`).
  - Rows off screen are missing from the AX tree, because the list draws only visible rows (`bachuc-timeline`).
  - Terminal output text is not in the AX tree; the pane shows only its label (`bachuc-terminal`).
  - The new-agent dialog shows no selected state on the chosen provider and model (`bachuc-app`).
  - The element behind the warning "focused element (FocusId(5v1)) has no accessibility node" is not named.
- Owner questions that leave with the plan: Q-057 (export the `server_info` schema in the next GPUI plan), and the DEFAULTED rows Q-055, Q-056, Q-058, Q-060 to Q-064, and Q-066 to Q-068 in [QUESTIONS.md](watchtower/archive/20261015-gpui-desktop-client/QUESTIONS.md).
- Untested app cases from TASK-141: zero agents, a wrong host password, Cmd+W on a live tab, and Cmd+2 to Cmd+9.
- Reopen check: no accepted ADR `Revisit If` holds now. The ADR-0034 VoiceOver trigger stays unchecked until the VoiceOver pass.

## Lessons

- Record live protocol fixtures before a spec names frame shapes or message fields.
- Measure CPU anti-goals at two turn ages, because per-flush work can grow with the turn.
- Set a scroll CPU limit against a no-content control run.
- Check that `create_agent` works before a run that needs Paseo builders.
- Wrap every `cua_repl` step in a timeout, and keep a GPUI window in front with an `open` loop during reads.
- Pass formatter file lists as separate arguments, and treat "Expected at least one target file" as a failed check.
