# NEXT

## Current Active Plan

- Title: GPUI composer and outline polish
- Slug: 20261019-gpui-composer-and-outline-polish
- Status: ACTIVE
- Updated: 2026-10-19

## Tracker

One row per TASK. Group C holds TASK-253 and TASK-254, because both write `composer.rs`; TASK-254 runs after TASK-253. Group O shares no file with group C and runs in parallel.

| Order | TASK                                             | Group | Status | Spec                                                                                                                     | Deps     | Context                                        | Notes                        |
| ----- | ------------------------------------------------ | ----- | ------ | ------------------------------------------------------------------------------------------------------------------------ | -------- | ---------------------------------------------- | ---------------------------- |
| 1     | TASK-253 Toolbar labels shrink like Electron     | C     | TODO   | [watchtower/tasks/TASK-253-toolbar-labels-shrink.md](watchtower/tasks/TASK-253-toolbar-labels-shrink.md)                 | -        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-162.                       |
| 2     | TASK-254 Placeholder stops before the focus hint | C     | TODO   | [watchtower/tasks/TASK-254-placeholder-before-focus-hint.md](watchtower/tasks/TASK-254-placeholder-before-focus-hint.md) | TASK-253 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-163. Shares `composer.rs`. |
| 3     | TASK-255 Outline bars ease over 140 ms           | O     | TODO   | [watchtower/tasks/TASK-255-outline-bar-ease.md](watchtower/tasks/TASK-255-outline-bar-ease.md)                           | -        | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-168.                       |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.

## Plan Verify

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-composer -p bachuc-timeline` -> pass, in one cargo call.
- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> builds, no new warnings in changed files.
- `./cargo.sh fmt --check -p bachuc-composer -p bachuc-timeline` -> exit 0.
- Screen check by computer use, per [watchtower/CONTEXT.md](watchtower/CONTEXT.md).
- `./cargo.sh clean --profile dev` after the last check, when no other cargo runs.

## Review Focus

- A model name longer than the free toolbar width at zoom 150% -> the label ends in "…" and no pill is cut; owner TASK-253.
- The send button appears while the toolbar is near the density floor -> no pill is cut; owner TASK-253.
- The composer is focused, then blurred, while empty -> the hint hides and returns, and the placeholder width follows; owner TASK-254.
- Reduced motion on -> the bars change at once; owner TASK-255.
- A fast sweep across many bars -> each animation ends within 140 ms, and idle CPU returns under 1%; owner TASK-255.

## Handoff

- Next action: run `/watchtower implement`.

## Archive

- None.
