# TASK-271 Read-only timeline for a provider subagent

Group: B (the `bachuc-timeline` crate; shares `lib.rs` with TASK-265, TASK-266, TASK-267, and TASK-269)
Class: risky

## Brief

Goal: The GPUI timeline view can show a provider subagent's timeline read-only, with the child's running state.

Change: `TimelineView` serves only real agents -> it also serves a provider subagent key, with every action that needs a real agent hidden.

Design: [packages/app/src/panels/provider-subagent-panel.tsx](packages/app/src/panels/provider-subagent-panel.tsx), which renders `AgentStreamView` with `readOnly`, no permissions, and `isActive` from the child's status.

Boundaries:

- Follows [ADR-0051](watchtower/decisions/ADR-0051-read-only-provider-subagent-tab-over-jump-to-parent.md).
- The view reads the rows through `Client::timeline(key)` and pages through `Client::load_older(key)`, from TASK-270.
- Running state: the child runs when its row in `Client::subagents(parent)` has status `running`, as Electron reads `descriptor.status === "running"`. This drives the turn grouping, the clock, and the TASK-266 running footer.
- Hide what needs a real agent: the fork menu in every footer, the rewind menu and actions under user messages, and permission rows. Keep copy and links.
- The view uses its ID in more places. Each must be safe for a key or skipped in read-only mode:
  - `view_agent` at creation (`lib.rs:480`) and again on `Connected` (`:435-437`).
  - `list_prompts(&agent_id)` (`:720-723`), a daemon request: skip it.
  - `timeline_epoch` (`:732`), `agent_labels` (`:796`), `rewind_modes` (`:2022`), and the footer fork boundary (`:588`).
  - The fork event (`:630`) and the rewind event (`:1257`): never emitted in read-only mode.
  - `turn_active` (`:786`) reads `agents()`, where a key never runs. Use the child's row instead.
  - `projector.sync` (`:828`) passes the ID to the plugin host; a key there is harmless.
- The fork menu is built at `lib.rs:1967`, the rewind menu at `:2022`, and permission rows from `state.pending_permissions` (`:818`, pushed at `:914`).
- `Client::subagents(parent)` rows carry the raw provider `status` (`packages/desktop-gpui/crates/client/src/core.rs:1104`) and no parent ID; get the parent from `parse_provider_subagent_key`. `DiagramRenderer` is `bachuc_diagram::DiagramRenderer` (`lib.rs:33`).
- Planner call: the TASK-266 running footer shows the loader and the clock but no fork button in read-only mode.

How:

- Add a read-only constructor that takes the key.
- Read the running state from the parent's subagent rows in read-only mode.
- Skip the fork, rewind, and permission parts in read-only mode.
- Put each read-only rule in a `pub` pure function, so the test file can reach it, and test it in [packages/desktop-gpui/crates/timeline/tests/read_only.rs](packages/desktop-gpui/crates/timeline/tests/read_only.rs).

Files:

- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (read-only constructor, running state, hidden actions)
- [packages/desktop-gpui/crates/timeline/tests/read_only.rs](packages/desktop-gpui/crates/timeline/tests/read_only.rs) (new tests)

Interfaces:

- Consumes: `bachuc_client::provider_subagent_key(parent_agent_id: &str, subagent_id: &str) -> String` from TASK-270
- Consumes: `bachuc_client::parse_provider_subagent_key(key: &str) -> Option<(String, String)>` from TASK-270
- Consumes: `Client::view_agent`, `Client::unview_agent`, `Client::timeline`, and `Client::load_older` accept a provider subagent key as `agent_id`, from TASK-270
- Produces: `TimelineView::read_only(client: Entity<Client>, key: String, diagrams: Entity<DiagramRenderer>, window: &mut Window, cx: &mut Context<TimelineView>) -> TimelineView` for TASK-272

Expected result:

- A read-only view shows the child's steps, notes, and tool calls, grouped as an agent turn.
- While the child runs, the step list shows its running state and the running footer shows the loader and clock.
- No fork, rewind, or permission control shows.
- Scrolling up loads older rows.
- A normal agent timeline is unchanged.

Anti-goal: the normal agent timeline keeps the same rows and actions; tripwire; read from every `bachuc-timeline` test and a read check of each changed call site, before changes and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> the new `read_only` tests and every old test pass.
- `tests/read_only.rs`: the running rule -> a child row with status `running` gives a running turn; `completed`, `failed`, or a missing row gives a finished turn.
- `tests/read_only.rs`: the action rule -> read-only mode gives no fork, no rewind, and no permission rows; agent mode keeps them.
- Read check: in read-only mode no daemon request carries the key as `agentId`.
- Screen check: done in TASK-272, which opens the view.
- Anti-goal: the old test count before changes equals the old tests still passing at completion; record both counts.
