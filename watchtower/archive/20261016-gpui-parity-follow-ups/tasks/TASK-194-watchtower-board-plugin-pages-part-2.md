# TASK-194 watchtower-board plugin pages, part 2

Group: S (writes `plugins/watchtower-board`; runs after TASK-190 in the same group)
Class: risky

## Brief

Goal: Finish the native watchtower-board crate with the full overlay overview: header, lifecycle stepper, autorun strip, pace line, plan tree with the lead row, Needs you with its actions, manual checks, decisions with the decision modal, archived plans, and the wide, stacked, and narrow layouts.

Change: after TASK-190 the Watchtower overlay shows only the rail cards in one column -> it shows the Electron overview, and its Needs you actions send prompts to the workspace agent.

Design: the Electron client is the look and the behavior to match. Paths are under [hiep-plugins/plugins/watchtower-board](hiep-plugins/plugins/watchtower-board).

- Overview: [client/overview.tsx](hiep-plugins/plugins/watchtower-board/client/overview.tsx) (breakpoints 34-40, `PaceLine` 60-94, data and derived values 105-190, header 196-237, error state 279-311, loading skeleton 312-356, no-plan state 357-388, body 389-443, grid 444-480, layout 483-526).
- Cells: [client/overview-cells.tsx](hiep-plugins/plugins/watchtower-board/client/overview-cells.tsx) (`phaseIcons` 125-131, `Segments` 133-156, `LifecycleStepper` 158-265, `ManualChecksCell` 267-321, `HistoryRow` 323-380, `DecisionsCell` 382-417, `ArchivedPlansCell` 419-444) and [client/run-cells.tsx](hiep-plugins/plugins/watchtower-board/client/run-cells.tsx) (`AutorunStrip` 18-65).
- Needs you: [client/needs-you.tsx](hiep-plugins/plugins/watchtower-board/client/needs-you.tsx) (agent note 73-78 and 252-277, `run` 111-120, answer form 130-174, ADR two-step reject 198-232, row 283-347) and the send flow in [client/agent-target.ts](hiep-plugins/plugins/watchtower-board/client/agent-target.ts) (46-63).
- Plan tree: [client/plan-tree-view.tsx](hiep-plugins/plugins/watchtower-board/client/plan-tree-view.tsx) (constants 11-16, `ago` 28-34, spinner 36-86, `StateIcon` 90-118, secondary line 146-164, `TaskRow` 166-296, `LeadRow` 298-385, waves 443-488).
- Decision modal: [client/decision-modal.tsx](hiep-plugins/plugins/watchtower-board/client/decision-modal.tsx) (block styles 22-66, query and states 75-142).

Boundaries:

- Follows [ADR-0045](watchtower/decisions/ADR-0045-native-rust-plugin-clients-over-webview-panes-or-shared-logic.md): native Rust client, unchanged server part.
- Same crate and dependency rules as TASK-190: `bachuc-plugin-watchtower-board`, no view crate, no other plugin crate, `gpui-kit.workspace = true` with no `features` key, and `plugin()` stays the only public item. Replace the overlay body in `src/page.rs`; keep TASK-190's workspace choice and placeholders. Add no contribution and no request path: every read and send goes through `WatchtowerStore`.
- Initial task (lead, from the spec check): once the board loads, the page selects the task that TASK-190's store kept from `params.task`, as Electron does (`client/page.tsx:18,74`). An unknown id selects none.
- Load rules copy Electron: the overview and the board load once per workspace and again only on the header Refresh, which refreshes both. The decision modal loads only while it is open, with no retry.
- Derived data copies `overview.tsx:125-190`: builders from `WatchtowerStore::builders`, the live board, the plan tree, the timeline, the answer-by deadlines appended to Needs you rows unless the run is stopped, and tree times set to `None` when there is no run.
- Run state text, in this order: no run -> "No autorun yet"; stopped -> "Run stopped at HH:MM"; finished -> "Run finished"; a start clock -> "Autorun since HH:MM"; else "Autorun running". It is the `aside` of the Plan cell. A stopped run hides the estimated finish in the pace line.
- Needs you actions copy Electron: "Answer" opens the answer form (Enter or "Send", disabled when blank, "Cancel" clears); "Use default" shows only with a default; ADR rows show "View" (only for an id that passes `is_decision_id`), "Accept", and a two-step "Reject" then "Confirm reject" with "Cancel"; the checks row shows "Mark done". Buttons are disabled with no agent, a busy agent, or a send in flight.
- Each action calls `WatchtowerStore::send` with the prompt from the matching TASK-190 builder. On success it calls `PluginContext::show_message("Sent to <name>", MessageLevel::Info, cx)` and opens the agent with `PluginContext::open_agent`; on an error it calls `show_message(<error text>, MessageLevel::Error, cx)`, as Electron's toasts do (`agent-target.ts:55-59`).
- The agent note shows "No agent in this workspace" or "Agent busy" in `status_warning`, else "Actions go to <name>" with the name as an `accent` link that opens the agent.
- The plan tree secondary line shows "active <ago>" from the builder's `active_ago`, which TASK-190 computes from `Client::agent_updated_at`.
- Decision modal: an overlay drawn by the page (a backdrop and a centered panel; Escape and a backdrop click close it), titled "<id> <title>" with `IconName::Scale`, padding 20, gap 8. It shows the file path in `MONO` 11 px and the `parse_markdown` blocks: heading sizes 17, 15, and 13.5; bullets indented `depth * 14` with a muted "•"; code on `surface2`, radius 8, `MONO` 11.5 px. Inline marks stay plain text. The error state shows "Could not load <id>. <msg>" with Retry.
- Icons (TASK-195 and existing): decisions `Scale` (warning color for "proposed"), checks row `ClipboardCheck`, question rows `MessageCircleQuestion`, waiting state `Hand`, running state `LoaderCircle` rotating once per 1,000 ms, danger rows `CircleAlert`, lead row `Bot`, archived `Archive`, refresh `RefreshCw`, no plan `ListChecks`. The spinner requests frames only while a running row is in view.
- Layout: measure the page width from its bounds and keep Electron's breakpoints: stacked below 720, rail 360 wide on `surface1` with a right border, wide grid from 900 of main width, narrow below 520, compact stepper below 720, gap 14, padding 12 stacked and 24 otherwise.
- Look: tokens from `bachuc_theme` with Electron's names; copy the measures in Design line by line: stepper radius 10 with a 2 px accent bar and 6% accent overlay on the current step, implement segments 4 px, plan-tree spine 2 px at inset 8 with 14 px gap and 16 px ticks, task rows padding 7 by 8 and radius 8, lead row radius 10, skeleton bars 8 px high at 12% muted. Text line height near 1.2 unless Electron sets one.
- Lists render lazily: the main column and the stacked page are each one GPUI `list`, one item per cell or row group; plan-tree waves, task rows, Needs you rows, and log rows are their own items. `ShowMore` keeps its limit of 5.
- Every button, row, link, and input gets a role and a label; decision rows use the label "Open <id> <title>". Never toggle `display` from hover.
- Keep each rule in a pure function and unit-test it (`MEMORY.md`, 2026-10-16); view code is checked only by reading it.

