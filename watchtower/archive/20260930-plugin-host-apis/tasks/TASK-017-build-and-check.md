# TASK-017 Build Paseo Dev and check the plugins on a real host

Group: K (build and runtime)
Class: risky

## Brief

Goal: Run the new host in Paseo Dev with the 7 plugins and check each one by hand.

Change: Paseo Dev runs `0.10.0-beta.1` -> Paseo Dev runs `0.10.2-beta.900` with the moved plugins.

Boundaries:

- Never stop or restart the daemon on port 6767. This session runs under it.
- Build with `hardenedRuntime=false`, as [watchtower/MEMORY.md](watchtower/MEMORY.md) says.
- Install `skill-pins` into Paseo Dev with `PASEO_HOME=~/.paseo-dev paseo plugin ...`, because it is missing there.
- Use `cua_repl` for desktop UI checks.

How:

- Build the desktop app, restart Paseo Dev, reload the plugins.
- Check each plugin's real host line from its TASK.

Files:

- No source files. Build output in `packages/desktop/release/mac-arm64`.

Expected result:

- Paseo Dev reports `0.10.2-beta.900`, and all 7 plugins load with no error.
- Each plugin's real host check passes.

Anti-goal: The stable daemon keeps running; tripwire; limit the PID from `lsof -nP -iTCP:6767 -sTCP:LISTEN -t` stays the same; read before the build and after the checks.

## Verify

- `PASEO_HOME=~/.paseo-dev paseo plugin ls` -> the 7 plugins are enabled with no error.
- Manual checks (cua_repl) -> each plugin's real host line passes.
- Anti-goal: `lsof -nP -iTCP:6767 -sTCP:LISTEN -t` -> same PID as before.
