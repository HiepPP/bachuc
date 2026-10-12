# TASK-075 Overview as the default detail level

Group: D (writes the settings storage files; standalone)
Class: mechanical

## Brief

Goal: Make `overview` the default tool call detail level, so new installs get the turn step list on desktop and web (Q-027 default).

Change: default `toolCallDetailLevel` `detailed` -> `overview`.

Boundaries:

- Stored values win. A stored `detailed` stays `detailed`; the old `concise` value still maps to `overview`.
- Release and live keep their stored setting. The owner switches it in Settings.

How:

- Change the default in [packages/app/src/hooks/use-settings/storage.ts](packages/app/src/hooks/use-settings/storage.ts), including the fallback when nothing is stored.
- Update or add cases in [packages/app/src/hooks/use-settings/storage.test.ts](packages/app/src/hooks/use-settings/storage.test.ts).

Files:

- [packages/app/src/hooks/use-settings/storage.ts](packages/app/src/hooks/use-settings/storage.ts) (default value)
- [packages/app/src/hooks/use-settings/storage.test.ts](packages/app/src/hooks/use-settings/storage.test.ts) (default cases)

Expected result:

- Empty storage -> `overview`.
- Cases: stored `detailed` -> `detailed`; stored `concise` -> `overview`; stored `compactToolCalls: false` with no level -> follow the existing migration rule and say which value it gives in the outcome.

Anti-goal: stored user values never change on read; 0 test cases where a stored value maps to a different level than before, except the empty default; tripwire; read from the storage test before and after the change.

## Verify

- `cd packages/app && npx vitest run src/hooks/use-settings/storage.test.ts --bail=1` -> pass with the new default case.
- `npm run typecheck` -> exit 0.
- `npm run lint -- packages/app/src/hooks/use-settings/storage.ts packages/app/src/hooks/use-settings/storage.test.ts` -> exit 0.
- Anti-goal: `git diff -- packages/app/src/hooks/use-settings/storage.test.ts` -> only the empty-default expectation changes.
