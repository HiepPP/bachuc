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

## Fix Round 1

Run on 2026-10-16 against the 4 parity mismatches in `## UI Checks`. Code checks only; screen checks are DEFERRED (UI checker).

### Changed

- `packages/desktop-gpui/crates/composer/src/composer.rs`: placeholder, model pill logo, mode pill hiding, slash menu order, anchor, and detail card.
- `packages/desktop-gpui/crates/composer/src/slash.rs`: client commands first; `arrow_delta`; tests.
- `packages/desktop-gpui/crates/composer/assets/claude.svg` (new): Electron `ClaudeIcon` path, with `{FILL}` and `{OPACITY}` placeholders.

### Mismatches

| Mismatch          | Fix                                                                                                                               | State |
| ----------------- | --------------------------------------------------------------------------------------------------------------------------------- | ----- |
| Placeholder       | `composer.placeholders.desktop`: "Message the agent, tag @files, or use /commands and /skills"; AX label stays "Message"          | fixed |
| Model pill icon   | Claude agents get the Claude logo (SVG image in `foreground_muted`, rebuilt on a theme change); other providers keep `Bot`        | fixed |
| Mode pill at rest | Hidden while the agent reports no modes, as `AgentModeControl` returns null; the reference was a closed agent, not an empty input | fixed |
| Lightning button  | Not fixed: it is an agent feature toggle (`features`, icon `zap`), and `bachuc_client` exposes no agent features                  | open  |
| Slash menu order  | Client commands first, then daemon commands (`mergeSlashCommandSources`); rows draw reversed, best match at the bottom, selected  | fixed |
| Slash menu anchor | List pins to the bottom on each query change; Up moves away from the bottom row; the scroll follows the selection                 | fixed |
| Slash detail box  | Separate box above the list with the selected `/name` and full description (`Autocomplete` `detailCard`)                          | fixed |
| Slash menu style  | `surface1`, 1 px `border_accent`, radius 8, shadow md, rows min 36 / 12 / 8, list max 220                                         | fixed |

`STOP_RED` stays a literal: `bachuc_theme` has `#dc2626` only as the light terminal red `ansi[1]` (dark is `#e07070`), while Electron uses `palette.red[600]` in both themes.

### Verified

- `CARGO_BUILD_JOBS=4 cargo.sh test -p bachuc-composer` -> PASS: 23 passed (new: client order, alias hiding, arrow direction, logo fill).
- `cargo.sh fmt --check -p bachuc-composer` -> PASS.
- `CARGO_BUILD_JOBS=4 cargo.sh clippy -p bachuc-composer --all-targets` -> PASS: no composer warning (2 pre-existing warnings in `crates/protocol/build.rs`).
- Public API unchanged: `Composer::new`, `agent_id`, `focus`, `attachment_count`, `ComposerEvent`.
- Parity re-check of placeholder, model logo, mode pill, slash menu -> DEFERRED (UI checker).

### Questions

- The lightning button needs agent features (id, icon, value, set) in `bachuc_client`; then the composer can draw `DesktopFeatureItem` toggles after the mode pill.
- Provider logos other than Claude (Codex, Copilot, OpenCode, Pi) are filled SVGs that `bachuc_icons` does not carry; the composer falls back to `Bot`. They belong in `bachuc_icons` as a provider icon set, then the composer's `claude.svg` can go.

### Re-check 2 (after Fix Round 2; sits above Re-check 1 by an editing slip)

Run by the UI checker on 2026-10-16, HEAD 9fec90f18 (fix commit 9fec90f18 included), light theme. Bundle `BachucComposer` rebuilt before the run. The example ran on `ws://127.0.0.1:6770/ws` against an idle Claude agent made for this check: `7d4ad155` (haiku-5-5, `/tmp/gpui-bench`, title gpui-ui-check; one turn with a fixed markdown reply, no tools). The reference is the in-place Electron build on 6770, same agent. Two gotchas: a GPUI window draws only while in front, so the first screenshots were stale until an `open` loop ran; and the reference Electron window stops painting while the GPUI window covers it (AX still updates), so reference captures were taken with the example quit. The example window is fixed at 720 x 592 and cannot be resized, so offsets inside the card compare. Screenshots: `/tmp/gpui-ui-check/F/` (`*-r3*.png`).

