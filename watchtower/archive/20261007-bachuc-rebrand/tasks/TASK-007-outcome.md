# TASK-007 Outcome

## Outcome

Status: DONE

Changed:

- New [NOTICE](NOTICE) with the exact text of the spec.
- [README.md](README.md) has the fork line at the top. The rest is unchanged.
- Settings > About: the "What's new" row became a "Based on Paseo" row. It opens `https://github.com/getpaseo/paseo`. The community links under About are gone. See [packages/app/src/screens/settings-screen.tsx](packages/app/src/screens/settings-screen.tsx).
- The open project screen has no community links. See [packages/app/src/screens/open-project-screen.tsx](packages/app/src/screens/open-project-screen.tsx).
- The sidebar help menu has no "What's new" item and no "Report an issue" section (Discord and upstream GitHub issue). See [packages/app/src/components/sidebar/sidebar-help-menu.tsx](packages/app/src/components/sidebar/sidebar-help-menu.tsx).
- New key `settings.about.basedOn` (`Based on {{upstream}}`) in all 9 locale files.
- The changelog module is unchanged. Nothing opens it now, except the update callout, which stays off with TASK-006.
- `community-links.tsx` and `icons/discord-icon.tsx` are now unused. The run kept them, so upstream merges of these files stay clean. `npm run knip` would list them.

Contract:

- The About line passes "Paseo" as the `upstream` value, so the TASK-002 brand transform keeps the word.
- Docs links to `paseo.sh/docs` are unchanged (Q-004).

Verified:

- `rg -c "Mohamed Boudra" NOTICE` -> 1.
- `cd packages/app && npx vitest run src/i18n/resources.test.ts --bail=1` -> 36 passed.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed files>` -> 0 warnings, 0 errors.
- `npm run format:files -- <changed files>` -> exit 0.
- `git diff --stat main -- 'hiep-plugins/plugins/*/package.json' packages/plugin/src` -> no output.
- Human check on live: UNVERIFIED (autonomous run).

Anti-goal:

- Before changes: `git diff --exit-code main -- <license files>` exit 0; 2026-10-08 11:31.
- After the UI and locale edits (one batch): exit 0; 2026-10-08 11:34.
- Final: exit 0; 2026-10-08 11:36, before the commit.
- Result: PASS against "license and notice files unchanged".

Lessons:

- [packages/app/e2e/browser/sidebar-help.spec.ts](packages/app/e2e/browser/sidebar-help.spec.ts) still asserts the removed help items, the in-app changelog, and the app name `Paseo`. This run does not run e2e specs, so the spec stays stale until the owner updates it. Source: the spec, lines 13 and 46-104.
