# TASK-197 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/theme/src/tokens.rs`: `Tokens` fields `palette_yellow_400`, `palette_green_400`, `palette_blue_400` (`Hsla`), set in `light()` and `claude_dark()`; new test `feature_palette_tokens_match_theme_ts_in_both_themes`.
- `packages/desktop-gpui/crates/theme/src/custom.rs`: `tokens_outside_the_mapping_keep_the_base_value` also checks the three tokens. `Tokens::from_custom` needed no code change, because it starts from the base theme of the scheme.
- `packages/desktop-gpui/crates/composer/src/agent_controls/features.rs`: new pure `feature_tint(&AgentFeature, &Tokens) -> Hsla` and a test for each feature id, on, off, and as a select, in both themes.
- `packages/desktop-gpui/crates/composer/src/agent_controls/mod.rs`: `highlight_color` is removed; `feature_pills` calls `feature_tint(feature, &t)`.

## Contract

- Produces: `bachuc_theme::Tokens` fields `palette_yellow_400: Hsla` (`#fbbf24`), `palette_green_400: Hsla` (`#4ade80`), `palette_blue_400: Hsla` (`#60a5fa`).
- `theme.ts` gives the same `baseColors` palette to the light and the dark theme (`palette: baseColors` in both builders), so the three values are equal in every built-in set and in plugin themes. A plugin theme cannot change them.
- Fast mode on is yellow, auto accept on is green, plan mode on is blue. An off toggle, a select, and any other toggle id keep `foreground_muted`.
- `ThemeChoice`, `Tokens::from_custom`, and every existing token and value are unchanged. The board plugin builds `Tokens` with `..Tokens::light()`, so the new fields do not break it.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> exit 0. 18 unit tests, 5 `syntax_colors` tests; `light_tokens_match_theme_ts` and `claude_dark_tokens_match_theme_ts` pass unchanged.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> exit 0. 92 tests.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-theme -p bachuc-composer` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0. One warning, `next_theme` never used in `bachuc-app`; not in my files.
- UI check: DEFERRED (UI checker). The spec has no `UI check:` line.

## Anti-goal

- Checkpoint: `light_tokens_match_theme_ts`, `claude_dark_tokens_match_theme_ts`, and `tint_alphas_are_the_theme_ts_rule` pass unchanged; source `/tmp/wt197-theme.txt`; at completion.
- Tripwire: not tripped. No existing token line changed.

## Lessons

- `block(source, "  blue: {")` in the token test finds the palette scales by their unique marker lines (46, 61, 94 of `theme.ts`).
