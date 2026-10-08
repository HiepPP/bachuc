# TASK-007 Attribution and upstream links

Group: F (new `NOTICE`, `README.md`, `packages/app/src/screens/settings-screen.tsx`, `packages/app/src/i18n/resources/`, `packages/app/src/components/community-links.tsx`, changelog entry points)
Class: code

## Brief

Goal: Bachuc names its origin as "based on Paseo" in a NOTICE file, the README, and Settings > About. The UI no longer sends users to upstream community pages.

Change: no attribution and upstream community links -> NOTICE, a README note, an About line, and no Discord, Sponsors, upstream GitHub, or changelog links (Q-004).

Boundaries:

- Apache 2.0 Section 4(b) asks for a notice that the files changed. Section 4(c) asks to keep the original copyright. Section 6 allows naming the origin.
- Keep [LICENSE](LICENSE) unchanged. Put the owner's name in `NOTICE`, not in `LICENSE`.
- The About line passes "Paseo" as an interpolation value, for example `Based on {{upstream}}`. Then the TASK-002 transform keeps the word.
- Add the new About key to all 9 locale files, as the repo i18n rule asks. Check [packages/app/src/i18n/resources.test.ts](packages/app/src/i18n/resources.test.ts) for key parity.
- Q-004, answered 2026-10-08: remove the Discord, Sponsors, upstream GitHub, and changelog links from the UI.
- Keep docs links to `paseo.sh/docs`. They describe the shared code.
- `NOTICE` holds exactly this text:

```text
Bachuc
Copyright 2026 Phuoc Hiep

Bachuc is based on Paseo (https://github.com/getpaseo/paseo).
Paseo is Copyright (c) 2025-present Mohamed Boudra and is licensed
under the Apache License, Version 2.0. See LICENSE.

Phuoc Hiep changed the original files. The git history lists each change.
Third-party components keep their own licenses.
```

- The README gets one line at the top and keeps the rest unchanged:

```text
> Bachuc is a personal fork of [Paseo](https://github.com/getpaseo/paseo) by Mohamed Boudra, under the Apache License 2.0. See [NOTICE](NOTICE).
```

How:

- Write `NOTICE` and the README line.
- Add the About line in [packages/app/src/screens/settings-screen.tsx](packages/app/src/screens/settings-screen.tsx), with a link to `https://github.com/getpaseo/paseo`.
- Remove the Discord, Sponsors, and upstream GitHub links. Check where [packages/app/src/components/community-links.tsx](packages/app/src/components/community-links.tsx) is used, for example in [packages/app/src/screens/open-project-screen.tsx](packages/app/src/screens/open-project-screen.tsx).
- Hide the entry points that open the changelog from `paseo.sh/changelog` ([packages/app/src/changelog/changelog-host.tsx](packages/app/src/changelog/changelog-host.tsx), [packages/app/src/changelog/internal/changelog-sheet.tsx](packages/app/src/changelog/internal/changelog-sheet.tsx), and the "What's new" hint in About).

Files:

- [NOTICE](NOTICE) (new)
- [README.md](README.md) (one line at the top)
- [packages/app/src/screens/settings-screen.tsx](packages/app/src/screens/settings-screen.tsx) (About line, changelog entry)
- [packages/app/src/i18n/resources](packages/app/src/i18n/resources) (one new key in each of the 9 locale files)
- [packages/app/src/components/community-links.tsx](packages/app/src/components/community-links.tsx) and its call sites (remove upstream links)
- [packages/app/src/changelog/changelog-host.tsx](packages/app/src/changelog/changelog-host.tsx) (hide the upstream changelog)

Expected result:

- Settings > About shows "Based on Paseo" with a link to the upstream repo.
- The UI shows no Discord, Sponsors, upstream GitHub, or changelog link.
- `NOTICE` exists with the exact text above.

Anti-goal: the license and notice files stay unchanged; tripwire; read before changes and at completion with the `git diff --exit-code` command in Verify.

## Verify

- `rg -c "Mohamed Boudra" NOTICE` -> 1.
- `cd packages/app && npx vitest run src/i18n/resources.test.ts --bail=1` -> pass.
- `npm run typecheck` -> exit 0.
- `npm run lint -- <changed files>` -> exit 0.
- Human check on live: Settings > About shows "Based on Paseo". The sidebar, the open project screen, and Settings show no Discord, Sponsors, upstream GitHub, or changelog link.
- Anti-goal: `git diff --exit-code main -- LICENSE packages/expo-two-way-audio/LICENSE packages/highlight/src/astro/LICENSE packages/desktop/src/features/browser-automation/aria-snapshot-script.ts packages/app/src/terminal/local-links/terminal-local-link-provider.ts packages/app/src/terminal/local-links/terminal-local-link-parsing.ts` -> exit 0.
