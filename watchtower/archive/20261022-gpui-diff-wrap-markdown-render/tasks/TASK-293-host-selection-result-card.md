# TASK-293 Host selection result card API

Group: E (writes plugin-host and timeline selection files and docs/plugins.md; no other TASK writes them)
Class: risky

## Brief

Goal: Let a GPUI plugin add a selection action whose answer is text, and have the host show that text in a card at the selection.

Change: a selection action only runs a handler and closes -> a result action shows a host-drawn card with "Đang dịch..." style loading, then the text or an error.

Boundaries:

- Follows [ADR-0063](watchtower/decisions/ADR-0063-host-drawn-selection-result-card-over-plugin-views.md): the host draws the card; the plugin returns only text.
- Add a new contribution type next to `SelectionAction` in [packages/desktop-gpui/crates/plugin-host/src/contributions.rs](packages/desktop-gpui/crates/plugin-host/src/contributions.rs). Do not add a field to `SelectionAction`: the Cite crate builds it as a struct literal.
- The toolbar shows plain actions first, then result actions, in plugin order. A result action button looks the same as a plain one.
- The card replaces the toolbar at the same origin from `toolbar_origin` in [packages/desktop-gpui/crates/timeline/src/plugins.rs](packages/desktop-gpui/crates/timeline/src/plugins.rs), clamped inside the timeline bounds.
- The card has: the action title as a small muted header, a body, and a Copy button. The body is `Loading...` text from the action (`loading` field) while the task runs, then the text rendered with `TextView::markdown`, or the error message in the error color.
- Planner call: the card is at most 420 px wide and 320 px high before zoom; a longer body scrolls inside the card.
- The card closes on a click outside it, on Esc, and on a new selection. A late answer for a closed card is dropped.
- Copy writes the result text (not the markdown render) to the clipboard. Copy is hidden while loading and on error.
- Trap: `run_selection_action` in [packages/desktop-gpui/crates/timeline/src/plugin_view.rs](packages/desktop-gpui/crates/timeline/src/plugin_view.rs) takes `self.selection` and clears the text selection. The card must keep its own copy of the anchor origin, because the selection and its bounds are gone after the press.
- Trap: the press runs on mouse down (see the comment in `render_selection_toolbar`). The card's click-outside close must not fire on that same press.
- `SelectionResultHandler` takes no `&mut Window`, unlike `Handler<I>` in [packages/desktop-gpui/crates/plugin-host/src/lib.rs](packages/desktop-gpui/crates/plugin-host/src/lib.rs). Keep it that way; `ctx.rpc` needs only `App`.
- Trap: `text_state_changed` and `render_selection_toolbar` in `plugin_view.rs` return early when `selection_actions()` is empty. Both must count result actions too, or a plugin with only result actions never sees a selection.
- Trap: `run_selection_action` clears the selection, which fires `text_state_changed` with no payload. Only a new `Some` payload closes the card; a `None` must not.
- Planner call: give the card its own state with an id counter (`id: u64`). A task answer carries the id it started with; the timeline drops it when the id differs or the card is closed. A second press starts a new id.
- Planner call: the card takes a focus handle when it opens and closes on Esc in its own `on_key_down`. The timeline Esc handler only fires when the timeline has focus. Click outside uses `on_mouse_down_out`, as the timeline already does elsewhere; the card renders after the opening press, so that press does not reach it.
- The `Copy` button uses `ClipboardItem::new_string` with `cx.write_to_clipboard`, as [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) does. The timeline crate already imports `TextView`.

How:

- Add `SelectionResultAction` and `SelectionResultHandler` in `contributions.rs`, a `selection_result_actions` field and `add_selection_result_action` on `Contributions`, and `selection_result_actions` on the index (filled in `build`, as `selection_actions` is) in [packages/desktop-gpui/crates/plugin-host/src/index.rs](packages/desktop-gpui/crates/plugin-host/src/index.rs); export them from [packages/desktop-gpui/crates/plugin-host/src/lib.rs](packages/desktop-gpui/crates/plugin-host/src/lib.rs).
- In `plugin_view.rs`, add a card state (origin, title, loading text, status) and render it in place of the toolbar; spawn the action task and store its answer.
- Put the pure card size rule in `plugins.rs` as `pub fn card_size(content: Size<Pixels>, zoom: f32) -> Size<Pixels>`, clamped to 420 by 320 px times `zoom`. Place the card with the existing `toolbar_origin`. Test both in [packages/desktop-gpui/crates/timeline/tests/plugins.rs](packages/desktop-gpui/crates/timeline/tests/plugins.rs), next to the `toolbar_origin` tests.
- In [docs/plugins.md](docs/plugins.md), add one line on GPUI result actions to `## GPUI native clients`, and fix the selection action text that calls them web-only so it names the GPUI support.

