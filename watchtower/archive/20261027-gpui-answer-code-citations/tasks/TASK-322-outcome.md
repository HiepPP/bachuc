# TASK-322 Outcome

## Outcome

Status: DONE (implement: manual checks pending)

Changed:

- `start_and_verify` in `scripts/paseo-release.sh` opens the app with `open -g -n`.
- `docs/gpui.md` Daily app and `docs/development.md` Installing a release describe the menu bar app.

Verified:

- `bash -n scripts/paseo-release.sh` -> exit 0.
- `scripts/paseo-release.sh --dry-run --skip-build | grep -c '^\['` -> 12 before and 12 after; the release daemon stayed PID 65180.

Anti-goal:

- Final: 12 numbered steps; 2026-10-27.
- Result: PASS.

Lessons:

- none
