# NEXT

## Current Active Plan

- Title: Rebrand the fork as Bachuc
- Slug: 20261007-bachuc-rebrand
- Status: ACTIVE
- Updated: 2026-10-08

## Tracker

One row per TASK. Group ties together items that write the same files.

| Order | TASK                                            | Group | Status | Spec                                                                                             | Deps               | Context                                        | Notes                                                      |
| ----- | ----------------------------------------------- | ----- | ------ | ------------------------------------------------------------------------------------------------ | ------------------ | ---------------------------------------------- | ---------------------------------------------------------- |
| 1     | TASK-001 Logo concepts with Codex               | A     | DONE   | [watchtower/tasks/TASK-001-logo-concepts.md](watchtower/tasks/TASK-001-logo-concepts.md)         | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending                                |
| 2     | TASK-002 Bachuc name in the app UI              | B     | DONE   | [watchtower/tasks/TASK-002-app-ui-name.md](watchtower/tasks/TASK-002-app-ui-name.md)             | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending                                |
| 3     | TASK-003 Desktop identity and release install   | C     | DONE   | [watchtower/tasks/TASK-003-desktop-identity.md](watchtower/tasks/TASK-003-desktop-identity.md)   | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending                                |
| 4     | TASK-004 Apply the mark to icons and logo       | D     | DONE   | [watchtower/tasks/TASK-004-apply-icons.md](watchtower/tasks/TASK-004-apply-icons.md)             | TASK-001           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending                                |
| 5     | TASK-005 Remove upstream network defaults       | E     | DONE   | [watchtower/tasks/TASK-005-network-defaults.md](watchtower/tasks/TASK-005-network-defaults.md)   | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending                                |
| 6     | TASK-006 Turn off desktop auto-update           | C     | DONE   | [watchtower/tasks/TASK-006-auto-update-off.md](watchtower/tasks/TASK-006-auto-update-off.md)     | TASK-003           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending                                |
| 7     | TASK-007 Attribution and upstream links         | F     | DONE   | [watchtower/tasks/TASK-007-attribution-links.md](watchtower/tasks/TASK-007-attribution-links.md) | TASK-002           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending                                |
| 8     | TASK-008 Docs for the Bachuc names              | G     | DONE   | [watchtower/tasks/TASK-008-docs.md](watchtower/tasks/TASK-008-docs.md)                           | TASK-003, TASK-006 | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | auto: manual checks pending                                |
| 9     | TASK-009 Rename the macOS executable to Bachuc  | C+G   | TODO   | [watchtower/tasks/TASK-009-executable-name.md](watchtower/tasks/TASK-009-executable-name.md)     | TASK-008           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Q-005. Shares files with TASK-003, TASK-006, and TASK-008. |
| 10    | TASK-010 Update the help menu e2e spec          | I     | TODO   | [watchtower/tasks/TASK-010-help-menu-e2e.md](watchtower/tasks/TASK-010-help-menu-e2e.md)         | -                  | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Follow-up of TASK-007.                                     |
| 11    | TASK-011 Nix relay without the upstream default | J     | TODO   | [watchtower/tasks/TASK-011-nix-relay.md](watchtower/tasks/TASK-011-nix-relay.md)                 | TASK-005           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Follow-up of the TASK-005 review.                          |
| 12    | TASK-012 Hub docs without the hosted fallback   | K     | TODO   | [watchtower/tasks/TASK-012-hub-docs.md](watchtower/tasks/TASK-012-hub-docs.md)                   | TASK-005           | [watchtower/CONTEXT.md](watchtower/CONTEXT.md) | Follow-up of the TASK-005 review.                          |

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

- Next action: the second autorun pass builds TASK-009 to TASK-012. The owner answered Q-005, Q-006, and Q-007 on 2026-10-08. Finish updates draft PR #38. TASK-001 is DONE; the owner still checks the mark in [brand/bachuc/mark.svg](brand/bachuc/mark.svg) at 16 px and 1024 px.
- TASK-008 is DONE. [CLAUDE.md](CLAUDE.md) and [docs/development.md](docs/development.md) name `/Applications/Bachuc.app`, the log folder `~/Library/Logs/Bachuc/`, and the userData folder `Bachuc Dev` of an opened build. The signing identity `Paseo Fork Local` stays, because the release script still uses it.
- TASK-007 is DONE. The owner still checks on live that Settings > About shows "Based on Paseo", and that the sidebar help menu, the open project screen, and Settings show no Discord, Sponsors, upstream GitHub, or changelog link. The e2e spec [packages/app/e2e/browser/sidebar-help.spec.ts](packages/app/e2e/browser/sidebar-help.spec.ts) still asserts the removed items.
- TASK-005 is DONE. Q-007 raised the limit to 104, and the branch `auto/TASK-005-network-defaults` merged into `bachuc-rebrand`. A new home has no relay endpoint, no CORS origin, no app base URL, and no hub origin. Hub commands need `--hub` or `PASEO_HUB_URL`. The owner still checks on live that the app connects on `127.0.0.1:6768` and that the daemon log shows relay off.
- TASK-006 is DONE. Updates are off through two constants: `APP_UPDATES_ENABLED` in [packages/desktop/src/features/app-updates-enabled.ts](packages/desktop/src/features/app-updates-enabled.ts) and `DESKTOP_APP_UPDATES_ENABLED` in [packages/app/src/desktop/updates/desktop-updates.ts](packages/app/src/desktop/updates/desktop-updates.ts). The owner still checks on live that Settings > About has no "App updates" or "Release channel" rows. After the next release install, the owner checks that the desktop log has no update check.
- TASK-004 is DONE. The owner still checks on live that the welcome screen, the startup splash, the open project screen, the tab favicon in its three states, and the Dock icon show the mark. The dev icon carries a `</>` badge (Q-006, answered: keep it).
- TASK-003 is DONE. It kept the macOS executable `Paseo`; Q-005 now asks for `Bachuc`, and TASK-009 renames it. The owner still checks the live window title and the first release install. In-app browser logins may reset after that install, because the app name changes.
- TASK-002 is DONE. The owner still checks on live that Settings, the help menu, the welcome screen, and the tab title say Bachuc, and that the `$PASEO_PORT` hint is unchanged.
- After implement, the owner runs `scripts/paseo-release.sh` from Terminal.app. The agent never runs it.
- After the first Bachuc install, the owner grants macOS permissions again (microphone, screen recording, accessibility, automation), because the bundle ID changes.
- After the first Bachuc install, the owner removes the `app.baseUrl` key from `~/.paseo/config.json` and `.dev/paseo-home/config.json`. Both may still hold `https://app.paseo.sh`.

## Archive

- [watchtower/archive/20260930-active-machine-switch/](watchtower/archive/20260930-active-machine-switch/)
- [watchtower/archive/20260930-plugin-host-apis/](watchtower/archive/20260930-plugin-host-apis/)
- [watchtower/archive/20261006-t3code-orchestration-port/](watchtower/archive/20261006-t3code-orchestration-port/)
- [watchtower/archive/20261007-watchtower-process-overview/](watchtower/archive/20261007-watchtower-process-overview/)
