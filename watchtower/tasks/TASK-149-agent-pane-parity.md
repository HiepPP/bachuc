# TASK-149 Agent pane parity

Group: E (writes `crates/timeline`)
Class: risky

## Brief

Goal: Make the GPUI agent pane look and work like the Electron agent panel: tool call detail views, file links, images, plan and question cards, find, and the Electron colors and sizes.

Change: the timeline has turn grouping, markdown, plain diffs, diagrams, and permission rows on gpui-kit colors -> the Electron message, tool detail, and card designs on `bachuc-theme` tokens.

Design: [packages/app/src/components/message.tsx](packages/app/src/components/message.tsx), [packages/app/src/components/tool-call-details.tsx](packages/app/src/components/tool-call-details.tsx), [packages/app/src/tool-calls/detail-level/turn/view.tsx](packages/app/src/tool-calls/detail-level/turn/view.tsx), [packages/app/src/components/plan-card.tsx](packages/app/src/components/plan-card.tsx), [packages/app/src/components/question-form-card.tsx](packages/app/src/components/question-form-card.tsx), [packages/app/src/assistant-file-links](packages/app/src/assistant-file-links), [packages/app/src/assistant-image](packages/app/src/assistant-image), [packages/app/src/pane-find/index.tsx](packages/app/src/pane-find/index.tsx), [packages/app/src/components/markdown/renderer.tsx](packages/app/src/components/markdown/renderer.tsx)

Boundaries:

- Keep `TimelineView::new` unchanged, and keep the incremental sync of [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md): per-flush work stays in proportion to the change.
- Keep the turn design ADRs: [ADR-0018](watchtower/decisions/ADR-0018-all-trailing-messages-over-last-message-as-answer.md), [ADR-0019](watchtower/decisions/ADR-0019-always-open-step-list-over-folding-group.md), [ADR-0020](watchtower/decisions/ADR-0020-scheme-label-colors-over-palette-tokens-for-turn-steps.md), [ADR-0021](watchtower/decisions/ADR-0021-note-opens-its-step-over-tools-before-the-note.md), and [ADR-0037](watchtower/decisions/ADR-0037-hidden-webview-png-snapshots-over-svg-and-inline-webviews.md).
- Tool detail views per kind, as in `tool-call-details.tsx`: shell (command and output), read, edit and write (diff with syntax colors and the Electron line colors), search, fetch, sub_agent, and plan.
- File links: paths in assistant text and tool rows are links, as `assistant-file-links` parses them. A click emits `TimelineEvent::OpenFile`; the app opens the file pane.
- Images: assistant images load through `read_file` with `FileKind::Image` bytes, as [packages/app/src/assistant-image/file-acquisition.ts](packages/app/src/assistant-image/file-acquisition.ts) line 47 does. User images come from the attachment metadata in the stream ([packages/app/src/types/stream.ts](packages/app/src/types/stream.ts) line 94); trace where Electron gets their bytes and do the same. A click opens a larger view, as [packages/app/src/components/attachment-lightbox.tsx](packages/app/src/components/attachment-lightbox.tsx) does.
- Plan approval and question forms: render the Electron cards ([packages/app/src/components/plan-card.tsx](packages/app/src/components/plan-card.tsx), [packages/app/src/components/question-form-card.tsx](packages/app/src/components/question-form-card.tsx)) and send the same `agent_permission_response` payload that `view.tsx` line 1594 sends, through `respond_permission_with`.
- Find: Cmd+F opens a find bar in the pane that highlights matches in loaded rows and steps through them.
- Content width is at most `MAX_CONTENT_WIDTH` 820, centered, as in Electron.
- Use only tokens, metrics, and `bachuc_icons`; remove the hard-coded running blue `0x256ABF` (`crates/timeline/src/lib.rs` line 33).
- Fix the timeline accessibility gaps from [packages/desktop-gpui/docs/a11y-audit.md](packages/desktop-gpui/docs/a11y-audit.md): the diff toggle gets its own button with a label.

How:

- Restyle rows to the Electron message and step design.
- Add the tool detail views, file links, images, cards, and find.
- Extend the timeline example to show each kind.

Files:

- [packages/desktop-gpui/crates/timeline](packages/desktop-gpui/crates/timeline) (`Cargo.toml`, `src`, `tests`, `examples/timeline.rs`)

Interfaces:

