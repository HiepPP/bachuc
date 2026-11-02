# ADR-0081 Ponytail Full by default over Off by default

Status: accepted
Date: 2026-11-02
Supersedes: -
Scope: `hiep-plugins/plugins/ponytail/**`, `packages/desktop-gpui/plugins/ponytail/**`

## Context

- The `ponytail` plugin adds Ponytail rules to each Claude and Codex turn, per agent ([ADR-0080](watchtower/decisions/ADR-0080-disabled-native-ponytail-with-bridge-hook.md)).
- The Caveman pill defaults to "Default", which adds nothing until the user picks a mode.
- The Ponytail README says Ponytail is active every session, at level `full`.

## Decision

- An agent with no saved Ponytail mode runs `full`. This includes agents that existed before the plugin was installed.
- `DEFAULT_MODE` in `hiep-plugins/plugins/ponytail/shared/modes.ts` holds it; the pill shows `Ponytail: Full` for an unknown mode.
- Hiep chose this on 2026-11-02 during `watchtower new`, and confirmed it at archive.

## Consequences

- Every Claude and Codex agent changes its reply style on its next turn after install.
- About 2.9 KB of rules go into the first turn and every 10th turn of each agent.
- An agent that must not use Ponytail needs the pill set to Off, or `/ponytail off`.

## Options Considered

- Off by default, like the Caveman pill: rejected. Hiep wants Ponytail on every agent without a per-agent pick.

## Revisit If

- Hiep sets the pill to Off on most new agents, so Full costs more picks than it saves.