Files:

- [packages/desktop-gpui/plugins/watchtower-board](packages/desktop-gpui/plugins/watchtower-board) (`src/page.rs`; new `src/overview.rs`, `src/cells.rs`, `src/run_cells.rs`, `src/plan_tree_view.rs`, `src/needs_you.rs`, `src/decision_modal.rs`, and `src/model/overview_view.rs` with the derived-data and text rules and their unit tests)
- [packages/desktop-gpui/Cargo.lock](packages/desktop-gpui/Cargo.lock) (generated)

Interfaces:

- Consumes: crate-private `crate::store::WatchtowerStore` (entity, one per activation, created in `contribute` in `src/lib.rs` and captured by each contribution's closures; `board(&self, workspace_id: &str) -> Loaded<Board>`, `project_board(&self, project_id: &str) -> Loaded<Board>`, `overview(&self, workspace_id: &str) -> Loaded<Overview>`, `refresh(&mut self, workspace_id: &str, cx: &mut Context<WatchtowerStore>)`, `decision(&mut self, workspace_id: &str, id: &str, cx: &mut Context<WatchtowerStore>) -> Task<anyhow::Result<DecisionDetail>>`, `builders(&self, workspace_id: &str, tasks: &[OverviewTask], now_ms: i64, cx: &App) -> BuilderMatch`, `agent_target(&self, workspace_id: &str, cx: &App) -> AgentTarget`, `send(&mut self, workspace_id: &str, prompt: String, cx: &mut Context<WatchtowerStore>) -> Task<anyhow::Result<(String, String)>>` with the agent id and name) and `crate::model::Loaded<T>` (enum `Loading`, `Ready(T)`, `Error(String)`) from TASK-190
- Consumes: crate-private `crate::model` types `Board`, `Task`, `Question`, `Run`, `RunEntry`, `Overview`, `OverviewTask`, `OverviewRun`, `OverviewRunEntry`, `OverviewQuestion`, `DecisionDetail`, `BuilderMatch`, `AgentTarget` (enum `Ready { id: String, name: String }`, `Busy`, `NoAgent`), `Block` (the `parse_markdown` block), and the pure functions `parse_markdown`, `task_group`, `with_live_status`, `dashboard_summary`, `questions_for`, `task_title`, `answer_prompt`, `default_answer_prompt`, `accept_decision_prompt`, `reject_decision_prompt`, `manual_checks_prompt`, `lifecycle_phases`, `needs_you`, `live_task_ids`, `build_plan_tree`, `build_timeline`, `clock_at`, `commit_shas`, `ago`, and `is_decision_id`, plus the views `crate::cell::overview_cell`, `crate::cell::show_more`, `crate::rail::{ProgressCard, WaitingCard, TasksCard, AutorunLogCard}`, and `crate::task_list::TaskList`, from TASK-190
- Consumes: `bachuc_plugin_host::PluginContext` (`Clone`; `plugin_id(&self) -> &'static str`, `client(&self, cx: &App) -> Option<Entity<Client>>`, `rpc(&self, method: &str, input: serde_json::Value, cx: &mut App) -> Task<anyhow::Result<serde_json::Value>>`, `rpc_as<I: serde::Serialize, O: serde::de::DeserializeOwned + Send + 'static>(&self, method: &str, input: &I, cx: &mut App) -> Task<anyhow::Result<O>>`, `settings(&self, settings_id: &str, cx: &mut App) -> Entity<PluginSettings>`, `active_workspace_id(&self, cx: &App) -> Option<String>`, `active_agent_id(&self, cx: &App) -> Option<String>`, `open_surface(&self, surface_id: &str, options: OpenSurfaceOptions, cx: &mut App)`, `open_panel(&self, panel_id: &str, target: PanelTarget, cx: &mut App)`, `open_settings(&self, settings_id: &str, cx: &mut App)`, `set_composer_text(&self, agent_id: &str, text: String, cx: &mut App)`, `add_composer_attachment(&self, attachment: ComposerAttachment, cx: &mut App)` (sets `plugin_id`), `reveal_timeline_passage(&self, passage: TimelinePassage, cx: &mut App)`, `open_new_workspace(&self, request: NewWorkspaceRequest, cx: &mut App)`, `open_plugins_page(&self, cx: &mut App)`, `open_agent(&self, agent_id: &str, cx: &mut App)`, `open_workspace(&self, workspace_id: &str, cx: &mut App)`, `notify(&self, cx: &mut App)`, `invalidate_timeline(&self, cx: &mut App)`, `sidebar_filter_changed(&self, direction: Option<SwipeDirection>, cx: &mut App)`) and `bachuc_plugin_host::OpenSurfaceOptions` (`Default`; `plugin_id: Option<String>`, `presentation: SurfacePresentation`, `params: std::collections::BTreeMap<String, String>`) from TASK-170
- Consumes: `PluginContext::show_message(&self, message: SharedString, level: MessageLevel, cx: &mut App)` and `bachuc_plugin_host::MessageLevel` (enum `Info`, `Error`) from TASK-170
- Consumes: `bachuc_icons::IconName` variants `Route`, `Columns3`, `CornerLeftUp`, `CornerDownRight`, `Laptop`, `PencilOff`, `Scale`, `ClipboardCheck`, `MessageCircleQuestion`, `Hand`, `LoaderCircle`, `Quote`, `MessagesSquare`, `Power`, `Languages`, `Send`, `FileImage`, `FileCode`, and `CloudDownload`, each with a `lucide/<file>.svg` path served by `bachuc_icons::Assets`, from TASK-195

Expected result:

- The Watchtower overlay shows the Electron overview for its workspace, in the wide, stacked, or narrow layout by width, with every cell, the plan tree, and the autorun strip.
- A Needs you action sends the Electron prompt to the chosen agent, shows "Sent to <name>", and opens the agent; a failure shows the error message.
- Decisions and ADR rows open the decision modal with the rendered markdown.
- Cases:
  - `run_state_text`: no run, stopped at 14:05, finished, started 09:30, and running with no start.
  - `layout_for(width)`: 700 -> stacked; 1,100 -> rail with the middle grid; 1,300 -> rail with the wide grid; 860 -> rail with the narrow grid (main 500).
  - `needs_you_rows` appends ", answer by HH:MM" from the deadlines, and not when the run is stopped.
  - `pace_items`: no done bar -> no average; a stopped run -> no estimated finish.
  - `reject_step`: first press -> confirm; confirm -> send; cancel -> back.
  - Action buttons: disabled for `NoAgent`, `Busy`, and while a send runs.
  - `secondary_line` for done with a commit, waiting, running with "active 5m ago", blocked, and planned.
  - Decision rows sort by id descending; a row with an id that fails `is_decision_id` is not pressable.
  - Tree times are `None` when the overview has no run.

Anti-goal: the overview does no per-frame work: the derived data recomputes only when the overview, the board, the builder set, or the minute changes, and 1,000 recomputes for a 40-task plan take under 200 ms in a debug test; tripwire; read when the derived-data rules land and at completion.

## Verify

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-watchtower-board` -> pass, with one test per expected case, the 40-task timing test, and every TASK-190 test unchanged
- `rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app|plugin-[a-z-]+)\b" packages/desktop-gpui/plugins/watchtower-board/Cargo.toml | rg -v plugin-host` -> no match
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-watchtower-board` -> exit 0
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0
- `git diff --stat -- hiep-plugins/plugins/watchtower-board` -> empty
- UI check: after TASK-191, with watchtower-board running on live and a workspace that has an active watchtower plan, a background screenshot (`open -g`, no pointer, no foreground) of the GPUI Watchtower overlay at a width of 1,300 and of 700, compared with the Electron 6770 build -> same header, stepper, cells, plan tree, and stacked order; a stale or blank frame is UNVERIFIED