| Line                              | Result     | Evidence                                                                                                                                                                                                                                                                                               |
| --------------------------------- | ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Placeholder look                  | PASS       | Same words, 12 px, light grey in both. Text starts 12 to 13 px from the card edge in both (GPUI card x 16, text x 29; reference card x 350, text x 363). Text run to "/command" is about 270 px in both.                                                                                               |
| Pill chevrons                     | PASS       | A down chevron follows every pill label in GPUI (model "claude-haiku-5-5", mode "Bypass") as in the reference ("Opus 5.5", thinking, mode).                                                                                                                                                            |
| Slash menu, first open            | PASS       | Fresh launch, typed `/`: the list shows bottom to top `/exit` (selected), `/clear`, `/__remote-workflow`, `/agent-sdk-dev:new-sdk-app`, `/agents`, `/anthropic-skills:skill-creator`, pinned to the bottom within 0.7 s of the first `/`. The reference shows the same list.                           |
| Detail card height                | PASS       | `/exit` card y 204 to 264 = 60 px (1 px under the reference 61 px, y 387 to 448); card to list gap 5 px (reference 6 px); list rows 36 px in both; list 220 px high (reference 219).                                                                                                                   |
| Composer card inset 30 px         | PASS       | Cannot make the 880 px pane in the fixed example window. At 720 px the card is inset 16 px each side (x 16 to 704). With the window zoomed to the 2560 px screen the card is 819 px wide and centered (x 870 to 1690 of 2000 displayed). Reference card is 820 px from x 350 to 1170 in a 880 px pane. |
| Focus hint                        | PASS       | With the input unfocused and empty, "⌘L to focus" shows at the top right of the input, light grey, 14 px from the card edge (reference 13 px).                                                                                                                                                         |
| Pill menus                        | PASS       | The mode pill opened a menu of 5 modes (Plan Mode, Always Ask, Accept File Edits, Auto mode, Bypass) with a check on the current one.                                                                                                                                                                  |
| Lightning, microphone, diff row   | known gap  | Q-079 and the Fix Round 2 notes. Recorded, not failed.                                                                                                                                                                                                                                                 |
| Live send, steer, queue, thinking | UNVERIFIED | Live (6768) is down; the reference has no mock provider.                                                                                                                                                                                                                                               |

Side effect: clicking the mode pill in the example changed agent `7d4ad155` from Always Ask to Bypass on 6770. The agent is archived after the TASK-149 check. Cleanup: the example was quit; the lock was released; 6767 untouched.

### Lessons

- A filled, non-lucide SVG draws through `img(Arc<Image>)` with `ImageFormat::Svg`; gpui rasterizes it at 2x the SVG's own size, so the asset sets `width`/`height` 56 for a crisp 14 px icon.

### Re-check 1

Run by the UI checker on commit 9ffc6f3a9 (HEAD d552985b8), 2026-10-16. Bundle `BachucComposer` rebuilt before the run, light theme. Live (6768) is down and the reference daemon on 6770 has no mock provider, so the example ran on the existing Claude agent `380c019` ("hi", Opus 5.5, low, `~/Projects/eva`): `BachucComposer.app --args ws://127.0.0.1:6770/ws 380c019`. The checker could not create an idle Claude agent without a prompt (`paseo run` needs one) and sent nothing, so it did not archive `380c019`: it is an existing agent, not one the checker made. Window sizes differ (example 720 x 592, reference 1200 x 800 with an 820 px composer), so only offsets inside the composer and the slash menu compare. Screenshots: `/tmp/gpui-ui-check/F/` (`gpui-rest-r2.png`, `ref-rest-r2.png`, `gpui-slash-r2.png`, `gpui-slash-r2c.png`, `ref-slash-r2.png`, `*-zoom.png`).

