# TASK-006 Turn off desktop auto-update

Group: C (shares [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml) and [packages/desktop/src/main.ts](packages/desktop/src/main.ts) with TASK-003)
Class: code

## Brief

Goal: The desktop app never checks, downloads, or installs an update from upstream `getpaseo/paseo`. New versions come only from [scripts/paseo-release.sh](scripts/paseo-release.sh).

Change: electron-updater reads the GitHub feed of `getpaseo/paseo` -> no update feed and no update UI.

Boundaries:

- Follows [ADR-0005](watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md).
- Reason: the fork version is `1.0.0-hiep`. An upstream release such as `1.0.0` is newer, so the updater could install upstream Paseo over Bachuc.
- Remove the `publish` block from `electron-builder.yml`, so builds ship no `app-update.yml`.
- Gate the updater in one place, so it never calls electron-updater. Keep the updater module, because deleting it raises merge cost.
- In Settings > About, hide the "App updates" and "Release channel" rows when updates are off. Keep "App version".
- Hide links built from `RELEASE_DOWNLOAD_BASE_URL` in [packages/app/src/desktop/updates/desktop-updates.ts](packages/app/src/desktop/updates/desktop-updates.ts) and the Rosetta callout download link.

How:

- Run GitNexus `impact` on the updater runtime `configure` method and its callers in `main.ts`.
- Find how the app learns the update state over the desktop bridge. Add an "updates off" state there, or reuse an existing one.
- Add the gate, then hide the rows and links.
- Add a test case in [packages/desktop/src/features/auto-updater.test.ts](packages/desktop/src/features/auto-updater.test.ts): with the gate off, nothing calls `checkForUpdates`.

Files:

- [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml) (remove `publish`)
- [packages/desktop/src/features/auto-updater.ts](packages/desktop/src/features/auto-updater.ts) (gate)
- [packages/desktop/src/features/auto-updater.test.ts](packages/desktop/src/features/auto-updater.test.ts) (new case)
- [packages/desktop/src/main.ts](packages/desktop/src/main.ts) (only if the gate needs a call site change)
- [packages/app/src/desktop/updates/desktop-updates.ts](packages/app/src/desktop/updates/desktop-updates.ts) (update state, download links)
- [packages/app/src/desktop/components/desktop-updates-section.tsx](packages/app/src/desktop/components/desktop-updates-section.tsx) (hide rows)
- [packages/app/src/desktop/updates/rosetta-callout-source.tsx](packages/app/src/desktop/updates/rosetta-callout-source.tsx) (hide upstream link)

Expected result:

- A packaged build makes no request to `github.com/getpaseo`.
- Settings > About shows the app version, with no update or release channel rows.
- The existing updater tests still pass.

Anti-goal: changed lines in existing upstream files (tests excluded) stay at or below 40; drift gauge; read with the numstat command in Verify before changes (0), after the gate, and at completion.

## Verify

- `npx vitest run packages/desktop/src/features/auto-updater.test.ts --bail=1` -> pass, including the new case.
- `rg -n "^publish:" packages/desktop/electron-builder.yml` -> no match.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed files>` -> exit 0.
- Human check on live desktop: Settings > About shows the app version and no "App updates" or "Release channel" rows.
- Owner check after the next release install: the desktop log has no line that checks for updates.
- Anti-goal: `git diff --numstat main -- packages/desktop packages/app/src/desktop ':(exclude)**/*.test.ts'` minus the TASK-003 lines -> 40 or less.
