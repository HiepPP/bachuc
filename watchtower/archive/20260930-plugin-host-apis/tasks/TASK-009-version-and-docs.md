# TASK-009 Fork version and plugin API docs

Group: C (version and docs files)
Class: code

## Brief

Goal: Give the fork a version that plugins can require, and document the new APIs.

Change: repo version `0.10.0-beta.1` -> `0.10.2-beta.900`; docs list the new APIs.

Boundaries:

- Use the repo version sync script. Do not edit versions by hand.
- Document every new API in [public-docs/plugins/reference.md](public-docs/plugins/reference.md) and every new `add*` method in [public-docs/plugins/migration.md](public-docs/plugins/migration.md).
- Update [docs/plugins.md](docs/plugins.md) and [skills/paseo-plugin/SKILL.md](skills/paseo-plugin/SKILL.md) only where they list APIs.

How:

- Set the root version and run `node scripts/sync-workspace-versions.mjs`.
- Write the docs.

Files:

- [package.json](package.json) and the workspace `package.json` files the sync script changes, plus [package-lock.json](package-lock.json)
- [public-docs/plugins/reference.md](public-docs/plugins/reference.md) (API reference)
- [public-docs/plugins/migration.md](public-docs/plugins/migration.md) (method table)
- [docs/plugins.md](docs/plugins.md) (API list)
- [skills/paseo-plugin/SKILL.md](skills/paseo-plugin/SKILL.md) (API list)

Expected result:

- Every workspace package reports `0.10.2-beta.900`.
- The docs describe each new API with a short example.

Anti-goal: Docs test coverage; tripwire; limit [packages/plugin/src/migration-doc.test.ts](packages/plugin/src/migration-doc.test.ts) passes; read at completion.

## Verify

- `node -p "require('./packages/server/package.json').version"` -> `0.10.2-beta.900`.
- `npx vitest run packages/plugin/src/migration-doc.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- Anti-goal: migration-doc test passes.
