# TASK-321 Electron release app as a menu bar app

Group: F (`packages/desktop/src` main process files; standalone files)
Class: risky

## Brief

Goal: The release `Bachuc.app` shows no Dock icon and no window. A menu bar icon shows the daemon status and offers Open Bachuc GPUI, Restart daemon, Open logs, and Quit.

Change: launch opens the primary Electron window, whose renderer starts the daemon -> launch hides the Dock icon, starts the daemon from the main process, and adds a `Tray`.

Boundaries:

- Follows [ADR-0074](watchtower/decisions/ADR-0074-menu-bar-daemon-host-over-electron-windows.md): no Electron window on release.
- Scope is the installed release app on macOS. Gate the mode with `IS_RELEASE_APP` in `main.ts` (packaged and run from `/Applications/Bachuc.app`) and `process.platform === "darwin"`. `app.isPackaged` alone also matches a packaged Bachuc Dev build. Live and other builds keep today's window flow.
- Trap: `daemon-start-service.ts` in `packages/app` starts the daemon from the renderer. With no window, nothing starts it, so the main process reads `resolveDesktopDaemonStatus` first and calls `startDaemon` in `daemon/daemon-manager.ts` only when the daemon is not running (the login item may have started it).
- Trap: `startDaemon` throws when `manageBuiltInDaemon` is off (`assertBuiltInDaemonManagementEnabled`). Catch it, log it, and show "Daemon stopped" in the menu.
- Trap: `bootstrapComplete` in `main.ts` resolves after `openPrimary`. Menu bar mode skips `openPrimary` but must still resolve it, or `second-instance` launches wait forever.
- Trap: `openPrimary`, the `activate` handler, `second-instance`, and `open-url` in `main.ts` open windows. In menu bar mode each opens `/Applications/Bachuc GPUI.app` instead.
- Trap: Quit must keep the daemon. `handleBeforeQuit` in `quit-lifecycle.ts` stops it unless `keepRunningAfterQuit` is on. Planner call: menu Quit calls `app.exit(0)`, which skips `before-quit`, so the daemon stays. SIGTERM (used by `scripts/paseo-release.sh`) keeps today's path.
- Planner call: if `/Applications/Bachuc GPUI.app` is missing, log the error from `shell.openPath` and do nothing else.
- Use a monochrome template image for the tray icon (`trayTemplate.png` 16 px and `trayTemplate@2x.png` 32 px), so macOS tints it for light and dark menu bars. Planner call: make it from the light tree shape in `assets/icon.png` (light pixels to black, the rest transparent). `electron-builder.yml` ships only listed `extraResources`, so add the two files there and load them from `process.resourcesPath`.
- Read the status from `resolveDesktopDaemonStatus` in `daemon-manager.ts`; refresh it when the menu opens, not on a fast timer.

How:

- Add `features/menu-bar.ts` with the tray, its menu, a pure `buildMenuBarItems(status)` that returns the item labels and enabled flags, and a pure `needsDaemonStart(status)`.
- In `main.ts`, in menu bar mode: call `app.dock.hide()`, start the daemon, create the tray, and route window openings to GPUI.

Files:

- [packages/desktop/src/main.ts](packages/desktop/src/main.ts) (menu bar mode at startup, window routes)
- [packages/desktop/src/features/menu-bar.ts](packages/desktop/src/features/menu-bar.ts) (new: tray and menu)
- [packages/desktop/src/features/menu-bar.test.ts](packages/desktop/src/features/menu-bar.test.ts) (new: tests of `buildMenuBarItems`)
- [packages/desktop/src/daemon/daemon-manager.ts](packages/desktop/src/daemon/daemon-manager.ts) (export `startDaemon`, `restartDaemon`, and `getDaemonLogs`; today they are module-private)
- [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml) (ship the tray images as `extraResources`)
- `packages/desktop/assets/trayTemplate.png` and `trayTemplate@2x.png` (new: monochrome icon)

Interfaces:

- Produces: menu bar mode starts the daemon from the main process on launch, for TASK-322.

Expected result:

- WHEN the packaged app launches on macOS THE app SHALL show a menu bar icon and no Dock icon and no window.
- WHEN the daemon is not running at launch THE main process SHALL start it.
- WHEN the menu opens THE first item SHALL show "Daemon running · 127.0.0.1:6767" or "Daemon stopped".
- WHEN Hiep picks Open Bachuc GPUI, Restart daemon, or Open logs THE app SHALL open the GPUI app, restart the daemon, or open the daemon log.
- WHEN Hiep picks Quit THE app SHALL exit and the daemon SHALL keep running.
- IF the app runs from `npm run dev:desktop` THEN it SHALL keep today's window flow.

Red tests:

File: [packages/desktop/src/features/menu-bar.test.ts](packages/desktop/src/features/menu-bar.test.ts)

The builder writes `buildMenuBarItems` tests for a running and a stopped status: the first item's label, and that Restart daemon is enabled only when the daemon runs. It also writes `needsDaemonStart` tests: false for a running status, true for a stopped one.

Anti-goal: the existing `packages/desktop` tests keep passing with 0 edited expected values; tripwire; read with `npx vitest run` on `quit-lifecycle.test.ts`, `daemon-manager.test.ts`, and `menu.test.ts` before and at completion.

## Verify

- `npx vitest run packages/desktop/src/features/menu-bar.test.ts --bail=1` -> pass.
- `npx vitest run packages/desktop/src/daemon/quit-lifecycle.test.ts packages/desktop/src/daemon/daemon-manager.test.ts packages/desktop/src/features/menu.test.ts --bail=1` -> pass (anti-goal).
- `npm run typecheck` and `npm run lint -- <changed files>` -> pass.
- Owner check after `scripts/bachuc-release.sh`: one Bachuc icon in the Dock (GPUI), a menu bar icon, each menu item works, and Quit leaves `paseo daemon status` running.
