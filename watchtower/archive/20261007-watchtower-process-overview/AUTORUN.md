# Autorun

Started: 2026-10-07. The user reviews watchtower-overview and merges it to main.

## Config

```text
integration_branch: watchtower-overview
protected_branches: main next t3code-port
branch_prefix: auto/
forbidden_paths: .env* */.env* .dev/* */.dev.vars
max_line: 100
header_pattern: ^(feat|fix|docs|refactor|test|chore|perf)(\([a-z0-9-]+\))?: .+
runner: loop
loop_delay_seconds: 60
paseo_every: 5m
paseo_profile: Opus
```

## Repo Rules

- Open every PR on `HiepPP/paseo` with `--repo HiepPP/paseo`. Never open or touch a PR on `getpaseo/paseo`.
- Never push to `t3code-port` and never touch PR #13. The integration branch starts from `t3code-port`, so the final draft PR to `main` also carries the PR #13 commits until #13 merges.
- Never restart or stop the release daemon on port 6767. Never run `scripts/paseo-release.sh` or `npm run build:desktop`.
- Install and reload plugins on live only (`npm run cli -- plugin ...`). Never install, reload, or remove a plugin on release.
- Do not start, stop, or restart the live daemon or the live Electron app. If live is down, write each live check as `UNVERIFIED (autonomous run)`.
- Run only the test files a TASK changes, with `--bail=1` for vitest. In the plugin folder, `npm test` is allowed, because the plugin suite is small.
- Run GitNexus `impact` before you edit a symbol. Run `detect_changes` before each commit, per the root `CLAUDE.md`.
- The gate has no `formatter` line: oxfmt 0.46.0 rejects `--check` together with `--stdin-filepath`. Format with `npm run format:files -- <changed files>` before you stage. This includes `watchtower/` markdown, because the lefthook pre-commit hook checks the format of every staged `*.md` file. Put path globs in code spans first; the formatter reads a bare `**` as emphasis.
- The plan files were written at Start and are not committed yet. Ship them with the first iteration's commit.
- Create the schedule and each handoff agent with thinking `xhigh` (`--thinking xhigh`). This overrides the `high` of the `Opus` profile.
- Skip: none.

## Verify Extras

- After a TASK that changes the plugin: in `hiep-plugins/plugins/watchtower-board`, run `npm test`, `npm run typecheck`, and `npm run lint`.
- Before each commit: `npm run format:check:files -- <changed source files>` -> exit 0.
- After a TASK that changes `packages/*`: `npm run typecheck` and `npm run lint -- <changed files>`. This plan should not change `packages/*`.

## Environment

- Start: none. The run uses the live daemon only when it is already running.
- Stop: none.
- Destructive database steps: never.

## Known Failures

- None yet.
