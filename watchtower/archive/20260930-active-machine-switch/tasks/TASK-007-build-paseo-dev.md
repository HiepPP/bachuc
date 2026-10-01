# TASK-007 Build the Paseo Dev app

Group: F (build output only)
Class: mechanical

## Brief

Goal: Build the desktop app from this checkout as Paseo Dev, with its own daemon on port 6770.

Change: installed app has no machine switch -> fresh `Paseo Dev.app` build with the switch.

Boundaries:

- Never restart or stop the stable daemon on port 6767.
- The packaged build uses home `~/.paseo-dev` and port 6770, set in [packages/desktop/src/main.ts](packages/desktop/src/main.ts).
- Do not install over `/Applications/Paseo.app`.
- Pipe long build output to a file and read the end.

How:

- Record the PID that listens on port 6767.
- Run `npm run build:desktop -- --dir -c.mac.hardenedRuntime=false -c.mac.notarize=false > /tmp/paseo-dev-build.txt 2>&1` from the repo root. `--dir` builds only the `.app`. An ad-hoc signature with hardened runtime fails to launch with a Team ID error, so hardened runtime must be off for local builds.
- Confirm the bundle exists and has the dev bundle ID.
- Record the PID on port 6767 again.

Files:

- [packages/desktop/release/mac-arm64/Paseo.app](packages/desktop/release/mac-arm64/Paseo.app) (build output, not tracked). The folder takes its name from `executableName: Paseo`. The bundle name is still "Paseo Dev".

Expected result:

- The build exits 0.
- `packages/desktop/release/mac-arm64/Paseo.app` exists with bundle ID `sh.paseo.desktop.dev`.

Anti-goal: The stable daemon keeps running; tripwire; limit the PID from `lsof -nP -iTCP:6767 -sTCP:LISTEN -t` stays the same; read before the build and after the build.

## Verify

- `tail -20 /tmp/paseo-dev-build.txt` -> no error, build exit 0.
- `/usr/libexec/PlistBuddy -c "Print CFBundleIdentifier" packages/desktop/release/mac-arm64/Paseo.app/Contents/Info.plist` -> `sh.paseo.desktop.dev`.
- `lsof -nP -iTCP:6767 -sTCP:LISTEN -t` -> same PID as before the build.
- `codesign --verify --deep --strict packages/desktop/release/mac-arm64/Paseo.app` -> exit 0, and the app launches.
- Manual check (human): open Paseo Dev, use the machine switch, and confirm the Plan Verify manual check.
