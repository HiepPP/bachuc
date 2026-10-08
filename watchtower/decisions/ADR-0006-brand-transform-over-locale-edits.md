# ADR-0006 Brand transform over locale edits

Status: accepted
Date: 2026-10-08
Supersedes: -
Scope: `packages/app/src/brand.ts`, `packages/app/src/i18n/i18next.ts`, `packages/app/src/i18n/resources/**`

## Context

- On 2026-10-07, 38 English UI strings held the word "Paseo", and the 8 other locales held the same Latin word. No locale had a transliteration.
- Upstream changes the locale files in [packages/app/src/i18n/resources](packages/app/src/i18n/resources) often, so each edited line conflicts at merge time.
- TASK-002 of plan `20261007-bachuc-rebrand` shipped `withBrandName` in [packages/app/src/brand.ts](packages/app/src/brand.ts). [packages/app/src/i18n/i18next.ts](packages/app/src/i18n/i18next.ts) applies it once at init.

## Decision

- Show the brand through one transform at i18n init. It replaces the whole word `Paseo` in resource strings with `BRAND_NAME`.
- Keep "Paseo" in the locale files. A new upstream string that names Paseo shows Bachuc with no edit.
- Pass "Paseo" as an interpolation value when a string must keep the upstream name, such as "Based on Paseo".
- The owner chose this on 2026-10-08.

## Consequences

- A test that expects the literal UI text must build it from `BRAND_NAME`, as [packages/app/src/desktop/daemon/daemon-management-error.test.ts](packages/app/src/desktop/daemon/daemon-management-error.test.ts) does.
- A UI string outside i18n does not get the transform. Change it by hand.

## Options Considered

- Edit "Paseo" to "Bachuc" in every locale file: rejected. It changes about 340 lines that upstream edits often, so most upstream merges would conflict.

## Revisit If

- The fork stops merging upstream Paseo.
- A locale starts to translate or transliterate "Paseo", so the word no longer matches.
