# TASK-322 Release script for the menu bar app

Group: G (`scripts/paseo-release.sh` and docs; ordered after TASK-321, no shared file)
Class: code

## Brief

Goal: `scripts/paseo-release.sh` installs the menu bar app and starts the daemon without an Electron window, and the docs describe the new flow.

Change: the script runs `open -n "$APP"` and waits for the daemon the window starts -> it opens a new instance in the background (`open -g -n`) and waits for the daemon that the main process starts.

Boundaries:

- Follows [ADR-0074](watchtower/decisions/ADR-0074-menu-bar-daemon-host-over-electron-windows.md).
- Keep `start_and_verify`, `daemon_matches`, the rollback, and the step numbers; `--dry-run` must still change nothing.
- The rollback starts the previous app, which still has windows; that is fine.
- Trap: `app_pid` and `stop_release` find the app by its main executable; a menu bar app has the same executable, so they keep working.

How:

- Change the open call in `start_and_verify` to `open -g -n`; keep `-n`, so a second launch does not hit the single-instance lock.
- Rewrite the release part of `docs/development.md` and the "Daily app" part of `docs/gpui.md` for one Dock icon and the menu bar app.

Files:

- [scripts/paseo-release.sh](scripts/paseo-release.sh) (open the app in the background)
- [docs/development.md](docs/development.md) (release flow)
- [docs/gpui.md](docs/gpui.md) (daily app section)

Interfaces:

- Consumes: menu bar mode starts the daemon from the main process on launch, from TASK-321.

Expected result:

- WHEN the script installs the release THE daemon SHALL start and `daemon_matches` SHALL pass with no Electron window.
- WHEN the script runs with `--dry-run` THE script SHALL print the steps and change nothing.

Anti-goal: the dry run keeps the same 12 numbered steps; tripwire; read with `scripts/paseo-release.sh --dry-run --skip-build | grep -c '^\['` before and at completion.

## Verify

- `bash -n scripts/paseo-release.sh` -> exit 0.
- `scripts/paseo-release.sh --dry-run --skip-build | grep -c '^\['` -> the same count as before the change; the release daemon PID does not change.
- `npm run format:files -- docs/development.md docs/gpui.md` -> clean.
- Owner check: `scripts/bachuc-release.sh` from Terminal.app ends with the daemon running and no Electron window.
