# Plan Context

## Shared Context

- The provider id is `glm`. The label is `GLM`. It runs the Claude Code client against the Z.AI Anthropic endpoint `https://api.z.ai/api/anthropic`.
- The key comes from the daemon process env: `ZAI_API_KEY` first, then `GLM_API_KEY`. A provider config `env` value for `ANTHROPIC_AUTH_TOKEN` wins over both. Desktop gets the shell env through [packages/desktop/src/login-shell-env.ts](packages/desktop/src/login-shell-env.ts), and the owner sets `ZAI_API_KEY` in `~/.zshrc`.
- The glm provider never reads or writes `~/.claude`. It sets `CLAUDE_CONFIG_DIR` to a folder inside `PASEO_HOME`, for example `$PASEO_HOME/providers/glm/claude-config`. Claude Code creates its own `.claude.json` there on first run.
- The model list is `glm-5.3-flash` (default), `glm-5.3`, `glm-5-turbo`, and `glm-4.7`. Set `ANTHROPIC_DEFAULT_OPUS_MODEL`, `ANTHROPIC_DEFAULT_SONNET_MODEL`, `ANTHROPIC_DEFAULT_HAIKU_MODEL`, and `ANTHROPIC_SMALL_FAST_MODEL` to the chosen model, so subagents and small tasks stay on GLM.
- `WebSearch` is an Anthropic server-side tool. Z.AI does not support it, so the glm provider always adds it to `disallowedTools`.
- Test on live (port 6768) only. Never restart the release daemon on port 6767. Run only the test files a TASK changes.

## Decisions

- Built-in glm provider on the Claude client, with its own Claude config dir. Reason: [ADR-0030](watchtower/decisions/ADR-0030-built-in-glm-on-claude-client-over-acp-and-config.md).
- Default model is `glm-5.3-flash`. The owner chose it on 2026-10-14 because it passed the live probe and uses less Coding Plan quota.
- The release config keeps its `glm-acp-agent` provider. The owner turns it off by hand after trying glm.

## Open Decisions

- None.

## References

- Live probe on 2026-10-14: a custom `zai` provider with `extends: "claude"`, `CLAUDE_CONFIG_DIR=/tmp/zai-claude`, and model `glm-5.3-flash` passed an image read, 3 `thinking` blocks, one general-purpose subagent, and a tool list without `WebSearch`. All 8 assistant messages, subagent included, had `message.model` `glm-5.3-flash`.
- Probe command shape: `npm run cli -- run --provider glm --mode bypassPermissions --thinking high --cwd /tmp/glm-native-probe --image /tmp/glm-native-probe/probe.png "<prompt>"`, then `npm run cli -- logs <id>`.
- [docs/custom-providers.md](docs/custom-providers.md) holds the current Z.AI setup and the `extends: "claude"` WebSearch gotcha.
- [docs/providers.md](docs/providers.md) holds the checklist for a new provider.
