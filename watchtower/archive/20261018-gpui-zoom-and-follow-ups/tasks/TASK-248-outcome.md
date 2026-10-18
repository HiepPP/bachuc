# TASK-248 Outcome

## Status

DONE (UI checker lines DEFERRED).

## Changed

- `crates/app/src`: `chrome.rs`, `new_workspace.rs`, `plugin_views.rs`, `shell.rs`, `toast.rs`, `workspace_draft.rs`.
- `crates/workspace/src/workspace.rs`.
- `crates/sidebar/src`: `sidebar.rs`, `ring.rs`.
- `panes.rs`, `main.rs`, and the `metrics` modules were not changed.

## Contract

- Every fixed padding, gap, icon, control size, radius, popup width, popup offset, and `text_size`/`line_height` in these files now reads `bachuc_theme::zoomed(X, cx)`.
- Helpers that had no `cx` take `cx: &App` as a new parameter: `menu_item` (chrome, sidebar), `editor_glyph_element`, `tab_glyph`, `close_overlay`, `icon_button`, `alert_disc`, `dot`, `workspace_menu`, `kebab`, `render_resize_handle`. `kebab` and `new_workspace_button` return `impl IntoElement + use<..>`, so they do not hold the borrow.
- Workspace tab layout: `layout_tabs` takes `zoom`. Label widths and the row width go into `tab_layout::compute` divided by zoom, and the widths come out multiplied by zoom. `track_drop` divides widths and x by zoom before `drop_slot`. Stored `tab_layouts` widths are real (zoomed) px.
- The running-status ring scales its painted shapes by `bachuc_theme::zoom(cx)`.

## Verified

Group-level run (shared by the group, not per TASK):

- `./cargo.sh test -p bachuc-app -p bachuc-workspace -p bachuc-sidebar` -> exit 0, all suites pass.
- `./cargo.sh build -p bachuc-app` -> builds. The one warning (`next_theme` unused, `state.rs`) is in a file this TASK did not change.
- `./cargo.sh fmt --check -p bachuc-app -p bachuc-workspace -p bachuc-sidebar` -> exit 0 for each. The app crate was formatted with `rustfmt` on the six changed files only.
- Remaining `px(` sites (16, excluding `.px(` method calls and `panes.rs`):
  - 1 px hairline: 2 (`sidebar.rs` 1485, `shell.rs` 3525)
  - `DIVIDER` hairline of the pane model: 2 (`workspace.rs`)
  - zero values: 4 (`px(0.)` in `ring.rs` x2, `workspace.rs` x2)
  - already zoomed: 2 (`ring.rs` `scaled`, `workspace.rs` tab `width`)
  - width the user dragged and saved: 2 (`shell.rs` explorer, `sidebar.rs` sidebar)
  - read from layout or list state: 2 (`sidebar.rs` slide `left`, `OVERDRAW`)
  - window geometry at creation (size, traffic lights, min size): 3 (`main.rs`)

## Anti-goal

DEFERRED (UI checker): 0 changed pixel rows at zoom 100% needs screenshots. At 100%, `zoomed(X, cx) == px(X)` in every swap, and the tab layout maps to itself at zoom 1.

## Lessons

- In edition 2024, an `impl IntoElement` return captures every `&` and `&mut` argument. A builder that takes `cx: &App` and returns `impl IntoElement` holds the borrow in its caller. Declare `+ use<..>` to release it.
- A regex swap of `px(` also matches the `.px(` method. Check `.px(zoomed(..))` after a bulk swap.

## UI Checks

- UI checker 1200x800 zoom 150%: header, tab bar with 3 tabs, sidebar with projects and agents, split pane -> UNVERIFIED (unattended): background frame
- Anti-goal: zoom 100%, 0 changed pixel rows before vs after -> UNVERIFIED (unattended): background frame

Evidence: /tmp/wt-ui/zoom/before-initial-1200.png and before-initial-1200b.png (8 s apart), before-key-try.png, before-key-try2.png: the 1200x800 window still shows "Connecting / live" while the app holds an ESTABLISHED socket to 127.0.0.1:6768. GPUI does not redraw a background window (Q-069), so no frame shows thread, composer, or zoom. A Cmd+= and a Cmd+- key reached the app (appearance.json changed 1.25 -> 1.5 -> 1.25) but the window did not redraw. The check bundle was built (exit 0); a baseline copy is at `Bachuc GPUI Zoom Before.app`. The checker did not use the front, the pointer, or AX clicks.

## UI Checks

Run 2026-10-19 on live with the check bundle; screenshots in `/tmp/wt-ui/zoom/`.

- Zoom 150%: header, tab bar, sidebar with projects and agents -> PASS.
- Split pane at 150%: UNVERIFIED (unattended): not opened.
