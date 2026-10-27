# NEXT

## Current Active Plan

- Title: Clearer GPUI answers, Bash descriptions on release, and a menu bar daemon host
- Slug: 20261027-gpui-answer-code-citations
- Status: ACTIVE
- Updated: 2026-10-27

## Tracker

| Order | TASK                                                      | Group | Status      | Spec                                                                                                                 | Deps     | Context                                        | Notes                                                |
| ----- | --------------------------------------------------------- | ----- | ----------- | -------------------------------------------------------------------------------------------------------------------- | -------- | ---------------------------------------------- | ---------------------------------------------------- |
| 1     | TASK-316 Keep the shell description on the wire           | A     | DONE        | [watchtower/tasks/TASK-316-shell-description-on-wire.md](watchtower/tasks/TASK-316-shell-description-on-wire.md)     | -        | -                                              | Protocol, server, and GPUI tool summary. Q-198.      |
| 2     | TASK-317 Code block header hook in the vendored text view | B     | DONE        | [watchtower/tasks/TASK-317-code-block-header-hook.md](watchtower/tasks/TASK-317-code-block-header-hook.md)           | -        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Vendor only. Hot render path.                        |
| 3     | TASK-318 Code citation card in the GPUI answer            | C     | DONE        | [watchtower/tasks/TASK-318-code-citation-card.md](watchtower/tasks/TASK-318-code-citation-card.md)                   | TASK-317 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Starts after TASK-317. Q-197.                        |
| 4     | TASK-319 Investigation answer rule                        | D     | DONE        | [watchtower/tasks/TASK-319-investigation-answer-rule.md](watchtower/tasks/TASK-319-investigation-answer-rule.md)     | -        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Edits the global rules file outside the repo. Q-197. |
| 5     | TASK-320 Bash description reaches the stored timeline     | E     | DONE        | [watchtower/tasks/TASK-320-bash-description-on-release.md](watchtower/tasks/TASK-320-bash-description-on-release.md) | -        | -                                              | TASK-316 fails on release (Q-200).                   |
| 6     | TASK-321 Electron release app as a menu bar app           | F     | IN PROGRESS | [watchtower/tasks/TASK-321-electron-menu-bar-mode.md](watchtower/tasks/TASK-321-electron-menu-bar-mode.md)           | -        | -                                              | ADR-0074.                                            |
| 7     | TASK-322 Release script for the menu bar app              | G     | TODO        | [watchtower/tasks/TASK-322-release-script-menu-bar.md](watchtower/tasks/TASK-322-release-script-menu-bar.md)         | TASK-321 | -                                              | Starts after TASK-321.                               |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` and the `gpui-base` text tests -> pass.
- `npm run typecheck` and `npm run lint` -> pass.
- Screen check on live (human or UI checker): ask a live agent "find the cause of X" after TASK-319 -> the answer has Symptom, Cause, and Fix, and each code excerpt shows a "file Ln a-b" header that opens the file.
- After the checks, quit the GPUI app and run `cd packages/desktop-gpui && ./cargo.sh clean --profile dev` when no other cargo or rustc process runs.

## Review Focus

- The daemon is already running when the menu bar app launches (login item) -> no second daemon, status shows running; owner TASK-321.
- The daemon crashes while the app runs -> the menu shows stopped when opened, and Restart starts it; owner TASK-321.
- A Bash call whose description streams after the command -> the stored row still ends with the description; owner TASK-320.
- A fence meta that is not a path, such as `title="x"` -> no header, plain block; owner TASK-318.
- A cited path that does not exist in the thread cwd -> the header still shows; the click does what a broken file link does today; owner TASK-318.
- A citation fence while the message still streams (fence not closed) -> no header flicker; the block renders as code until closed; owner TASK-317.
- A shell call stored before TASK-316 -> the summary shows the command; owner TASK-316.
- A message with no code block -> no extra layout pass; owner TASK-318.

## Handoff

- Next action: run `/watchtower implement` for TASK-320 to TASK-322; then Hiep runs `scripts/bachuc-release.sh` and the owner checks.

## Archive

- None.