Files:

- [packages/desktop-gpui/crates/plugin-host/src/contributions.rs](packages/desktop-gpui/crates/plugin-host/src/contributions.rs) (new type and adder)
- [packages/desktop-gpui/crates/plugin-host/src/index.rs](packages/desktop-gpui/crates/plugin-host/src/index.rs) (store and list result actions)
- [packages/desktop-gpui/crates/plugin-host/src/lib.rs](packages/desktop-gpui/crates/plugin-host/src/lib.rs) (exports)
- [packages/desktop-gpui/crates/timeline/src/plugin_view.rs](packages/desktop-gpui/crates/timeline/src/plugin_view.rs) (toolbar buttons, card state, card render, close rules)
- [packages/desktop-gpui/crates/timeline/src/plugins.rs](packages/desktop-gpui/crates/timeline/src/plugins.rs) (`card_size`)
- [packages/desktop-gpui/crates/timeline/tests/plugins.rs](packages/desktop-gpui/crates/timeline/tests/plugins.rs) (tests of `card_size`)
- [docs/plugins.md](docs/plugins.md) (one line on result actions)

Interfaces:

- Produces: `pub type SelectionResultHandler = Rc<dyn Fn(AssistantSelection, &mut App) -> Task<anyhow::Result<String>>>` in `bachuc_plugin_host`, for TASK-295
- Produces: `pub struct SelectionResultAction { pub id: &'static str, pub title: SharedString, pub loading: SharedString, pub run: SelectionResultHandler }` in `bachuc_plugin_host`, for TASK-295
- Produces: `Contributions::add_selection_result_action(&mut self, action: SelectionResultAction)`, for TASK-295

Expected result:

- WHEN a plugin adds a result action THE selection toolbar SHALL show its button after the plain actions.
- WHEN the owner presses a result action THE timeline SHALL show a card at the toolbar place with the `loading` text.
- WHEN the task returns text THE card SHALL show it as markdown with a Copy button.
- IF the task returns an error THEN THE card SHALL show the error message and no Copy button.
- WHEN the owner clicks outside the card, presses Esc, or makes a new selection THE card SHALL close.
- IF the answer arrives after the card closed THEN THE timeline SHALL drop it.
- WHILE no plugin adds a result action, THE toolbar SHALL look and behave as before.

Anti-goal: the Cite action keeps working the same way (press adds one Quote chip, toolbar closes); tripwire; read from a screen check of Cite on live before the change and at completion.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-plugin-host -p bachuc-timeline` -> all tests pass, including new `card_size` tests: a 200 by 100 px content stays 200 by 100; a 900 by 900 px content becomes 420 by 320 at zoom 1 and 525 by 400 at zoom 1.25.
- `cd packages/desktop-gpui && ./cargo.sh clippy -p bachuc-plugin-host -p bachuc-timeline -- -D warnings` -> no warnings.
- `cd packages/desktop-gpui && ./cargo.sh check -p bachuc-plugin-assistant-cite` -> builds with no change to the Cite crate.
- Screen check (UI checker, `cua_repl`, live GPUI build with TASK-295 installed): select assistant text, press "Dịch" with the AX action -> a card shows "Đang dịch...", then Vietnamese text and a Copy button.
- Screen check: press "Dịch" twice fast -> one card shows the second answer only.
- Screen check: with the card open, press Esc -> the card closes; open it again, then select other text -> the card closes.
- Screen check: stop the plugin server (`npm run cli -- plugin disable selection-translate`), press "Dịch" on a host where the button still shows -> the card shows an error and no Copy button. If the button hides when the plugin stops, record that instead.
- Owner check: click outside the card -> it closes; press Copy -> the clipboard has the plain translation text.
- Anti-goal: select text and press "Cite" -> one Quote chip in the composer and the toolbar closes, as before.
