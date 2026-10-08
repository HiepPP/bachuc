# TASK-003 Outcome

## Outcome

Status: DONE (auto: manual checks pending)

Changed:

- [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml): `appId: io.github.hieppp.bachuc` (Q-003), `productName: Bachuc Dev`, protocol name `Bachuc agent link`, `Bachuc-` artifact names, `vendor: Bachuc`, and the owner as `maintainer`. The `paseo` scheme, `--class=Paseo`, and the `publish` block stay (TASK-006 owns the update feed).
- [packages/desktop/src/main.ts](packages/desktop/src/main.ts): `IS_RELEASE_APP` matches `/Bachuc.app/`. `APP_NAME` is `Bachuc` for the release app and `Bachuc Dev` otherwise. `PASEO_TEST_APP_NAME` still wins.
- [scripts/paseo-release.sh](scripts/paseo-release.sh): installs `/Applications/Bachuc.app` and checks the new bundle ID. It builds with `-c.productName=Bachuc` and refuses a reused build with another `CFBundleName`. `app_pid` matches any main executable, so it finds both the old and the new app. After the plugins run, it links `~/.local/bin/paseo` to the new CLI. Then it moves `Paseo Fork.app` to the Trash whenever it exists. It prints the permission reminder when the bundle ID is new. Rollback trashes the failed `Bachuc.app`, puts the previous app back where it ran, and restores or removes the CLI link.
- [packages/desktop/src/daemon/runtime-paths.ts](packages/desktop/src/daemon/runtime-paths.ts): `resolveNodeExecPath` falls back to the `* Helper.app` found in Frameworks, as [packages/desktop/bin/paseo](packages/desktop/bin/paseo) does. A test covers it in [packages/desktop/src/daemon/runtime-paths.test.ts](packages/desktop/src/daemon/runtime-paths.test.ts).
- [packages/desktop/src/daemon/desktop-packaging.test.ts](packages/desktop/src/daemon/desktop-packaging.test.ts) expects the new protocol name. The comment in [packages/desktop/bin/paseo](packages/desktop/bin/paseo) names the new helper names.

Contract:

- Deviation (Q-005, DEFAULTED): `executableName` stays `Paseo`. electron-builder names the macOS bundle after `executableName` (`node_modules/app-builder-lib/out/appInfo.js` line 57). The hooks [packages/desktop/scripts/after-pack.js](packages/desktop/scripts/after-pack.js) and [packages/desktop/scripts/after-sign.js](packages/desktop/scripts/after-sign.js) expect `Paseo.app`, as do [packages/desktop/e2e/packaged-app-smoke.js](packages/desktop/e2e/packaged-app-smoke.js) and [nix/desktop-package.nix](nix/desktop-package.nix). A build named `Bachuc.app` would also match `IS_RELEASE_APP` in place and take the release home. The installed bundle is still named `Bachuc.app`, and the menu, Dock, and window say Bachuc.
- The release build is named `Bachuc` through `-c.productName`, because the daemon finds its helper app by the executable name or `app.name`. Other builds stay `Bachuc Dev`, as the spec asked.
- Files beyond the spec list: `runtime-paths.ts`, its test, `desktop-packaging.test.ts`, and `bin/paseo` (comment only). The reviewer found that a script build opened in place named no helper it could find, so the daemon started through the main executable.
- The old app moves to the Trash after `STOPPED=0`. A failure there leaves the new release running and does not roll back.
- The grep Verify line matches one line: the `OLD_APP` constant that the Trash move and the rollback use.
- GitNexus `impact`: `IS_RELEASE_APP` and `APP_NAME` were UNKNOWN; a text search found only `main.ts` uses. `resolveNodeExecPath` was LOW. The shell script is not indexed. `detect_changes` reported risk low with no affected processes.
- Review: one `reviewer` pass found one should-fix (the helper lookup) and four script nits. All five are fixed.

Verified:

- `bash -n scripts/paseo-release.sh` -> exit 0. `sh -n packages/desktop/bin/paseo` -> exit 0.
- `rg -n "Paseo Fork.app|MacOS/Paseo" scripts/paseo-release.sh packages/desktop/src/main.ts` -> one line, the `OLD_APP` constant.
- Sourced the script without running `main`: `current_app` and `installed_cli` resolve to `Paseo Fork.app`. `app_pid` finds the running release (PID 274), the same PID as the old pattern. `preflight_home` reports no problems.
- `cd packages/desktop && npx vitest run src/daemon/runtime-paths.test.ts src/daemon/desktop-packaging.test.ts --bail=1` -> PASS (13), FAIL (0).
- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed .ts files>` -> 0 warnings, 0 errors.
- Plan Verify: the license diff exits 0, and the plugin diff has no output.
- UNVERIFIED (autonomous run): the human check on live desktop. A `main.ts` change needs an `npm run dev:desktop` restart, and the run does not restart live.
- UNVERIFIED (autonomous run): the owner check after the next release install.

Anti-goal:

- Before changes: `git diff main -- packages/desktop/src/main.ts | rg '^[-+].*(RELEASE_USER_DATA|PASEO_DEV_HOME|PASEO_DEV_LISTEN) ='` -> no match, 2026-10-08 10:21.
- After the `main.ts` edit: no match, 10:24.
- Final: no match, 10:38.
- Result: PASS.

Lessons:

- electron-builder names the macOS bundle after `executableName`, not `productName`. `productName` names `CFBundleName` and the helper apps.
- The desktop daemon starts through `<name> Helper.app`. If `app.name` names no helper, it used to fall back to the main executable.
- The new app name likely changes the macOS keychain key for Electron cookies, so in-app browser logins may reset. Not checked.
