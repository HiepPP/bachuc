# Plan Context

## Shared Context

- This plan adds host plugin APIs in this fork, then moves 7 plugins in [hiep-plugins/plugins](hiep-plugins/plugins) onto them. P1 to P7 come from the plugin review on 2026-09-30.
- Server hooks run in [packages/server/src/server/plugins/runtime.ts](packages/server/src/server/plugins/runtime.ts). Before-hooks chain in plugin-ID order. Each result must keep the request shape. The default request timeout is 30 s.
- A new before-hook name needs [packages/plugin/src/server/lifecycle.ts](packages/plugin/src/server/lifecycle.ts) and `beforeHookNames` plus `beforeSchemas` in [packages/server/src/server/plugins/lifecycle/index.ts](packages/server/src/server/plugins/lifecycle/index.ts).
- The server SDK is types only. A new server method needs the `setup` object in [packages/server/src/server/plugins/plugin-process.ts](packages/server/src/server/plugins/plugin-process.ts) and the type in [packages/plugin/src/server/contracts.ts](packages/plugin/src/server/contracts.ts).
- The client plugin context is built in `runPluginClientBundle` in [packages/app/src/plugins/evaluate.ts](packages/app/src/plugins/evaluate.ts). A new client method touches [packages/plugin/src/client/contracts.ts](packages/plugin/src/client/contracts.ts), [packages/app/src/plugins/types.ts](packages/app/src/plugins/types.ts), and [packages/app/src/plugins/registry.ts](packages/app/src/plugins/registry.ts). Test fixtures that build `InstalledPlugin` literals need the new field.
- [packages/plugin/src/migration-doc.test.ts](packages/plugin/src/migration-doc.test.ts) requires every `add*` client method in [public-docs/plugins/migration.md](public-docs/plugins/migration.md).
- Plugins are separate npm projects. Each has its own `npm run typecheck`, `npm run lint`, and `npm test` scripts. They use `@getpaseo/plugin` from npm, so new host types must be linked from [packages/plugin](packages/plugin) for plugin typecheck.
- Paseo Dev runs its daemon on port 6770 with home `~/.paseo-dev`. It loads plugins from [hiep-plugins/plugins](hiep-plugins/plugins). The stable daemon on port 6767 loads plugins from `~/Projects/hiep-paseo-plugin`. This session runs under the stable daemon.
- Native provider hooks in `~/.claude/settings.json` and `~/.codex/hooks.json` point to `~/Projects/hiep-paseo-plugin`. They read `PASEO_HOME/plugin-data/<plugin>/`. Do not edit those configs.
- Repo rules: run `npm run typecheck` and `npm run lint -- <files>` after changes. Run only changed test files with `npx vitest run <file> --bail=1`. Never run the full suite. Run `npm run format:files -- <files>` for formatting.
- GitNexus `impact` is required before editing a function. Warn on HIGH or CRITICAL risk.
- Do not commit, push, or switch branches unless the user asks.

## Decisions

- Scope: add host APIs, then move plugins onto them. The user chose this on 2026-09-30.
- workspace-spaces stays a plugin and uses a new sidebar API. It does not become a core feature.
- Plugins drop their DOM code. They raise `requirements.paseo` to the fork version, so the stable 0.10.1 host refuses them with a clear error.
- The fork version becomes `0.10.2-beta.900`, because the repo reports `0.10.0-beta.1`, which is lower than stable `0.10.1`. [packages/app/native-release-version.js](packages/app/native-release-version.js) accepts only `X.Y.Z` or `X.Y.Z-beta.N`, so a `-hiep.0` suffix breaks the desktop build. Beta 900 avoids an upstream beta number.
- The `agent.prompt` hook changes only what the provider receives. The timeline keeps the text the user typed.
- Skill pins and Caveman modes apply when the daemon sends the prompt. Queued prompts use the selection at send time. Queue snapshots go away.
- New-thread composers get host defaults for skill pins and Caveman mode. Per-draft selection goes away.
- prompt-translate replaces Cmd+Enter enhancement with a composer pill toggle, because the composer API has no key events.
- Plugin MCP tools are served by the daemon `paseo` MCP server under the name the plugin gives. Agents see them as `mcp__paseo__<name>`.
- Board drops its DOM that moves header buttons into the pane. The buttons stay in the Paseo header.

## Open Decisions

- None.

## References

- [packages/server/src/server/agent/agent-manager.ts](packages/server/src/server/agent/agent-manager.ts)
- [packages/server/src/server/agent/tools/paseo-tools.ts](packages/server/src/server/agent/tools/paseo-tools.ts)
- [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx)
- [packages/app/src/plugins/timeline/model.ts](packages/app/src/plugins/timeline/model.ts)
- [packages/app/src/components/sidebar/sidebar-model.tsx](packages/app/src/components/sidebar/sidebar-model.tsx)
- [public-docs/plugins/reference.md](public-docs/plugins/reference.md)
- [docs/protocol-compatibility.md](docs/protocol-compatibility.md)
