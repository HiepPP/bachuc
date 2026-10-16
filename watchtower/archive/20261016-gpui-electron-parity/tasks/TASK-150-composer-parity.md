# TASK-150 Composer parity

Group: F (writes `crates/composer`)
Class: code

## Brief

Goal: Make the GPUI composer look and work like the Electron composer: slash commands, steer while a turn runs, the thinking pill, attachment pills, and the Electron size and style.

Change: a textarea with Send, Stop, image chips, and mode and model pills on gpui-kit colors -> the Electron composer surface with its controls.

Design: [packages/app/src/composer/index.tsx](packages/app/src/composer/index.tsx), [packages/app/src/composer/input/input.tsx](packages/app/src/composer/input/input.tsx), [packages/app/src/composer/submit.ts](packages/app/src/composer/submit.ts), [packages/app/src/composer/agent-controls](packages/app/src/composer/agent-controls), [packages/app/src/composer/pill-styles.ts](packages/app/src/composer/pill-styles.ts), [packages/app/src/components/attachment-pill.tsx](packages/app/src/components/attachment-pill.tsx), [packages/app/src/client-slash-commands/index.ts](packages/app/src/client-slash-commands/index.ts), [packages/app/src/components/context-window-meter.tsx](packages/app/src/components/context-window-meter.tsx)

Boundaries:

- Keep `Composer::new` unchanged.
- Sizes, using the web and desktop values: input min height 21, plus 21 per line, max height `max(160, 0.5 * window height)` ([packages/app/src/composer/input/input.tsx](packages/app/src/composer/input/input.tsx) lines 197-204); surface radius 16 (line 1953); padding 8 vertical and 12 horizontal ([packages/app/src/composer/pill-styles.ts](packages/app/src/composer/pill-styles.ts) lines 6-10). Text is `FONT_CONTENT` with a 1.4 line height (line 1994).
- Slash commands: typing `/` at the start opens the command menu with the daemon commands from `list_commands` plus the client commands in `client-slash-commands`. Arrow keys move, Enter picks, Escape closes.
- Send while a turn runs, as [packages/app/src/composer/submit.ts](packages/app/src/composer/submit.ts) lines 45-52 and `composer/index.tsx` line 1786 do: Enter sends with `TurnBehavior::Steer`, the Electron default `sendBehavior`. Cmd+Enter queues with `enqueue_prompt` when `supports_queue()` is true, and sends with steer when it is false. Planner call: no send-behavior setting in this plan; steer is fixed.
- Show the queued prompts above the input, as Electron does, with a cancel button each. Editing, reordering, promoting, and resuming queued prompts are out of scope.
- Pills: mode, model, and thinking, in the Electron order and style. Click opens the menu (Q-055 default stands; no wheel behavior).
- Primary button, as `resolvePrimaryActionKind` (`input.tsx` lines 817-827): Send shows when the input has text or an attachment, also during a run; Stop shows only while the agent runs with an empty input; nothing shows when idle and empty. This replaces the Q-062 default.
- Fix the composer accessibility gap from [packages/desktop-gpui/docs/a11y-audit.md](packages/desktop-gpui/docs/a11y-audit.md): pill menu items get a role and a selected state.
- Redraw only when the agent's modes, models, thinking, or running state change, not on every `AgentsChanged` (Q-067).
- Out of scope: dictation, provider usage, the skills picker, the send-behavior setting, and queue editing.

How:

- Restyle the surface, input, and pills.
- Add the slash command menu, steer, and the thinking pill.
- Update the composer example.

Files:

- [packages/desktop-gpui/crates/composer](packages/desktop-gpui/crates/composer) (`Cargo.toml`, `src`, `examples/composer.rs`)

Interfaces:

