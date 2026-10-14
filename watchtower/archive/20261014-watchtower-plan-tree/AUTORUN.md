# Autorun

Started: 2026-10-14. The user reviews `feat/watchtower-plan-tree` and merges it to `main`.

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
- Never install, reload, or remove a plugin on release. Reload `watchtower-board` on live only when live already runs.
- Plugin code lives in `hiep-plugins/plugins/watchtower-board`. Run its checks from that folder: `npm run typecheck`, `npm run lint`, `npx tsx --test tests/<file>.test.ts`, and `npm run format` for its files. The root `npm run lint` skips `hiep-plugins`.
- Run GitNexus `impact` before you edit a symbol. Run `detect_changes` before each commit, per the root `CLAUDE.md`.
- The gate has no `formatter` line. Format `watchtower/` markdown with `npm run format:files -- <files>` from the repo root before you stage; the lefthook pre-commit hook checks every staged `*.md` file.
- Builders run Claude with `bypassPermissions`, because nobody answers a permission. They still obey every rule in this file.
- Builders carry the label `watchtower.task=<first TASK ID of the group>` (Q-037).
- Never push and never open a PR.
- No TASK waits on a question. Q-036 takes its default.
- Skip: none.

## Verify Extras

- Before each commit: `npm run format:check:files -- <changed watchtower and root files>` -> exit 0, and the plugin formatter left no diff.
- After a TASK that changes the plugin: `npm run typecheck` and `npm run lint` in `hiep-plugins/plugins/watchtower-board`.

## Environment

- Start: none. The run uses the live daemon only when it is already running.
- Stop: none.
- Destructive database steps: never.

## Known Failures

- None.
