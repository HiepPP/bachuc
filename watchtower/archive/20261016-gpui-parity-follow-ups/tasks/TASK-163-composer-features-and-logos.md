# TASK-163 Composer features, diff row, and logos

Group: G (writes `crates/composer`)
Class: code

## Brief

Goal: Add the Electron agent feature toggles (the lightning button), the diff-count row, and the provider logos to the composer (Q-079, Q-078).

Change: no feature controls, no diff-count row, and a local `assets/claude.svg` (loaded with `include_str!` at `crates/composer/src/composer.rs:45`) with a bot for other providers -> Electron feature items, the workspace diff stat row, and `bachuc_icons` logos.

Design: [packages/app/src/composer/agent-controls/index.tsx](packages/app/src/composer/agent-controls/index.tsx) (`DesktopFeatureItem` line 1351), [packages/app/src/agent-controls/icons.ts](packages/app/src/agent-controls/icons.ts) (`getAgentFeatureIcon` line 63), and the Electron diff-count row above the input.

Boundaries:

- Draw the features after the mode pill, as `DesktopFeatureItem` does: a toggle is a button with its icon and tooltip; a select is a menu of its options.
- Map feature `icon` names to `bachuc_icons` icons as `getAgentFeatureIcon` does; an unknown name uses the Electron fallback.
- After a failed `set_agent_feature`, show the value the daemon reports, not the clicked value.
- The diff row reads `workspace_diff_stat` of the agent's workspace; `None` hides the row.
- Use `bachuc_icons::provider_logo` with `provider_fallback_icon`, and delete `crates/composer/assets/claude.svg` and its loader.
- Keep `Composer::new`, `agent_id`, `focus`, `attachment_count`, and `ComposerEvent` unchanged. Redraw only when the watched snapshot changes (Q-067 pattern already in `composer.rs`).

Files:

- [packages/desktop-gpui/crates/composer](packages/desktop-gpui/crates/composer) (`Cargo.toml`, `src`, `assets`, `examples/composer.rs`)

Interfaces:

- Consumes: `bachuc_client::AgentFeature` (`id: String`, `label: String`, `description: Option<String>`, `tooltip: Option<String>`, `icon: Option<String>`, `kind: AgentFeatureKind`), `bachuc_client::AgentFeatureKind` (enum `Toggle { value: bool }`, `Select { value: Option<String>, options: Vec<AgentFeatureOption> }`), `bachuc_client::AgentFeatureOption` (fields as the daemon sends), `Client::agent_features(&self, agent_id: &str) -> Vec<AgentFeature>`, and `Client::set_agent_feature(&mut self, agent_id: &str, feature_id: &str, value: serde_json::Value, cx) -> Task<anyhow::Result<()>>` from TASK-157
- Consumes: `bachuc_client::DiffStat` (struct `additions: u32`, `deletions: u32`), `Client::workspace_diff_stat(&self, workspace_id: &str) -> Option<DiffStat>`, and `Client::workspace_pinned_at(&self, workspace_id: &str) -> Option<String>` from TASK-157
- Consumes: `bachuc_icons::provider_logo_svg(provider: &str) -> Option<&'static str>`, `bachuc_icons::provider_logo(provider: &str, fill: gpui_kit::Hsla) -> Option<std::sync::Arc<gpui_kit::Image>>`, and `bachuc_icons::provider_fallback_icon(provider: &str) -> bachuc_icons::IconName` from TASK-158
- Produces: the same `bachuc_composer::Composer` and `ComposerEvent` API for TASK-167

Expected result:

- A Codex agent with `fast_mode` shows the lightning toggle; a click flips it through `set_agent_feature`.
- A workspace with changes shows `+N -M` above the input.

Anti-goal: one click sends exactly one `set_agent_feature` request, and an unchanged feature list causes no redraw; tripwire: tests for both; read at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> pass, including tests for feature ordering, a toggle request payload, a failed request that keeps the daemon value, a select with a null value and an option change, the diff row with and without a stat, the provider logo and fallback icon, and the no-redraw test
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-composer` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
