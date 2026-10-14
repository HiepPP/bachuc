# TASK-107 Outcome

Lead re-check, wave 3: format check passes, `git diff --check` is clean, `/logout` has no match, and the base URL count is 1 (limit 3). The override default is safe: the registry uses `agents.providers.glm.models` in place of the built-in list when it is set. Commit `ecc4d67bc`.

## Status

DONE.

## Changed

- `docs/custom-providers.md`: rewrote the Z.AI section for the built-in `glm` provider and removed the `zai` sample from the full example.
- `public-docs/custom-providers.md`: the Z.AI section now points to the built-in provider and links to the full setup.
- `public-docs/supported-providers.md`: GLM added to Native support. The GLM Agent ACP line stays.

## Contract

- Docs say: set `ZAI_API_KEY`, pick GLM. Key order is provider config `ANTHROPIC_AUTH_TOKEN`, `ZAI_API_KEY`, `GLM_API_KEY`.
- Config dir `$PASEO_HOME/providers/glm/claude-config`; `~/.claude` hooks, plugins, and history do not apply.
- Models: `glm-5.3-flash` (default), `glm-5.3`, `glm-5-turbo`, `glm-4.7`. `WebSearch` is always blocked.
- The usage card reads only the daemon env key (Q-042).
- Override example uses `agents.providers.glm` with `env` only. The generic `extends: "claude"` WebSearch gotcha stays.

## Verified

Measured 2026-10-14.

- `rg -n "/logout" docs/custom-providers.md public-docs/custom-providers.md` -> no match.
- `rg -n "ZAI_API_KEY" docs/custom-providers.md` -> 3 matches in the Z.AI section.
- `npm run format:files -- <3 files>` -> exit 0; `git diff --check` -> no output.
- `rg -c "api.z.ai/api/anthropic" docs public-docs` -> total 1.

## Anti-goal

Drift gauge: `rg -c "api.z.ai/api/anthropic" docs public-docs`.

| Checkpoint     | Value | Source        | Time (UTC)           |
| -------------- | ----- | ------------- | -------------------- |
| Before changes | 3     | command above | 2026-10-14T07:44Z    |
| At completion  | 1     | command above | 2026-10-14T07:48:58Z |

Result: PASS (limit 3).

## Lessons

- The override example avoids `models`/`additionalModels` for glm, because no outcome documents how they merge with the static list.
- The anchor `#zai-glm-coding-plan` in the public doc is assumed from the heading slug; not rendered.
