# Autorun

Started: 2026-10-14. The user reviews `feat/native-glm-provider` and merges it to `main`.

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
- Do not start, stop, or restart the live daemon or the live Electron app (owner answer, 2026-10-12). If a check needs a live restart, write it as `UNVERIFIED (unattended)`.
- Never install, reload, or remove a plugin on release.
- Never read, write, or create files under `~/.claude` for a test. Never write the Z.AI key to a file.
- Run GitNexus `impact` before you edit a symbol. Run `detect_changes` before each commit, per the root `CLAUDE.md`.
- The gate has no `formatter` line. Format changed files with `npm run format:files -- <files>` from the repo root before you stage; the lefthook pre-commit hook checks every staged `*.md` file.
- Run only the test files a TASK changes, one at a time, with `npx vitest run <file> --bail=1`. Never run a full workspace test suite.
- Builders run Claude with `bypassPermissions`, because nobody answers a permission. They still obey every rule in this file.
- Builders carry the label `watchtower.task=<first TASK ID of the group>`.
- Never push and never open a PR.
- Skip: none.

## Verify Extras

- After a change in `packages/protocol` or `packages/server`: `npm run build:server` before `npm run typecheck`, so workspace declarations are current.
- Before each commit: `npm run format:check:files -- <changed files>` -> exit 0.

## Environment

- Start: none. The run does not restart live, so a server change is not loaded on live during the run.
- Stop: none.
- Destructive database steps: never.

## Known Failures

- None.
