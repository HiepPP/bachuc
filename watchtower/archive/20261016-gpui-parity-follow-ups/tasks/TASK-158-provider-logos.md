# TASK-158 Provider logos in bachuc-icons

Group: B (writes `crates/icons`)
Class: code

## Brief

Goal: Draw the Electron provider logos in the GPUI client.

Change: only a local `crates/composer/assets/claude.svg`, and a bot icon for every other provider -> one logo set in `bachuc-icons` for every Electron provider id.

Design: the provider mapping in [packages/app/src/components/provider-icons.ts](packages/app/src/components/provider-icons.ts) (line 31 maps `pi`) and [packages/app/src/components/provider-icon-name.ts](packages/app/src/components/provider-icon-name.ts) (line 14 adds `glm`); the built-in names in [packages/protocol/src/provider-icon-names.ts](packages/protocol/src/provider-icon-names.ts) (claude, codex, copilot, kiro, minimax, omp, opencode, pi); the glyphs in the React components [packages/app/src/components/icons](packages/app/src/components/icons) (`claude-icon.tsx`, `codex-icon.tsx`, `copilot-icon.tsx`, `minimax-icon.tsx`, `omp-icon.tsx`, `opencode-icon.tsx`, `pi-icon.tsx`) and the GLM logo `packages/app/src/assets/acp-provider-icons/glm-acp-agent.svg`.

Boundaries:

- Take each glyph from the component Electron draws (Builder Check: the `acp-provider-icons` SVGs are catalog art, not the drawn glyphs, except GLM). Write the SVGs into `crates/icons/assets/providers/` with a small generator next to `codegen/generate-icons.mjs`, so a second run gives no change.
- `kiro` draws a lucide icon in Electron (`PackagePlus`); `provider_fallback_icon` returns it when `IconName` has it, else `Bot`. Unknown ids return `Bot`.
- The logos are filled SVGs. Draw them as images with the fill written into the SVG text, rasterized at 2x, and cached per color, as the composer does today ([watchtower/archive/20261016-gpui-electron-parity/tasks/TASK-150-outcome.md](watchtower/archive/20261016-gpui-electron-parity/tasks/TASK-150-outcome.md) Fix Round 1 lessons).
- Keep every existing public item of `bachuc-icons`.

Files:

- [packages/desktop-gpui/crates/icons](packages/desktop-gpui/crates/icons) (`Cargo.toml`, `src`, `codegen`, `assets/providers/*.svg`, `tests`)

Interfaces:

- Produces: `bachuc_icons::provider_logo_svg(provider: &str) -> Option<&'static str>`, `bachuc_icons::provider_logo(provider: &str, fill: gpui_kit::Hsla) -> Option<std::sync::Arc<gpui_kit::Image>>`, and `bachuc_icons::provider_fallback_icon(provider: &str) -> bachuc_icons::IconName` for TASK-163, TASK-165, and TASK-167

Expected result:

- Each Electron provider id with a glyph gives its logo; `kiro` and unknown ids give `None` and a fallback icon.
- Two calls with the same provider and color return the same cached image.

Anti-goal: the provider assets stay under 200 KB in total; tripwire; read with `du -sh packages/desktop-gpui/crates/icons/assets/providers` at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-icons` -> pass, with a test per provider id of `provider-icon-names.ts` plus `glm`, a fallback test for `kiro` and an unknown id, and a cache test
- The provider generator run twice -> the second run changes no file
- `npm run lint -- <the new generator file>` -> exit 0
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-icons` -> exit 0
- Anti-goal: `du -sh packages/desktop-gpui/crates/icons/assets/providers` -> under 200K
