# ADR-0017 Native host features over thin host changes

Status: accepted
Date: 2026-10-12
Scope: `packages/**`, `hiep-plugins/**`, `CLAUDE.md`, `hiep-plugins/AGENTS.md`

## Context

- The old fork rule said: build every feature in a plugin, and keep host changes thin (a hook, a contribution point, a context field), so upstream merges stay cheap.
- The owner said on 2026-10-12 that the fork has moved too far from upstream for merge cost to drive design.
- A new plugin API is where bugs tend to appear. Plugin code cannot use the stream model, app primitives, or host tests directly. [ADR-0016](watchtower/decisions/ADR-0016-host-turn-step-list-over-plugin-group-renderer.md) already chose the host for the turn step list on this ground.

## Decision

- Put each feature where it runs natively with the fewest bugs.
- Build it in the host (`packages/*`) when it needs app internals such as the stream model, layout, navigation, or a hot path, or when a plugin would need a new plugin API.
- Use a plugin for a self-contained extra that the existing plugin APIs already cover.
- Upstream merge cost is not a reason to pick a plugin.

## Consequences

- Host features reuse host tests and primitives, with fewer new contracts.
- Upstream merges conflict more often in the files that the fork changes.
- `hiep-plugins` keeps the extras that are easy to turn off or replace.

## Options Considered

- Thin host changes, feature in a plugin: rejected. It needs new plugin APIs for deep features, and the merge savings it bought no longer matter.
- Host first for everything: rejected. Self-contained extras stay easier to toggle and reload as plugins.

## Revisit If

- The owner plans to merge upstream releases on a regular schedule again.
- Two upstream merges in a row cost more than a day each because of fork changes in the host.
