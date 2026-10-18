# TASK-243 Outcome

## Status

DONE (code checks pass; UI checker lines DEFERRED).

## Changed

- `packages/desktop-gpui/vendor/gpui-base/src/text/style.rs`: field `list_spacing: Option<(Rems, Rems)>` (default `None`), `with_list_spacing`, `list_spacing`, and `PartialEq`.
- `packages/desktop-gpui/vendor/gpui-base/src/text/node.rs`: the top-level `BlockNode::List` takes `pt(top)` and `pb(bottom)` in place of `pb(mb)`; `bottom` applies only when `mb` is not 0.
- `packages/desktop-gpui/vendor/gpui-base/src/text/document.rs` (not in the spec `Files:`): new `NodeRenderOptions.in_blockquote`, set by the blockquote branch. Needed because `depth` stays 0 inside a blockquote, so `depth == 0` alone would also match a list in a blockquote.
- `packages/desktop-gpui/vendor/gpui-base/BACHUC-PATCH.md`: a section for the patch (ADR-0044).
- `packages/desktop-gpui/crates/timeline/src/lib.rs`: constants `LIST_SPACING_TOP` (4) and `LIST_SPACING_BOTTOM` (16); the style sets them zoomed through `window_rems(zoomed(X, cx), rem_size(cx))`.

## Contract

- `TextViewStyle::with_list_spacing(self, top: Rems, bottom: Rems) -> Self` and `TextViewStyle::list_spacing(&self) -> Option<(Rems, Rems)>`, as TASK-246 consumes them. The timeline already calls them zoomed.
- A top-level list (`depth == 0`, not in a blockquote) gets `pt(top)`; `pb(bottom)` replaces `pb(mb)` only when `mb` is not 0 (a last list keeps 0). A nested list and a list in a blockquote keep today's spacing.

## Verified

- Group-level run (one cargo call for TASK-243 to TASK-246): `./cargo.sh test -p bachuc-timeline -p bachuc-composer -p bachuc-app` -> pass.
- Group-level: `./cargo.sh build -p bachuc-app` -> builds; no warning in the vendor crate or in changed files.
- Group-level: `./cargo.sh fmt --check -p bachuc-timeline`, `-p bachuc-composer`, `-p bachuc-app` -> exit 0.

## Anti-goal

DEFERRED (UI checker): the other rows of the TASK-233 gap table within 1 px of run 2. All four `UI checker:` lines of the spec are DEFERRED (UI checker).

## Lessons

- A list inside a blockquote is still `depth == 0`; `NodeRenderOptions.in_blockquote` tells them apart.
- The TASK-233 outcome moved to `watchtower/archive/20261018-gpui-polish-and-fixes/tasks/`; the spec link is stale but the gap table is not needed for the code.

## UI Checks

- UI checker: TASK-233 thread vs /tmp/wt-ui/ref-233-electron.png, paragraph to list and list end to code block within 2 px -> UNVERIFIED (unattended): background frame
- UI checker: last-node list and nested list message, bottom gap only under the top-level list -> UNVERIFIED (unattended): background frame
- UI checker: list-first message and blockquote list, 4 px on top of the first -> UNVERIFIED (unattended): background frame
- UI checker: Cmd+= once, list gaps grow with the zoom -> UNVERIFIED (unattended): background frame
- Anti-goal: TASK-233 gap table rows within 1 px of run 2 -> UNVERIFIED (unattended): background frame

Evidence: /tmp/wt-ui/zoom/before-initial-1200.png and before-initial-1200b.png (8 s apart), before-key-try.png, before-key-try2.png: the 1200x800 window still shows "Connecting / live" while the app holds an ESTABLISHED socket to 127.0.0.1:6768. GPUI does not redraw a background window (Q-069), so no frame shows thread, composer, or zoom. A Cmd+= and a Cmd+- key reached the app (appearance.json changed 1.25 -> 1.5 -> 1.25) but the window did not redraw. The check bundle was built (exit 0); a baseline copy is at `Bachuc GPUI Zoom Before.app`. The checker did not use the front, the pointer, or AX clicks.

## UI Checks

Run 2026-10-19 on live with the check bundle; screenshots in `/tmp/wt-ui/zoom/`.

- Owner hand check, 2026-10-18: paragraph to list 4 px, list end 16 px, nested list adds no gap -> PASS (Electron match).
- Agent, zoom 150%, 1200x800: list gaps grow with the zoom -> PASS (`/tmp/wt-ui/zoom/after-150-1200-longrun.png`).
