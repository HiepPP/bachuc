# TASK-008 Outcome

## Outcome

Status: DONE (auto: manual checks pending)

Changed:

- [CLAUDE.md](CLAUDE.md): one brand line that links [ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md). The release row names `/Applications/Bachuc.app` and the log folder `~/Library/Logs/Bachuc/`. The install line and the build-output line name `Bachuc.app`, and an opened build's userData is `~/Library/Application Support/Bachuc Dev`.
- [docs/development.md](docs/development.md): the release endpoint, the install line, and the `main.ts` line name `Bachuc.app`. The install line also says that the script links `~/.local/bin/paseo` to the new CLI.
- [watchtower/MEMORY.md](watchtower/MEMORY.md): `## Core Intent` names the brand and the install path.

Contract:

- Paths read from code: `APP_NAME` in [packages/desktop/src/main.ts](packages/desktop/src/main.ts) is `Bachuc` for the release app and `Bachuc Dev` otherwise. `app.setName(APP_NAME)` runs before the first log line. electron-log 5.4.3 takes the folder from `app.name` (`node_modules/electron-log/src/main/ElectronExternalApi.js`), so release logs go to `~/Library/Logs/Bachuc/`. The release userData stays `Paseo` (`RELEASE_USER_DATA`).
- The build output stays `packages/desktop/release/mac-arm64/Paseo.app` (`executableName`, Q-005). The install log stays `~/Library/Logs/Paseo/release-install.log` (`OUTPUT_LOG` in [scripts/paseo-release.sh](scripts/paseo-release.sh)). Both are unchanged in the docs.
- Deviation: the Verify line `rg -n "Paseo Fork"` matches one line, the signing identity `Paseo Fork Local` in [docs/development.md](docs/development.md). `SIGN_IDENTITY` in [scripts/paseo-release.sh](scripts/paseo-release.sh) still has that name, and TASK-003 did not change it. The doc keeps it, because the owner needs the exact name to make the certificate again. `rg -n "Paseo Fork\.app"` has no match, which is the Expected result.
- No symbol-level impact applies: the TASK edits Markdown only.

Verified:

- `rg -n "Paseo Fork" CLAUDE.md docs/development.md` -> one match, the signing identity (see Contract).
- `rg -n "Paseo Fork\.app" CLAUDE.md docs/development.md` -> no match.
- `npm run format:check:files -- CLAUDE.md docs/development.md watchtower/MEMORY.md` -> exit 0.
- Plan Verify: the license diff exits 0, and the plugin diff has no output.

Anti-goal:

- Before changes: `git diff --numstat main -- CLAUDE.md docs/development.md` -> no output (0 lines), 2026-10-08 11:41.
- Final: `8 6 CLAUDE.md` and `3 3 docs/development.md` -> 20 lines, 11:42.
- Result: PASS (20 of 30).

Lessons:

- The old name stays in two places on purpose: `OLD_APP` and `SIGN_IDENTITY` in `scripts/paseo-release.sh`. A grep for `Paseo Fork` needs the `.app` suffix to find stale install paths.
- electron-log names the desktop log folder after `app.name`, so an `APP_NAME` change moves the log folder too.
