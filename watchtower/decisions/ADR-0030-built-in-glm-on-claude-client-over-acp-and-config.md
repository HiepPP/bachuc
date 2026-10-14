# ADR-0030 Built-in GLM on the Claude client over the ACP agent and a config entry

Status: accepted
Date: 2026-10-14
Supersedes: -
Scope: `packages/protocol/src/provider-manifest.ts`, `packages/server/src/server/agent/provider-registry.ts`, `packages/server/src/server/agent/providers/glm/**`, `packages/server/src/services/quota-fetcher/**`, `packages/app/src/components/provider-icons.ts`

## Context

- GLM ran in two ways: a hand-written `extends: "claude"` config entry against `https://api.z.ai/api/anthropic`, or the third-party ACP agent `glm-acp-agent`.
- A live probe on 2026-10-14 with the Claude path and model `glm-5.3-flash` passed an image read, thinking blocks, one subagent, and a tool list without `WebSearch`.
- The generic ACP client turns off rewind, has no Hub contract, and ignores `disallowedTools`. See [packages/server/src/server/agent/provider-registry.ts](packages/server/src/server/agent/provider-registry.ts).
- A Claude path that shares `~/.claude` runs the owner's hooks and 22 plugins in GLM sessions, and mixes GLM sessions into `claude --resume`.

## Decision

- Add a built-in `glm` provider that wraps the Claude client, with Z.AI defaults, `WebSearch` blocked, and its own `CLAUDE_CONFIG_DIR` under `PASEO_HOME`.
- The key comes from `ZAI_API_KEY`, then `GLM_API_KEY`. The Z.AI quota fetcher moves to the `glm` provider id.
- The owner chose this on 2026-10-14, after the live probe.

## Consequences

- GLM works with no config entry when `ZAI_API_KEY` is set.
- GLM sessions do not get the owner's `~/.claude` hooks, plugins, skills, or `CLAUDE.md`. The owner must copy them into the glm config dir to use them.
- Claude-only app code that checks `provider === "claude"` must also accept `glm`.

## Options Considered

- `glm-acp-agent` as the main path: rejected. Generic ACP has no rewind and no Hub contract, and the agent is a third-party npm package.
- A documented `extends: "claude"` config entry only: rejected. It needs the key pasted into config, and the quota card never matches it.
- The Claude path with the shared `~/.claude`: rejected. It runs the owner's hooks and plugins on GLM quota and mixes session history.

## Revisit If

- Z.AI drops or breaks its Anthropic-compatible endpoint, or a probe shows images, thinking, or subagents fail on it.
- The owner wants `~/.claude` hooks or plugins in GLM sessions by default.
- The generic ACP client gains rewind and a Hub contract.
