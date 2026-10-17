# TASK-173 New workspace screen

Group: D (writes `crates/app`; TASK-174 and TASK-191 follow in the same group)
Class: risky

## Brief

Goal: Replace the GPUI New agent draft with the Electron New workspace screen, which creates through `workspace.create`, and turn the in-workspace draft tab into the Electron workspace draft tab with the draft composer.

Change: `new_agent.rs` draws a "New agent" card with a folder field, provider and model chips, and a Create button, and calls `create_agent_request` -> a New workspace screen with the project, isolation, and base pickers over the TASK-172 draft composer, which sends one `workspace.create.request`.

Design:

- Screen: [packages/app/src/screens/new-workspace-screen.tsx](packages/app/src/screens/new-workspace-screen.tsx): root, header, and body (lines 2456-2481); `NewWorkspaceLayout` in a centered `ComposerDock` (2484-2509); styles (2511-2625): surface0 root; title "New workspace", 2xl, normal weight, foreground, margin-bottom 32, padding left 24 and right 16 (`composerTitleContainer` 2525, `composerTitle` 2530); the desktop form row: center aligned, margin-bottom 32, padding 16 at both sides, gap 8 (`formStackDesktop` 2544); badge: height 28 (`BADGE_HEIGHT` 229), max width 240, padding 8, radius 2xl, gap 4, hover surface2, pressed surface0, disabled opacity 0.6, text base foregroundMuted (2578-2611); chevron foregroundExtraMuted, 1 px lower (`MetaChevron` 215, `chevronContainer` 2612); error text base, destructive, line height 20 (`errorText` 2535).
- Row order: project, host, isolation, base, then a spacer and the launch control (lines 1623-1634). The host picker shows only with more than one host (1442). Isolation shows only when a worktree is possible (1541); the base picker shows when the daemon lacks multiplicity or isolation is a worktree (`useWorkspaceIsolation` 716-746). "Add project" is the last project option (`AddProjectPickerAction` 620).
- Strings: [packages/app/src/i18n/resources/en.ts](packages/app/src/i18n/resources/en.ts) lines 1268-1316 ("New workspace", "Local", "New worktree", "Starting ref", "Search branches and PRs", "Start from", and the tooltips).
- Create: `createMultiplicityWorkspace` (811-865) and the submit path (2060-2104, 2118-2180): an empty submit creates the workspace with no agent; text creates it with the first agent in the same request; one pending submit per draft (945-951).
- Base default: `defaultBasePickerItem` in [packages/app/src/screens/new-workspace-picker-item.ts](packages/app/src/screens/new-workspace-picker-item.ts) lines 163-179 (`refs/heads/<current branch>` when the daemon sends no upstream ref) and the branch-off request at 186-187.
- Workspace draft tab: [packages/app/src/composer/draft/workspace-tab.tsx](packages/app/src/composer/draft/workspace-tab.tsx) (bottom `ComposerDock`, lines 653-690; create through `createAgent`, line 209).

Boundaries:

- Keep every existing app test except the 3 tests of `new_agent.rs`, which go with the file; their rules move to the new pure functions and their tests. Update the `every_focus_target_in_this_crate_has_a_role` source list in `actions.rs` to the new file names.
- Planner call: the screen opens where the draft opens today with no workspace: the Agent launch of the home new tab. In a workspace, the Agent launch and `ReplaceWithDraft` open the workspace draft tab instead, as Electron does. `Shell::open_new_workspace` also opens the screen, for TASK-174.
- Planner call: the host picker never shows; the GPUI shell talks to one host. Terminal launch stays out, so the launch control is not drawn; the form row ends at the base picker.
- Planner call: the base picker lists `branch_suggestions` and is searchable. A typed name that is not listed shows as a row only when `validate_branch` says it exists. GitHub PR rows stay out (owner: PR features out of scope).
- Planner call: the project picker lists `client.projects()` by name, as the sidebar does (Q-085). The default is the preset project, else the first one. "Add project" opens a folder dialog, then `add_project`, then selects the result.
- `worktree_slug`: GPUI has no `mnemonic-id` word list, so send the first 8 hex digits of a new UUID. The daemon would default the branch name to "worktree" (`resolve-worktree-creation-intent.ts:73`), which collides on a second create.
- Isolation: a worktree needs a git checkout (`checkout_details` `is_git`). Without `server_info.features.workspaceMultiplicity`, every create is a worktree, as Electron does (line 2068).
- Build each request in one pure function from the form and the `DraftSubmission`: an empty text with no images or attachments gives `agent: None` and no prompt. Text gives `agent: Some(InitialAgent)` with the submission config, feature values, environment, images, and attachments, and `first_agent_prompt: Some(text)`. No project gives `Err("Choose a project")`; text with no config gives `Err("Select a model")`.
- One submit at a time: a second `DraftSubmit` while one is pending sends nothing. While pending, call `set_submitting(true)`. On `Err`, show the text under the composer and call `set_submitting(false)`. On `Ok`, open the new workspace when the client lists it, then its agent tab when `agent_id` is set.
- Workspace draft tab: a bottom `composer_dock` with a draft composer (1 line, arrow icon, no empty submit) in the workspace folder. It creates through `Client::create_agent_with` (TASK-169) with the draft's full config, feature values, plugin environment, images, and attachments (lead, from the spec check). A test checks that the request carries the mode, thinking, and feature values.
- `uuid`: add `uuid = { version = "1", features = ["v4"] }` to `crates/app/Cargo.toml` for the 8-hex `worktreeSlug` (lead, from the spec check).
- Route the TASK-171 composer events in the shell: `OpenChanges` opens the all-files diff tab of that workspace's folder, as `ExplorerDiffEvent::OpenAllDiffs` does with no focus file. `OpenAgent` opens that agent's tab in its own workspace.
- Every new button, row, picker, and menu item gets a role and a label. Use `bachuc_theme` tokens and metrics and `bachuc_icons` icons.
- Keep each rule in a pure function and test it, because GPUI views have no headless test context (`MEMORY.md`, 2026-10-16).
- In the Interfaces lines, `cx` means `cx: &mut Context<Client>` for `Client` methods.

