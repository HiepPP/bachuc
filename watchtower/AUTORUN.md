# Autorun

Started: 2026-10-06. The user reviews t3code-port and merges it to main.

## Config

```text
integration_branch: t3code-port
protected_branches: main next
branch_prefix: auto/
forbidden_paths: .env* */.env* .dev/* */.dev.vars
max_line: 100
header_pattern: ^(feat|fix|docs|refactor|test|chore|perf)(\([a-z0-9-]+\))?: .+
runner: loop
loop_delay_seconds: 60
paseo_every: 15m
```

## Repo Rules

- Open every PR on `HiepPP/paseo` with `--repo HiepPP/paseo`. Never open or touch a PR on `getpaseo/paseo`.
- Never restart or stop the release daemon on port 6767. Never run `scripts/paseo-release.sh` or `npm run build:desktop`.
- Install and reload plugins on live only (`npm run cli -- plugin ...`). Never install, reload, or remove a plugin on release.
- Do not start, stop, or restart the live daemon or the live Electron app. If live is down, write each live check as `UNVERIFIED (autonomous run)`.
- Run only the test files a TASK changes, with `--bail=1`. Never run a whole workspace suite.
- Run GitNexus `impact` before you edit a symbol. Run `detect_changes` before each commit, per the root `CLAUDE.md`.
- The gate has no `formatter` line: oxfmt 0.46.0 rejects `--check` together with `--stdin-filepath`. Format with `npm run format:files -- <changed files>` before you stage. This includes `watchtower/` markdown, because the lefthook pre-commit hook checks the format of every staged `*.md` file. Put path globs in code spans first; the formatter reads a bare `**` as emphasis.
- Skip: none.

## Verify Extras

- After a TASK that changes `packages/*`: `npm run typecheck` and `npm run lint -- <changed files>`.
- Before each commit: `npm run format:check:files -- <changed source files>` -> exit 0.
- After a TASK that changes `packages/protocol` or `packages/client`: `npm run build:client` before `npm run typecheck`.
- After a TASK that adds a plugin: `npm install` in the plugin folder first, then `npm test`, `npm run typecheck`, and `npm run lint` there.

## Environment

- Start: none. The run uses the live daemon only when it is already running.
- Stop: none.
- Destructive database steps: never.

## Known Failures

- None yet.