- Consumes: `bachuc_theme::tokens(cx: &gpui_kit::App) -> Tokens` from TASK-145
- Consumes: `bachuc_theme::metrics` (module of `pub const` `f32` values: `SPACE_0_5`, `SPACE_1`, `SPACE_1_5`, `SPACE_2`, `SPACE_3`, `SPACE_4`, `SPACE_6`, `SPACE_8`, `RADIUS_SM`, `RADIUS_BASE`, `RADIUS_MD`, `RADIUS_LG`, `RADIUS_XL`, `RADIUS_2XL`, `FONT_CODE`, `FONT_SM`, `FONT_BASE`, `FONT_CONTENT`, `FONT_LG`, `FONT_XL`, `FONT_2XL`, `ICON_XS`, `ICON_SM`, `ICON_MD`, `ICON_LG`, `CONTROL_XS`, `CONTROL_SM`, `CONTROL_MD`, `HEADER_HEIGHT`, `TAB_BAR_HEIGHT`, `MAX_CONTENT_WIDTH`, `SIDEBAR_WIDTH`, `SIDEBAR_MIN_WIDTH`, `SIDEBAR_MAX_WIDTH`, `TRAFFIC_LIGHT_WIDTH`, `TRAFFIC_LIGHT_HEIGHT`, `SETTINGS_SIDEBAR_WIDTH`, `SETTINGS_DETAIL_MIN_WIDTH`, `SETTINGS_CONTENT_WIDTH`) from TASK-145
- Consumes: `bachuc_icons::IconName` (enum with one variant per generated icon, named as the lucide import, for example `FolderPlus`, `GitBranch`, `X`), `bachuc_icons::icon(name: IconName) -> gpui_kit::component::Icon` (sized with `ICON_MD`; change it with `with_size` and `text_color`), and `bachuc_icons::Assets` (a GPUI `AssetSource` that serves the SVGs and falls through to the gpui-kit assets) from TASK-145
- Consumes: `Client::agent_thinking(&self, agent_id: &str) -> Option<ThinkingControls>` with `ThinkingControls { options: Vec<(String, String)>, current: Option<String> }`, and `Client::set_thinking(&mut self, agent_id: &str, option_id: &str, cx: &mut gpui_kit::Context<Client>)` from TASK-146
- Consumes: `Client::list_commands(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>) -> gpui_kit::Task<anyhow::Result<Vec<SlashCommand>>>` with `SlashCommand { name: String, description: String }` from TASK-146
- Consumes: `bachuc_client::TurnBehavior` (enum `Interrupt`, `Steer`) and `Client::send_prompt_with(&mut self, agent_id: &str, text: String, images: Vec<serde_json::Value>, behavior: TurnBehavior, cx: &mut gpui_kit::Context<Client>)` from TASK-146
- Consumes: `Client::supports_queue(&self) -> bool`, `Client::enqueue_prompt(&mut self, agent_id: &str, text: String, images: Vec<serde_json::Value>, cx: &mut gpui_kit::Context<Client>)`, `Client::queued_prompts(&self, agent_id: &str) -> Vec<QueuedPrompt>` with `QueuedPrompt { id: String, text: String }`, and `Client::cancel_queued(&mut self, agent_id: &str, queued_id: &str, cx: &mut gpui_kit::Context<Client>)` from TASK-146
- Produces: the same `bachuc_composer::Composer::new(client: gpui_kit::Entity<bachuc_client::Client>, agent_id: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Composer>) -> Composer`, unchanged, for TASK-155

Expected result:

- The composer matches the reference app at rest, with text, with an image pill, and while a turn runs.
- `/` shows the commands of a live agent, and picking one inserts it.
- Enter during a run sends with the Electron behavior; the daemon logs show it.

Anti-goal: the composer example stays at or below 10% CPU and 250 MB on a mock stream with typing; tripwire; read with `measure.sh 60` after the restyle and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> pass, including slash-command filter and key tests, the steer choice, and the input height math
- Agent check: bundle `packages/desktop-gpui/bundle.sh example composer BachucComposer io.github.hieppp.bachuc-composer`; compare with the reference app on 6770 using `cua_repl` screenshots at rest, with text, with an image pill, with the slash menu open, and during a run -> the parity list in the outcome shows match or records each mismatch
- Agent check on live: type `/`, pick a command, and send -> `npm run cli -- logs <id>` shows it
- Agent check on live: send during a mock run -> `npm run cli -- logs <id>` shows the message steered into the running turn
- Agent check on live: press Cmd+Enter during a mock run, then cancel the queued prompt -> the queued item shows above the input, and after cancel it is gone and never reaches the agent
- Agent check: idle and empty, then type text, then clear it during a run -> the primary button shows nothing, then Send, then Stop
- Agent check: change the thinking pill on a live claude agent -> `npm run cli -- inspect <id>` shows the new thinking option
- Agent check: `cua_repl` `getAXState` -> the input, pills, menu items with selected state, and Send and Stop have roles and labels
- Anti-goal: `measure.sh 60 comp="pids:<pid>"` on a mock stream with typing -> CPU at most 10.0 and footprint at most 250
