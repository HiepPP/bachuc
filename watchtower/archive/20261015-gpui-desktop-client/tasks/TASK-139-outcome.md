# TASK-139 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/composer/Cargo.toml`: new crate `bachuc-composer` (deps: `bachuc-client`, `gpui-kit =0.7.3`, `base64 0.22`, `log`, `serde_json`; dev-dep `bachuc-theme` for the example).
- `packages/desktop-gpui/crates/composer/src/lib.rs`, `composer.rs`, `attachments.rs`: the composer view, its events, and image attachments.
- `packages/desktop-gpui/crates/composer/examples/composer.rs`: the composer for one agent above a plain text log of its stream.
- `packages/desktop-gpui/Cargo.lock`: updated by cargo (`base64 0.22` entry for this crate; the file is shared with the other builders).

## Contract

- `bachuc_composer::Composer` (a GPUI view) with `Composer::new(client: gpui_kit::Entity<bachuc_client::Client>, agent_id: String, window: &mut gpui_kit::Window, cx: &mut gpui_kit::Context<Composer>) -> Composer`, as the spec gives it.
- Extra public items: `ComposerEvent::Sent { text, images }` (emitted by `Composer` after it calls `send_prompt`), `Composer::agent_id`, `Composer::focus(&self, window, cx: &mut App)`, `Composer::attachment_count`, `Attachment`, `can_send`, `mime_for_path`.
- Behavior: a multiline textarea (1 to 8 rows) with `submit_on_enter`. Enter or Cmd+Enter sends. Shift+Enter adds a line. Blank text with no image sends nothing. Text keeps its lines and loses trailing whitespace. An image alone sends empty text.
- Send calls `Client::send_prompt` with no `activeTurnBehavior`, so a send during a run goes to the running turn. Stop calls `Client::cancel_agent` and is disabled when the agent is not running.
- Images: the Attach button opens `prompt_for_paths` (files, multiple); a paste of a clipboard image or copied image files adds chips. Accepted types are png, jpeg, gif, and webp. Each chip has a Remove button. Each image goes out as `{"data": <base64>, "mimeType": ...}`.
- Pills: Mode and Model are ghost buttons that open a `PopupMenu` upward. The items come from `AgentControls`, and a click calls `Client::set_mode` or `Client::set_model`. A pill with no choices is disabled. The pills refresh on `ClientEvent::AgentsChanged` and `Connected`.
- Accessibility labels (`getAXState`): text entry "Message" (id `composer-input`), "Attach images", "Mode: <label>" (id `composer-mode`), "Model: <label>" (id `composer-model`), "Stop", "Send", and "Remove <name>" per chip.

## Verified

