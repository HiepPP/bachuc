# TASK-180 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/theme/src/custom.rs` (new): `CustomThemeColors`, `CustomTheme`, `Tokens::from_custom`, `active_custom`, `next_builtin_choice`, and the private pure `CustomState` (list, stored selection, active theme with cached tokens) plus `cycle_builtin`. 10 unit tests.
- `packages/desktop-gpui/crates/theme/src/lib.rs`: `State` holds a `CustomState`; new `set_custom_themes`, `custom_themes`, `set_custom_theme`, `custom_theme`, `active_custom_theme`, `cycle_builtin_theme`. `tokens`, `resync`, and the appearance observer read the active custom theme.

## Contract

- Every Produces line is implemented with its name and type. `bachuc-theme` has no new dependency.
- With an active custom theme: `tokens(cx)` returns its cached tokens, `resync` sets the gpui-kit mode from its scheme, and a system appearance change does not resync. With none, behavior is as before.
- A selected id that the list lacks stays stored and `ThemeChoice` applies; the theme returns when the list holds it.
- `set_custom_themes` and `set_custom_theme` call `resync` only when the active theme (id, name, scheme, colors) changed.
- `ThemeChoice` and every existing public item are unchanged. `set_theme` still sets the choice only; a selected custom theme still wins.
- Additions beyond the spec (Q-103 cycle shortcut):
  - `next_builtin_choice(ThemeChoice) -> ThemeChoice`, order Light, ClaudeDark, System, Light. The spec names no order, so this is a default.
  - `cycle_builtin_theme(cx) -> ThemeChoice`: clears the stored custom selection, sets the next built-in choice, and resyncs. No shortcut handler exists in the GPUI crates yet; TASK-177 or the app should call it.
- The cycle clears the selection (it does not come back on its own), because a kept selection would make the shortcut change nothing on screen.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-theme` -> exit 0. 17 unit tests, 5 `syntax_colors` tests; includes `light_tokens_match_theme_ts`, `claude_dark_tokens_match_theme_ts`, `every_theme_matches_the_highlight_package_in_light_and_dark` unchanged. New tests cover: light and dark `from_custom` field by field, accent fallback, base status/tint/dot/diff/destructive kept, `active_custom` with a present id, a missing id, `None`, and an empty list, selection before the list, dropped active theme keeps the id, `None` selection, resync only on an active change, and the cycle.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-theme` -> exit 0.
- `rg -n "bachuc-plugin-host" packages/desktop-gpui/crates/theme/Cargo.toml` -> no match (exit 1).
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0. `git status` shows my edits only in `crates/theme`; `Cargo.lock` was already modified before this run.
- GitNexus impact on `tokens`: 29 candidates, risk UNKNOWN to CRITICAL (169 callers). The signature and the no-custom behavior are unchanged, so no caller changes.
- Not covered by a test: `tokens(cx)`, `resync`, and `Theme::change` need an `App`, and the crate has no `App` test harness. They are checked by reading only. UNVERIFIED (unattended): code tests only.
- UI checks: none in this spec.

## Anti-goal

- Checkpoint: `light_tokens_match_theme_ts`, `claude_dark_tokens_match_theme_ts`, and `every_theme_matches_the_highlight_package_in_light_and_dark` pass unchanged; source `/tmp/wt-j180-test.txt`; at completion.
- `tokens(cx)` with no custom theme: the code path is the old `match` on `state.light` or `state.dark`, which hold `Tokens::light()` and `Tokens::claude_dark()`. Checked by reading, not by an `App` test.
- Tripwire: not tripped.

## Lessons

- The crate has no `App` test harness, so state rules live in the pure `CustomState` and the tests call it directly.
