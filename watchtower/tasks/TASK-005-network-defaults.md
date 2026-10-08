# TASK-005 Remove upstream network defaults

Group: E (`packages/server/src/server/` config and pairing files, `packages/cli/src/commands/hub/`, `packages/cli/src/commands/onboard.ts`)
Class: risky

## Brief

Goal: The daemon and the CLI have no default that points at an upstream hosted service. Each service needs an explicit setting.

Change: defaults `relay.paseo.sh:443`, `https://app.paseo.sh`, and `https://hub.paseo.sh` -> no defaults.

Boundaries:

- Follows [ADR-0005](watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md).
- Keep the config fields and env vars (`PASEO_RELAY_ENDPOINT`, `PASEO_HUB_URL`, `app.baseUrl`), so the owner can set own services later.
- Relay stays off by default. If relay is on and no endpoint is set, the daemon logs one clear warning and keeps relay off. It still starts.
- Remove `https://app.paseo.sh` from the default CORS `allowedOrigins`. Then the upstream hosted web app cannot call the daemon.
- Pairing links: with no app base URL, never build an `app.paseo.sh` link. After impact analysis, pick the smaller change: use the daemon's own web UI URL, or hide the link and keep direct pairing.
- Keep the parser for old `app.paseo.sh` pairing links in [packages/protocol/src/connection-offer.ts](packages/protocol/src/connection-offer.ts). The protocol stays backward-compatible ([docs/protocol-compatibility.md](docs/protocol-compatibility.md)).
- The code does not rewrite existing config files. The release home still holds `app.baseUrl: https://app.paseo.sh` (checked 2026-10-07). The Handoff tells the owner to remove it.
- Hub: `paseo hub` commands need `--hub` or `PASEO_HUB_URL`. `paseo hub init` drops the `hub.paseo.sh` choice.
- `paseo onboard` stops listing `https://app.paseo.sh`.
- Never restart or stop the release daemon on port 6767. Test on the live daemon only.

How:

- Run GitNexus `impact` on `resolveRelayConfig`, `DEFAULT_PERSISTED_CONFIG`, `DEFAULT_HUB_ORIGIN`, and the app base URL fallbacks in [packages/server/src/server/bootstrap.ts](packages/server/src/server/bootstrap.ts) lines 543 and 668 to 673.
- Record the pass and fail counts of the test files in Verify before you edit.
- Remove the defaults. Add the relay warning. Change the pairing link per the boundary above.
- Update tests that expect the old defaults. Add one test per new rule: no default relay endpoint, no default CORS origin, and no default hub origin.

Files:

- [packages/server/src/server/config.ts](packages/server/src/server/config.ts) (relay endpoint and app base URL defaults)
- [packages/server/src/server/persisted-config.ts](packages/server/src/server/persisted-config.ts) (default CORS origin and app base URL)
- [packages/server/src/server/bootstrap.ts](packages/server/src/server/bootstrap.ts) (app base URL fallbacks)
- [packages/server/src/server/pairing-offer.ts](packages/server/src/server/pairing-offer.ts) (pairing link base)
- [packages/cli/src/commands/hub/authority.ts](packages/cli/src/commands/hub/authority.ts), [packages/cli/src/commands/hub/init.ts](packages/cli/src/commands/hub/init.ts), [packages/cli/src/commands/hub/help.ts](packages/cli/src/commands/hub/help.ts) (hub origin)
- [packages/cli/src/commands/onboard.ts](packages/cli/src/commands/onboard.ts) (onboarding text)
- Matching tests: [packages/server/src/server/config-relay.test.ts](packages/server/src/server/config-relay.test.ts), [packages/server/src/server/persisted-config.test.ts](packages/server/src/server/persisted-config.test.ts), [packages/cli/src/commands/hub/origin.test.ts](packages/cli/src/commands/hub/origin.test.ts), [packages/cli/src/commands/hub/init-flow.test.ts](packages/cli/src/commands/hub/init-flow.test.ts)

Expected result:

- A new empty home starts with relay off, no CORS origin, and no app base URL from `paseo.sh`.
- `PASEO_RELAY_ENABLED=true` without an endpoint logs the warning, and the daemon starts with relay off.
- A hub command without `--hub` and `PASEO_HUB_URL` prints how to set the origin, and makes no network call.
- The live app still connects to the live daemon on `127.0.0.1:6768`.

Anti-goal: changed lines in non-test upstream files stay at or below 60; drift gauge; read with the numstat command in Verify before changes (0), after the server edits, and at completion.

## Verify

- `cd packages/server && npx vitest run src/server/config-relay.test.ts src/server/persisted-config.test.ts --bail=1` -> pass.
- `cd packages/cli && npx vitest run src/commands/hub/origin.test.ts src/commands/hub/init-flow.test.ts --bail=1` -> pass.
- `rg -n "relay\.paseo\.sh|app\.paseo\.sh|hub\.paseo\.sh" packages/server/src packages/cli/src --glob '!**/*.test.*'` -> each match is a comment.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed files>` -> exit 0.
- Live check: restart `npm run dev` (live, port 6768 only). The live app connects, and the daemon log shows relay off.
- Anti-goal: `git diff --numstat main -- packages/server/src packages/cli/src ':(exclude)**/*.test.ts'` -> the sum of added and removed lines is 60 or less.
