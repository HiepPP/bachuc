# TASK-007 Outcome

## Outcome

Status: DONE

Changed:

- Built `packages/desktop/release/mac-arm64/Paseo.app` with `npm run build:desktop -- --dir`. The build removed the older `Paseo Dev.app` folder from the same directory.

Contract:

- Bundle ID `sh.paseo.desktop.dev`, bundle name "Paseo Dev", helpers named `Paseo Dev Helper`. The signature is ad-hoc, and notarization was skipped.

Verified:

- `npm run build:desktop -- --dir` -> exit 0; log in `/tmp/paseo-dev-build.txt`.
- `PlistBuddy Print CFBundleIdentifier` -> `sh.paseo.desktop.dev`.
- The first launch failed. dyld reported "mapping process and mapped file (non-platform) have different Team IDs", because ad-hoc signing was combined with `hardenedRuntime: true`.
- It was repackaged with `electron-builder --config electron-builder.yml --mac --arm64 --dir --publish never -c.mac.hardenedRuntime=false -c.mac.notarize=false`. `codesign --verify --deep --strict` passed, and the app launched with its daemon on 6770.
- Manual check in the app -> done, 2026-09-30. See the Handoff in [watchtower/NEXT.md](watchtower/NEXT.md).

Anti-goal:

- Before build: PID 1188 on port 6767; `lsof -nP -iTCP:6767 -sTCP:LISTEN -t`, 2026-09-30.
- Final: PID 1188 on port 6767; same command after the build, 2026-09-30.
- After repackage and two launches: PID 1188 on port 6767; 2026-09-30T17:40.
- Result: PASS against "PID unchanged".
