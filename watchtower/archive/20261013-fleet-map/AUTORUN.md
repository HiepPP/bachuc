# Autorun

Started: 2026-10-13. The user reviews `feat/fleet-map` and merges it to `main`.

## Config

```text
protected_branches: main next t3code-port
branch_prefix: feat/
forbidden_paths: .env* */.env* .dev/* */.dev.vars
max_line: 100
header_pattern: ^(feat|fix|docs|refactor|test|chore|perf)(\([a-z0-9-]+\))?: .+
```

## Repo Rules

- Never restart or stop the release daemon on port 6767. Never run `scripts/paseo-release.sh` or `npm run build:desktop`.
- Do not start, stop, or restart the live daemon or the live Electron app (owner answer, 2026-10-12). If live is down, write each live check as `UNVERIFIED (unattended)`.
- Never install, reload, or remove a plugin on release.
- Run only the test files a TASK changes, with `--bail=1` for vitest. Run app vitest files from `packages/app`. Never pass `--dir` to vitest. Send the output of a large test file to a file under `/tmp`.
- Run GitNexus `impact` before you edit a symbol. Run `detect_changes` before each commit, per the root `CLAUDE.md`.
- The gate has no `formatter` line: oxfmt rejects `--check` together with `--stdin-filepath`. Format with `npm run format:files -- <changed files>` before you stage. This includes `watchtower/` markdown, because the lefthook pre-commit hook checks the format of every staged `*.md` file. Put path globs in code spans first.
- Builders run Claude Sonnet with `bypassPermissions`, because nobody answers a permission. They still obey every rule in this file.
- Never push and never open a PR.
- No TASK waits on a question. Q-034 takes its default.
- Skip: none.

## Verify Extras

- Before each commit: `npm run format:check:files -- <changed files>` -> exit 0.
- After a TASK that changes `packages/*`: `npm run typecheck` and `npm run lint -- <changed files>`. If a cross-package type error appears, run `npm run build:client` or `npm run build:server` first, per the root `CLAUDE.md`.

## Environment

- Start: none. The run uses the live daemon only when it is already running.
- Stop: none.
- Destructive database steps: never.

## Known Failures

- None.