Files:

- [packages/desktop-gpui/crates/app/src/new_agent.rs](packages/desktop-gpui/crates/app/src/new_agent.rs) (deleted)
- [packages/desktop-gpui/crates/app/src/new_workspace.rs](packages/desktop-gpui/crates/app/src/new_workspace.rs) (new: the screen, after `new-workspace-screen.tsx`)
- [packages/desktop-gpui/crates/app/src/workspace_draft.rs](packages/desktop-gpui/crates/app/src/workspace_draft.rs) (new: the workspace draft tab, after `workspace-tab.tsx`)
- [packages/desktop-gpui/crates/app/src](packages/desktop-gpui/crates/app/src) (`main.rs`, `shell.rs`, `actions.rs`, `panes.rs` for the wiring)
- [packages/desktop-gpui/crates/app/Cargo.toml](packages/desktop-gpui/crates/app/Cargo.toml) (`uuid`)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (generated)

Interfaces:

- Consumes: `bachuc_client::DraftConfig` (`provider: String`, `cwd: String`, `model: Option<String>`, `mode_id: Option<String>`, `thinking_option_id: Option<String>`), `Client::provider_modes(&mut self, provider: &str, cwd: Option<&str>, cx) -> Task<anyhow::Result<Vec<(String, String)>>>`, `Client::draft_features(&mut self, config: DraftConfig, cx) -> Task<anyhow::Result<Vec<AgentFeature>>>`, `Client::default_model(&self, provider: &str) -> Option<String>`, and `Client::model_thinking(&self, provider: &str, model: &str) -> Option<ThinkingControls>` (current is the model default) from TASK-169
- Consumes: `Client::create_agent_with(&mut self, agent: InitialAgent, cx) -> Task<anyhow::Result<String>>` (one `create_agent_request` with the full config, feature values, env, images, attachments, and labels of `InitialAgent`; resolves to the new agent id) from TASK-169
- Consumes: `bachuc_client::CreateWorkspace` (`source: WorkspaceSource`, `title: Option<String>`, `first_agent_prompt: Option<String>`, `agent: Option<InitialAgent>`), `bachuc_client::WorkspaceSource` (enum `Directory { path: String, project_id: Option<String> }`, `Worktree { cwd: String, project_id: Option<String>, action: Option<WorktreeAction>, ref_name: Option<String>, base_branch: Option<String>, branch_name: Option<String>, worktree_slug: Option<String> }`), `bachuc_client::WorktreeAction` (enum `BranchOff`, `Checkout`), `bachuc_client::InitialAgent` (`config: DraftConfig`, `feature_values: std::collections::BTreeMap<String, serde_json::Value>`, `env: std::collections::BTreeMap<String, String>`, `initial_prompt: String`, `images: Vec<serde_json::Value>`, `attachments: Vec<serde_json::Value>`, `labels: std::collections::BTreeMap<String, String>`), `bachuc_client::CreatedWorkspace` (`workspace_id: String`, `agent_id: Option<String>`, `setup_terminal_id: Option<String>`), and `Client::create_workspace(&mut self, request: CreateWorkspace, cx) -> Task<anyhow::Result<CreatedWorkspace>>` from TASK-169
- Consumes: `bachuc_client::BranchSuggestion` (`name: String`, `committer_date: Option<f64>`, `has_local: Option<bool>`, `has_remote: Option<bool>`, `local_ahead: Option<u32>`, `local_behind: Option<u32>`), `Client::branch_suggestions(&mut self, cwd: &str, query: Option<&str>, limit: Option<u32>, cx) -> Task<anyhow::Result<Vec<BranchSuggestion>>>`, `bachuc_client::BranchValidation` (`exists: bool`, `resolved_ref: Option<String>`, `is_remote: bool`), and `Client::validate_branch(&mut self, cwd: &str, branch_name: &str, cx) -> Task<anyhow::Result<BranchValidation>>` from TASK-169
- Consumes: `bachuc_client::AddedProject` (`id: String`, `name: String`, `root: String`, `view_key: String`) and `Client::add_project(&mut self, cwd: &str, cx) -> Task<anyhow::Result<AddedProject>>` from TASK-169
- Consumes: `bachuc_composer::ComposerEvent` keeps `Sent { text: String, images: usize }` and `ReplaceWithDraft`, and adds `OpenChanges { workspace_id: String }` (the diff stat pill) and `OpenAgent { agent_id: String }` (a Paseo subagent row) from TASK-171
- Consumes: `bachuc_composer::composer_dock(centered: bool, content: AnyElement, composer: AnyElement) -> AnyElement` (the Electron `ComposerDock`) from TASK-171
- Consumes: `bachuc_composer::SubmitIcon` (enum `Arrow`, `Return`), `bachuc_composer::DraftOptions` (`cwd: Option<String>`, `min_lines: usize`, `submit_icon: SubmitIcon`, `allow_empty_submit: bool`), and `Composer::new_draft(client: Entity<Client>, key: String, options: DraftOptions, window: &mut Window, cx: &mut Context<Composer>) -> Composer` (`agent_id()` returns `key` for a draft) from TASK-172
- Consumes: `bachuc_composer::DraftSubmission` (`text: String`, `images: Vec<serde_json::Value>`, `attachments: Vec<serde_json::Value>`, `config: Option<DraftConfig>`, `feature_values: std::collections::BTreeMap<String, serde_json::Value>`, `environment: std::collections::BTreeMap<String, String>`), `Composer::draft_submission(&self, cx: &App) -> Option<DraftSubmission>` (`None` for an agent composer), `Composer::set_draft_cwd(&mut self, cwd: Option<String>, cx: &mut Context<Composer>)`, `Composer::set_submitting(&mut self, submitting: bool, cx: &mut Context<Composer>)`, and `Composer::clear(&mut self, window: &mut Window, cx: &mut Context<Composer>)` from TASK-172
- Consumes: `ComposerEvent::DraftSubmit` (a draft submit press; the text may be empty) and `ComposerEvent::ShowNewWorkspacePanel { plugin_id: &'static str, panel_id: &'static str }` from TASK-172
- Produces (crate-internal): `new_workspace::NewWorkspacePreset` (`Default`; `cwd: Option<String>`, `project_id: Option<String>`, `title: Option<String>`), `new_workspace::NewWorkspaceScreen` (view; `NewWorkspaceScreen::new(client: Entity<Client>, key: String, preset: NewWorkspacePreset, window: &mut Window, cx: &mut Context<NewWorkspaceScreen>) -> NewWorkspaceScreen`, `side_panel_target(&self) -> Option<(String, String)>` as project id and folder, and `composer(&self) -> Entity<Composer>`), and `Shell::open_new_workspace(&mut self, preset: NewWorkspacePreset, window: &mut Window, cx: &mut Context<Shell>)` for TASK-174

