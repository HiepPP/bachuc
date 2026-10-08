# ADR-0004 Display brand over full rename

Status: superseded by ADR-0009
Date: 2026-10-07
Supersedes: -
Scope: `packages/app/src/i18n/**`, `packages/app/app.config.js`, `packages/app/public/**`, `packages/app/assets/**`, `packages/desktop/electron-builder.yml`, `packages/desktop/src/main.ts`, `packages/desktop/assets/**`, `scripts/paseo-release.sh`, `packages/cli/**`, `packages/plugin/**`

## Context

- The fork renames its product from Paseo to Bachuc. The owner chose this name after Ba Chúc, the owner's home place in An Giang.
- The fork merges upstream Paseo. The root [CLAUDE.md](CLAUDE.md) asks for thin host changes, so merges stay cheap.
- The plugins in `hiep-plugins/plugins/` import `@getpaseo/plugin` and set `requirements.paseo`.
- Agents and skills call the `paseo` CLI. On 2026-10-07, `~/.local/bin/paseo` linked to `/Applications/Paseo Fork.app/Contents/Resources/bin/paseo`.
- Release data lives in `~/.paseo` and `~/Library/Application Support/Paseo`.
- Users see the product name, icons, window titles, and links. They do not see npm scopes, env vars, or home paths in normal use.
- The owner builds only the macOS desktop app and the web UI that the daemon serves. The owner does not build the iOS or Android app.

## Decision

- Rename only what users see: the app name, window and menu titles, UI text, icons, the desktop bundle ID, installer names, and links.
- Keep the `paseo` CLI command, `PASEO_*` env vars, `~/.paseo` and `.dev/paseo-home`, the `Paseo` userData folder, the `@getpaseo/*` npm scope, the plugin API and `requirements.paseo`, RPC names, the `paseo://` URL scheme, and code symbols such as `PaseoLogo`.
- Rebrand the macOS desktop app and the web UI only. The iOS and Android app keep `sh.paseo` and are not rebuilt.
- The owner chose this on 2026-10-07.

## Consequences

- The UI says Bachuc, but the terminal command stays `paseo`.
- Upstream merges touch few files.
- A mobile build would still show Paseo.

## Options Considered

- Full rename of CLI, env vars, home paths, and npm scope: rejected. It breaks every plugin, skill, and data path above, and it conflicts with most upstream merges.
- Mobile rebrand: rejected. It needs Apple Developer, Google Play, Firebase, and an EAS project for a new bundle ID, and the owner does not build mobile.

## Revisit If

- The owner publishes Bachuc for other users.
- The owner stops merging upstream Paseo.
- The owner starts to build the iOS or Android app.
