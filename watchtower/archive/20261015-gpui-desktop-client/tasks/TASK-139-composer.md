# TASK-139 Composer with mode and model controls

Group: E (writes only `crates/composer`)
Class: code

## Brief

Goal: Add the crate `bachuc-composer` with the message box under the timeline: multiline input, send, stop, image attachments, and mode and model pills.

Change: the prototype's one-line input -> a composer for daily use.

Boundaries:

- Enter sends; Shift+Enter adds a new line. Send is disabled for empty text with no image.
- While the agent runs, the send button sends to the running turn with the default `activeTurnBehavior`, and a Stop button calls `cancel_agent`.
- Images come from a file picker (`prompt_for_paths`) and from a clipboard paste. Each image is a JSON object with `data` (base64) and `mimeType`, per `ImageAttachmentSchema` in [packages/protocol/src/messages.ts](packages/protocol/src/messages.ts); the client puts them in the `images` field of `send_agent_message_request`.
- The model pill lists `AgentControls.models`, which the client fills from `list_provider_models_request` for the agent's provider.
- Mode and model pills open a menu on click and call `set_mode` or `set_model`. The wheel behavior of [ADR-0025](watchtower/decisions/ADR-0025-wheel-opens-the-list-and-steps-over-closed-pill-step.md) is not in this TASK; see Q-055.
- Slash commands and the skill picker are a later plan.
- Give the input, buttons, and pills `a11y` roles and labels.

How:

- Build the composer view with a gpui-kit multiline input.
- Add the image picker, paste handling, and chips that remove an image.
- Add the pills from `agent_controls`.
- Add `examples/composer.rs` that shows the composer for one agent above a plain text log of the stream.

Files:

- [packages/desktop-gpui/crates/composer/Cargo.toml](packages/desktop-gpui/crates/composer/Cargo.toml) (new crate `bachuc-composer`)
- [packages/desktop-gpui/crates/composer/src](packages/desktop-gpui/crates/composer/src) (composer view)
- [packages/desktop-gpui/crates/composer/examples/composer.rs](packages/desktop-gpui/crates/composer/examples/composer.rs) (example)

Interfaces:

- Consumes: `bachuc_client::Client` (a GPUI entity) with `Client::connect(host: HostConfig, cx: &mut gpui_kit::App) -> gpui_kit::Entity<Client>` from TASK-136
- Consumes: `Client::send_prompt(&mut self, agent_id: &str, text: String, images: Vec<serde_json::Value>, cx: &mut gpui_kit::Context<Client>)`, `Client::cancel_agent(&mut self, agent_id: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::set_mode(&mut self, agent_id: &str, mode_id: &str, cx: &mut gpui_kit::Context<Client>)`, `Client::set_model(&mut self, agent_id: &str, model_id: &str, cx: &mut gpui_kit::Context<Client>)`, and `Client::agent_controls(&self, agent_id: &str) -> Option<AgentControls>` with `AgentControls { modes: Vec<(String, String)>, current_mode: Option<String>, models: Vec<(String, String)>, current_model: Option<String>, running: bool }` from TASK-136
- Produces: `bachuc_composer::Composer` (a GPUI view) with `Composer::new(client: gpui_kit::Entity<bachuc_client::Client>, agent_id: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Composer>) -> Composer` for TASK-141

Expected result:

- Typing and Enter sends a prompt that the live agent receives. Shift+Enter adds a line.
- During a run, Stop cancels the turn.
- A pasted or picked PNG shows as a chip and reaches the agent as an image.
- The mode and model pills show the current values and change them on the daemon.
- Empty text with no image does not send.

Anti-goal: the example stays within 10% CPU and 250 MB phys_footprint on the mock stream in the foreground for 60 seconds, typing included; tripwire; read with `measure.sh 60` after the input works and at completion.

## Verify

- `packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> key handling and empty-send tests pass
- Agent check with a live mock agent: `packages/desktop-gpui/bundle.sh example composer BachucComposer io.github.hieppp.bachuc-composer`, type with `cua_repl` and press Enter -> `npm run cli -- logs <id>` shows the new user message
- Agent check: type a line, press Shift+Enter, type a second line, press Enter -> `npm run cli -- logs <id>` shows one message with two lines
- Agent check: pick a PNG from `/tmp` with the attach button through `cua_repl` -> a chip shows; after Enter, `npm run cli -- inspect <id>` or the timeline shows the user message with an image
- Agent check: change the mode pill to `approval-test` on a mock agent -> `npm run cli -- inspect <id>` shows the new mode
- Agent check: `cua_repl` `getAXState` -> the input, Send, Stop, and both pills appear with labels
- Anti-goal: `measure.sh 60 comp="BachucComposer.app"` -> CPU at most 10.0 and footprint at most 250
