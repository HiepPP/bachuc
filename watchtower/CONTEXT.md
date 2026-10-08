# Plan Context

## Shared Context

- Goal: the fork shows the brand Bachuc instead of Paseo. The name comes from Ba Chúc, the owner's home place in An Giang, Vietnam.
- The code is under Apache 2.0, see [LICENSE](LICENSE). Keep [LICENSE](LICENSE) and its line `Copyright (c) 2025-present Mohamed Boudra` unchanged.
- Keep the Microsoft copyright headers in [packages/desktop/src/features/browser-automation/aria-snapshot-script.ts](packages/desktop/src/features/browser-automation/aria-snapshot-script.ts), [packages/app/src/terminal/local-links/terminal-local-link-provider.ts](packages/app/src/terminal/local-links/terminal-local-link-provider.ts), and [packages/app/src/terminal/local-links/terminal-local-link-parsing.ts](packages/app/src/terminal/local-links/terminal-local-link-parsing.ts).
- Keep the MIT files [packages/expo-two-way-audio/LICENSE](packages/expo-two-way-audio/LICENSE) and [packages/highlight/src/astro/LICENSE](packages/highlight/src/astro/LICENSE).
- Name the origin only as "based on Paseo". Never use the Paseo butterfly or the name Paseo as the product name.
- The brand scope follows [ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md). Upstream services follow [ADR-0005](watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md).
- Keep upstream merges cheap. Touch few upstream files and few lines. Prefer one central constant or transform over many edits.
- Release safety from the root [CLAUDE.md](CLAUDE.md): never restart or stop the release daemon on port 6767. Never run `scripts/paseo-release.sh` or `npm run build:desktop`. The owner runs the release script from Terminal.app.
- Test source changes on live: `npm run dev` (daemon, port 6768), then `npm run dev:desktop`.
- Agents use the `paseo` CLI through `~/.local/bin/paseo`. On 2026-10-07 it linked to `/Applications/Paseo Fork.app/Contents/Resources/bin/paseo`. A renamed app breaks it unless the release script links it again.
- `IS_RELEASE_APP` in [packages/desktop/src/main.ts](packages/desktop/src/main.ts) line 121 matches `/Paseo Fork.app/`. If the new install path does not match, the app silently runs on `~/.paseo-dev` and port 6770 instead of the release home.
- Checks: `npm run typecheck`, `npm run lint -- <files>`, and `npx vitest run <file> --bail=1` for changed test files only. Run app vitest files from `packages/app`. Format with `npm run format:files -- <files>`, including plan files.
- Run GitNexus `impact` before you edit a symbol. Run `detect_changes` before each commit.
- Tools on this machine: `iconutil`, `sips`, and `magick`. `rsvg-convert` and `potrace` are missing.
- Codex CLI 0.160.0 has the `image_generation` feature on (checked 2026-10-07).

## Decisions

- The display name is `Bachuc` by default (Q-001).
- The macOS executable is renamed to `Bachuc` too (Q-005, answered 2026-10-08). TASK-009 does it.
- TASK-002 brands UI text with one transform at i18n init in [packages/app/src/i18n/i18next.ts](packages/app/src/i18n/i18next.ts). It does not edit locale files. Reason: 38 English strings hold "Paseo", and the 8 other locales hold the same Latin word. No locale has a transliteration (checked 2026-10-07).
- The attribution line passes "Paseo" as an i18n interpolation value. The transform changes resource strings only, so the line keeps "Paseo".
- The desktop bundle ID is `io.github.hieppp.bachuc` by default (Q-003).
- The release app keeps the userData folder `~/Library/Application Support/Paseo`, so the host list and settings stay. A build opened in place gets a new userData folder named after its new product name. That folder held no cloned data.
- Mobile is out of scope ([ADR-0004](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md)). Leave mobile bundle IDs, fastlane, maestro flows, and mobile e2e files.
- [packages/website](packages/website) is out of scope. The fork does not deploy it. Never deploy it as Bachuc while `packages/website/public/social-proof/` holds upstream testimonials.
- A Codex agent made raster logo concepts. The owner picked concept 1, the sugar palm (Q-002). The final mark is hand-written SVG geometry, because no vectorizer is installed.

## Open Decisions

- None.

## References

- [watchtower/decisions/ADR-0004-display-brand-over-full-rename.md](watchtower/decisions/ADR-0004-display-brand-over-full-rename.md)
- [watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md](watchtower/decisions/ADR-0005-upstream-services-off-over-self-hosting.md)
- [scripts/paseo-release.sh](scripts/paseo-release.sh)
- [docs/development.md](docs/development.md)
