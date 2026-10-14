# TASK-107 GLM docs

Group: D (docs only; follows TASK-104 and TASK-105 by ordering alone)
Class: docs

## Brief

Goal: Make the docs say that GLM is a built-in provider, and how to set its key.

Change: the docs teach a hand-written `zai` config entry -> they point to the built-in `glm` provider.

Boundaries:

- Follow "Writing docs" and "Doc voice" in [CLAUDE.md](CLAUDE.md). Integrate into the doc that owns the subject. Do not append.
- One fact, one doc. The built-in glm setup lives in [docs/custom-providers.md](docs/custom-providers.md). Other docs link to it.
- Keep the generic `extends: "claude"` WebSearch gotcha, because other proxies still need it.

How:

- Rewrite the "Z.AI (Zhipu) coding plan" section in [docs/custom-providers.md](docs/custom-providers.md). Say: set `ZAI_API_KEY`, pick provider GLM. Say that it uses its own Claude config dir, so `~/.claude` hooks, plugins, and history do not apply. Drop the old `zai` config sample from the full example.
- Add GLM to the built-in list in [public-docs/supported-providers.md](public-docs/supported-providers.md). Keep the GLM Agent ACP line.
- Update the Z.AI part of [public-docs/custom-providers.md](public-docs/custom-providers.md) to point to the built-in provider.

Files:

- [docs/custom-providers.md](docs/custom-providers.md) (rewrite the Z.AI section and the full example)
- [public-docs/supported-providers.md](public-docs/supported-providers.md) (list GLM as built-in)
- [public-docs/custom-providers.md](public-docs/custom-providers.md) (point to the built-in provider)

Expected result:

- A reader learns to set `ZAI_API_KEY` and pick GLM, with no config entry.
- No doc tells the reader to run `/logout` in Claude Code to use GLM.

Anti-goal: copies of the Z.AI base URL `api.z.ai/api/anthropic` across docs stay at 3 or fewer; drift gauge; read from `rg -c "api.z.ai/api/anthropic" docs public-docs` at completion.

## Verify

- `rg -n "/logout" docs/custom-providers.md public-docs/custom-providers.md` -> no match in a GLM or Z.AI section.
- `rg -n "ZAI_API_KEY" docs/custom-providers.md` -> at least one match in the Z.AI section.
- `npm run format:files -- docs/custom-providers.md public-docs/supported-providers.md public-docs/custom-providers.md` -> exit 0, then `git diff --check` -> no output.
- Anti-goal: `rg -c "api.z.ai/api/anthropic" docs public-docs` -> total of 3 or fewer.
