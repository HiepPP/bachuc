# TASK-002 Bachuc name in the app UI

Group: B (`packages/app/src/i18n/i18next.ts`, new `packages/app/src/brand.ts`, `packages/app/public/`, `packages/app/app.config.js`)
Class: code

## Brief

Goal: Every UI text that names the product says Bachuc on desktop and web. No locale file changes.

Change: 38 English strings and their translations say "Paseo" -> one transform at i18n init shows the brand name.

Boundaries:

- Follows [ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md): keep the `paseo` command, `PASEO_*` names such as `$PASEO_PORT`, and code symbols.
- The transform replaces the whole word `Paseo`, case-sensitive. So `PASEO_PORT` and `paseo` stay.
- Do not edit files under [packages/app/src/i18n/resources](packages/app/src/i18n/resources). Upstream changes them often.
- Keep the name in one constant in a new [packages/app/src/brand.ts](packages/app/src/brand.ts). Its value is `Bachuc` (Q-001, answered 2026-10-08).
- The transform changes resource strings only, not interpolation values. TASK-007 needs this for the "based on Paseo" line.
- Follows [ADR-0005](watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md): remove `owner: "getpaseo"` and `extra.eas.projectId` from [packages/app/app.config.js](packages/app/app.config.js). Keep the mobile bundle IDs.
- Leave developer error text in [packages/app/src/plugins/hosts/index.ts](packages/app/src/plugins/hosts/index.ts), [packages/app/src/plugins/actions.ts](packages/app/src/plugins/actions.ts), and [packages/app/src/diagnostics/app-diagnostic-report.ts](packages/app/src/diagnostics/app-diagnostic-report.ts).

How:

- Add `brand.ts` with the name constant and a pure function that returns a copy of a resources object with the word replaced.
- Add `brand.test.ts`. Cover nested objects, plural keys, `PASEO_PORT`, lowercase `paseo`, and `{{...}}` placeholders.
- In `i18next.ts`, pass each locale through the function once.
- Set the production `name` in `app.config.js` to `Bachuc`. Check on live web that the page title follows it through `%WEB_TITLE%`.
- Set `name` and `short_name` in [packages/app/public/manifest.json](packages/app/public/manifest.json), and `apple-mobile-web-app-title` in [packages/app/public/index.html](packages/app/public/index.html).
- Check the UI literals outside i18n: [packages/app/src/screens/settings/browser-tools-config.ts](packages/app/src/screens/settings/browser-tools-config.ts) line 5, [packages/app/src/screens/workspace/workspace-route-state.ts](packages/app/src/screens/workspace/workspace-route-state.ts) line 87, and [packages/app/src/git/use-actions.tsx](packages/app/src/git/use-actions.tsx) line 1070. Change only text that users read. If line 1070 is a key that matches a daemon message, keep it.

Files:

- [packages/app/src/brand.ts](packages/app/src/brand.ts) (new: name constant and transform)
- [packages/app/src/brand.test.ts](packages/app/src/brand.test.ts) (new: unit test)
- [packages/app/src/i18n/i18next.ts](packages/app/src/i18n/i18next.ts) (apply the transform)
- [packages/app/app.config.js](packages/app/app.config.js) (production name, EAS owner and project ID)
- [packages/app/public/manifest.json](packages/app/public/manifest.json) (PWA names)
- [packages/app/public/index.html](packages/app/public/index.html) (Apple web app title)
- [packages/app/src/screens/settings/browser-tools-config.ts](packages/app/src/screens/settings/browser-tools-config.ts), [packages/app/src/screens/workspace/workspace-route-state.ts](packages/app/src/screens/workspace/workspace-route-state.ts), [packages/app/src/git/use-actions.tsx](packages/app/src/git/use-actions.tsx) (UI literals, if users read them)

Expected result:

- On live desktop and web, Settings, the help menu, the welcome screen, and the quit dialog say Bachuc.
- A hint that names `$PASEO_PORT` still shows `$PASEO_PORT`.
- The browser tab title and the PWA name say Bachuc.
- No locale file changes.

Anti-goal: changed lines in existing upstream files (all files except `brand.ts` and `brand.test.ts`) stay at or below 40; drift gauge; read with the numstat command in Verify before changes (0), after the transform, and at completion.

## Verify

- `cd packages/app && npx vitest run src/brand.test.ts --bail=1` -> pass.
- `git diff --stat main -- packages/app/src/i18n/resources` -> no output.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed files>` -> exit 0.
- Human check on live (`npm run dev`, then `npm run dev:desktop`): Settings, the help menu, the welcome screen, and the browser tab title say Bachuc. The `$PASEO_PORT` hint is unchanged.
- Anti-goal: `git diff --numstat main -- packages/app ':(exclude)packages/app/src/brand.ts' ':(exclude)packages/app/src/brand.test.ts'` -> the sum of added and removed lines is 40 or less.
