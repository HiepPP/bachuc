# TASK-105 GLM quota on the glm provider

Group: B (quota fetcher files only; follows TASK-104 by ordering alone)
Class: code

## Brief

Goal: Show Z.AI plan usage on the `glm` provider, using the same key as the glm agent.

Change: the quota fetcher has id `zai` and matches no provider -> it has id `glm` and reads the key through the TASK-104 resolver.

Boundaries:

- The usage tooltip finds a card by `usage.providerId` equal to the agent provider id, case-insensitive. See [packages/app/src/provider-usage/tooltip-section.tsx](packages/app/src/provider-usage/tooltip-section.tsx) line 13. So the fetcher id must be `glm`.
- Keep the Z.AI usage endpoint and the response schema as they are. This TASK changes the id, label, and key source only.
- The usage e2e spec already uses a mock id `glm` in [packages/app/e2e/browser/provider-usage-settings.spec.ts](packages/app/e2e/browser/provider-usage-settings.spec.ts). Do not change it.

How:

- Change `providerId` to `glm` and `displayName` to `GLM` in [packages/server/src/services/quota-fetcher/providers/zai.ts](packages/server/src/services/quota-fetcher/providers/zai.ts).
- Read the key with the resolver from [packages/server/src/server/agent/providers/glm/runtime.ts](packages/server/src/server/agent/providers/glm/runtime.ts).
- Change the manifest row in [packages/server/src/services/quota-fetcher/manifest.ts](packages/server/src/services/quota-fetcher/manifest.ts) to `glm`.
- Update the `zai` case in [packages/server/src/services/quota-fetcher/service.test.ts](packages/server/src/services/quota-fetcher/service.test.ts) to `glm`.

Files:

- [packages/server/src/services/quota-fetcher/providers/zai.ts](packages/server/src/services/quota-fetcher/providers/zai.ts) (id, label, key source)
- [packages/server/src/services/quota-fetcher/manifest.ts](packages/server/src/services/quota-fetcher/manifest.ts) (row id)
- [packages/server/src/services/quota-fetcher/service.test.ts](packages/server/src/services/quota-fetcher/service.test.ts) (test id)

Expected result:

- The usage list has a `glm` entry, and no `zai` entry.
- With no key, the `glm` entry is unavailable, as the `zai` entry was before.
- A glm agent tooltip shows the GLM usage card.

Anti-goal: the number of quota fetchers in the manifest stays the same as before the TASK; drift gauge; read from `rg -c "providerId:" packages/server/src/services/quota-fetcher/manifest.ts` at completion.

## Verify

- `npx vitest run packages/server/src/services/quota-fetcher/service.test.ts --bail=1` -> pass, and the case finds `glm`.
- `rg -n '"zai"' packages/server/src/services/quota-fetcher` -> no match.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/server/src/services/quota-fetcher/providers/zai.ts packages/server/src/services/quota-fetcher/manifest.ts` -> exit 0.
- In the live app, open the provider usage settings -> a GLM card shows. Needs a human or a browser check.
- Anti-goal: `rg -c "providerId:" packages/server/src/services/quota-fetcher/manifest.ts` -> the same count as the baseline in the outcome sidecar.
