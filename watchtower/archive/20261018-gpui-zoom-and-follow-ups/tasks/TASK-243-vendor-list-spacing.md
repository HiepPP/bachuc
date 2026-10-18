# TASK-243 Vendor list spacing option

Group: T (timeline crate, vendor text view; shared files with TASK-244 to TASK-246)
Class: risky

## Brief

Goal: list start and end gaps in a thread match Electron, as Q-144 answered.

Change: a top-level list has no top space and `mb` under it -> a `TextViewStyle` list option, and the timeline sets top 4 px and bottom 16 px.

Boundaries:

- Q-144 (owner, 2026-10-18): add `TextViewStyle::with_list_spacing(top, bottom)`; the timeline sets 4 and 16 px.
- The option defaults to `None`, so other `TextViewStyle` users keep today's look.
- Planner call: "top-level" means `depth == 0`. A nested list and a list inside a blockquote keep today's spacing. `NodeRenderOptions.in_list` is never set to true (`document.rs:25`), so do not rely on it.
- Planner call: at depth 0, `pt(top)` always applies, as Electron's list `marginTop: 4`. `pb(bottom)` replaces `pb(mb)` only when `mb` is not 0; a list that is the last node keeps 0.
- Vendor patch per [ADR-0044](watchtower/decisions/ADR-0044-vendored-gpui-base-patch-over-git-fork-or-log.md). Add `with_list_spacing` to `PartialEq` too.
- Convert the 4 and 16 px with `window_rems(zoomed(X, cx), rem_size(cx))`, as the paragraph gap does.
- Non-goal: the list item gap (3.75 vs 4 px) and the rule gaps of the TASK-233 outcome table.

How:

- Read the gap table in [TASK-233-outcome.md](watchtower/archive/20261018-gpui-polish-and-fixes/tasks/TASK-233-outcome.md).
- Add `list_spacing: Option<(Rems, Rems)>` and `with_list_spacing(top: Rems, bottom: Rems) -> Self` in `style.rs`.
- In `node.rs` near line 3948, on the top-level list div: `.pt(top)` and `.pb(bottom)` in place of `.pb(mb)` when `mb` is not 0 and the option is set.
- Set it next to `with_paragraph_gap` in the timeline.

Files:

- [packages/desktop-gpui/vendor/gpui-base/src/text/style.rs](packages/desktop-gpui/vendor/gpui-base/src/text/style.rs) (field, builder, getter, `PartialEq`)
- [packages/desktop-gpui/vendor/gpui-base/src/text/node.rs](packages/desktop-gpui/vendor/gpui-base/src/text/node.rs) (top-level list padding)
- [packages/desktop-gpui/crates/timeline/src/lib.rs](packages/desktop-gpui/crates/timeline/src/lib.rs) (set the option near line 1599)

Interfaces:

- Produces: `TextViewStyle::with_list_spacing(self, top: Rems, bottom: Rems) -> Self` and `TextViewStyle::list_spacing(&self) -> Option<(Rems, Rems)>` for TASK-246, which keeps the call zoomed.

Expected result:

- In the TASK-233 reference thread, paragraph to list and last list item to code block are within 2 px of Electron.

Anti-goal: every other row of the TASK-233 gap table stays within 1 px of its run 2 value; read from the UI checker screenshot pair after the change.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-timeline` -> pass.
- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds; no new warnings in the vendor crate.
- UI checker: the TASK-233 thread (heading, 2 paragraphs, 3-item list, code block, paragraph) next to `/tmp/wt-ui/ref-233-electron.png` or a new Electron shot -> paragraph to list and list end to code block within 2 px.
- UI checker: a message whose last node is a list, and a message with a nested list -> bottom gap only under the top-level list; the nested list has no extra 4 or 16 px.
- UI checker: a message that starts with a list, and a list inside a blockquote -> the first gets 4 px on top; the blockquote list looks as before.
- UI checker: Cmd+= once while the TASK-233 thread shows -> the list gaps grow with the zoom (the `PartialEq` change makes the view rebuild).
- Anti-goal: the other rows of the gap table -> within 1 px of run 2.
