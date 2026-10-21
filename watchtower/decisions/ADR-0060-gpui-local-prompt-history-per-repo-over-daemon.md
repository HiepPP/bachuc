# ADR-0060 GPUI-local prompt history per repo over daemon storage

Status: accepted
Date: 2026-10-21
Supersedes: -
Scope: `packages/desktop-gpui/crates/composer/**`

## Context

- Hiep wants Up and Down in the composer to recall earlier prompts of a repo, as in the Claude Code CLI.
- Before this, neither the GPUI nor the Electron composer kept a prompt history.
- Bachuc workspaces are often worktrees, so one repo has many cwds.
- GPUI is the daily app, and it already keeps per-client files in its support folder, such as `agent-defaults.json` ([ADR-0056](watchtower/decisions/ADR-0056-gpui-local-new-agent-defaults-over-last-used-or-daemon-config.md)).

## Decision

- The GPUI client keeps `prompt-history.json` in its support folder, keyed by the project root. All worktrees of one repo share one list.
- Hiep chose this on 2026-10-21.

## Consequences

- No protocol change.
- Prompts sent from Electron, mobile, or another machine are not in the list.

## Options Considered

- Key by cwd, exactly as the CLI: rejected. A new worktree would start with an empty list.
- Store history in the daemon through a new RPC: rejected. It needs a protocol and server change, and GPUI is the only client that uses it.

## Revisit If

- Hiep wants the same history in Electron, mobile, or on another machine.