| Line                                  | Result    | Evidence                                                                                                                                                                                                                                                                        |
| ------------------------------------- | --------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Placeholder text                      | PASS      | AX placeholder "Message the agent, tag @files, or use /commands and /skills" in both. AX label stays "Message".                                                                                                                                                                 |
| Placeholder look                      | FAIL      | Same words, different style. Reference: darkest pixel `(207,207,209)` (light grey), text 342 px wide, starts 12 px from the card edge. GPUI: darkest pixel `(107,106,112)` (mid grey), text 412 px wide, starts 23 px from the card edge.                                       |
| Model pill logo                       | PASS      | Claude sparkle in both, same size and place (Opus 5.5 pill starts 50 px from the card left in both).                                                                                                                                                                            |
| Thinking pill                         | PASS      | Brain icon and "Low" in both.                                                                                                                                                                                                                                                   |
| Pill chevrons                         | FAIL      | The reference draws a down chevron after "Opus 5.5" and after "Low"; GPUI draws none.                                                                                                                                                                                           |
| Mode pill at rest                     | PASS      | Hidden in the first seconds in the example (AX: Attach, Model, Thinking only) and in the first reference screenshot; after the agent loads, both show "Auto mode" at rest (the reference with a chevron; the GPUI app composer showed `Mode: Auto mode` at rest too).           |
| Slash menu order                      | PASS      | After the query changes, the list reads bottom to top: `/exit` (selected), `/clear`, `/__remote-workflow`, `/agent-sdk-dev:new-sdk-app`, `/agents`, `/anthropic-skills:skill-creator`; the reference reads the same.                                                            |
| Slash menu anchor, first open         | FAIL      | On the first `/` (daemon commands arrive after the menu opens) the list shows the top: `/xlsx`, `/workflow-launch-exec`, `/workflow-authoring`, `/watchtower`, `/voyager`, `/verify`, while the detail card says `/exit`. The reference shows `/exit` at the bottom.            |
| Slash menu anchor, after query change | PASS      | Backspace then `/` again pins the list to the bottom with `/exit` selected, as in the reference.                                                                                                                                                                                |
| Slash row                             | PASS      | Row 36 px high, selected fill `(244,244,246)`, name then muted description on one line, name 13 px from the card edge: all equal in both.                                                                                                                                       |
| Detail card                           | FAIL      | Present with `/exit` and "Archive the current agent" in both, but GPUI is 71 px high (y 201 to 272) and the reference 61 px (y 387 to 448); the description sits 48 px below the card top in GPUI and 41 px in the reference. Gap list to composer: GPUI 5 px, reference 13 px. |
| Lightning toggles, @file mentions     | known gap | Q-079 and Q-080; the reference shows a lightning button after the mode pill, GPUI none. Recorded, not failed.                                                                                                                                                                   |

Cleanup: the example was quit; the typed `/` in the reference composer was cleared (nothing was sent to any agent); 6767 untouched; no agent or terminal was created.

## Fix Round 2

Run on 2026-10-16 against the failed lines of `### Re-check 1` and the composer rows of the TASK-155 parity table. Code checks only; screen checks are DEFERRED (UI checker).

### Changed

- `packages/desktop-gpui/crates/composer/src/composer.rs`: own placeholder, input inset, focus hint, pill chevrons, slash anchor on load, detail card line height and gap, 16 px outer inset with an 820 px max width.
- `packages/desktop-gpui/crates/composer/Cargo.toml`: adds `bachuc-command` (focus shortcut chord).
- `packages/desktop-gpui/Cargo.lock`: updated by cargo.

### Mismatches

