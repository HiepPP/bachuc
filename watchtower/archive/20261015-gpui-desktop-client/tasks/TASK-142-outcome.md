# TASK-142 Outcome

## Status

DONE. The VoiceOver check is UNVERIFIED (unattended).

## Changed

- `packages/desktop-gpui/docs/a11y-audit.md`: new report (one row per element, plus "How to re-run").

## Contract

- The report lists the sidebar, host switcher, tab bar, status line, timeline rows and steps, permission buttons, composer, pills, terminal, and the new-agent dialog. Each row has the expected role and label, the seen role and label, and pass or gap.
- Gaps: host option and pill menu items without a role or selected state (`bachuc-app`, `bachuc-composer`); the diff toggle inside the step label (`bachuc-timeline`); rows scrolled off screen missing from the tree (`bachuc-timeline`); no terminal output text (`bachuc-terminal`); no selected state in the new-agent dialog (`bachuc-app`); the `FocusId(5v1)` element not named.

## Verified

- Agent walk with `cua_repl` `getAXState` on `Bachuc GPUI.app` against live: mock agent `7d7ed6f` and claude agent `c45ffc4` in `default` mode (Write permission buttons Allow and Deny seen); host switcher, mode pill menu, new-agent dialog (Cmd+N), and terminal tab (Cmd+T) read -> labels in the report.
- VoiceOver pass -> UNVERIFIED (unattended).
- `rg -c "^\| " packages/desktop-gpui/docs/a11y-audit.md` -> 43 rows, one or more per element in How. Lead re-read on 2026-10-16: 43.
- Cleanup: app quit, keep-alive loop stopped, lock released, both agents allowed, stopped, archived; terminals `2ccb70e9` and `36aca98d` killed. Release 6767 not touched. Lead re-check on 2026-10-16: no `bachuc-desktop` process, `terminal ls --all` empty, no screen lock left.
- Format: the builder's format run skipped the files. The lead ran `npm run format:files` on 2026-10-16; `npx oxfmt --check` on the report and this outcome then passed.

## Anti-goal

`git status --short -- packages/desktop-gpui` at 2026-10-15T18:17:44Z: only `?? packages/desktop-gpui/docs/` (the report). Result: PASS.

## Lessons

- The AX tree lags a click: three terminals opened because the first reads showed no tab. Read the tree twice after a click, and wait before you repeat it.
- A `cua_repl` click on `Cancel new agent` looked ignored for 2 s; Escape closed the dialog.
- The timeline list exposes only drawn rows, so an audit of a long turn needs a scroll step.
- `cua_repl` already holds helpers `getax()` and `clickLabel(regex)` from earlier runs.
