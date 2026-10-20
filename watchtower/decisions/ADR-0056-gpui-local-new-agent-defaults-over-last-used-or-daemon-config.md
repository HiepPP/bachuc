# ADR-0056 GPUI-local new agent defaults over last-used memory or daemon config

Status: accepted
Date: 2026-10-20
Supersedes: -
Scope: `packages/desktop-gpui/crates/settings/src/agent_defaults.rs`, `packages/desktop-gpui/crates/settings/src/pane_agent_defaults.rs`, `packages/desktop-gpui/crates/composer/src/draft.rs`, `packages/desktop-gpui/crates/composer/src/composer.rs`

## Context

- Hiep wants each new GPUI draft (New workspace and a workspace draft tab) to start with Opus 5.5, effort high, and mode bypass.
- Before, a draft took the first available provider and the host's default model, with no effort and no mode.

## Decision

- Settings has a "New agent defaults" section with Model, Effort, and Mode pickers. Hiep chose an explicit setting on 2026-10-20.
- The values live in `agent-defaults.json` in the GPUI support folder (`~/Library/Application Support/BachucGpui`), next to `hosts.json` and `shortcuts.json`. Shape: `{"provider","model","thinkingOptionId","modeId"}`, and unset fields are left out.
- Each new draft reads the file again. No file watch is needed.
- If the stored provider is not available, the draft starts as before. Otherwise, each field the host does not offer falls back by itself. Effort and mode never carry over to a fallback provider.
- A fork seed replaces all four fields.

## Consequences

- With no file, drafts start exactly as before.
- The setting is per client. Electron and other machines do not see it.
- The stored mode waits for the provider's mode list, as a seeded mode does.
- If a draft is built before the host sends the model's effort options, the stored effort is not applied. This was not verified on 2026-10-20.

## Options Considered

- Remember the last choice, as Electron `use-form-preferences` does: rejected by Hiep. The default would move with each pick.
- Store the defaults in the daemon `config.json`: rejected. It needs a protocol change, and GPUI is the only client that uses it.
- Both a setting and last-used memory: rejected. More code and tests for no current need.

## Revisit If

- Hiep wants the same defaults in Electron or on another host.
- The stored effort is lost on the first draft after the app starts.
