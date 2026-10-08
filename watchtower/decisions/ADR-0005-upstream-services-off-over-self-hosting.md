# ADR-0005 Upstream services off over self-hosting

Status: accepted
Date: 2026-10-07
Supersedes: -
Scope: `packages/server/src/server/config.ts`, `packages/server/src/server/persisted-config.ts`, `packages/server/src/server/bootstrap.ts`, `packages/server/src/server/pairing-offer.ts`, `packages/cli/src/commands/hub/**`, `packages/cli/src/commands/onboard.ts`, `packages/desktop/src/features/auto-updater.ts`, `packages/desktop/electron-builder.yml`, `packages/app/src/desktop/updates/**`, `packages/app/app.config.js`

## Context

- Apache 2.0 gives rights to the code. It gives no right to use upstream hosted services.
- Upstream services in the defaults on 2026-10-07:
  - Relay `relay.paseo.sh:443` in [packages/server/src/server/config.ts](packages/server/src/server/config.ts) line 39.
  - Web app `https://app.paseo.sh` in [packages/server/src/server/config.ts](packages/server/src/server/config.ts) line 40 and [packages/server/src/server/persisted-config.ts](packages/server/src/server/persisted-config.ts) lines 348 and 355, also as a CORS origin.
  - Hub `https://hub.paseo.sh` in [packages/cli/src/commands/hub/authority.ts](packages/cli/src/commands/hub/authority.ts) line 16.
  - Desktop updates from GitHub `getpaseo/paseo` in [packages/desktop/electron-builder.yml](packages/desktop/electron-builder.yml) lines 34 to 37.
  - EAS owner `getpaseo` and its project ID in [packages/app/app.config.js](packages/app/app.config.js) lines 193 and 196.
- The release home had `daemon.relay.enabled: false` on 2026-10-07.
- The fork version is `1.0.0-hiep`. An upstream release such as `1.0.0` is newer, so the updater could install upstream Paseo over Bachuc.
- The owner reaches the host directly, over LAN or Tailscale.

## Decision

- Remove the upstream defaults for relay, web app, CORS origin, and hub. Each needs an explicit setting.
- Turn off desktop auto-update. New versions come only from `scripts/paseo-release.sh`.
- Remove the EAS owner and project ID.
- The owner chose this on 2026-10-07.

## Consequences

- Remote access works only through a direct connection, unless the owner sets an own relay endpoint.
- Pairing links no longer point to `app.paseo.sh`.
- The app shows no update offers.

## Options Considered

- Self-host the relay on the owner's Cloudflare account: rejected. It needs a domain, an account, and upkeep, and the owner connects directly.
- Keep the upstream services: rejected. Bachuc would depend on upstream services and their terms, and the updater could replace the fork.

## Revisit If

- The owner needs phone access without LAN or Tailscale.
- The owner publishes Bachuc for other users.
