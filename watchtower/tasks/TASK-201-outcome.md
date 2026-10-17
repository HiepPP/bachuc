# TASK-201 Outcome

## Status

DONE (code). The UI check and the owner hand test are DEFERRED.

## Changed

- `packages/desktop-gpui/crates/app/src/shell.rs`:
  - `open_agent_pane` subscribes to `TimelineForkEvent` on each agent pane's timeline and calls the new `Shell::fork`. Its signature and other subscriptions did not change.
  - `fork` follows `useForkAgent`. It builds the seed from the source agent, decides the target, sets `set_fork_pending(true)`, calls `Client::fork_context`, and sets `set_fork_pending(false)` when the reply comes. An error shows a `show_toast(false, ...)` and opens no draft. Success opens the seeded draft. Nothing is sent by itself.
  - Tab target: a new draft tab in the source tab's workspace, in the agent's `cwd` (Electron's `setup.cwd`), else the workspace folder. A tab in the home view, or with no folder, gets "This agent is not in a workspace." before any request.
  - Workspace target: `open_new_workspace` with a preset of the source workspace's folder and project (the agent's `workspace_id`, else the pane's workspace), else the agent's `cwd`.
  - `open_draft` now calls the extracted `open_draft_pane`, which returns the draft entity. `open_new_workspace` and `open_new_workspace_screen` now return the screen entity. Their behavior did not change.
  - Pure rules with tests: `fork_draft_seed` (`buildForkDraftSetup`), `fork_tab_cwd`, `fork_workspace_preset`, `fork_attachment_label`, and `fork_error`. The constants are `FORK_FAILED`, `FORK_MISSING_WORKSPACE`, and `FORK_ATTACHMENT_LABEL`.
- `packages/desktop-gpui/crates/app/src/workspace_draft.rs`: `WorkspaceDraftPane::seed_fork(seed: Option<DraftSeed>, label, attachment, window, cx)`.
- `packages/desktop-gpui/crates/app/src/new_workspace.rs`: `NewWorkspaceScreen::seed_fork` with the same shape. The source folder comes in the `NewWorkspacePreset`.

## Contract

- Consumes, with exact names: `ForkTarget`, `TimelineForkEvent::Fork { agent_id, target, boundary }`, `TimelineView::set_fork_pending`, `ForkBoundary` (passed through unchanged), `ForkContext`, `Client::fork_context`, `DraftSeed`, `Composer::set_draft_seed`, `Composer::add_raw_attachment`, and `Shell::show_toast`. `fork_supported` is not called: the timeline shows the button only when it is true, and `fork_context` fails with "Update the host to use this." without the feature. `fork_cursor_supported` is not called, because the boundary is the message id only (Q-129).
- Seed: `fork_draft_seed` gives `None` for an agent with no provider, so the draft keeps its own settings (Electron passes no setup). Model is `AgentSummary.model`, else `AgentControls.current_model` (Electron's `agent.model ?? runtimeInfo.model`). Mode is `AgentControls.current_mode`. Thinking is `ThinkingControls.current`. Features are a JSON object of each feature's value (a bool for a toggle, an option id or null for a select).
- `fork_context.attachment` goes unchanged to `add_raw_attachment`. The label is `ForkContext.title` (the attachment title), else "Text attachment".
- `set_draft_seed` runs right after the draft is made. The composer reads the providers from the client at construction, so a connected fork has them.
- `create_workspace` was not touched. `on_client_event` keeps the `ClientEvent::TimelineChanged { .. }` early return.

## Verified

- Anti-goal read 1, before any edit: `./cargo.sh test -p bachuc-composer --lib a_plain_draft_payload_is_unchanged` -> 1 passed.
- Anti-goal read 2, at completion: `./cargo.sh test -p bachuc-composer` -> 113 passed, including `a_plain_draft_payload_is_unchanged`.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-app` -> 74 passed. New: `a_fork_seed_carries_the_source_agent_settings` (the seed build test), `a_fork_tab_needs_a_workspace_and_a_folder`, `a_fork_workspace_opens_on_the_source_folder_and_project`, and `a_fork_labels_its_chip_and_names_its_failure`.
- `./cargo.sh build -p bachuc-app` -> exit 0. The only warning is the existing unused `next_theme` in `state.rs`.
- `./cargo.sh fmt --check -p bachuc-app` -> exit 0.
- `./cargo.sh clippy -p bachuc-app --tests` -> no warnings in the changed files.
- `./cargo.sh metadata --format-version 1 > /dev/null` -> exit 0.
- GitNexus impact (index 45 commits stale, advisory): `open_agent_pane` HIGH (4 direct callers). Its signature and existing behavior did not change; only one subscription was added. `open_draft` LOW, `open_new_workspace` LOW. `rg` finds one caller of `open_new_workspace` and 3 of `open_draft`, all statements, so the new return values need no caller change.
- UI check on live (fork a mock agent turn into a new tab, see the chip, send "continue") -> DEFERRED (UI checker).
- Owner hand test (fork into a new workspace from a real thread) -> DEFERRED (owner).

## Lessons

- The GPUI client has no `projectPlacement`. The workspace target uses the source workspace's folder and project from the project tree as Electron's `checkout.cwd`. The route's `displayName` is the project name, and the screen shows the project by its id, so `NewWorkspacePreset.title` stays unset. Setting `title` would name the new workspace, which Electron does not do.
- The fork error toast shows the error text, as Electron's `toErrorMessage(error) || forkFailed`. The client's "Nothing to fork" (a null attachment) maps to "Failed to fork chat", because Electron throws `forkFailed` in that case.
- On the New workspace screen the draft cwd stays the project root (`sync_project`), not the agent's cwd. Electron remaps the fork cwd onto the created workspace only after the create.
- A fork from a draft tab opened by the workspace target goes through the normal draft submit. The raw attachment is in `DraftSubmission.attachments`, so the created agent gets the chat history.
