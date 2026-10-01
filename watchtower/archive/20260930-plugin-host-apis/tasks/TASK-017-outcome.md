# TASK-017 Outcome

## Outcome

Status: DONE

Changed:

- Built Paseo Dev three times with `npm run build:desktop -- --dir -c.mac.hardenedRuntime=false -c.mac.notarize=false`; the app reports `0.10.2-beta.900`. The first build failed on the `-hiep.0` version, see TASK-009.
- Restarted Paseo Dev (daemon 6770) after each build; all 19 existing agents were idle. Installed `skill-pins` into `~/.paseo-dev`.
- Host checks found three bugs, now fixed: plugin context in the user bubble for prompts without a client message ID and after a daemon restart (TASK-001 outcome), a sidebar filter that React Compiler never re-ran (TASK-007 outcome), and a Caveman timeout inside the desktop app (TASK-012 outcome).
- Archived the 3 test agents and 3 `/tmp/pd-check` workspaces. Workspace 2 of the Spaces test remains in Paseo Dev.

- User report after the plan, 2026-10-01: with MacAir active, a second Spaces strip showed. It came from the old DOM workspace-spaces on macmini (0.10.1, `~/Projects/hiep-paseo-plugin`), because Paseo Dev ran every connected host's plugin client code. [packages/app/src/plugins/catalog-sync.tsx](packages/app/src/plugins/catalog-sync.tsx) now loads plugin client code only for the active host; "All hosts" still loads every host.

Verified:

- After that fix and a rebuild: MacAir active shows only the new strip and the Workspaces heading; plugins reload about 1 to 4 seconds after a host switch. `cd packages/app && npx vitest run --project unit src/plugins src/hosts` -> 162 passed; app typecheck exit 0.

- `PASEO_HOME=~/.paseo-dev paseo plugin ls` -> 14 plugins running with no error, including `skill-pins`.
- Per-plugin real host lines -> recorded in TASK-010, 011, 012, 013, 014, and 016.
- Final build: Spaces filter live, bubble clean after `paseo daemon restart`, `paseo run` bubble clean with Caveman mode applied.
- Plan Verify, 2026-10-01T01:05: `npm run typecheck` exit 0; `npm run lint` on 37 changed host files 0 errors after one fix; each changed plugin typecheck, lint, and test pass (gate 7, skill-pins 14, prompt-translate 41, next-prompt-actions 71, workspace-spaces 19, board 99); the DOM grep over the four plugin client folders has no match.

Anti-goal:

- Before the first build: PID 1188 on 6767; 2026-10-01T00:12.
- After each restart: PID 1188; last read 2026-10-01T01:05.
- Result: PASS against "PID unchanged".
