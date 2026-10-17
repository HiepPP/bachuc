# TASK-178 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/crates/timeline/Cargo.toml`: adds `bachuc-plugin-host` and `log`.
- `packages/desktop-gpui/Cargo.lock` (generated): the two new dependency lines of `bachuc-timeline`.
- `packages/desktop-gpui/crates/timeline/src/plugins.rs` (new, `pub mod plugins`): the pure rules.
  - `transform_entry`, `check_result`, `is_valid_kind`, `item_id`, `phase_of`, `shown_text`.
  - `Projection` and `parts`, `step_parts`, `call_listed`, `renderer_for_item`, `daemon_item`.
  - `Projector`: the cache by entry key with `seqEnd`, phase, and result.
  - `assistant_selection`, `selection_text`, `toolbar_origin`, `unique_match`, `reveal_plan`.
- `packages/desktop-gpui/crates/timeline/src/plugin_view.rs` (new): plugin item views, the "Plugin timeline item unavailable." note, the selection toolbar, and `TimelineView::reveal_passage`.
- `packages/desktop-gpui/crates/timeline/src/lib.rs`:
  - `sync` also compares the host `timeline_revision` and the labels hash, runs `Projector::sync` after the grouper, and passes the touched entries to `update_messages` and `update_hashes`.
  - `hash_entry` folds in the projection hash and the revision.
  - `render_entry` draws projections, and `"plugin"` entries draw through `render_daemon_item`.
  - `render_step` uses `step_parts` for notes and items.
  - `space_below` gives a hidden entry no space.
  - `render` records the timeline bounds and draws the toolbar.
  - The view subscribes to `PluginHostEvent`.
- `packages/desktop-gpui/crates/timeline/src/find_bar.rs`: `ring_color` returns `t.accent`; ring width and offset are unchanged.
- `packages/desktop-gpui/crates/timeline/tests/plugins.rs` (new): 15 pure tests.

## Contract

- Produces `TimelineView::reveal_passage(&mut self, message_id: &str, text: Option<&str>, window: &mut Window, cx: &mut Context<TimelineView>)` for TASK-191.
- Produces the step rule for TASK-184, `plugins::step_parts(projection, is_edit)`:
  - With no projection, an edit draws `render_edit` and another call draws nothing, as today.
  - With a projection, plugin items always draw, in call order.
  - A kept edit source keeps its edit row at its `First` or `Last` place.
  - `call_listed` takes a replaced call out of a picked step's call list.
- Consumes `bachuc_plugin_host::host`, `PluginHost::index`, `PluginHost::timeline_revision`, `PluginHostEvent`, `ContributionIndex::{transformers_for, renderer_for, selection_actions}`, the timeline kinds, and `Client::agent_labels` with their exact TASK-168 and TASK-170 names.
- `TimelineView::new`, `find`, and `TimelineEvent` are unchanged. With no host global, as in `examples/timeline.rs`, nothing is projected.
- Events: `TimelineTransformsChanged` and `ContributionsChanged` sync, and `PluginStateChanged` only calls `cx.notify()`.
- Safe defaults and choices, for the lead:
  - An empty item `id` is rejected, as in Electron's `parseTransformResult`. The spec names only the kind and version checks.
  - Transformed items also need a renderer of their own plugin, as Electron's `useInstalledPlugin(item.pluginId)` does. The spec states this rule only for daemon items.
  - The selection toolbar runs its action on mouse down. A gpui-base text view clears every selection in the capture phase of a press, so the payload is read when the selection settles, before the press.
  - The toolbar is centered by its measured size. The first frame uses an estimate of 120 by 30, and a `canvas` then measures it and refreshes once.
  - A reveal matches text in the segments' Markdown sources (`rendered_text().source()`). These sources have code spans linkified by `linkify_code_spans`. The match is mapped with `range_for_source`. User messages only scroll.
  - On a revision or labels change, every entry is rehashed and turn rows are rebuilt, and plugin views are cleared. This is rare, and an O(n) cost is allowed here by ADR-0039.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-timeline` -> exit 0, 80 passed, 0 failed (baseline 64).
  - `incremental_rows_match_full_regroup` and every grouping, tools, cards, and links test pass unchanged.
  - The new `tests/plugins.rs` has 15 tests. They cover first-wins order, the kind, version, and id checks, replace, `First` with text, `Last` with text, the hidden entry with grouping unchanged, and the item id rule.
  - They also cover the phase rule, daemon item lookup with a matching and a foreign `pluginId`, the step rule, labels and revision re-runs, the selection payload with and without a source range, toolbar placement, the reveal match rule, and the 20,000-entry count and timing test.
  - The unit test `focus_ring_uses_the_accent_color` covers the find ring color.
  - Log: `/tmp/wt-h-v1.txt`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-timeline` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0, with no warnings from this crate.
  - The first run was stopped by the harness while compiling. The rerun at 22:48 passed. Log: `/tmp/wt-h-app.txt`.
  - `git status` shows edits only under `crates/timeline` and in `Cargo.lock`. The untracked `crates/composer` files belong to group C.
- UI check (find ring screenshot against Electron 6770) -> DEFERRED (UI checker).
- Plugin rows, the selection toolbar, and the reveal on screen -> UNVERIFIED (unattended): code tests only. No plugin crate is registered until TASK-191.

## Anti-goal

- Checkpoint 0, before the first change: no projection step existed. The `bachuc-timeline` tests had 64 passed. Source: `/tmp/wt-h-base.txt`, 2026-10-16 19:14.
- Checkpoint 1, after the projection cache landed: one append to 20,000 entries ran the transformer once. The projection step took 13.8 us in a debug test, under the 2 ms limit. A revision change ran it once per `tool_call` (20,000). Source: `test --test plugins appending -- --nocapture`, 2026-10-16 19:22.
- Checkpoint 2, at completion: one run, and the step took 17.6 us. Source: the same command, 2026-10-16 22:48.
- Tripwire: not tripped.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground).

- UI check (find ring screenshot against Electron 6770) -> UNVERIFIED (unattended): needs pointer. A thread view cannot open: AX press on the GPUI sidebar rows changed nothing (`/tmp/wt-ui/gpui-t144.jpg`, `/tmp/wt-ui/gpui-tsc.jpg`), and the find bar needs a focused thread. The reference also lacks the plugin: every plugin on 6770 shows `failed`.

## Lessons

- A gpui-base `TextViewState` emits no selection event, but it notifies on each selection change, with `is_selecting` updated. `cx.observe` on each state is enough to track a settled selection.
- Any left press clears text selections in the capture phase, before a click lands. A toolbar that acts on the selection must read the selection before the press and act on mouse down.
- Projecting after grouping leaves the grouper and its tests alone. Turn rows are reused while a group's `Rc` is unchanged, so a projection change outside `grouper.changed()` must set `rows_stale`.

## UI Checks, round 2

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground).

- UI check (find ring screenshot against Electron 6770) -> UNVERIFIED (unattended): needs foreground. The thread view now opens (`/tmp/wt-ui2/after-press-ws31.jpg`), but its AX tree has no find button, so the find bar needs a keyboard shortcut, which is not an AX action. The reference also lacks the plugin (`failed` on 6770).
