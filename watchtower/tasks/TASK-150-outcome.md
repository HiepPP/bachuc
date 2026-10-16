# TASK-150 Outcome

## Status

DONE. Screen checks are deferred to the UI checker.

## Changed

- `packages/desktop-gpui/crates/composer/Cargo.toml`: adds `bachuc-icons` and `bachuc-theme` as dependencies (theme was a dev dependency).
- `packages/desktop-gpui/crates/composer/src/composer.rs`: rewritten view (surface, input, pills, slash menu, queue track, primary button).
- `packages/desktop-gpui/crates/composer/src/slash.rs` (new): slash menu model (client commands, filter, insert text, client command resolve) and tests.
- `packages/desktop-gpui/crates/composer/src/actions.rs` (new): send plan (steer or queue), primary action, input row math, and tests.
- `packages/desktop-gpui/crates/composer/src/attachments.rs`: `Attachment` gets a `preview: Arc<gpui_kit::Image>` for the thumbnail pill.
- `packages/desktop-gpui/crates/composer/src/lib.rs`: module list and docs.
- `packages/desktop-gpui/crates/composer/examples/composer.rs`: registers `bachuc_icons::Assets`.
- `packages/desktop-gpui/Cargo.lock`: updated by cargo.

## Contract

- `Composer::new(client: Entity<Client>, agent_id: String, window: &mut Window, cx: &mut Context<Composer>) -> Composer` is unchanged. `agent_id`, `focus`, and `attachment_count` are unchanged.
- `ComposerEvent` gets one variant, `ReplaceWithDraft`, sent after the `/clear` client command archives the agent. `Sent` is unchanged. Nothing outside the crate matches `ComposerEvent`.
- Sends call `Client::send_prompt_with(.., TurnBehavior::Steer, ..)` for Enter. Cmd+Enter calls `Client::enqueue_prompt` while the agent runs and `supports_queue()` is true; otherwise it steers.
- Pill order is the Electron order: model, thinking, mode (`agent-controls/index.tsx` renders the thinking trigger, then `AgentModeControl`, after the provider and model pill). The spec lists "mode, model, and thinking" without an order.
- Redraw: the view stores a `Watched` snapshot (controls, thinking, queued prompts, `supports_queue`) and calls `notify` on `AgentsChanged` or `Connected` only when the snapshot differs (Q-067). It also redraws on a theme change.
- Accessibility ids and labels: input `composer-input` "Message"; pills `composer-model`, `composer-thinking`, `composer-mode` labeled "<Kind>: <value>"; menu items are `MenuItem` with `aria_selected` for the current choice; slash rows are `MenuItem` with `aria_selected` for the highlighted row; `composer-send` "Send"; `composer-stop` "Stop"; `composer-attach` "Attach images"; queued rows `ListItem` "Queued: <text>" with a "Cancel queued message" button; chips `ListItem` "Attachment <name>" with a "Remove attachment" button.

## Verified

- `packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> PASS: 21 passed. Covers the slash filter and ranking, client command resolution, menu stepping, the steer or queue choice, the primary button states, and the input height math (21 px a line; max `max(160, 0.5 * window)`).
- `packages/desktop-gpui/cargo.sh clippy -p bachuc-composer --all-targets` -> PASS: no warning.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-composer` -> PASS.
- `cargo.sh build -p bachuc-composer --example composer` and `cargo.sh build -p bachuc-app` -> PASS.
- GitNexus `impact` on `Composer` (struct) -> LOW, 4 impacted; only `crates/app/src/shell.rs` constructs it.
- Bundle and compare with the reference app (rest, text, image pill, slash menu, run) -> DEFERRED (UI checker).
- Live: type `/`, pick a command, send; check `npm run cli -- logs <id>` -> DEFERRED (UI checker). Use an idle Claude agent: mock agents cannot list commands.
- Live: send during a mock run; `logs <id>` shows the steered message -> DEFERRED (UI checker).
- Live: Cmd+Enter during a mock run, then cancel the queued prompt -> DEFERRED (UI checker).
- Agent check: idle and empty, then type, then clear during a run (nothing, Send, Stop) -> DEFERRED (UI checker).
- Live: change the thinking pill on a live Claude agent; `npm run cli -- inspect <id>` -> DEFERRED (UI checker). The pill hides when `agent_thinking` is `None`.
- `cua_repl` `getAXState` roles and labels -> DEFERRED (UI checker).

