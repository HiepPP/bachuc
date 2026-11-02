# ADR-0079 Own ponytail plugin over extending prompt-translate

Status: accepted
Date: 2026-11-02
Supersedes: -
Scope: `hiep-plugins/plugins/ponytail/**`, `packages/desktop-gpui/plugins/ponytail/**`, `hiep-plugins/plugins/prompt-translate/**`

## Context

- Ponytail ([github.com/dietrichgebert/ponytail](https://github.com/dietrichgebert/ponytail), v5.3.3) is a prompt skill with levels `lite`, `full`, `ultra`, and `off`.
- The Caveman pill, per-agent mode store, and `agent.prompt` hook already live in [hiep-plugins/plugins/prompt-translate](hiep-plugins/plugins/prompt-translate), next to the translation features.
- [ADR-0062](watchtower/decisions/ADR-0062-selection-translate-plugin-copies-llm-logic-over-sharing-prompt-translate.md) bans imports of prompt-translate server modules from another plugin.

## Decision

- Ponytail gets its own Paseo plugin `ponytail`: server part in [hiep-plugins/plugins/ponytail](hiep-plugins/plugins/ponytail), GPUI client crate `bachuc-plugin-ponytail`.
- It copies the small mode-store and pill patterns from prompt-translate. It imports nothing from prompt-translate.
- Hiep chose this on 2026-11-02 during `watchtower new`.

## Consequences

- prompt-translate stays unchanged.
- The new plugin repeats about 100 lines of mode store and pill code.
- A Ponytail client change needs a GPUI build, like every plugin crate.

## Options Considered

- Extend prompt-translate with a second pill: rejected. It mixes an unrelated feature into the translate plugin, and Hiep wants Ponytail separate.

## Revisit If

- A third prompt-mode pill is planned, so a shared mode-pill helper would remove more code than it adds.
