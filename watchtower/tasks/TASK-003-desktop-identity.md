# TASK-003 Desktop identity and release install

Group: C (shares [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml) and [packages/desktop/src/main.ts](packages/desktop/src/main.ts) with TASK-006)
Class: risky

## Brief

Goal: The desktop build is named Bachuc and uses the new bundle ID. It installs as `/Applications/Bachuc.app` and keeps the release home, the release data, and the `paseo` CLI.

Change: a `Paseo Dev` build with `sh.paseo.desktop.dev`, installed as `Paseo Fork.app` -> a `Bachuc Dev` build with the Q-003 bundle ID, installed as `Bachuc.app`.

Boundaries:

- Follows [ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md): keep `RELEASE_USER_DATA` as `appData/Paseo`, the `paseo` URL scheme, `PASEO_*` env vars, `~/.paseo-dev`, port 6770, and the `Paseo-<worktree>` dev folders.
- `IS_RELEASE_APP` must match the new install path. If it does not, the app silently runs on `~/.paseo-dev` and port 6770, and the release data looks lost.
- The release app shows `Bachuc` in its menu and window title. A build opened in place shows `Bachuc Dev`.
- Keep the signing identity `Paseo Fork Local`. It is a local keychain item that users do not see. A new name needs a new certificate.
- The release script must link `~/.local/bin/paseo` to the new app CLI. Agents call that CLI.
- The script moves `/Applications/Paseo Fork.app` to the Trash only after the new daemon passes its check. Rollback puts the old app and the old CLI link back.
- `app_pid` in [scripts/paseo-release.sh](scripts/paseo-release.sh) line 129 matches `Contents/MacOS/Paseo`. It must match the new executable name.
- macOS ties privacy grants to the bundle ID. The script's final output tells the owner to grant microphone, screen recording, accessibility, and automation again.
- Linux and Windows: change only `vendor`, `maintainer`, and artifact names. Keep `--class=Paseo` and `Paseo.desktop`. The owner does not build them.
- Do not run the release script or `npm run build:desktop`. The owner runs the script from Terminal.app.

How:

- Run GitNexus `impact` on `IS_RELEASE_APP` users and on the release script functions you change.
- In `electron-builder.yml`: set `appId: io.github.hieppp.bachuc` (Q-003, answered 2026-10-08), `productName: Bachuc Dev`, `executableName: Bachuc`, the protocol `name: Bachuc agent link`, `Bachuc-` artifact names, `vendor: Bachuc`, and the owner as `maintainer`.
- In `main.ts`: match `/Bachuc.app/` in `IS_RELEASE_APP`. Use `Bachuc` as the app name for the release app and `Bachuc Dev` otherwise. Keep `PASEO_TEST_APP_NAME` as an override. Update the comment at line 119.
- In `paseo-release.sh`: set `APP` to `/Applications/Bachuc.app` and `BUILD_BUNDLE_ID` to the new ID. Match the new executable in `app_pid`. Rename the Trash names. Link the CLI after a good install. Move the old `Paseo Fork.app` to the Trash at the end. Print the permission reminder.
- Read the script's existing check functions first, and reuse them.

Files:

- [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml) (identity, artifact names, vendor, maintainer)
- [packages/desktop/src/main.ts](packages/desktop/src/main.ts) (app name, release detection)
- [scripts/paseo-release.sh](scripts/paseo-release.sh) (install path, bundle ID, process match, CLI link, old app, rollback)

Expected result:

- `bash -n scripts/paseo-release.sh` passes.
- A first install moves `Paseo Fork.app` to the Trash and leaves `Bachuc.app` running on `~/.paseo` and port 6767.
- `~/.local/bin/paseo` points into `Bachuc.app`, and `paseo daemon status` works from any agent.
- A failed install restores `Paseo Fork.app` and its CLI link.

Anti-goal: the release data path stays `~/Library/Application Support/Paseo` with home `~/.paseo`; tripwire; read before changes, after the `main.ts` edit, and at completion with `git diff main -- packages/desktop/src/main.ts | rg '^[-+].*(RELEASE_USER_DATA|PASEO_DEV_HOME|PASEO_DEV_LISTEN) ='` -> no match.

## Verify

- `bash -n scripts/paseo-release.sh` -> exit 0.
- `rg -n "Paseo Fork.app|MacOS/Paseo" scripts/paseo-release.sh packages/desktop/src/main.ts` -> only the line that moves the old app to the Trash, and the rollback lines.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/desktop/src/main.ts` -> exit 0.
- Human check on live desktop: the macOS menu and window title say `Bachuc Dev`.
- Owner check after the next release install, from Terminal.app: `Bachuc.app` starts. `PASEO_HOME=~/.paseo paseo daemon status` shows `~/.paseo` on `127.0.0.1:6767`. `readlink ~/.local/bin/paseo` points into `/Applications/Bachuc.app`. Agents and the host list are still there.
- Anti-goal: `git diff main -- packages/desktop/src/main.ts | rg '^[-+].*(RELEASE_USER_DATA|PASEO_DEV_HOME|PASEO_DEV_LISTEN) ='` -> no match.
