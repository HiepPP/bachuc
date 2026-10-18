# TASK-244 Outcome

## Status

DONE (code checks pass; UI checker lines DEFERRED).

## Changed

- `packages/desktop-gpui/crates/timeline/src/plugin_view.rs`: `render_plugin_item(&self, id) -> Option<AnyElement>` returns None when no view exists, so no "unavailable" note draws. `render_daemon_item` no longer takes `cx`; a malformed item draws an empty div. New `TimelineView::shown_parts` asks the contribution index for a renderer per item. `render_projected` draws only `shown_parts`.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: the two step callers (`notes.extend`, `edits.extend`) skip a None; `space_below` takes `cx: &App` and returns None for a projected entry with no shown part, so the row adds no `pb`.
- `packages/desktop-gpui/crates/timeline/src/plugins.rs` (not in the spec `Files:`): pure `plugins::shown_parts(projection, has_renderer)`. The test crate cannot reach `plugin_view` (private module), so the rule lives here.
- `packages/desktop-gpui/crates/timeline/tests/plugins.rs`: `an_item_with_no_renderer_is_hidden`.

## Contract

- No public API except `plugins::shown_parts<'a>(&'a Projection, impl Fn(&ProjectedItem) -> bool) -> Vec<Part<'a>>`.
- Show rule: the kept source always shows; an item shows only when `renderer_for_item(index, plugin_id, kind, version)` is Some.

## Verified

- Group-level run: `./cargo.sh test -p bachuc-timeline -p bachuc-composer -p bachuc-app` -> pass, including `an_item_with_no_renderer_is_hidden` (no renderer -> hidden; renderer -> shown; kept source with hidden items -> source only).
- `rg -n "item unavailable" packages/desktop-gpui/crates/timeline/src` -> no match.

## Anti-goal

DEFERRED (UI checker): TASK-233 row gaps change by 0 px. The three `UI checker:` lines are DEFERRED (UI checker).

## Lessons

- The gap must follow the renderer lookup, not the built view: `space_below` also runs in `sync`, before `ensure_plugin_views` builds any view.

## UI Checks

- UI checker: html-reply answer thread, no note and no blank band (also needs a daemon item that live lacks) -> UNVERIFIED (unattended): background frame
- UI checker: turn diff card thread, the card shows -> UNVERIFIED (unattended): background frame
- UI checker: step list with one rendered and one unrendered plugin item, no gap -> UNVERIFIED (unattended): background frame
- Anti-goal: TASK-233 row gaps change by 0 px -> UNVERIFIED (unattended): background frame

Evidence: /tmp/wt-ui/zoom/before-initial-1200.png and before-initial-1200b.png (8 s apart), before-key-try.png, before-key-try2.png: the 1200x800 window still shows "Connecting / live" while the app holds an ESTABLISHED socket to 127.0.0.1:6768. GPUI does not redraw a background window (Q-069), so no frame shows thread, composer, or zoom. A Cmd+= and a Cmd+- key reached the app (appearance.json changed 1.25 -> 1.5 -> 1.25) but the window did not redraw. The check bundle was built (exit 0); a baseline copy is at `Bachuc GPUI Zoom Before.app`. The checker did not use the front, the pointer, or AX clicks.

## UI Checks

Run 2026-10-19 on live with the check bundle; screenshots in `/tmp/wt-ui/zoom/`.

- Owner hand check on live mock thread `mock TASK-244 html-reply + diff`: no "unavailable" note, no blank band, turn diff card shows -> PASS.
- Found while checking: an expanded edit diff drew an empty strip, also on the main build. Fixed in `7d60a0f` (edit-diffs list height); owner confirmed the diff lines show.
