# NEXT

## Current Active Plan

- Title: Rebrand the fork as Bachuc
- Slug: 20261007-bachuc-rebrand
- Status: ACTIVE
- Updated: 2026-10-08

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                          | Group | Status | Spec                                                                                             | Deps               | Context                                        | Notes                              |
| ----- | --------------------------------------------- | ----- | ------ | ------------------------------------------------------------------------------------------------ | ------------------ | ---------------------------------------------- | ---------------------------------- |
| 1     | TASK-001 Logo concepts with Codex             | A     | DONE   | [watchtower/tasks/TASK-001-logo-concepts.md](watchtower/tasks/TASK-001-logo-concepts.md)         | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending        |
| 2     | TASK-002 Bachuc name in the app UI            | B     | TODO   | [watchtower/tasks/TASK-002-app-ui-name.md](watchtower/tasks/TASK-002-app-ui-name.md)             | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Locale files stay unchanged.       |
| 3     | TASK-003 Desktop identity and release install | C     | TODO   | [watchtower/tasks/TASK-003-desktop-identity.md](watchtower/tasks/TASK-003-desktop-identity.md)   | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | The owner runs the release script. |
| 4     | TASK-004 Apply the mark to icons and logo     | D     | TODO   | [watchtower/tasks/TASK-004-apply-icons.md](watchtower/tasks/TASK-004-apply-icons.md)             | TASK-001           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Uses the TASK-001 mark.            |
| 5     | TASK-005 Remove upstream network defaults     | E     | TODO   | [watchtower/tasks/TASK-005-network-defaults.md](watchtower/tasks/TASK-005-network-defaults.md)   | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Relay, web app, CORS, and hub.     |
| 6     | TASK-006 Turn off desktop auto-update         | C     | TODO   | [watchtower/tasks/TASK-006-auto-update-off.md](watchtower/tasks/TASK-006-auto-update-off.md)     | TASK-003           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Shares files with TASK-003.        |
| 7     | TASK-007 Attribution and upstream links       | F     | TODO   | [watchtower/tasks/TASK-007-attribution-links.md](watchtower/tasks/TASK-007-attribution-links.md) | TASK-002           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | NOTICE, README, and About.         |
| 8     | TASK-008 Docs for the Bachuc names            | G     | TODO   | [watchtower/tasks/TASK-008-docs.md](watchtower/tasks/TASK-008-docs.md)                           | TASK-003, TASK-006 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Instance names and log paths.      |

TASK Status labels: TODO, IN PROGRESS, BLOCKED, DONE.
Plan-level Status header: ACTIVE while any row is open, DONE when all rows DONE, ARCHIVED after archive.

## Plan Verify

- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed files>` -> exit 0.
- License notices stay unchanged: `git diff --exit-code main -- LICENSE packages/expo-two-way-audio/LICENSE packages/highlight/src/astro/LICENSE packages/desktop/src/features/browser-automation/aria-snapshot-script.ts packages/app/src/terminal/local-links/terminal-local-link-provider.ts packages/app/src/terminal/local-links/terminal-local-link-parsing.ts` -> exit 0.
- `rg -n "relay\.paseo\.sh|app\.paseo\.sh|hub\.paseo\.sh" packages/server/src packages/cli/src --glob '!**/*.test.*'` -> each match is a comment or the parser for old pairing links.
- `git diff --stat main -- 'hiep-plugins/plugins/*/package.json' packages/plugin/src` -> no output. Plugins keep working without change.
- Human check on live (`npm run dev`, then `npm run dev:desktop`): the window, menu, Settings, and welcome screen say Bachuc and show the new mark. Settings > About says the app is based on Paseo.
- Owner check after the next release install, run from Terminal.app: `/Applications/Bachuc.app` starts. `PASEO_HOME=~/.paseo paseo daemon status` shows home `~/.paseo` on `127.0.0.1:6767`. `readlink ~/.local/bin/paseo` points into `/Applications/Bachuc.app`.

## Handoff

- Next action: the autorun schedule builds TASK-002. TASK-001 is DONE; the owner still checks the mark in [brand/bachuc/mark.svg](brand/bachuc/mark.svg) at 16 px and 1024 px.
- After implement, the owner runs `scripts/paseo-release.sh` from Terminal.app. The agent never runs it.
- After the first Bachuc install, the owner grants macOS permissions again (microphone, screen recording, accessibility, automation), because the bundle ID changes.
- After the first Bachuc install, the owner removes the `app.baseUrl` key from `~/.paseo/config.json` and `.dev/paseo-home/config.json`. Both may still hold `https://app.paseo.sh`.

## Archive

- [watchtower/archive/20260930-active-machine-switch/](watchtower/archive/20260930-active-machine-switch/)
- [watchtower/archive/20260930-plugin-host-apis/](watchtower/archive/20260930-plugin-host-apis/)
- [watchtower/archive/20261006-t3code-orchestration-port/](watchtower/archive/20261006-t3code-orchestration-port/)
- [watchtower/archive/20261007-watchtower-process-overview/](watchtower/archive/20261007-watchtower-process-overview/)