## Anti-goal

Gauge: the composer example CPU at most 10.0% and footprint at most 250 MB on a mock stream with typing.

- Checkpoint "restyle landed": DEFERRED (UI checker). `measure.sh 60 comp="pids:<pid>"`.
- Checkpoint "completion": DEFERRED (UI checker). Same read after the restyle.

## Parity list

Not run. DEFERRED (UI checker). Values the code uses, from the cited Electron files, for the checker:

| Element     | Layout and style set in code                                                                               | State      |
| ----------- | ---------------------------------------------------------------------------------------------------------- | ---------- |
| Surface     | padding 8 / 12, bg `surface1`, 1 px `border_accent`, radius 16, gap 6                                      | unverified |
| Input       | `FONT_CONTENT` (scaled), line height 1.4 (21 px), 1 row up to `max(160, 0.5 * window)`, no border, no bg   | unverified |
| Toolbar row | margin -6 on both sides, attach (28 round, `Plus`) then pills, Send or Stop on the right                   | unverified |
| Pill        | 28 high, padding 8, gap 4, radius 16, `FONT_SM`, `foreground_muted`, 14 px icon, hover or open `surface2`  | unverified |
| Send        | 28 round, `accent`, `ArrowUp` in `accent_foreground`, left margin 4                                        | unverified |
| Stop        | 28 round, red `0xdc2626`, white 10 px square                                                               | unverified |
| Image pill  | 48 px thumbnail, 1 px `border_accent`, radius 6; remove button 24 round at top -8, left -8                 | unverified |
| Queue row   | padding 12 / 8, bg `surface1`, 1 px `border`, radius 8, text 2 lines, cancel 32 round `surface2`           | unverified |
| Slash menu  | overlay above the surface, `popover` bg, 1 px `border`, radius 8, rows 12 / 6, name then muted description | unverified |

Known differences from Electron, by design or limit:

- The attachment pill shows only the thumbnail (all attachments are images) and the remove button is always visible; Electron shows it on hover.
- Pill menus are gpui-kit popovers, not the Electron combobox with search and a wheel.
- The stop red is a literal because the theme crate carries no palette.
- No context window meter, dictation, skills picker, send-behavior setting, or queue editing (out of scope).

## Lessons

- The slash menu reads Up, Down, Tab, and Escape through `App::intercept_keystrokes`, only while the menu is open and the input has focus, as the command center does. Escape with the menu closed is not consumed, so a shell binding for it still fires.
- `TextareaState::set_value` emits no `Change` event and puts the caret at 0. `Composer::pick` sets the value and then `set_cursor_position`; the menu state comes from the text at render time, so no event is needed.
- The textarea row limit comes from `set_auto_grow(1, rows)` with `rows = floor(max(160, 0.5 * viewport height) / 21)`; the view updates it when the window height changes.
- Daemon commands load on the first `/` typed (`list_commands`). A failure (mock agent) is cached as empty until the text stops being a `/` query, so the client commands `/exit` and `/clear` still show.
- `/exit` archives the agent. `/clear` archives it and emits `ComposerEvent::ReplaceWithDraft`; TASK-155 decides what a fresh draft means. Picking a client command inserts `/name` with no space; Enter then sends it, which runs it.
- Enter with the menu open picks the highlighted row, unless the text already equals the row's insert text (a complete client command), where it sends.
- Popover pill triggers must implement `Selectable`; `PillTrigger` in `composer.rs` does. A pill with no choices draws as a disabled label and opens nothing.

## Questions

- Pill order: the spec says "mode, model, and thinking, in the Electron order". The code uses the order Electron renders: model, thinking, mode. Change `Composer::pills` if the checker finds a different order on screen.

## UI Checks

Run by the UI checker on 2026-10-16, working tree on HEAD 5afebcedd, bundle `BachucComposer` rebuilt before the run, light theme. Screenshots: `/tmp/gpui-ui-check/F/`.

