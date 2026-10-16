# TASK-138 Timeline view with the turn step list

Group: D (writes only `crates/timeline`)
Class: risky

## Brief

Goal: Add the crate `bachuc-timeline` with a virtualized timeline view that shows an agent's turns like the current app: full markdown, the always-open turn step list, edit diffs, diagrams, and permission prompts.

Change: the prototype's flat rows -> a timeline that follows the turn design ADRs.

Design: [packages/app/src/tool-calls/detail-level/turn](packages/app/src/tool-calls/detail-level/turn) and [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts)

Boundaries:

- Follows [ADR-0018](watchtower/decisions/ADR-0018-all-trailing-messages-over-last-message-as-answer.md): all trailing assistant messages form the turn answer.
- Follows [ADR-0019](watchtower/decisions/ADR-0019-always-open-step-list-over-folding-group.md): the step list is always open, with no fold or chevron.
- Follows [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md) and [ADR-0022](watchtower/decisions/ADR-0022-scheme-label-tones-in-the-theme-over-color-scheme-branches.md): label colors come from `bachuc_theme::turn_label_colors`.
- Follows [ADR-0021](watchtower/decisions/ADR-0021-note-opens-its-step-over-tools-before-the-note.md): a note opens its step.
- Follows [ADR-0037](watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md): ` ```mermaid ` blocks and math blocks render through `bachuc-diagram` as images.
- Planner call: write Rust grouping over the projected entries from `TimelineState`. Port the rules of `prepareTurnHistory` and `groupLiveTurns` in [packages/app/src/tool-calls/detail-level/grouping.ts](packages/app/src/tool-calls/detail-level/grouping.ts) and `buildTurnGroup` in [packages/app/src/tool-calls/detail-level/turn/model.ts](packages/app/src/tool-calls/detail-level/turn/model.ts). Do not port the app's stream reducer. No test calls these functions directly: the TypeScript tests in [packages/app/src/tool-calls/detail-level/turn/model.test.ts](packages/app/src/tool-calls/detail-level/turn/model.test.ts) go through `createStreamPresentation`. Rewrite each of their cases as projected-entry fixtures, and skip the case that needs the edit-diffs plugin presentation. Do not change the TypeScript files.
- Use a virtualized `list` with bottom alignment. Follow the bottom only when the user is at the bottom. Load older pages with `load_older` when the user scrolls near the top, and keep the scroll position when they arrive.
- Use gpui-kit markdown with code highlighting through its tree-sitter features.
- Edit tool calls show the unified diff with added and removed line colors.
- A pending permission shows Allow and Deny buttons that call `respond_permission`.
- Give rows, steps, messages, and buttons `a11y` roles and labels.

How:

- Port the turn grouping into Rust with tests.
- Build row views: user message, answer markdown, step list with labels, tool detail, diff, diagram, permission.
- Wire the list to `TimelineChanged` events and the 50 ms batched client state.
- Add `examples/timeline.rs` that connects to a host and shows one agent.

Files:

- [packages/desktop-gpui/crates/timeline/Cargo.toml](packages/desktop-gpui/crates/timeline/Cargo.toml) (new crate `bachuc-timeline`)
- [packages/desktop-gpui/crates/timeline/src](packages/desktop-gpui/crates/timeline/src) (grouping, rows, view)
- [packages/desktop-gpui/crates/timeline/tests](packages/desktop-gpui/crates/timeline/tests) (grouping tests ported from the TypeScript cases)
- [packages/desktop-gpui/crates/timeline/examples/timeline.rs](packages/desktop-gpui/crates/timeline/examples/timeline.rs) (example)

Interfaces:

- Consumes: `bachuc_client::Client` (a GPUI entity) with `Client::connect(host: HostConfig, cx: &mut gpui_kit::App) -> gpui_kit::Entity<Client>` from TASK-136
- Consumes: `bachuc_client::ClientEvent` (enum `Connected`, `Disconnected { reason: String }`, `AgentsChanged`, `TimelineChanged { agent_id: String }`, `PermissionRequested { agent_id: String }`, `AgentFinished { agent_id: String }`) emitted by `Client` from TASK-136
- Consumes: `Client::view_agent(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::unview_agent(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::timeline(&self, agent_id: &str) -> Option<&TimelineState>`, and `Client::load_older(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)` from TASK-136
- Consumes: `TimelineState { entries: Vec<serde_json::Value>, has_older: bool, version: u64, pending_permissions: Vec<serde_json::Value> }` (entries are projected timeline entries in order) from TASK-136
- Consumes: `Client::respond_permission(&mut self, agent_id: &str, request_id: &str, allow: bool, cx: &mut gpui_kit::Context<Client>)` from TASK-136
- Consumes: `bachuc_diagram::DiagramRenderer` (a GPUI entity) with `DiagramRenderer::new(window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<DiagramRenderer>) -> DiagramRenderer` from TASK-137
- Consumes: `bachuc_diagram::DiagramKind` (enum `Mermaid`, `Math`) and `DiagramRenderer::render(&mut self, kind: DiagramKind, source: &str, cx: &mut gpui_kit::Context<DiagramRenderer>) -> DiagramState` with `DiagramState` (enum `Pending`, `Ready(std::sync::Arc<gpui_kit::Image>)`, `Failed(String)`) from TASK-137
- Consumes: `DiagramRenderer` emits `DiagramReady { kind: DiagramKind, source: String }` when a pending render finishes, from TASK-137
- Consumes: `bachuc_theme::Scheme` (enum `Light`, `Dark`) and `bachuc_theme::scheme(cx: &gpui_kit::App) -> Scheme` from TASK-134
- Consumes: `bachuc_theme::TurnLabel` (enum `Explore`, `Edit`, `Verify`) and `bachuc_theme::turn_label_colors(label: TurnLabel, scheme: Scheme) -> TurnLabelColors` with fields `segment: Hsla`, `text: Hsla`, `border: Hsla` from TASK-134
- Produces: `bachuc_timeline::TimelineView` (a GPUI view) with `TimelineView::new(client: gpui_kit::Entity<bachuc_client::Client>, agent_id: String, diagrams: gpui_kit::Entity<bachuc_diagram::DiagramRenderer>, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<TimelineView>) -> TimelineView` for TASK-141

Expected result:

- The example shows a mock agent with turns grouped like the current app: steps with Explore, Edit, and Verify labels, and the answer below.
- Rust grouping tests cover the same cases as the TypeScript tests in [packages/app/src/tool-calls/detail-level/turn/model.test.ts](packages/app/src/tool-calls/detail-level/turn/model.test.ts), except the edit-diffs plugin case, and pass.
- Cases:
  - At the bottom, new items keep the view at the bottom.
  - Scrolled up, new items do not move the view.
  - Near the top with `has_older`, an older page loads and the visible row stays in place.
  - An empty timeline shows an empty state, not a blank pane.
  - A pending permission shows Allow and Deny. After a click the buttons go away when the daemon resolves it.
  - A mermaid block shows a diagram image; a broken one shows the code block.
  - A display math block (`$$...$$`) shows a formula image.

Anti-goal: the example stays within 10% CPU and 250 MB phys_footprint, WebKit helpers included, on the mock stream in the foreground for 60 seconds; tripwire; read with `measure.sh 60` before the turn grouping lands, after it lands, after diagrams land, and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> grouping tests pass
- Agent check with a live mock agent: `bundle.sh example timeline BachucTimeline io.github.hieppp.bachuc-timeline`, open it with the agent id, then use `cua_repl` `getAXState` and screenshots -> labeled steps, answer markdown with highlighted code, and diffs show; the AX tree lists messages and steps by label
- Agent check: scroll up during the stream with `cua_repl` -> the view stays where the user left it; scroll back down -> it follows again
- Agent check with a long agent history on live (more than one page) -> scrolling to the top loads older items without a jump
- `packages/desktop-gpui/cargo.sh test -p bachuc-timeline broken_mermaid` -> a test row with a broken ` ```mermaid ` block renders as a code block when the renderer returns `Failed`
- `packages/desktop-gpui/cargo.sh test -p bachuc-timeline math_block` -> a `$$...$$` block becomes a `DiagramKind::Math` request
- Agent check: send a prompt to a live `claude` agent in `default` permission mode that needs a tool approval -> Allow and Deny show and work
- Anti-goal: `measure.sh 60 tl="BachucTimeline.app" wk="pids:<WebKit pids>"` -> CPU at most 10.0 and footprint sum at most 250
