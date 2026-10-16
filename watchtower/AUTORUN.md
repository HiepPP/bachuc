# Autorun

Started: 2026-10-16. The user reviews `feat/bachuc-gpui` and merges it to `main`.

## Config

```text
protected_branches: main next t3code-port
branch_prefix: feat/
forbidden_paths: .env* */.env* .dev/* */.dev.vars
max_line: 100
header_pattern: ^(feat|fix|docs|refactor|test|chore|perf)(\([a-z0-9-]+\))?: .+
max_builders: 2
build_dir_env: CARGO_TARGET_DIR
```

## Repo Rules

- Never restart or stop the release daemon on port 6767. Never run `scripts/paseo-release.sh` or `npm run build:desktop`.
- Do not start, stop, or restart the live daemon or the live Electron app (owner answer, 2026-10-12). If live is down, write each live check as `UNVERIFIED (unattended)`.
- Never send prompts to or create agents on release (6767) in a check. Live checks use port 6768 only.
- Run one test file at a time with `npx vitest run <file> --bail=1`. Never run a full suite.
- Run GitNexus `impact` before you edit an existing TypeScript symbol. The lead runs `detect_changes` only before a commit that edits an existing symbol (owner answer, 2026-10-16).
- Memory: the machine ran out of RAM and swap and panicked on 2026-10-16 at 11:16, with 6 builders and a UI checker running. Run at most 2 builders at once. Every cargo command runs with `CARGO_BUILD_JOBS=4`, one at a time (owner answer, 2026-10-16).
- The gate has no `formatter` line. Format changed JS, TS, JSON, and markdown files with `npm run format:files -- <file> <file>`, one argument per file, before you stage; the lefthook pre-commit hook checks staged files. Format Rust with `packages/desktop-gpui/cargo.sh fmt`.
- Use only `packages/desktop-gpui/cargo.sh` for cargo, so the rustup toolchain runs. A per-builder build folder goes under `packages/desktop-gpui/target/groups/<groupId>` through `CARGO_TARGET_DIR`, and only when the disk rule in the watchtower skill allows it (free disk was 14 GB and `target` was 22 GB on 2026-10-16, so builders share `target` until the owner frees disk).
- Builders run Claude with `bypassPermissions`, because nobody answers a permission. They still obey every rule in this file.
- Builders carry the label `watchtower.task=<first TASK ID of the group>` (Q-037).
- A test or UI checker agent never takes the user's mouse cursor (owner answer, 2026-10-16). Use only AX actions, AX reads, window screenshots, and keyboard input sent to the app. Record a line that needs the pointer (hover, drag, pointer scroll) as `UNVERIFIED (unattended): needs pointer`.
- A tested app never comes to the front or covers the user's active app (owner answer, 2026-10-16). Launch it with `open -g`, and never activate or raise a window. Record a line that needs the app in front as `UNVERIFIED (unattended): needs foreground`. No exception: the Q-088 run kept the app on top with a loop and blocked the owner's other apps, so the owner stopped it (2026-10-16). Run a foreground check only when the owner starts it in the same prompt and is away from the machine.
- Builders run no screen check. They return each app screenshot, `cua_repl` step, AX tree read, and `measure.sh` read as `DEFERRED (UI checker)`.
- Builders and the UI checker archive or kill every mock agent and terminal they create on live, except the run-start fixture.
- Never push and never open a PR.
- Skip: none. TASK-156 has one manual VoiceOver check; record it as `UNVERIFIED (unattended)`.

## Verify Extras

- Before each commit: `npm run format:check:files -- <changed JS, TS, JSON, or markdown files>` -> exit 0.
- After each group with Rust changes: `packages/desktop-gpui/cargo.sh fmt --check -p <crate>` -> exit 0.
- After each group with JS or TS changes: `npm run lint -- <changed source files>` -> exit 0.

## Environment

- Start: none. The run uses the live daemon only when it is already running.
- Stop: none.
- Run-start fixtures: at run start, the lead starts one mock agent on live titled `gpui-age`: `npm run cli -- run --provider mock --model thirty-minute-stream --title gpui-age --cwd /tmp/gpui-bench -d -q "start bench"`. A CPU read that needs a turn age of about 15 minutes uses it when its age is 12 to 25 minutes. Otherwise the UI checker starts a fresh mock at the start of its run, does every other check while the mock ages, and takes the 15-minute reads last. The mock ends after 30 minutes. The lead stops and archives every `gpui-age` agent at Finish.
- UI timeouts: pass `timeout_ms` to every `cua_repl` call and wrap each step in `Promise.race` with a 60-second limit. A screen lock folder `/tmp/gpui-ui-lock` older than 15 minutes is stale: break it with `find /tmp/gpui-ui-lock -maxdepth 0 -mmin +15 -exec rmdir {} \;` before waiting.
- Destructive database steps: never.

## Known Failures

- `bachuc-command` test `search_500_items_stays_under_16_ms_per_keystroke`: a timing test that fails under a high load average (41 ms at load 5.5). Confirm by: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-command --test search` alone -> pass.
- GPUI log "has no accessibility node ... it has an id but no role" for a focused gpui-kit Input or Textarea: an upstream gap in gpui-base 0.7.3 (Q-087). Confirm by: `grep -n 'id("input-state")' -A2 ~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/gpui-base-0.7.3/src/input/base/state.rs` -> no `.role(` after the id.
