# Autorun

Started: 2026-10-15. The user reviews `feat/sqlite-ram-cpu-cuts` and merges it to `main`.

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
- Run one test file at a time with `npx vitest run <file> --bail=1`. Never run a full suite.
- Run GitNexus `impact` before you edit a symbol. Run `detect_changes` before each commit, per the root `CLAUDE.md`.
- The gate has no `formatter` line. Format changed source files with `npm run format:files -- <files>` and `watchtower/` markdown the same way before you stage; the lefthook pre-commit hook checks staged files.
- Builders run Claude with `bypassPermissions`, because nobody answers a permission. They still obey every rule in this file.
- Builders carry the label `watchtower.task=<first TASK ID of the group>` (Q-037).
- Never push and never open a PR.
- Skip: none.

## Verify Extras

- Before each commit: `npm run format:check:files -- <changed files>` -> exit 0.
- After each group: `npm run lint -- <changed source files>` -> exit 0.

## Environment

- Start: none. The run uses the live daemon only when it is already running.
- Stop: none.
- Destructive database steps: never.

## Known Failures

- `packages/server/src/server/mcp-server.test.ts`: 7 cases fail with a TypeError that reads `size` of `backgroundProcesses` at `agent-projections.ts:149`, without this run's changes (build E, 2026-10-15). Confirm by: `npx vitest run packages/server/src/server/mcp-server.test.ts` -> the same TypeError.
