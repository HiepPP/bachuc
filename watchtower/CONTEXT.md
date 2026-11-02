# Plan Context

## Shared Context

- Source: [github.com/dietrichgebert/ponytail](https://github.com/dietrichgebert/ponytail) v5.3.3, commit `96608e0`. It is one prompt skill, `skills/ponytail/SKILL.md`, plus hooks in `hooks/`.
- `hooks/ponytail-instructions.js` exports `getPonytailInstructions(mode)`. It returns the SKILL.md body filtered to one level, and reads SKILL.md from `../skills/ponytail/SKILL.md` relative to itself.
- Modes in this plan: `off`, `lite`, `full`, `ultra`. Labels: `Off`, `Lite`, `Full`, `Ultra`. The default for an agent with no saved mode is `full`.
- Plugin id `ponytail`. Data folder: `$PASEO_HOME/plugin-data/ponytail/`. Per-agent mode: `agents/<agentId>/mode.json` holding `{"mode": "<mode>"}`.
- The installer records the Ponytail install folder as `{"ponytailRoot": "<abs path>"}` in `$PASEO_HOME/plugin-data/ponytail/hook-runtime.json`.
- The closest pattern is the Caveman part of [hiep-plugins/plugins/prompt-translate](hiep-plugins/plugins/prompt-translate). Copy its patterns; never import its modules ([ADR-0062](watchtower/decisions/ADR-0062-selection-translate-plugin-copies-llm-logic-over-sharing-prompt-translate.md), [ADR-0079](watchtower/decisions/ADR-0079-ponytail-plugin-over-extending-prompt-translate.md)).

## Decisions

- [ADR-0079](watchtower/decisions/ADR-0079-ponytail-plugin-over-extending-prompt-translate.md): Ponytail is its own plugin, server and GPUI crate.
- [ADR-0080](watchtower/decisions/ADR-0080-disabled-native-ponytail-with-bridge-hook.md): the native Claude plugin stays disabled; the daemon hook serves Paseo agents and a bridge hook serves Claude outside Paseo.
- Default mode is `full` for every agent with no saved mode, including agents that existed before this plan. Hiep chose this on 2026-11-02.
- Claude and Codex agents both get the daemon context. Hiep chose this on 2026-11-02.

## Open Decisions

- None.

## References

- [hiep-plugins/plugins/prompt-translate/index.server.ts](hiep-plugins/plugins/prompt-translate/index.server.ts)
- [packages/desktop-gpui/plugins/prompt-translate/src/prefs.rs](packages/desktop-gpui/plugins/prompt-translate/src/prefs.rs)
- [docs/plugins.md](docs/plugins.md)
