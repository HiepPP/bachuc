# Autorun

Started: 2026-10-16. The user reviews `feat/bachuc-gpui` and merges it to `main`.

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
- Do not start, stop, or restart the live daemon or the live Electron app (owner answer, 2026-10-12). Live was down at run start, so each live check is `UNVERIFIED (unattended)`.
- Proof is code tests only (owner, 2026-10-16): unit tests, crate tests, and fake-daemon tests. Open no app window and run no UI checker, screenshot, `cua_repl`, AX read, or `measure.sh` check. Record such a line as `UNVERIFIED (unattended): code tests only`.
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
- Destructive database steps: never.

## Known Failures

- `bachuc-command` test `search_500_items_stays_under_16_ms_per_keystroke`: a timing test that fails under a high load average (41 ms at load 5.5). Confirm by: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-command --test search` alone -> pass.
