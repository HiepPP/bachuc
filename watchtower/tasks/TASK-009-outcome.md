# TASK-009 Outcome

## Outcome

Status: DONE (auto: manual checks pending)

Changed:

- [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml): `mac.executableName: Bachuc`. The top-level `executableName: Paseo` stays for Linux and Windows.
- [packages/desktop/src/main.ts](packages/desktop/src/main.ts): `IS_RELEASE_APP` needs `process.execPath.startsWith("/Applications/Bachuc.app/")`.
- [packages/desktop/scripts/after-pack.js](packages/desktop/scripts/after-pack.js) and [packages/desktop/scripts/after-sign.js](packages/desktop/scripts/after-sign.js): `EXECUTABLE_NAME` is `Bachuc`. Both use it only for the macOS bundle path.
- [packages/desktop/e2e/packaged-app-smoke.js](packages/desktop/e2e/packaged-app-smoke.js): a new `MAC_EXECUTABLE_NAME` (`Bachuc`) builds the macOS executable paths. `EXECUTABLE_NAME` stays `Paseo` for Linux and Windows. The usage text names `Bachuc.app`.
- [.github/workflows/nix.yml](.github/workflows/nix.yml): the macOS bundle check tests `Bachuc.app` and `MacOS/Bachuc`. This file is outside `Files:`, but the rename broke these two lines.
- [nix/desktop-package.nix](nix/desktop-package.nix): the macOS install finds, copies, and links `Bachuc.app` and `MacOS/Bachuc`.
- [scripts/paseo-release.sh](scripts/paseo-release.sh): `BUILD_APP` is `release/mac-arm64/Bachuc.app`. `OLD_APP` and `SIGN_IDENTITY` are unchanged.
- [packages/desktop/bin/paseo](packages/desktop/bin/paseo): the macOS helper fallback is `Bachuc Helper`. The glob still finds the real helper. The Linux fallbacks `Paseo.bin` and `Paseo` stay, because Linux keeps the executable `Paseo`.
- [CLAUDE.md](CLAUDE.md) and [docs/development.md](docs/development.md): the build output and the nix output are `Bachuc.app`. Release detection names the install path `/Applications/Bachuc.app`.

Contract:

- Deviation: `executableName: Bachuc` is set under `mac:`, not at the top level. electron-builder also names the Linux binary and the Windows `.exe` after `executableName`. A top-level change would break [packages/desktop/scripts/linux-sandbox/index.js](packages/desktop/scripts/linux-sandbox/index.js), the Linux fallbacks in [packages/desktop/bin/paseo](packages/desktop/bin/paseo), `Paseo.exe` in [packages/desktop/bin/paseo.cmd](packages/desktop/bin/paseo.cmd), and [packages/desktop/e2e/linux-artifact-smoke.js](packages/desktop/e2e/linux-artifact-smoke.js). These files are outside this TASK. Source: `node_modules/app-builder-lib/out/appInfo.js` line 56 reads the platform `executableName` first, `macPackager.js` line 69 passes the `mac` options, and `linuxPackager.js` line 15 reads its own.
- Helper apps are named after `productName`, not `executableName` (`electronMac.js` line 47). The release helper stays `Bachuc Helper`, and [packages/desktop/src/daemon/runtime-paths.ts](packages/desktop/src/daemon/runtime-paths.ts) finds it unchanged.
- The release script's supervisor check greps `Paseo` in the process command. The supervisor sets its title to `Paseo Supervisor` (checked on PID 338 of release), so the check still works.
- Impact: `IS_RELEASE_APP` gave `UNKNOWN` (no edges for a module const). A text search finds four reads, all in [packages/desktop/src/main.ts](packages/desktop/src/main.ts). `pruneNativeModules` gave `LOW` (one caller, `afterPack`). The index was 14 commits behind.

Verified:

- `bash -n scripts/paseo-release.sh` -> exit 0.
- `node --check` on `after-pack.js`, `after-sign.js`, and `packaged-app-smoke.js` -> exit 0, also after the review fix.
- Review: one `reviewer` agent (Opus, read-only). It confirmed the mac-only override, the release detection, the helper lookup, the hooks, nix, and the release script. It found two defects, both fixed and checked against the source. Medium: the smoke script built `Bachuc.app/Contents/MacOS/Paseo`, so every macOS build with `PASEO_DESKTOP_SMOKE=1` would fail in `assertExecutable`. Low: [.github/workflows/nix.yml](.github/workflows/nix.yml) tested `Paseo.app`.
- `rg -n "Paseo\.app|MacOS/Paseo" packages/desktop/scripts packages/desktop/e2e nix/desktop-package.nix scripts/paseo-release.sh` -> no match.
- `npx vitest run packages/desktop/src/daemon/desktop-packaging.test.ts packages/desktop/src/daemon/runtime-paths.test.ts --bail=1` -> 2 files, 13 tests pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed .ts and .js files>` -> 0 warnings, 0 errors, also after the review fix.
- `npm run format:check:files -- <changed files>` -> exit 0.
- Plan Verify: the license diff exits 0, and the plugin diff has no output.
- UNVERIFIED (autonomous run): the owner check after the next release install. Activity Monitor shows `Bachuc`, and `PASEO_HOME=~/.paseo paseo daemon status` shows `~/.paseo` on `127.0.0.1:6767`.
- UNVERIFIED (autonomous run): no build ran, by the Repo Rules. The bundle name `Bachuc.app` comes from the electron-builder source above.

Anti-goal:

- Before changes, 12:01: `includes("/Bachuc.app/")` would match the renamed build at `packages/desktop/release/mac-arm64/Bachuc.app`.
- After the `main.ts` edit and at completion, 12:03: the predicate `startsWith("/Applications/Bachuc.app/")` returns false for `/Users/x/paseo/packages/desktop/release/mac-arm64/Bachuc.app/Contents/MacOS/Bachuc`, true for `/Applications/Bachuc.app/Contents/MacOS/Bachuc`, and false for `/Applications/Bachuc.app.bak/Contents/MacOS/Bachuc` (node one-liner).
- Result: PASS.

Follow-ups:

- `findDesktopApp` in [packages/cli/src/commands/open.ts](packages/cli/src/commands/open.ts) lines 10-11 still looks for `/Applications/Paseo.app`, so `paseo open` does not find `/Applications/Bachuc.app`. This came from the TASK-003 install rename and is outside this TASK.
- [.github/workflows/nix.yml](.github/workflows/nix.yml) line 107 still expects the bundle ID `sh.paseo.desktop`, stale since TASK-003. The job runs only on pushes to `main`. This TASK left it unchanged.

Lessons:

- electron-builder names the macOS bundle, the Linux binary, and the Windows `.exe` after `executableName`. Set a mac-only name under `mac:` to leave the Linux and Windows launchers alone.
- Release detection must check the install path, not the bundle name, because the build output and the install now share the name `Bachuc.app`.
- `rg "Paseo\.app|MacOS/Paseo"` misses names built from a constant, such as `EXECUTABLE_NAME` in the smoke script. Search for the constant too.
