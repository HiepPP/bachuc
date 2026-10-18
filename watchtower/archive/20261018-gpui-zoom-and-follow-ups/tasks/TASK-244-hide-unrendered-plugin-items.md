# TASK-244 Hide plugin items with no renderer

Group: T (timeline crate; shared files with TASK-243, TASK-245, TASK-246)
Class: code

## Brief

Goal: a plugin timeline item with no GPUI renderer draws nothing, as Q-148 answered. Today html-reply rows show "Plugin timeline item unavailable."

Change: `render_plugin_item` draws the unavailable note -> the item takes no space, and its row adds no gap when it has nothing else.

Boundaries:

- Q-148 (owner, 2026-10-18): hide, do not add an html-reply GPUI renderer.
- Keep the source entry of a projection when the transformer keeps it; hide only the item parts.
- A row whose every part is hidden must not add its `pb(space)`, or an empty gap stays.
- Planner call: put the rule "show this part" in a pure function and test it in [crates/timeline/tests/plugins.rs](packages/desktop-gpui/crates/timeline/tests/plugins.rs).

How:

- Callers: `plugin_view.rs` lines 271 and 280, and `lib.rs` lines 2069 and 2087.
- Return no element (or filter the part out) when `plugin_views` has no view for the id.
- Skip the row gap of an entry with no visible part: `render_projected` (`plugin_view.rs:274`) returns an empty `v_flex`, and the row adds `pb(space)` at `lib.rs:1219`; fix it through `space_below` (`lib.rs:1229`).
- Planner call: the malformed daemon item (`render_plugin_item("", ..)` at `plugin_view.rs:281`) is hidden too.

Files:

- [packages/desktop-gpui/crates/timeline/src/plugin_view.rs](packages/desktop-gpui/crates/timeline/src/plugin_view.rs)
- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (the two callers and the row gap)
- [packages/desktop-gpui/crates/timeline/tests/plugins.rs](packages/desktop-gpui/crates/timeline/tests/plugins.rs)

Interfaces:

- Consumes: nothing from TASK-243; runs after it only because both write `lib.rs`.
- Produces: no public API.

Expected result:

- A thread with html-reply items shows no "unavailable" note and no empty gap where it was.
- The turn diff card of thread-branch (has a renderer) still shows.

Anti-goal: rows with a renderer keep their spacing; the row gaps of the TASK-233 table change by 0 px; read from the UI checker shot.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> pass, with a new test for the show rule: no renderer -> hidden; renderer -> shown; projection with kept source and hidden items -> source only.
- `rg -n "Plugin timeline item unavailable" packages/desktop-gpui/crates/timeline/src` -> no match, or only a comment.
- UI checker: a thread with an html-reply answer -> no note, no blank band between the rows around it.
- UI checker: a thread with a turn diff card -> the card shows.
- UI checker: a step list where one plugin item has a renderer and one has none -> the first shows, the second leaves no gap.
