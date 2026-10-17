# TASK-197 Feature toggle palette colors

Group: W (writes `crates/theme` and `crates/composer`; one group because the composer consumes the new tokens)
Class: code

## Brief

Goal: Tint the composer feature toggles with the Electron palette 400 colors (Q-092, owner, 2026-10-17).

Change: an enabled fast mode, auto accept, or plan toggle uses `status_warning`, `status_success`, or `accent` ([packages/desktop-gpui/crates/composer/src/agent_controls/mod.rs](packages/desktop-gpui/crates/composer/src/agent_controls/mod.rs) lines 181-182) -> the Electron `yellow[400]`, `green[400]`, and `blue[400]`.

Design: [packages/app/src/composer/agent-controls/index.tsx](packages/app/src/composer/agent-controls/index.tsx) `getFeatureIconColor` (line 215) with `getFeatureHighlightColor` ([packages/app/src/composer/agent-controls/utils.ts](packages/app/src/composer/agent-controls/utils.ts) line 38); palette values in [packages/app/src/styles/theme.ts](packages/app/src/styles/theme.ts): blue 400 `#60a5fa` (line 51), green 400 `#4ade80` (line 64), yellow 400 `#fbbf24` (line 95). Confirm each value and whether the palette differs between light and dark.

Boundaries:

- Add three tokens to `bachuc_theme::Tokens` (for example `palette_yellow_400`, `palette_green_400`, `palette_blue_400`) in every built-in token set, and in `Tokens::from_custom` for plugin themes. Keep every existing token and its value ([ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md)).
- A disabled toggle keeps `foreground_muted`, as Electron does.

Files:

- [packages/desktop-gpui/crates/theme](packages/desktop-gpui/crates/theme) (`src`, `tests`)
- [packages/desktop-gpui/crates/composer](packages/desktop-gpui/crates/composer) (`src`)

Interfaces:

- Produces: `bachuc_theme::Tokens` fields `palette_yellow_400: Hsla`, `palette_green_400: Hsla`, and `palette_blue_400: Hsla` for this TASK's composer change

Expected result:

- Fast mode on shows the Electron yellow; auto accept green; plan blue; off shows the muted foreground.

Anti-goal: no existing token changes value; tripwire: the existing token comparison tests pass unchanged; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> pass, including the existing token tests and a test that the three tokens equal the Electron hex values within 1 unit per channel
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> pass, including a test of the feature tint for each feature id, on and off
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-theme -p bachuc-composer` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
