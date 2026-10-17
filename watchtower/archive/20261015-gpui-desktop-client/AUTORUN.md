# Autorun

Started: 2026-10-15. The user reviews `feat/bachuc-gpui` and merges it to `main`.

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
- Never send prompts to or create agents on release (6767) in a check. Live checks use port 6768 only.
- Run one test file at a time with `npx vitest run <file> --bail=1`. Never run a full suite.
- Run GitNexus `impact` before you edit an existing TypeScript symbol. Run `detect_changes` before each commit, per the root `CLAUDE.md`.
- The gate has no `formatter` line. Format changed JS, TS, JSON, and markdown files with `npm run format:files -- <files>` before you stage; the lefthook pre-commit hook checks staged files. Format Rust with `packages/desktop-gpui/cargo.sh fmt`.
- Use only `packages/desktop-gpui/cargo.sh` for cargo, so the rustup toolchain runs. Keep one shared `target` folder.
- Builders run Claude with `bypassPermissions`, because nobody answers a permission. They still obey every rule in this file.
- Builders carry the label `watchtower.task=<first TASK ID of the group>` (Q-037).
- Builders archive or kill every mock agent and terminal they create on live.
- Never push and never open a PR.
- Skip: none. TASK-142 has one manual VoiceOver check; the builder records it as `UNVERIFIED (unattended)`.

## Verify Extras

- Before each commit: `npm run format:check:files -- <changed JS, TS, JSON, or markdown files>` -> exit 0.
- After each group with Rust changes: `packages/desktop-gpui/cargo.sh fmt --check -p <crate>` -> exit 0.
- After each group with JS or TS changes: `npm run lint -- <changed source files>` -> exit 0.

## Environment

- Start: none. The run uses the live daemon only when it is already running.
- Stop: none.
- Destructive database steps: never.

## Known Failures

- None yet.