| Line                     | Fix                                                                                                                                                                                                                                                                                            | State |
| ------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----- |
| Placeholder look         | Electron sets `[data-composer-input]::placeholder` to `sm` (12 px) in `surface4` (`apply-root-font.web.ts`, `input.tsx` line 70). gpui-kit draws the placeholder in its muted color at the input size, so the view draws its own: `FONT_SM`, `surface4`, 21 px line height, one truncated line | fixed |
| Placeholder 12 px inset  | The textarea is `XSmall` (editor padding 0 top and bottom, 4 at the sides) with a -4 px side margin, so text starts at the 12 px surface padding. The input row is now 21 px, as in Electron                                                                                                   | fixed |
| Pill chevrons            | `ChevronDown` 14 px, `foreground_muted`, 1 px low, after the label of every pill (`ComboboxTrigger`)                                                                                                                                                                                           | fixed |
| Slash anchor, first open | When `list_commands` returns, the menu resets the selection to the best match and pins the list to the bottom                                                                                                                                                                                  | fixed |
| Detail card height       | Line height 1.2 (React Native Web `normal`): 2 + 24 + 16.8 + 4 + 14.4 = 61 px. Gap to the composer is 12 px (`OFFSET_FROM_ANCHOR`)                                                                                                                                                             | fixed |
| Composer card inset      | The composer root is Electron's `inputAreaContainer`: 16 px at the sides and bottom, content centered at `MAX_CONTENT_WIDTH` 820. In an 880 px pane the card runs from 30 to 850 px. Set in the composer crate, not in `crates/app`                                                            | fixed |
| `⌘L to focus` hint       | `sm`, `foreground_muted` at 0.5 opacity, top right of the input, while the input is empty and unfocused. The chord comes from the `message-input.focus` shortcut; unassigned shows nothing                                                                                                     | fixed |
| Diff-count row           | Not added. Electron shows the workspace `diffStat` (against the base ref) from the workspace descriptor. `bachuc_client` has no workspace diff stat; `checkout_status` gives uncommitted files only and gets no updates without `watch_checkout`, which only the Explorer may call             | open  |

### Verified

- `CARGO_BUILD_JOBS=4 cargo.sh test -p bachuc-composer` -> PASS: 24 passed (new: focus hint chord and unassigned case).
- `cargo.sh fmt --check -p bachuc-composer` -> PASS.
- `CARGO_BUILD_JOBS=4 cargo.sh clippy -p bachuc-composer --all-targets` -> PASS: no composer warning (2 pre-existing warnings in `crates/protocol/build.rs`).
- `CARGO_BUILD_JOBS=4 cargo.sh build -p bachuc-app` -> PASS.
- Public API unchanged: `Composer::new`, `agent_id`, `focus`, `attachment_count`, `ComposerEvent`.
- Parity re-check of placeholder look and inset, chevrons, first-open anchor, detail card, card inset, and focus hint -> DEFERRED (UI checker).

### Questions

- The AX placeholder attribute is gone from `composer-input`: the textarea has no placeholder now, so gpui-kit sets no `aria_placeholder`. The AX label stays "Message". gpui-kit has no per-input placeholder color; a theme-level placeholder color in gpui-kit would let the state keep it.
- The diff-count row needs a workspace diff stat (`additions`, `deletions`) on `WorkspaceSummary` or `WorkspaceNode` in `bachuc_client`, kept fresh from the daemon's workspace updates.
- The focus hint reads the shortcut table once per composer. A rebind in settings shows in composers opened after it. Electron also shows it only in the active composer (`isActiveComposer`); GPUI shows it in every unfocused empty composer, which matters only with split panes.

### Lessons

- gpui-kit sets a multi-line input's editor padding from its `Size` (`input_px`, `input_py`) in render, so `.px_0()` on the textarea does not remove it. Use `.xsmall()` (4 px sides, 0 top and bottom) and a negative side margin.
- A ScrollHandle `scroll_to_bottom()` is a one-shot flag used at the next prepaint with that frame's content size. Call it again after rows arrive asynchronously.
