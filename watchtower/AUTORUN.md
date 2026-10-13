# Autorun

Started: 2026-10-13. The user reviews `composer-pill-wheel-select` and merges it to `main`.

## Config

```text
integration_branch: composer-pill-wheel-select
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

- Open every PR on `HiepPP/bachuc` with `--repo HiepPP/bachuc`. Never open or touch a PR on `getpaseo/paseo`.
- Never deploy, never force-push, and never delete a branch.
- Never restart or stop the release daemon on port 6767. Never run `scripts/paseo-release.sh` or `npm run build:desktop`.
- Do not start, stop, or restart the live daemon or the live Electron app (owner answer, 2026-10-12). Write each live or mouse check as `UNVERIFIED (autonomous run)`.
- Never install, reload, or remove a plugin on release.
- Run only the test files a TASK changes, with `--bail=1` for vitest. Run app vitest files from `packages/app`. Never pass `--dir` to vitest.
- Run GitNexus `impact` before you edit a symbol. Run `detect_changes` before each commit, per the root `CLAUDE.md`.
- The gate has no `formatter` line: oxfmt rejects `--check` together with `--stdin-filepath`. Format with `npm run format:files -- <changed files>` before you stage. This includes `watchtower/` markdown, because the lefthook pre-commit hook checks the format of every staged `*.md` file. Put path globs in code spans first; the formatter reads a bare `**` as emphasis. Format `hiep-plugins/` files with the plugin's own `npm run format`.
- Use HTTPS for every fetch and push, because SSH to GitHub resets on port 22 (owner answer, 2026-10-13): `git -c credential.helper='!gh auth git-credential' push https://github.com/HiepPP/bachuc.git <branch>`. After a fetch, move the `origin/<branch>` ref with `git update-ref`.
- No TASK waits on a question.
- Skip: none.

## Verify Extras

- Before each commit: `npm run format:check:files -- <changed files>` -> exit 0.
- After a TASK that changes `packages/*`: `npm run typecheck` and `npm run lint -- <changed files>`. If a cross-package type error appears, run `npm run build:client` or `npm run build:server` first, per the root `CLAUDE.md`.
- After a TASK that changes `watchtower/` files: `cd hiep-plugins/plugins/watchtower-board && npx tsx --test tests/overview.test.ts` -> pass.

## Environment

- Start: none. The run uses the live daemon only when it is already running.
- Stop: none.
- Destructive database steps: never.

## Known Failures

- None.
