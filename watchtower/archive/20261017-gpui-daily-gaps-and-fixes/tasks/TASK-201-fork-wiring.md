# TASK-201 Fork into a new tab or a new workspace

Group: S (shared file [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs))
Class: risky

## Brief

Goal: Picking a fork choice in a turn footer opens a draft that carries the chat history, in a new tab or on the New workspace screen, as Electron does.

Change: no fork -> the app builds the fork context and opens a seeded draft.

Boundaries:

- Electron: [packages/app/src/hooks/use-fork-agent.ts](packages/app/src/hooks/use-fork-agent.ts) lines 79-197.
  - The draft gets the `chat_history` attachment from `fork_context` (lines 79-90).
  - The draft is seeded from the source agent: provider, cwd, mode, model, thinking option, and feature values (lines 93-111).
  - Nothing runs by itself. The user types a prompt and sends.
  - Tab target: a draft tab in the same workspace (lines 162-172). No workspace -> the "forkMissingWorkspace" error.
  - Workspace target: the New workspace screen with source directory `checkout.cwd || cwd`, display name, project, and the draft (lines 175-193).
- The composer seed and raw attachment come from TASK-219. Pass them to the draft composer in [packages/desktop-gpui/crates/app/src/workspace_draft.rs](packages/desktop-gpui/crates/app/src/workspace_draft.rs) and [packages/desktop-gpui/crates/app/src/new_workspace.rs](packages/desktop-gpui/crates/app/src/new_workspace.rs).
- While the fork runs, call `TimelineView::set_fork_pending(true)`, and `false` when it ends.
- On error, show Electron's "forkFailed" text with `Shell::show_toast(false, ...)` (TASK-202 lands first).

How:

- Subscribe to `TimelineForkEvent` on each agent pane's timeline.
- Call `Client::fork_context`, then open the draft for the target and seed it.
- Put "build the `DraftSeed` from the source agent" in a pure function with a test.

Files:

- [packages/desktop-gpui/crates/app/src/shell.rs](packages/desktop-gpui/crates/app/src/shell.rs) (subscription, fork flow)
- [packages/desktop-gpui/crates/app/src/workspace_draft.rs](packages/desktop-gpui/crates/app/src/workspace_draft.rs) (pass seed and attachment)
- [packages/desktop-gpui/crates/app/src/new_workspace.rs](packages/desktop-gpui/crates/app/src/new_workspace.rs) (pass seed, source folder, and attachment)

Interfaces:

- Consumes: `ForkTarget` enum `{ Tab, Workspace }` and `TimelineForkEvent::Fork { agent_id: String, target: ForkTarget, boundary: ForkBoundary }` from TASK-208.
- Consumes: `TimelineView::set_fork_pending(&mut self, pending: bool, cx: &mut Context<TimelineView>)` from TASK-208.
- Consumes: `ForkCursor { epoch: String, seq: u64 }` and `ForkBoundary { cursor: Option<ForkCursor>, message_id: Option<String> }` from TASK-205.
- Consumes: `Client::fork_supported(&self) -> bool` and `Client::fork_cursor_supported(&self) -> bool` from TASK-205.
- Consumes: `ForkContext { attachment: serde_json::Value, title: Option<String>, item_count: u32 }` and `Client::fork_context(&mut self, agent_id: &str, boundary: ForkBoundary, cx: &mut Context<Client>) -> Task<anyhow::Result<ForkContext>>` from TASK-205.
- Consumes: `DraftSeed { provider: Option<String>, model: Option<String>, mode: Option<String>, thinking_option_id: Option<String>, features: Option<serde_json::Value> }` from TASK-219.
- Consumes: `Composer::set_draft_seed(&mut self, seed: DraftSeed, window: &mut Window, cx: &mut Context<Composer>)` and `Composer::add_raw_attachment(&mut self, label: String, attachment: serde_json::Value, cx: &mut Context<Composer>)` from TASK-219.
- Consumes: `Shell::show_toast(&mut self, ok: bool, message: String, cx: &mut Context<Self>)` from TASK-202.

Expected result:

- Fork in a new tab -> a draft tab opens in the same workspace with a chat history chip and the source agent's provider and model.
- Fork in a new workspace -> the New workspace screen opens with the source folder and the chip.
- Send a prompt from the fork draft -> the new agent gets the chat history attachment.
- A fork on an agent with no workspace in a new tab -> the "forkMissingWorkspace" error toast.
- A fork whose context is empty -> the "forkFailed" toast and no draft.

Anti-goal: creating a normal agent from a draft must not change; tripwire; the TASK-219 unchanged payload test still passes, read before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-app` -> passes, including the seed build test.
- `./cargo.sh test -p bachuc-composer` -> the unchanged payload test passes.
- UI check on live: fork a mock agent turn into a new tab -> a draft with the chip shows; send "continue" -> the new agent starts.
- Owner hand test: fork into a new workspace from a real thread.
- Anti-goal: the unchanged payload test -> passes at both reads.