Expected result:

- The home Agent launch opens "New workspace" with the project chip, the isolation chip on a git project, the base chip for a worktree, and the 5-line draft composer below.
- Return with text creates a workspace with its first agent, then opens the workspace and the agent tab.
- Cases:
  - Empty text, Local -> one request with `source: directory`, no `agent`, no `firstAgentContext`; the new workspace opens.
  - Text, New worktree, base `main` -> `source: worktree` with `action: branch-off`, `refName: refs/heads/main`, and an 8-hex `worktreeSlug`.
  - A non-git project -> no isolation chip and no base chip; the request is `directory`.
  - `workspaceMultiplicity` false -> the base chip shows and the request is `worktree`.
  - No project -> "Choose a project" under the composer and no request.
  - Two submits while the first is pending -> one request.
  - The daemon answers with an `error` -> its text shows, and the composer unlocks with its text kept.
  - In a workspace, the Agent launch -> the workspace draft tab; Return -> `create_agent` in that folder.
  - The diff stat pill -> the all-files diff tab; a Paseo subagent row in another workspace -> that workspace and the agent tab.

Anti-goal: one submit sends at most one `workspace.create.request`; tripwire: a test of the pure in-flight guard sends one request for two submits and one again after the first resolves; read when the submit path lands and at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-app` -> pass, including every existing test except the 3 removed with `new_agent.rs`, the focus-role test with the new file list, and tests for each Expected result case through the request builder, the isolation and base visibility rule, the base default from the branch, the project default, the slug format, the in-flight guard, and Return with empty text on a non-git project with a forced worktree, which shows the Electron error and builds no request (Review Focus)
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-app` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
- `test ! -e packages/desktop-gpui/crates/app/src/new_agent.rs` -> exit 0
- `rg -n "create_agent_request|create_agent\(" packages/desktop-gpui/crates/app/src/new_workspace.rs` -> no match
- UI check: open the GPUI build and the Electron 6770 build with `open -g`, never in front and with no pointer; take background screenshots of the New workspace screen on a git project in light theme -> the title, chip row, chip height (28 px), gaps (8 px), and composer top match within 2 px; a stale or blank frame is UNVERIFIED (Q-069)
