# TASK-173 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/app/src/new_agent.rs`: deleted, with its 3 tests.
- `packages/desktop-gpui/crates/app/src/new_workspace.rs` (new): the New workspace screen and its pure rules (`visibility`, `default_base_ref`, `branch_name_from_ref`, `worktree_slug`, `project_options`, `default_project`, `base_items`, `base_rows`, `is_empty_submission`, `build_request`, `SubmitGuard`), with 13 tests.
- `packages/desktop-gpui/crates/app/src/workspace_draft.rs` (new): the workspace draft tab and `draft_agent`, with 2 tests.
- `packages/desktop-gpui/crates/app/src/shell.rs`: `PaneContent::Draft` holds `WorkspaceDraftPane`; new `PaneContent::NewWorkspace`; `open_draft` opens the screen without a workspace folder and the draft tab in a workspace; `open_new_workspace`, `open_new_workspace_screen`, `open_pending` (opens the created workspace once listed, then the agent tab), and `open_workspace_changes`; the agent composer routes `OpenChanges` and `OpenAgent`.
- `packages/desktop-gpui/crates/app/src/main.rs`: the new modules.
- `packages/desktop-gpui/crates/app/src/actions.rs`: the focus-role test lists `new_workspace.rs` and `workspace_draft.rs` instead of `new_agent.rs`.
- `packages/desktop-gpui/crates/app/Cargo.toml`: `uuid = { version = "1", features = ["v4"] }`.
- `packages/desktop-gpui/Cargo.lock` (generated): `uuid` on `bachuc-app`.

## Contract

- Produces as specified: `new_workspace::NewWorkspacePreset` (`Default`; `cwd`, `project_id`, `title`), `NewWorkspaceScreen::new(client, key, preset, window, cx)`, `side_panel_target(&self) -> Option<(String, String)>`, `composer(&self) -> Entity<Composer>`, and `Shell::open_new_workspace(&mut self, preset, window, cx)`.
- Consumes the TASK-169, TASK-171, and TASK-172 names as specified: `CreateWorkspace`, `WorkspaceSource`, `WorktreeAction`, `InitialAgent`, `CreatedWorkspace`, `Client::create_workspace`, `create_agent_with`, `branch_suggestions`, `validate_branch`, `add_project`, `checkout_details`, `composer_dock`, `Composer::new_draft`, `DraftOptions`, `SubmitIcon`, `DraftSubmission`, `draft_submission`, `set_draft_cwd`, `set_submitting`, and `ComposerEvent::{DraftSubmit, OpenChanges, OpenAgent, ReplaceWithDraft}`. `Composer::clear` is not needed: a success replaces the screen or tab.
- Row: project, isolation (only when a worktree is possible), base (without multiplicity or with a worktree). No host picker and no launch control (planner calls). "Add project" is the last project row: folder dialog, `add_project`, then the result is chosen.
- One request per submit: empty text with no images or attachments gives `agent: None` and no prompt; text gives `agent: Some(InitialAgent)` and `first_agent_prompt: Some(text)`. `worktreeSlug` is the first 8 hex digits of a v4 UUID, made once per screen.
- On `Ok` the shell waits for `DirectoryEvent::ProjectsChanged` until the client lists the workspace, opens it, puts a new tab in place of the spent screen, and opens or focuses the agent tab (in place of the workspace's empty new tab).
- The workspace draft tab is a bottom `composer_dock` with a 1-line draft composer (arrow icon, no empty submit) and creates through `create_agent_with`.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-app` -> exit 0; 56 passed, 0 failed (`/tmp/wt-d-test2.txt`, 23:39). All 28 baseline tests besides the 3 `new_agent.rs` tests pass by name (`comm` against the HEAD test list), including `every_focus_target_in_this_crate_has_a_role` with the new file list. New tests: empty Local -> `directory` with no agent and no prompt; text, New worktree, base `main` -> `worktree`, `BranchOff`, `refs/heads/main`, slug `a1b2c3d4`, first agent with config, features, and env; images alone create the agent; non-git -> no isolation or base chip and a `directory` request; the visibility rule for Local and worktree; multiplicity false -> base chip and a `worktree` request; empty text on a non-git project with a forced worktree -> `Err("Failed to create worktree")`; no project -> "Choose a project"; text without a config -> "Select a model"; the in-flight guard; the base default and ref names; the 8-hex slug; the project default; base rows with a typed branch only when it exists; the draft tab request with mode, thinking, features, env, images, and attachments.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (`/tmp/wt-d-build5.txt`); one warning, `next_theme` unused in `state.rs` (see TASK-174).
- `test ! -e packages/desktop-gpui/crates/app/src/new_agent.rs` -> exit 0.
- `rg -n "create_agent_request|create_agent\(" packages/desktop-gpui/crates/app/src/new_workspace.rs` -> no match (exit 1).
- UI check (New workspace screen against Electron 6770: title, chip row, 28 px chips, 8 px gaps, composer top) -> DEFERRED (UI checker).
- The view wiring (error text under the composer, unlock on `Err` with the text kept, pickers, folder dialog, opening the created workspace) needs a window: UNVERIFIED (unattended): code tests only. Live daemon down: no live check.
- GitNexus `impact open_draft` -> HIGH, 8 impacted, all inside `crates/app/src/shell.rs` (`launch`, `run_action`, the `ReplaceWithDraft` route); `PaneContent`, `DraftPane`, `render_header` -> ambiguous or UNKNOWN; `rg` confirmed every caller is in `crates/app`.

## Anti-goal

- Checkpoint 1, when the submit path landed: `the_guard_sends_one_request_for_two_submits_and_again_after_the_first_resolves` passed (1 request for 2 submits, 2 after `finish`); source `/tmp/wt-d-test1.txt`; 2026-10-16 23:26.
- Checkpoint 2, at completion: the same test passed; source `/tmp/wt-d-test2.txt`; 2026-10-16 23:39.
- Tripwire: not tripped. The guard is taken before the request is built and released on a build error or on the answer.

## Lessons

- Safe defaults taken (see questions): the forced-worktree error on a non-git project is the Electron string "Failed to create worktree" (`newWorkspace.errors.createWorktreeFailed`), shown before any request; the project and isolation chips use the `Folder` and `GitBranch` icons, not a project favicon or initial; the base rows leave out the divergence text.
- The client gets no upstream ref, so the base default is always `refs/heads/<current branch>`.
- Builds failed twice only inside other groups' crates (`crates/sidebar`, `crates/timeline`, and a half-created `plugins/edit-diffs`); a retry after 4 to 7 minutes passed.
