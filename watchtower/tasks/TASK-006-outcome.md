# TASK-006 Outcome

## Outcome

Status: DONE

Changed:

- [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml): the `publish` block is gone, so builds ship no `app-update.yml`.
- [packages/desktop/src/features/app-updates-enabled.ts](packages/desktop/src/features/app-updates-enabled.ts) is new. It holds the desktop gate `APP_UPDATES_ENABLED = false`. A separate module lets the test turn the gate on and off with `vi.mock`.
- [packages/desktop/src/features/auto-updater.ts](packages/desktop/src/features/auto-updater.ts): the update service gets `isPackaged: () => APP_UPDATES_ENABLED && app.isPackaged`. The service then returns "no update" from check, download, and install-on-quit, and never calls electron-updater. The updater module stays.
- [packages/app/src/desktop/updates/desktop-updates.ts](packages/app/src/desktop/updates/desktop-updates.ts): the new app gate `DESKTOP_APP_UPDATES_ENABLED = false` makes `shouldShowDesktopUpdateSection()` return false. That one function feeds `useDesktopAppUpdater`. So the "Release channel" and "App updates" rows, the automatic update checks, and the update callout are all off.
- [packages/app/src/desktop/updates/rosetta-callout-source.tsx](packages/app/src/desktop/updates/rosetta-callout-source.tsx): the Rosetta warning stays, but it has no Download button. That button was the only user of `RELEASE_DOWNLOAD_BASE_URL` and of the `paseo.sh/download` fallback.
- [packages/desktop/src/features/auto-updater.test.ts](packages/desktop/src/features/auto-updater.test.ts): a new case turns the gate off. Check, download, and install-on-quit then make no call to `checkForUpdates`, `downloadUpdate`, or `quitAndInstall`.
- Not changed: [packages/desktop/src/main.ts](packages/desktop/src/main.ts) and [packages/app/src/desktop/components/desktop-updates-section.tsx](packages/app/src/desktop/components/desktop-updates-section.tsx). The About rows live in `DesktopAppUpdateRow` in [packages/app/src/screens/settings-screen.tsx](packages/app/src/screens/settings-screen.tsx). That row already returns `null` when `useDesktopAppUpdater` reports `isDesktopApp: false`, so that file needs no edit.

Contract:

- The desktop app never checks, downloads, or installs an update. Each entry point goes through the service `isPackaged` dep, so the gate lives in one place per package.
- Settings > About keeps "App version" and "What's new". The "Release channel" and "App updates" rows are hidden.
- To turn updates on again, set both constants to `true` and restore the `publish` block. A merge that brings back only the `publish` block does not turn updates on.
- GitNexus impact: `shouldShowDesktopUpdateSection` is CRITICAL (5 modules). Its only direct caller is `useDesktopAppUpdater`, and every caller already handles `false`, as on plain web. That reach is the goal of this TASK. `configure` is UNKNOWN in the index. A text search shows that its only caller is [packages/desktop/src/features/app-update-service.ts](packages/desktop/src/features/app-update-service.ts), and this TASK does not edit it.

Verified:

- `npx vitest run packages/desktop/src/features/auto-updater.test.ts --bail=1` -> 13 passed, including the new case. The old cases pass with the gate on, and the new case passes with it off, so the mock toggles the gate for real.
- `rg -n "^publish:" packages/desktop/electron-builder.yml` -> no match.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <6 changed files>` -> 0 warnings, 0 errors.
- `npm run format:check:files -- <6 changed files>` -> all files use the correct format.
- `npx vitest run src/desktop/updates/desktop-updates.test.ts --bail=1` from `packages/app` -> 11 passed.
- `rg -n "getpaseo/paseo|github.com/getpaseo" packages/desktop/electron-builder.yml packages/desktop/src --glob '!**/*.test.*'` -> no match.
- Plan Verify: the license notice diff against `main` -> exit 0. The plugin diff stat -> no output.
- Human check on live desktop: Settings > About shows the app version and no "App updates" or "Release channel" rows. UNVERIFIED (autonomous run).
- Owner check after the next release install: the desktop log has no line that checks for updates. UNVERIFIED (autonomous run).

Anti-goal:

- Before changes: 52 lines from earlier TASKs (TASK-003 and TASK-004), so TASK-006 had used 0; `git diff --numstat main -- packages/desktop packages/app/src/desktop ':(exclude)**/*.test.ts'`, 2026-10-08 11:21.
- After the gate: 83 lines, so TASK-006 used 31; same command, 2026-10-08 11:24.
- Final: 31 lines in existing upstream files (`desktop-updates.ts` 6, `rosetta-callout-source.tsx` 17, `electron-builder.yml` 4, `auto-updater.ts` 4), plus the new 3-line `app-updates-enabled.ts`, so 34 in all; `git diff --numstat HEAD`, 2026-10-08 11:25.
- Result: PASS against 40.

Lessons:

- The Bash tool runs zsh, and zsh does not split an unquoted `$VAR`. `npm run lint -- $F` then passes one long path and reports "No files found". Use an array: `npm run lint -- "${F[@]}"`.
- A vitest `vi.mock` factory with a getter, `get APP_UPDATES_ENABLED() { return gate.enabled; }`, lets one test file flip a module constant per test. Source: [packages/desktop/src/features/auto-updater.test.ts](packages/desktop/src/features/auto-updater.test.ts).