- Consumes: `bachuc_theme::tokens(cx: &gpui_kit::App) -> Tokens` from TASK-145
- Consumes: `bachuc_theme::metrics` (module of `pub const` `f32` values: `SPACE_0_5`, `SPACE_1`, `SPACE_1_5`, `SPACE_2`, `SPACE_3`, `SPACE_4`, `SPACE_6`, `SPACE_8`, `RADIUS_SM`, `RADIUS_BASE`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_XL`, `RADIUS_2XL`, `FONT_CODE`, `FONT_SM`, `FONT_BASE`, `FONT_CONTENT`, `FONT_LG`, `FONT_XL`, `FONT_2XL`, `ICON_XS`, `ICON_SM`, `ICON_MD`, `ICON_LG`, `CONTROL_XS`, `CONTROL_SM`, `CONTROL_MD`, `HEADER_HEIGHT`, `TAB_BAR_HEIGHT`, `MAX_CONTENT_WIDTH`, `SIDEBAR_WIDTH`, `SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH`, `TRAFFIC_LIGHT_WIDTH`, `TRAFFIC_LIGHT_HEIGHT`, `SETTINGS_SIDEBAR_WIDTH`, `SETTINGS_DETAIL_MIN_WIDTH`, `SETTINGS_CONTENT_WIDTH`) from TASK-145
- Consumes: `bachuc_icons::IconName` (enum with one variant per generated icon, named as the lucide import, for example `FolderPlus`, `GitBranch`, `X`), `bachuc_icons::icon(name: IconName) -> gpui_kit::component::Icon` (sized with `ICON_MD`; change it with `with_size` and `text_color`), and `bachuc_icons::Assets` (a GPUI `AssetSource` that serves the SVGs and falls through to the gpui-kit assets) from TASK-145
- Consumes: `Client::read_file(&mut self, cwd: &str, path: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<FileContent>>` with `FileContent { path: String, kind: FileKind, text: Option<String>, bytes: Option<Vec<u8>>, mime_type: Option<String>, size: u64 }` and `bachuc_client::FileKind` (enum `Text`, `Image`, `Binary`) from TASK-146
- Consumes: `Client::respond_permission_with(&mut self, agent_id: &str, request_id: &str, response: serde_json::Value, cx: &mut gpui_kit::Context<Client>)` from TASK-146
- Produces: `bachuc_timeline::TimelineEvent` (enum `OpenFile { cwd: String, path: String, line: Option<u32> }`) emitted by `TimelineView` for TASK-155
- Produces: `bachuc_timeline::TimelineView::find(&mut self, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<TimelineView>)` (opens the find bar) for TASK-155

Expected result:

- A live agent shows messages, steps, and each tool kind like the reference app.
- A file path in a reply opens through `TimelineEvent::OpenFile`.
- Cases:
  - A shell tool with long output shows the Electron collapsed preview and expands.
  - A failed image load shows the Electron retry state.
  - Find with no match shows the Electron empty count.

Anti-goal: the app-level CPU from [ADR-0039](watchtower/decisions/ADR-0039-incremental-timeline-sync-over-throttle-or-budget.md) holds for the timeline example: at most 10% CPU and 250 MB on a mock stream at about 2 and about 15 minutes; tripwire; read with `measure.sh 60` before the restyle, after the tool views land, and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> pass, including `incremental_rows_match_full_regroup`, a file-link parse test that matches the `assistant-file-links` cases, and one test per tool detail kind
- Agent check: bundle `packages/desktop-gpui/bundle.sh example timeline BachucTimeline io.github.hieppp.bachuc-timeline`; open the same mock agent in it and in the reference app on 6770; take `cua_repl` screenshots of a turn, each tool kind, a plan card, and a question card -> the parity list in the outcome shows match or records each mismatch
- Agent check: click a file path in a reply -> the example logs `OpenFile` with the path and line
- Agent check on live: answer a question form and reject a plan on a live claude agent in plan mode -> the agent receives the answer and the rejection; `npm run cli -- logs <id>` shows both
- Agent check: a reply that shows a PNG from `/tmp/gpui-bench` -> the image renders and opens larger on click; a missing file shows the retry state
- Agent check: Cmd+F, type a word, press Enter twice -> matches highlight and the view steps between them
- Agent check: `cua_repl` `getAXState` -> the diff toggle is its own button with a label; tool rows and cards have roles and labels
- Anti-goal: `measure.sh 60 tl="pids:<pid>" wk="pids:<WebKit pids>"` at about 2 and about 15 minutes of a mock turn -> CPU at most 10.0 and footprint at most 250
