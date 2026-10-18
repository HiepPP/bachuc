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
