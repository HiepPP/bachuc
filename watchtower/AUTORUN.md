# Autorun

Started: 2026-10-19. The user reviews `feat/gpui-background-process-card` and merges it to `main`.

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

- Branch: `feat/gpui-background-process-card`, the default from the plan slug.
- Never restart, stop, or reload the release daemon on port 6767. Never run `scripts/paseo-release.sh` or `npm run build:desktop`. Never run `PASEO_HOME=~/.bachuc paseo plugin reload`.
- Test only on live (6768) and on the check bundle opened with `BACHUC_NO_ACTIVATE=1` (owner, 2026-10-18). Do not stop or restart live. If live is down, the lead starts it detached (`nohup npm run dev > /tmp/live-daemon.log 2>&1 < /dev/null & disown`) and confirms `127.0.0.1:6768` listens. Builders never touch live.
- Builders run code tests only: unit tests and crate tests. A builder opens no app window and returns each `UI checker:` line as `DEFERRED (UI checker)`.
- Every builder, fix builder, and UI checker runs provider `deepseek`, model `deepseek-flash[1m]`, thinking `max`, mode `bypassPermissions`, for every Class (owner, 2026-10-19). No `logic`, `frontend`, or Opus agent runs.
- The UI checker uses `cua_repl` AX actions only. It never takes the cursor and never brings an app to the front. Hover, drag, pointer scroll, and foreground-only checks are `UNVERIFIED (unattended)`. It quits every app it opens and restores the zoom to 1.25.
- Every cargo command runs with `CARGO_BUILD_JOBS=4`. Builders share `packages/desktop-gpui/target`, so the cargo lock runs one build at a time. Do not set `CARGO_TARGET_DIR`. Free disk was 31 GB at run start. The lead checks `df -h /` before each group starts and needs at least 20 GB free (owner, 2026-10-18).
- A build error only in a file that another running group owns is not your failure: wait 2 minutes and build again.
- Use only `packages/desktop-gpui/cargo.sh` for cargo.
- The gate has no `formatter` line. Format Rust with `packages/desktop-gpui/cargo.sh fmt -p <crate>`. Format changed JS, JSON, and markdown files with `npm run format:files -- <file> <file>` before you stage.
- Keep the owner's uncommitted text replacement work: `packages/desktop-gpui/vendor/gpui-base/src/input/**`, `vendor/gpui-base/Cargo.toml`, the `## Text Replacement` section of `vendor/gpui-base/BACHUC-PATCH.md`, and `packages/desktop-gpui/crates/composer/tests/`. Never edit, stage, stash, or revert them. TASK-269 adds its own section to `BACHUC-PATCH.md` below the existing text; the lead stages only that hunk.
- Group C runs as two builders in sequence (lead, 2026-10-19): C1 = TASK-268 and TASK-273 at once; C2 = TASK-272 after groups B and C1 close. They share files but never run at the same time.
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
- `bachuc-sidebar` tests `building_500_workspaces_in_project_mode_stays_under_2_ms` and `building_500_workspaces_with_meta_data_stays_under_2_ms`: timing tests that fail under a high load average (load 9.6 while a builder compiled, 2026-10-19). Confirm by: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-sidebar --test model` alone -> pass.
- `bachuc-client` test `one_thousand_workspace_updates_stay_cheap`: a timing test that failed at 45 ms under builder load and passed alone (2026-10-19). Confirm by: `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-client` alone -> pass.
