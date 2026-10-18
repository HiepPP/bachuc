# Autorun

Started: 2026-10-18. The user reviews `feat/gpui-zoom-and-follow-ups` and merges it to `main`.

## Config

```text
protected_branches: main next t3code-port
branch_prefix: feat/
forbidden_paths: .env* */.env* .dev/* */.dev.vars
max_line: 100
header_pattern: ^(feat|fix|docs|refactor|test|chore|perf|build)(\([a-z0-9-]+\))?: .+
max_builders: 6
```

## Repo Rules

- Branch: `feat/gpui-zoom-and-follow-ups`, the default from the plan slug.
- Never restart, stop, or reload the release daemon on port 6767. Never run `scripts/paseo-release.sh` or `npm run build:desktop`. Never run `PASEO_HOME=~/.bachuc paseo plugin reload`.
- Test only on live (6768) and on the check bundle opened with `BACHUC_NO_ACTIVATE=1` (owner, 2026-10-18). Do not stop or restart live. If live is down, the lead starts it detached (`nohup npm run dev > /tmp/live-daemon.log 2>&1 < /dev/null & disown`) and confirms `127.0.0.1:6768` listens. Builders never touch live.
- Builders run code tests only: unit tests and crate tests. A builder opens no app window and returns each `UI checker:` line as `DEFERRED (UI checker)`.
- One UI checker runs the `UI checker:` lines: it opens the check bundle and the Electron reference with `open -n -g --env BACHUC_NO_ACTIVATE=1`, never brings an app to the front, never moves the pointer, and takes window screenshots only. A stale or blank frame is `UNVERIFIED (unattended): background frame`. A line that needs the pointer, the front, or a real swipe or scroll is `UNVERIFIED (unattended): needs pointer` or `needs foreground`.
- The UI checker quits every app it opens.
- Six builders may run at once (owner, 2026-10-18). Every builder runs `claude-sonnet-5-5` with thinking high, also for risky TASKs (owner, 2026-10-18). Every cargo command runs with `CARGO_BUILD_JOBS=4`. Builders share `packages/desktop-gpui/target`, so the cargo lock runs one build at a time. Do not set `CARGO_TARGET_DIR`. Free disk was 30 GB at run start. The lead checks `df -h /` before each group starts and needs at least 20 GB free (owner, 2026-10-18).
- A build error only in a file that another running group owns is not your failure: wait 2 minutes and build again.
- Use only `packages/desktop-gpui/cargo.sh` for cargo.
- The gate has no `formatter` line. Format Rust with `packages/desktop-gpui/cargo.sh fmt -p <crate>`. Format changed JS, JSON, and markdown files with `npm run format:files -- <file> <file>` before you stage.
- Builders run Claude with `bypassPermissions`, because nobody answers a permission. They still obey every rule in this file.
- Builders carry the label `watchtower.task=<first TASK ID of the group>`.
- After the last cargo command of the run, run `packages/desktop-gpui/cargo.sh clean --profile dev` when no other cargo or rustc runs.
- Never push and never open a PR.
- Skip: none.

## Verify Extras

- After each group with Rust changes: `packages/desktop-gpui/cargo.sh fmt --check -p <crate>` -> exit 0.
- After each group with JS changes: `npm run lint -- <changed source files>` -> exit 0.

## Environment

- Start: none. Stop: none.
- Run-start fixtures: none.
- Check bundle for UI checks: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/bundle.sh bin bachuc-desktop "Bachuc GPUI Check" io.github.hieppp.bachuc-desktop-check`, then `open -n -g --env BACHUC_NO_ACTIVATE=1 "packages/desktop-gpui/target/bundles/Bachuc GPUI Check.app"`. It connects to live (6768); if live is down, each UI check that needs daemon data is `UNVERIFIED (unattended): live down`.
- Electron reference: `open -g packages/desktop/release/mac-arm64/Bachuc.app` (its own daemon on 6770, home `~/.paseo-dev`). Never rebuild it.
- UI timeouts: pass `timeout_ms` to every `cua_repl` call and wrap each step in `Promise.race` with a 60-second limit.
- Destructive database steps: never.

## Known Failures

- `bachuc-command` test `search_500_items_stays_under_16_ms_per_keystroke`: a timing test that fails under a high load average. Confirm by: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-command --test search` alone -> pass.
- `bachuc-plugin-watchtower-board` test `one_thousand_derives_of_a_40_task_plan_take_under_200_ms`: a timing test that fails under a high load average (209 ms at load 6.5, 2026-10-18). Confirm by: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-watchtower-board --lib one_thousand_derives` alone -> pass.
