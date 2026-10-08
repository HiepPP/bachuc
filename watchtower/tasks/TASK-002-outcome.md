# TASK-002 Outcome

## Outcome

Status: DONE (auto: manual checks pending)

Changed:

- Added [packages/app/src/brand.ts](packages/app/src/brand.ts) with `BRAND_NAME = "Bachuc"` (Q-001), `brandText`, and `withBrandName`. The transform replaces the whole word `Paseo`, case-sensitive, and skips `{{...}}` placeholders. It returns a copy and leaves the input unchanged.
- Added [packages/app/src/brand.test.ts](packages/app/src/brand.test.ts). It covers nested objects, plural keys, `$PASEO_PORT`, lowercase `paseo`, `PaseoLogo`, and `{{...}}` placeholders.
- [packages/app/src/i18n/i18next.ts](packages/app/src/i18n/i18next.ts) passes the whole `resources` map through `withBrandName` once. No locale file changed.
- [packages/app/app.config.js](packages/app/app.config.js): the production name is `Bachuc` and the development name is `Bachuc Debug`. Removed `owner: "getpaseo"` and `extra.eas.projectId` ([ADR-0005](watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md)). The mobile bundle IDs stay.
- [packages/app/public/manifest.json](packages/app/public/manifest.json) `name` and `short_name`, and the `apple-mobile-web-app-title` in [packages/app/public/index.html](packages/app/public/index.html), say `Bachuc`.
- UI literals: the browser tools warning in [packages/app/src/screens/settings/browser-tools-config.ts](packages/app/src/screens/settings/browser-tools-config.ts) and the recovery message in [packages/app/src/screens/workspace/workspace-route-state.ts](packages/app/src/screens/workspace/workspace-route-state.ts) use `BRAND_NAME`. The route state test now expects `Bachuc`.
- Kept [packages/app/src/git/use-actions.tsx](packages/app/src/git/use-actions.tsx) line 1070: it is a key that matches the daemon's English message, not text that users read.

Contract:

- The development name changed too, beyond the spec's production-only line. Live desktop and `npm run dev:app` run with `APP_VARIANT=development` ([packages/desktop/scripts/dev-runner.mjs](packages/desktop/scripts/dev-runner.mjs) line 146), so `%WEB_TITLE%` on live comes from that variant.
- Without the EAS project ID, `getExpoProjectId()` in [packages/app/src/push-notifications/internal/subscriptions.ts](packages/app/src/push-notifications/internal/subscriptions.ts) returns `null` on native. Mobile is out of scope ([ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md)).
- GitNexus `impact` on `i18next.ts` (file level) reported CRITICAL, because every translated string passes through it. The `i18n` export keeps its shape. `resolveWorkspaceRouteState` was LOW. `BROWSER_TOOLS_WARNING` was UNKNOWN; a text search found only its own file and its test. `detect_changes` reported risk low with 5 changed symbols.

Verified:

- `cd packages/app && npx vitest run src/brand.test.ts src/screens/workspace/workspace-route-state.test.ts src/screens/settings/browser-tools-config.test.ts src/i18n/resources.test.ts --bail=1` -> PASS (53), FAIL (0).
- `git diff --stat main -- packages/app/src/i18n/resources` -> no output.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed files>` -> 0 warnings, 0 errors.
- Plan Verify: the license diff exits 0, and the plugin diff has no output.
- UNVERIFIED (autonomous run): the human check on live. Live was not running (no listener on 6768 or 8081), and the run does not start it.

Anti-goal:

- Before changes: `git diff --numstat main -- packages/app ':(exclude)packages/app/src/brand.ts' ':(exclude)packages/app/src/brand.test.ts'` -> 0, 2026-10-08 10:08.
- After the transform: 5, 2026-10-08 10:10.
- Final: 28, 2026-10-08 10:14.
- Result: PASS against 40 or less.

Lessons:

- The agent shell is zsh. An unquoted `$FILES` with several paths passes as one argument, so `npm run format:files -- $FILES` formats nothing and prints "Expected at least one target file". Pass the paths literally.
- Live desktop uses the `development` app variant, so a change to the production `name` in `app.config.js` does not show on live.
