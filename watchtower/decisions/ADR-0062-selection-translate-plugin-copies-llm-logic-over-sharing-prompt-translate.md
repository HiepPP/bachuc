# ADR-0062 selection-translate plugin copies the LLM logic over sharing prompt-translate

Status: accepted
Date: 2026-10-22
Supersedes: -
Scope: `hiep-plugins/plugins/selection-translate/**`, `hiep-plugins/plugins/prompt-translate/server/**`

## Context

- A GPUI plugin reaches only its own server: `PluginContext::rpc` in [packages/desktop-gpui/crates/plugin-host/src/context.rs](packages/desktop-gpui/crates/plugin-host/src/context.rs) sends `self.plugin_id`.
- The LLM call, credentials, and cache live in [hiep-plugins/plugins/prompt-translate/server](hiep-plugins/plugins/prompt-translate/server).
- [hiep-plugins/AGENTS.md](hiep-plugins/AGENTS.md) says not to add shared packages without a concrete need.

## Decision

- Selection translate (English to Vietnamese) is a new plugin, `selection-translate`, with its own server, settings, and cache. Its server copies `llm.ts`, `credentials.ts`, and `store.ts` from prompt-translate.
- Hiep chose this on 2026-10-22 in the watchtower interview for TASK-293 to TASK-295.

## Consequences

- The plugin turns on and off apart from prompt-translate, as Cite does.
- A fix to the LLM call or the credential lookup must be made in both plugins. Each copied file names its source.

## Options Considered

- Add the RPC and the action to prompt-translate: rejected. Hiep wants a separate plugin like Cite.
- New plugin that imports `../prompt-translate/server/*`: rejected. It ties the two plugin folders together, against the hiep-plugins layout rule.

## Revisit If

- A third plugin needs the same LLM call, which makes a shared module a concrete need.
- A fix lands in one copy and is missed in the other.
