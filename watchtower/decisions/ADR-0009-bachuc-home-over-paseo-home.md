# ADR-0009 Bachuc home over paseo home

Status: accepted
Date: 2026-10-08
Supersedes: ADR-0004
Scope: `packages/server/src/server/paseo-home.ts`, `packages/cli/src/commands/hub/credentials.ts`, `packages/desktop/src/main.ts`, `scripts/paseo-release.sh`, `scripts/dev-home.sh`, `packages/app/src/i18n/**`, `packages/app/app.config.js`, `packages/app/public/**`, `packages/app/assets/**`, `packages/desktop/electron-builder.yml`, `packages/desktop/assets/**`, `packages/cli/**`, `packages/plugin/**`

## Context

- [ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md) renamed only what users see and kept the release home `~/.paseo` and the userData folder `Paseo`.
- After the rebrand, the owner wanted the settings in folders named for Bachuc, cloned 1:1 from the old ones (2026-10-08).
- The release home held 1.1 GB on 2026-10-08, mostly `models/` (984 MB). 639 agent records held the absolute path `/Users/hiep/.paseo/config.json` in MCP server env values.
- The plugins in `hiep-plugins/plugins/` import `@getpaseo/plugin` and set `requirements.paseo`. Agents and skills call the `paseo` CLI.

## Decision

- The default daemon home is `~/.bachuc`, and the release app's userData folder is `~/Library/Application Support/Bachuc`.
- `scripts/paseo-release.sh` clones `~/.paseo` and the `Paseo` userData folder to the new names once, after the old daemon stops. It rewrites `~/.paseo/` paths in the cloned JSON files, and it keeps the originals as a backup.
- Everything else from ADR-0004 stays: rename what users see, and keep the `paseo` CLI, `PASEO_*` env vars, the `@getpaseo/*` scope, the plugin API and `requirements.paseo`, RPC names, the `paseo://` scheme, code symbols, `.dev/paseo-home`, and `~/.paseo-dev`. Mobile stays out of scope.
- The owner chose this on 2026-10-08.

## Consequences

- A bare `paseo` command in any shell targets `~/.bachuc`. A shell that still exports `PASEO_HOME=~/.paseo` targets the old home until it restarts.
- A rollback of the first install moves the clone to the Trash, so the next run clones again.
- In-app browser logins may reset, because Electron ties its cookie key to the app name.

## Options Considered

- Keep `~/.paseo` and the `Paseo` userData folder (ADR-0004): rejected. The owner wants Bachuc folders.
- Move the old folders instead of a clone: rejected. A clone keeps the old folders for a rollback, and APFS clones take no extra space.
- Rename the CLI, env vars, npm scope, and plugin API too: rejected in ADR-0004 for the same reasons, which still hold.

## Revisit If

- The owner deletes the `~/.paseo` backup, so the clone step has nothing to do and can go.
- The owner publishes Bachuc for other users.
