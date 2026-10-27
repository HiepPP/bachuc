# TASK-321 Outcome

## Outcome

Status: DONE (implement: manual checks pending)

Changed:

- `features/menu-bar.ts` adds `buildMenuBarItems`, `needsDaemonStart`, and `startMenuBar` (tray, menu rebuilt from a fresh status on each open).
- `main.ts`: `MENU_BAR_MODE` (`IS_RELEASE_APP` on macOS) hides the Dock icon, starts the daemon when it is down, creates the tray, and resolves `bootstrapComplete` without a window. Deep links and second launches open `/Applications/Bachuc GPUI.app`. Quit calls `app.exit(0)`, so the daemon stays.
- `daemon-manager.ts` exports `startDaemon`, `restartDaemon`, and `getDaemonLogs`.
- `assets/trayTemplate.png` and `trayTemplate@2x.png`, made from the tree in `assets/icon.png`, ship as `extraResources`.

Verified:

- `npx vitest run src/features/menu-bar.test.ts` (in `packages/desktop`) -> 3 passed.
- `quit-lifecycle`, `daemon-manager`, `menu`, `menu-bar`, and `desktop-packaging` tests -> 30 passed.
- `npm run typecheck --workspace=@getpaseo/desktop` and `npm run lint` on the 4 files -> pass.
- Owner check after `scripts/bachuc-release.sh` -> pending.

Anti-goal:

- Final: existing desktop tests pass with 0 edited expected values; 2026-10-27.
- Result: PASS.

Lessons:

- Run `packages/desktop` tests from that folder; `npx vitest run` from the repo root finds 0 tests there.
