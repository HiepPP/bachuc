# Autorun

Started: 2026-10-17. The user reviews `feat/gpui-daily-gaps-and-fixes` and merges it to `main`.

## Config

```text
protected_branches: main next t3code-port
branch_prefix: feat/
forbidden_paths: .env* */.env* .dev/* */.dev.vars
max_line: 100
header_pattern: ^(feat|fix|docs|refactor|test|chore|perf|build)(\([a-z0-9-]+\))?: .+
max_builders: 2
```

## Repo Rules

- Never restart or stop the release daemon on port 6767. Never run `scripts/paseo-release.sh` or `npm run build:desktop`.
- Do not start, stop, or restart the live daemon or the live Electron app (owner answer, 2026-10-12). Exceptions (owner, 2026-10-17): the lead starts live for the UI checker, and before any hand test it asks of the owner. Start it detached (`nohup npm run dev > /tmp/live-daemon.log 2>&1 < /dev/null & disown`) and confirm `127.0.0.1:6768` listens; a `run_in_background` job dies at its time limit. Leave live running when the owner still has to test. Builders never touch live.
- Builders run code tests only: unit tests, crate tests, and fake-daemon tests. A builder opens no app window and returns each `UI check:` line as `DEFERRED (UI checker)`.
- One UI checker runs the `UI check:` lines (owner, 2026-10-16 and 2026-10-17): it opens the GPUI bundle and the Electron reference with `open -g`, never brings an app to the front, never moves the pointer, and takes window screenshots only. A stale or blank frame (Q-069) is `UNVERIFIED (unattended): background frame`. A line that needs the pointer or the front is `UNVERIFIED (unattended): needs pointer` or `needs foreground`.
- The UI checker quits every app it opens, and never runs `measure.sh`.
- Memory: at most 2 builders at once. Every cargo command runs with `CARGO_BUILD_JOBS=4`, one at a time, scoped with `-p <crate>` (owner answer, 2026-10-16).
- Builders share `packages/desktop-gpui/target`, so the cargo lock runs one build at a time. Do not set `CARGO_TARGET_DIR`.
- Use only `packages/desktop-gpui/cargo.sh` for cargo, so the rustup toolchain runs.
- The gate has no `formatter` line. Format Rust with `packages/desktop-gpui/cargo.sh fmt -p <crate>`. Format changed JS, JSON, and markdown files with `npm run format:files -- <file> <file>` before you stage.
- Builders run Claude with `bypassPermissions`, because nobody answers a permission. They still obey every rule in this file.
- Builders carry the label `watchtower.task=<first TASK ID of the group>`.
- Never push and never open a PR.
- Skip: none.

## Verify Extras

- After each group with Rust changes: `packages/desktop-gpui/cargo.sh fmt --check -p <crate>` -> exit 0.
- After each group with JS changes: `npm run lint -- <changed source files>` -> exit 0.

## Environment

- Start: none. Stop: none.
- Run-start fixtures: none.
- GPUI bundle for UI checks: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/bundle.sh bin bachuc-desktop "Bachuc GPUI" io.github.hieppp.bachuc-desktop`, then `open -g "packages/desktop-gpui/target/bundles/Bachuc GPUI.app"`. It connects to live (6768); if live is down, each UI check that needs daemon data is `UNVERIFIED (unattended): live down`.
- Electron reference: `open -g packages/desktop/release/mac-arm64/Bachuc.app` (built 2026-10-15; its own daemon on 6770, home `~/.paseo-dev`). Never rebuild it.
- UI timeouts: pass `timeout_ms` to every `cua_repl` call and wrap each step in `Promise.race` with a 60-second limit.
- Destructive database steps: never.

## Known Failures

- `bachuc-command` test `search_500_items_stays_under_16_ms_per_keystroke`: a timing test that fails under a high load average (41 ms at load 5.5). Confirm by: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-command --test search` alone -> pass.
