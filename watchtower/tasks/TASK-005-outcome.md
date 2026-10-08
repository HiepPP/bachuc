# TASK-005 Outcome

## Outcome

Status: TODO

Unblocked: Q-007 was answered on 2026-10-08, and the limit is now 104. The first pass stopped at 104 changed lines against the old limit of 60. The code is built, reviewed, and verified on the branch `auto/TASK-005-network-defaults` at `243160fa4`, pushed and not merged yet.

Changed (on `auto/TASK-005-network-defaults` only):

- [packages/server/src/server/config.ts](packages/server/src/server/config.ts): no default relay endpoint and no default app base URL. Relay TLS is off unless `PASEO_RELAY_USE_TLS` or `daemon.relay.useTls` turns it on. The old default was on only for `relay.paseo.sh:443`.
- [packages/server/src/server/persisted-config.ts](packages/server/src/server/persisted-config.ts): a new home gets no CORS origin and no `app.baseUrl`.
- [packages/server/src/server/relay-runtime.ts](packages/server/src/server/relay-runtime.ts): the relay transport never starts without an endpoint. Each start attempt, at startup or from a `relay.enabled` toggle, logs `Relay stays off: set PASEO_RELAY_ENDPOINT or daemon.relay.endpoint.` The daemon still starts. The relay setting itself stays as the owner set it.
- [packages/server/src/server/bootstrap.ts](packages/server/src/server/bootstrap.ts): no upstream fallbacks. The mutable config omits `app` when no base URL is set.
- [packages/server/src/server/pairing-offer.ts](packages/server/src/server/pairing-offer.ts): with relay on but no endpoint or no app base URL, the offer has relay on, no link, and no QR. The app then shows "pairing unavailable" instead of the relay consent screen again.
- [packages/cli/src/commands/daemon/pair.ts](packages/cli/src/commands/daemon/pair.ts): in that case `paseo daemon pair` prints `Relay pairing needs daemon.relay.endpoint and app.baseUrl in config.json.` (JSON code `RELAY_UNCONFIGURED`) and exits 1.
- [packages/cli/src/commands/hub/authority.ts](packages/cli/src/commands/hub/authority.ts): `DEFAULT_HUB_ORIGIN` is gone. With no command origin, `PASEO_HUB_URL`, or active stored login, hub commands fail with `HUB_ORIGIN_REQUIRED` ("Pass a Hub origin or set PASEO_HUB_URL.") before any network call.
- [packages/cli/src/commands/hub/init.ts](packages/cli/src/commands/hub/init.ts): the endpoint choice is gone. Init always asks for a Hub URL, prefilled with `PASEO_HUB_URL`, then the active login.
- [packages/cli/src/commands/hub/help.ts](packages/cli/src/commands/hub/help.ts) and [packages/cli/src/commands/onboard.ts](packages/cli/src/commands/onboard.ts): no upstream hub or web app URL.
- Tests: [packages/server/src/server/config-relay.test.ts](packages/server/src/server/config-relay.test.ts) (no default relay endpoint, TLS off), [packages/server/src/server/persisted-config.test.ts](packages/server/src/server/persisted-config.test.ts) (no default CORS origin or app base URL), [packages/server/src/server/relay-runtime.test.ts](packages/server/src/server/relay-runtime.test.ts) (no transport without an endpoint), [packages/cli/src/commands/hub/credentials.test.ts](packages/cli/src/commands/hub/credentials.test.ts) (no default hub origin), [packages/cli/src/commands/hub/commands.test.ts](packages/cli/src/commands/hub/commands.test.ts), [packages/cli/src/commands/hub/init-flow.test.ts](packages/cli/src/commands/hub/init-flow.test.ts) (no hosted choice, a new URL with an active login), and [packages/cli/src/commands/daemon/pair.test.ts](packages/cli/src/commands/daemon/pair.test.ts).

Contract:

- Config fields and env vars stay: `PASEO_RELAY_ENDPOINT`, `daemon.relay.endpoint`, `PASEO_APP_BASE_URL`, `app.baseUrl`, `PASEO_HUB_URL`. No protocol file changed. The old pairing link parser in [packages/protocol/src/connection-offer.ts](packages/protocol/src/connection-offer.ts) stays.
- Existing config files are not rewritten. The release home may still hold `app.baseUrl: https://app.paseo.sh`; the Handoff already tells the owner to remove it.
- Pairing link: the smaller change hides the link when no app base URL is set. The daemon web UI is off by default, so it gives no link base either.
- GitNexus `impact`: `generateLocalPairingOffer` HIGH (daemon pairing RPC, `paseo daemon pair`), `createInitialMutableDaemonConfig` HIGH (daemon startup), `resolveHubOrigin` HIGH (6 hub commands), `resolveRelayConfig`, `resolveStaticLoadConfigSettings`, `ensureLogin`, `createRelayRuntime`, and `outputPairingResult` LOW. No signature changed.

Verified:

- Baseline before edits: server config tests 70 passed, CLI hub tests 43 passed (10:57).
- `cd packages/server && npx vitest run` on `config-relay`, `persisted-config`, `relay-runtime`, `session/daemon/daemon-session`, and `daemon-config-store` tests -> 120 passed.
- `cd packages/cli && npx vitest run` on `hub/origin`, `hub/init-flow`, `hub/credentials`, `hub/commands`, `daemon/pair`, `output/pairing`, and `utils/daemon-target` tests -> 54 passed, after `npm run build:server`.
- `npm run typecheck` -> exit 0. `npm run lint -- <17 changed files>` -> 0 warnings, 0 errors. `npm run format:check:files -- <17 changed files>` -> exit 0.
- `rg -n "relay\.paseo\.sh|app\.paseo\.sh|hub\.paseo\.sh" packages/server/src packages/cli/src --glob '!**/*.test.*'` -> 4 matches, all explicit fixture values with relay off in the test harnesses `packages/server/src/server/test-utils/paseo-daemon.ts` and `packages/server/src/server/hub/test-utils/relationship-harness.ts`. No default and no runtime code matches. The Verify glob does not exclude `test-utils`.
- UNVERIFIED (autonomous run): the live check. The live daemon was not running, and the run does not start live.
- Review: one `reviewer` pass found 4 issues. Fixed: the relay consent loop in the app and `paseo daemon pair`, `hub init` ignoring a new URL while a login is active, and the relay status before the runtime starts. Not fixed, outside this TASK: `nix/module.nix` line 92 still defaults to relay mode `hosted`, which now means relay off. An explicit `relay.paseo.sh:443` without `useTls` now uses `ws://`. `docs/hub.md`, `public-docs/cli.md`, `public-docs/hub/daemons.md`, and `public-docs/hub/api.md` still name the `hub.paseo.sh` fallback.

Anti-goal:

- Before changes: `git diff --numstat main -- packages/server/src packages/cli/src ':(exclude)**/*.test.ts'` -> 0, 10:57.
- After the server edits: same command -> 53, 11:00.
- Before review: same command -> 85, 11:06.
- Final, after the review fixes: same command -> 104, 11:16. The `hub init` prompt is 39 of it.
- Result: FAIL (104, limit 60). The spec scope needs about 80 lines at least, so the limit cannot pass without cutting a Boundary.

Lessons:

- CLI tests import `@getpaseo/server` from its `dist`. After a server change, run `npm run build:server` before CLI tests, or they test the old code and pass.
- The app shows the relay consent screen whenever a pairing offer says relay is off. A relay that is on but cannot pair must report relay on with an empty link.
