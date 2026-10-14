# TASK-104 Built-in glm provider

Group: A (protocol manifest, provider registry, persisted config, new glm runtime module; shared file with no other TASK)
Class: risky

## Brief

Goal: Add a built-in `glm` provider that runs the Claude client against Z.AI. It works with no config entry when `ZAI_API_KEY` is set.

Change: GLM needs a hand-written `extends: "claude"` config entry -> `glm` is a built-in provider with Z.AI defaults and its own Claude config dir.

Boundaries:

- Follows [ADR-0030](watchtower/decisions/ADR-0030-built-in-glm-on-claude-client-over-acp-and-config.md): wrap the Claude client, do not use the ACP agent, while the live probe shows the Claude path keeps images, thinking, and subagents.
- Follows [ADR-0017](watchtower/decisions/ADR-0017-native-host-features-over-thin-host-changes.md): build it in the host, because a provider needs registry internals.
- Never read or write `~/.claude`. Never change the runtime settings of the built-in `claude` provider.
- Keep the protocol contract. The provider id is a string, so no wire schema change is needed. Read [docs/protocol-compatibility.md](docs/protocol-compatibility.md) before touching `packages/protocol`.
- A user `agents.providers.glm` entry still overrides label, env, models, and `enabled`, like other built-ins.
- Risk: [packages/server/src/server/agent/providers/claude/agent.ts](packages/server/src/server/agent/providers/claude/agent.ts) line 1716 throws when `config.provider !== "claude"`, and line 989 checks `metadata.provider === "claude"`. The derived `zai` probe worked, so the provider wrapper already maps ids. Confirm how `createResolvedProviderClient` in [packages/server/src/server/agent/provider-registry.ts](packages/server/src/server/agent/provider-registry.ts) does it, and reuse that path.

How:

- Run `impact` on `buildProviderRegistry`, `PROVIDER_CLIENT_FACTORIES`, and `AGENT_PROVIDER_DEFINITIONS` first. Report the risk.
- Add a `glm` entry to `AGENT_PROVIDER_DEFINITIONS` with label `GLM`, the Claude modes, and default mode `auto`. Keep voice off.
- Add `glm` to the `BUILTIN_PROVIDER_IDS` list in [packages/server/src/server/persisted-config.ts](packages/server/src/server/persisted-config.ts).
- Add a glm runtime module. It resolves the key (`ANTHROPIC_AUTH_TOKEN` from provider env, else `ZAI_API_KEY`, else `GLM_API_KEY`), the config dir under `PASEO_HOME`, the base URL, `API_TIMEOUT_MS`, and the model env vars. Export the key resolver for TASK-105.
- Add a `glm` factory that builds a `ClaudeAgentClient` with the glm runtime settings merged under the user override. Always add `WebSearch` to `disallowedTools`.
- Give `glm` the Claude contract in `PROVIDER_CONTRACTS`, so Hub and tool policy match Claude.
- Set the static model list from [watchtower/CONTEXT.md](watchtower/CONTEXT.md), with `glm-5.3-flash` as default. Give each model the thinking options the Claude client offers for a custom model id.
- When no key resolves, report the provider as unavailable with a short reason that names `ZAI_API_KEY`.
- Add tests to [packages/server/src/server/agent/provider-registry.test.ts](packages/server/src/server/agent/provider-registry.test.ts).

Files:

- [packages/protocol/src/provider-manifest.ts](packages/protocol/src/provider-manifest.ts) (add the `glm` definition)
- [packages/server/src/server/persisted-config.ts](packages/server/src/server/persisted-config.ts) (add `glm` to the built-in id list)
- [packages/server/src/server/agent/provider-registry.ts](packages/server/src/server/agent/provider-registry.ts) (factory, contract, runtime defaults)
- [packages/server/src/server/agent/providers/glm/runtime.ts](packages/server/src/server/agent/providers/glm/runtime.ts) (new: key resolver, config dir, env defaults, model list)
- [packages/server/src/server/agent/provider-registry.test.ts](packages/server/src/server/agent/provider-registry.test.ts) (new cases)

Expected result:

- With `ZAI_API_KEY` set, the provider list has `glm`, available, with 4 models and `glm-5.3-flash` as default.
- With no key, `glm` is listed as unavailable with a reason that names `ZAI_API_KEY`.
- A glm session env has `CLAUDE_CONFIG_DIR` under `PASEO_HOME`, `ANTHROPIC_BASE_URL` set to Z.AI, and `ANTHROPIC_AUTH_TOKEN` set to the key.
- A provider config env `ANTHROPIC_AUTH_TOKEN` wins over `ZAI_API_KEY`.
- `disallowedTools` for glm always holds `WebSearch`, also when the user override adds other tools.
- The `claude` provider runtime settings are the same as before.

Anti-goal: Claude Code sessions written under `~/.claude` by the glm provider stay at 0; tripwire; read from `ls ~/.claude/projects | rg -c glm-native-probe` before changes, after the first live probe, and at completion.

## Verify

- `npx vitest run packages/server/src/server/agent/provider-registry.test.ts --bail=1` -> pass, with new cases for key order, missing key, config dir, `WebSearch`, and an unchanged `claude` provider.
- `npm run build:server` then `npm run typecheck` -> exit 0.
- `npm run lint -- packages/protocol/src/provider-manifest.ts packages/server/src/server/agent/provider-registry.ts packages/server/src/server/agent/providers/glm/runtime.ts packages/server/src/server/persisted-config.ts` -> exit 0.
- Restart the live daemon (`npm run dev`), then `npm run cli -- provider ls` -> `glm` is `available`. `npm run cli -- provider models glm` -> 4 models.
- Live probe from [watchtower/CONTEXT.md](watchtower/CONTEXT.md) with `--provider glm` -> the image text is read, and the transcript under `$PASEO_HOME/providers/glm/claude-config/projects` has `thinking` blocks and only GLM `message.model` values.
- Anti-goal: `ls ~/.claude/projects | rg -c glm-native-probe` -> no match at each read point.
