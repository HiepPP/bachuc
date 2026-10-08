# TASK-009 Rename the macOS executable to Bachuc

Group: C+G (shares [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml) and [packages/desktop/src/main.ts](packages/desktop/src/main.ts) with TASK-003 and TASK-006, and [CLAUDE.md](CLAUDE.md) and [docs/development.md](docs/development.md) with TASK-008)
Class: risky

## Brief

Goal: The macOS executable and the build bundle are named Bachuc, so Activity Monitor shows Bachuc. An opened build never takes the release home.

Change: `executableName: Paseo` and build output `Paseo.app` -> `executableName: Bachuc` and build output `Bachuc.app`.

Boundaries:

- Q-005, answered 2026-10-08: rename the executable to `Bachuc`.
- electron-builder names the macOS bundle after `executableName`. So the build output in `packages/desktop/release/mac-arm64/` becomes `Bachuc.app`.
- `IS_RELEASE_APP` in [packages/desktop/src/main.ts](packages/desktop/src/main.ts) matches `/Bachuc.app/` today. A build opened in place would then match too, take `~/.paseo` and port 6767, and could restart the release daemon. Make release detection require the `/Applications/Bachuc.app/` prefix.
- Keep `RELEASE_USER_DATA`, `PASEO_DEV_HOME`, `PASEO_DEV_LISTEN`, and the `Paseo-<worktree>` dev folders ([ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md)).
- Update every file that expects `Paseo.app` or `MacOS/Paseo` for the build: [packages/desktop/scripts/after-pack.js](packages/desktop/scripts/after-pack.js), [packages/desktop/scripts/after-sign.js](packages/desktop/scripts/after-sign.js), [packages/desktop/e2e/packaged-app-smoke.js](packages/desktop/e2e/packaged-app-smoke.js), [nix/desktop-package.nix](nix/desktop-package.nix), and `BUILD_APP` in [scripts/paseo-release.sh](scripts/paseo-release.sh).
- [packages/desktop/bin/paseo](packages/desktop/bin/paseo) finds the helper by a glob. Change only its fallback names if they still say `Paseo`.
- Test fixtures that pass `/Applications/Paseo.app/...` as argv test argument parsing. Leave them, unless a test asserts the build name.
- The release script keeps `OLD_APP` (`Paseo Fork.app`) and `SIGN_IDENTITY` (`Paseo Fork Local`).
- Do not run the release script or `npm run build:desktop`. The owner runs the script from Terminal.app.

How:

- Run GitNexus `impact` on `IS_RELEASE_APP` users and on each hook function you change.
- Set `executableName: Bachuc` and update the comment above it.
- Change release detection to the `/Applications/Bachuc.app/` prefix. If you move the check into a small function, add a unit test for it.
- Update the hooks, the smoke script, the nix package, and the release script.
- Update the build output name in [CLAUDE.md](CLAUDE.md) and [docs/development.md](docs/development.md).

Files:

- [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml) (`executableName`)
- [packages/desktop/src/main.ts](packages/desktop/src/main.ts) (release detection)
- [packages/desktop/scripts/after-pack.js](packages/desktop/scripts/after-pack.js), [packages/desktop/scripts/after-sign.js](packages/desktop/scripts/after-sign.js) (bundle name)
- [packages/desktop/e2e/packaged-app-smoke.js](packages/desktop/e2e/packaged-app-smoke.js) (bundle name)
- [nix/desktop-package.nix](nix/desktop-package.nix) (bundle name and link)
- [scripts/paseo-release.sh](scripts/paseo-release.sh) (`BUILD_APP`)
- [packages/desktop/bin/paseo](packages/desktop/bin/paseo) (fallback names, only if needed)
- [CLAUDE.md](CLAUDE.md), [docs/development.md](docs/development.md) (build output name)

Expected result:

- A build produces `packages/desktop/release/mac-arm64/Bachuc.app` with `Contents/MacOS/Bachuc`.
- Opened in place, that build uses `~/.paseo-dev` and port 6770. Installed as `/Applications/Bachuc.app`, it uses `~/.paseo` and port 6767.
- The release script finds the new build and installs it.

Anti-goal: an opened build never counts as the release app; tripwire; read before changes, after the `main.ts` edit, and at completion. Read it with a unit test, or with a code read that a path under `packages/desktop/release/` fails the release check.

## Verify

- `bash -n scripts/paseo-release.sh` -> exit 0.
- `node --check packages/desktop/scripts/after-pack.js && node --check packages/desktop/scripts/after-sign.js && node --check packages/desktop/e2e/packaged-app-smoke.js` -> exit 0.
- `rg -n "Paseo\.app|MacOS/Paseo" packages/desktop/scripts packages/desktop/e2e nix/desktop-package.nix scripts/paseo-release.sh` -> no match.
- `npx vitest run packages/desktop/src/daemon/desktop-packaging.test.ts packages/desktop/src/daemon/runtime-paths.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed files>` -> exit 0.
- Owner check after the next release install, from Terminal.app: Activity Monitor shows `Bachuc`. `PASEO_HOME=~/.paseo paseo daemon status` shows `~/.paseo` on `127.0.0.1:6767`.
- Anti-goal: the new or existing test, or a code read recorded in the outcome, shows that `/Users/x/paseo/packages/desktop/release/mac-arm64/Bachuc.app/Contents/MacOS/Bachuc` is not a release app.