Setup limit: the live daemon (6768) was down and the checker does not start it. The reference daemon on 6770 has no mock provider ("Provider mock is not configured") and the checker creates no Claude agent there (only mock agents on 6770). The example ran against the existing closed Claude agent `380c019` ("hi", Opus 5.5, low, `~/Projects/eva`) on 6770: `BachucComposer.app --args ws://127.0.0.1:6770/ws 380c019`. The checker typed text but sent nothing. Starting the example moved that agent from `closed` to `idle` on the 6770 daemon (the client attaches to it); nothing else changed.

### Primary button states -> PARTIAL: idle and Send PASS, Stop UNVERIFIED

Idle and empty: no Send or Stop button in the AX tree and none on screen. After typing `hello`: a green round `Send` button (accent) at the right. After select-all and delete: the button is gone again (0 Send or Stop buttons). Stop needs a running turn, which needs a mock agent on live.

### Slash menu with daemon commands -> PARTIAL

Typing `/` opened the menu above the input with 153 daemon commands (AX `menu Slash commands (showing 0-100 of 153 items)`), first row selected; this shows that commands load for a Claude agent. Picking a command and sending it was not run (sending would resume a real agent and write to the reference data); `npm run cli -- logs` was not checked. UNVERIFIED: pick and send.

### Live send, steer, queue, cancel, thinking pill change -> UNVERIFIED

All four need a running mock agent or a live Claude agent on 6768; the daemon is down. Not run.

### Parity (light, input idle and with `/`) -> FAIL, 4 mismatches

Window sizes differ (example 720 x 600, reference 1200 x 800 with a 820 px composer), so only structure and styles are compared.

| Element                   | Reference (Electron)                                                                            | GPUI                                                                    | Result                        |
| ------------------------- | ----------------------------------------------------------------------------------------------- | ----------------------------------------------------------------------- | ----------------------------- |
| Surface                   | rounded card, 1 px border, white input row, grey toolbar row                                    | rounded card, 1 px border, grey bg                                      | match within the sampled look |
| Placeholder               | "Message the agent, tag @files, or use /commands and /skills"                                   | "Message"                                                               | MISMATCH                      |
| Attach button             | round plus at the left of the pills                                                             | round plus at the left of the pills                                     | match                         |
| Model pill icon           | Claude sparkle                                                                                  | generic bot icon                                                        | MISMATCH                      |
| Mode pill at rest         | not shown while the input is empty and unfocused                                                | shown at rest, greyed, label `auto`                                     | MISMATCH                      |
| Mode pill with text       | shield icon, "Auto mode", plus a lightning button                                               | shield icon, "Auto mode"; no lightning button                           | MISMATCH (lightning)          |
| Send button               | green round, white up arrow                                                                     | green round, white up arrow                                             | match                         |
| Slash menu list           | rows anchored at the bottom, newest or last row selected, `/exit` first in a separate box above | rows from the top in command order, first row selected, no separate box | MISMATCH (order and anchor)   |
| Slash row text            | name, muted description on the same line                                                        | name, muted description on the same line                                | match                         |
| Extra row above the input | diff counts `+742 -0`                                                                           | none                                                                    | not counted (agent panel row) |
| Voice buttons             | microphone and waveform                                                                         | none                                                                    | not counted (out of scope)    |

The spec's Questions note (pill order) was not settled: both apps show model, thinking, mode, in that order.

### Accessibility tree -> PASS

`text entry area (settable) Message` (id `composer-input`), `button Attach images`, `button Model: Opus 5.5`, `button Thinking: Low`, `button Mode: auto` (later `Mode: Auto mode`), `button Send`, and `menu Slash commands (showing 0-100 of 153 items)` with each row `(selected)` or plain and labelled `/name, description`. The menu items' selected state is exposed.

### CPU and memory -> PARTIAL

`measure.sh 60 comp="pids:82268"` with the text `typing sample` in the input, no stream, window kept in front by an `open` loop every 3 s: 1.2 CPU% (limit 10.0), 121 MB RSS, 101 MB footprint (limit 250). A screenshot after the read shows the full window, no cut. The required condition (a mock stream with typing) was not run.

### Cleanup

The example and the reference app were quit; the lock was released; 6767 untouched; no agent or terminal was created; no prompt was sent.