- `packages/desktop-gpui/cargo.sh test -p bachuc-composer` -> 8 unit tests pass: Enter, Shift+Enter, and secondary Enter mapping; empty and blank text; line keeping; image-only send; pill labels; mime by extension; JSON shape.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-composer` -> exit 0.
- `packages/desktop-gpui/cargo.sh clippy -p bachuc-composer --all-targets` -> no warnings in `bachuc-composer` (2 warnings remain in `crates/protocol/build.rs`, not owned here).
- `packages/desktop-gpui/bundle.sh example composer BachucComposer io.github.hieppp.bachuc-composer` -> built `target/bundles/BachucComposer.app`.
- Live mock agent `007fb27` (ten-second-stream), typed through `cua_repl` plus `Return` -> `npm run cli -- logs 007fb27` shows `[User] hello composer`.
- Same agent: `line one`, `shift+Return`, `line two`, `Return` -> `logs` shows one `[User]` message with two lines; the timeline cache row for the second multi-line message holds one `user_message` with 5 lines.
- Mode pill to "Approval Test" -> `npm run cli -- inspect 007fb27` shows `Mode approval-test`; pill label became "Mode: Approval Test". Model pill to "Thirty minute stream" -> `inspect` shows `Model thirty-minute-stream`. Mode set back to "Load Test" -> `inspect` shows `load-test`.
- Stop: with the agent running on `thirty-minute-stream`, click Stop -> `npm run cli -- ls -g` shows the agent `idle`.
- Send during a run (agent `a7e10d4` running): `Return` -> `logs` shows the new `[User]` message and a new cycle.
- Attach through the file picker: Attach opened the Open panel, `Cmd+Shift+G` `/tmp/composer-red.png` selected the PNG, and Attach added the chip `composer-red.png (168 B) x` (AX `Remove composer-red.png`). Then Return sent the message; the chip left.
- Clipboard paste: a 64x64 green PNG on the pasteboard (`swift` script) and `Cmd+V` -> chip `pasted-1.png` (AX `Remove pasted-1.png`). Return sent it to live Claude agent `ca82843` (claude-sonnet-5-5) -> its reply: "The image is a solid green square (about 64x64)". This proves the image arrives as an image on the daemon and the agent.
- `getAXState` on the example -> `text entry area Message (composer-input)`, `button Attach images`, `button Mode: Load Test (composer-mode)`, `button Model: Thirty minute stream (composer-model)`, `button Stop`, `button Send`.
- Not verified: `npm run cli -- inspect <id>` or the timeline showing the image on the user message. The timeline `user_message` item has no image field, and the mock provider ignores images, so that line cannot pass for any client. Replacement: the Claude agent described the pasted image (above). The picker-attached image reached the same `send_prompt` path (same `Attachment::to_json`), but no agent described that file: the second Open panel session hung and the app was quit (see Lessons).
- `npm run format:check:files -- watchtower/tasks/TASK-139-outcome.md` -> "All matched files use the correct format."
- GitNexus impact: not run, because the TASK edits no existing symbol; every file is new.

## Anti-goal

Gauge: example `composer` on the mock stream (`thirty-minute-stream`), foreground, typing included, CPU at most 10.0% and footprint at most 250 MB, read with `measure.sh 60`.

- Checkpoint 1, after the input worked, live mock agent `007fb27` running:
  - Read 1, 2026-10-15T10:59:45Z to 11:00:45Z, `measure.sh 60 comp="BachucComposer.app"` -> CPU 1.4%, RSS 104 MB, footprint 38 MB. Typing covered about 30 s of it (a `cua_repl` call timed out and reset). Weak evidence: the pattern also matched the `measure.sh` shell (procs 2).
  - Read 2, 2026-10-15T11:01:43Z to 11:02:43Z, `measure.sh 60 comp="BachucComposer.app/Contents/MacOS/composer"`, window raised, typing and Shift+Enter lines every 3 s -> CPU 4.2%, RSS 56 MB, footprint 90 MB. PASS.
- Checkpoint 2, at completion, agent `a7e10d4` running on `thirty-minute-stream`, same binary as the final code:
  - Read 3, 2026-10-15T14:31:47Z to 14:32:47Z, `measure.sh 60 comp="pids:14477"` (the app pid only), typing every 3 s with the input focused -> CPU 0.8%, RSS 73 MB, footprint 51 MB. A later screenshot showed the window had not repainted for part of it, so this read is low.
  - Read 4, 2026-10-15T14:34:00Z to 14:35:00Z, `measure.sh 60 comp="pids:14477"`, one character every 1.5 s plus Shift+Enter, window raised -> CPU 2.6%, RSS 73 MB, footprint 52 MB. A screenshot taken right after showed all typed text and the stream log current. PASS.
- Result: PASS. No tripwire crossed (limits 10.0% CPU, 250 MB).

## Lessons

- `gpui-kit` `test-support` as a dev-dependency breaks cargo resolution in this workspace: `failed to select a version for cocoa` (`gpui-fast-macos` wants `=0.26.0`, `gpui-pre-macos` locks 0.26.1), because the weak `gpui-fast?/...` features pull in `gpui-fast-platform`. Without the feature the crate resolves. So tests here cover key mapping and send rules as plain functions (`enter_sends`, `outgoing_text`), not through a headless window. A later TASK that needs `TestAppContext` must first fix that resolution (for example `cargo update -p cocoa --precise 0.26.0`, which adds the `gpui-fast` crates to the lock).
- `submit_on_enter(true)` on `TextareaState` makes plain Enter emit `InputEvent::PressEnter { shift: false }` with no newline. Shift+Enter inserts the newline itself and still emits `PressEnter { shift: true }`, so the handler must check `shift`. `Textarea::on_paste` gets the clipboard item before the text paste; return `true` to consume it. `use gpui_kit::*` in a module brings GPUI's `test` macro over `#[test]`; import names in test modules.
- `dropdown_menu_with_anchor(Anchor::BottomLeft, ...)` on a ghost `Button` opens the menu upward, which a bottom composer needs. The menu rebuilds when it is dismissed, so item state is fresh on each open. The checked item shows no check mark in the build.
- `cua_repl`: `click(index)` and `pressKey("Return")` (not "Enter"); `shift+Return` and `super+shift+g` work. The AX indices shift when a chip adds a row, and the Open panel numbers its own tree, so read the full tree (`getAXState({disableDiffing:true, emit:false})`) and match the index by label before a click. A click on a pill after an errored click left the popover in a state where it did not open until Escape.
- The AX `Value` of the text input lags (it was stale by many keystrokes); read the screenshot to see the current text. GPUI draws on events, so a screenshot taken without a fresh input can show an older frame.
- A `cua_repl` call that waits for a window can hang for a long time (it blocked 3230 s once, right after launching the bundle with `getApp`). Wrap `getApp` and each step in `Promise.race` with a timeout, and wait a few seconds after `open` before `getApp`. That hang kept the shared screen lock far past 15 minutes (from 11:21 to 14:20 by the machine clock); the lock was released at 14:20.
- The timeline `user_message` item has no image field, and the mock provider ignores images, so a mock agent cannot show an image check. A real Claude agent can: `claude-sonnet-5-5` named the colour of a pasted PNG.
- The mock `thirty-minute-stream` agent answers each new prompt with a new cycle; a prompt sent while it runs goes to the running turn.
- `measure.sh` with `comp="BachucComposer.app"` matches its own shell too; pass `pids:<pid>` from `pgrep` instead.

## Questions

- Q-055 default taken: click opens the menu; no wheel behavior.
- Should the Stop button hide when the agent is not running? It stays visible and disabled, so the tree shape is stable for `getAXState`. TASK-141 can change that.
- Should a later TASK add the `gpui-kit` `test-support` resolution fix so the composer can get window-level key tests? See Lessons.
