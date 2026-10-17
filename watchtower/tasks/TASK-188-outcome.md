# TASK-188 Outcome

## Status

DONE

## Changed

- `packages/desktop-gpui/plugins/workspace-spaces` (new crate `bachuc-plugin-workspace-spaces`):
  - `Cargo.toml`: `anyhow`, `bachuc-client`, `bachuc-icons`, `bachuc-plugin-host`, `bachuc-theme`, `futures`, `gpui-kit.workspace = true`, `log`, `serde`, `serde_json`.
  - `src/lib.rs`: `plugin()` and the four contributions (project filter, "Move to" project menu, sidebar section, Spaces surface).
  - `src/model.rs`: state types, pure rules, and 48 unit tests.
  - `src/state.rs`: the `SpacesState` entity (settings, catalog, saves, refresh timer, selection).
  - `src/strip.rs`: numbered tabs and "+".
  - `src/space_menu.rs`: the Space menu with its five pages.
  - `src/page.rs`: the Spaces page.
- `packages/desktop-gpui/Cargo.lock` (generated): gained `bachuc-plugin-workspace-spaces`. Other builders' entries are in the same file.

## Contract

- Produces `bachuc_plugin_workspace_spaces::plugin() -> Box<dyn NativePlugin>` with id `"workspace-spaces"`. It is the only public item. The crate is not yet in the `bachuc-app` build graph (TASK-191 wires it).
- Contributions, by Electron line: filter `spaces` (12-19), project menu `move` (20), section `spaces` (21-28), surface `spaces` (29). The surface has the title "Spaces" and `IconName::Layers`; Electron adds no sidebar item, so none is added.
- Planner calls kept:
  - The host key is `Client::server_info().server_id`. The "All hosts" root set is never changed, and `with_host_state` keeps it and the other hosts.
  - One `SpacesState` per activation. The selection resets when TASK-170 drops it on a host switch.
  - Catalog load after `Connected` and every 15,000 ms, one request in flight (`refresh_due`, `next_wait_ms`). A refresh does not run while a save runs or with no connection.
  - After each load, save, or selection change the state calls `sidebar_filter_changed`, with `Some(Next)` or `Some(Previous)` only when the selection moved.
- Defaults taken where the spec left a gap:
  - With no known host, a save fails with "Spaces unavailable. Refresh and try again." Electron would write the "All hosts" root set.
  - A save needs the settings `Ready`, not the catalog. Electron also needs its snapshot. The page's create, rename, and move work before the catalog loads.
  - A change that leaves the values equal (an already assigned project in `add_project`) skips the write and resolves `true`. Electron wrote the same values again.
  - `members` is a `BTreeMap`, so key order is sorted. If two legacy keys name one project, the first by key order wins. Electron used insertion order.
  - The page and the sidebar share one selection. In Electron the page keeps its own per-host map.
  - The page's drag handler reads a left-button press and release on the project region. A drag that starts on a card button still counts.
  - Escape is read by an `on_key_down` on the menu frame, which the menu focuses when it opens. If the input swallows Escape before the handler, only the outside click and Cancel close the menu. This is UNVERIFIED (no window).
  - `futures = "0.3"` is added to the dependency list of the spec. It is already in the tree through `bachuc-plugin-host`, and gives the oneshot channels for saves that wait for a refresh.
  - Literal sizes with no metric: menu radius 10, page title 24, menu trailing text 11, and the tab, button, and input sizes of the spec.

## Verified

- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh test -p bachuc-plugin-workspace-spaces` -> exit 0; 49 passed, 0 failed, no warning. Cases:
  - `add_space`, `rename_space` (trim, blank, missing id), `move_project`, `removal_target`, `remove_space`, `adjacent`, `host_state`, `with_host_state`, and `Preferences::parse` (empty Spaces, duplicate ids, an absent member, a bad host entry, a round trip).
  - `sidebar_membership` (legacy key, canonical wins, unparsable key), `view_membership`, `match_project`, `add_project` (assigned, new, no view key), `move_view`, `move_sidebar`, and the catalog decode.
  - Visibility, title, menu items, `select_direction`, `swipe_target`, `move_projects`, `created_space`, `removal_selection`, and `text_or`.
  - `touch_direction`, `drag_direction`, `refresh_due`, `next_wait_ms`, the menu origin, project order, remove prompt, menu heading, name rule, page items, and subtitle.
  - `plugin().id() == "workspace-spaces"`.
  - The 500 by 500 timing test.
- `sed -n '/^\[dependencies\]/,$p' packages/desktop-gpui/plugins/workspace-spaces/Cargo.toml | rg -n "bachuc-(composer|sidebar|timeline|command|settings|workspace|explorer|app)"` -> no match (exit 1). The unscoped grep also matches the crate's own name (Q-lesson from TASK-189).
- `rg -n "gpui-kit" packages/desktop-gpui/plugins/workspace-spaces/Cargo.toml` -> one line, `16:gpui-kit.workspace = true`.
- `packages/desktop-gpui/cargo.sh fmt --check -p bachuc-plugin-workspace-spaces` -> exit 0.
- `CARGO_BUILD_JOBS=4 packages/desktop-gpui/cargo.sh build -p bachuc-app` -> exit 0 (Finished; the one warning is in `bachuc-app`). The app does not depend on this crate yet, so this proves only that the workspace still builds with the new member.
- `git diff --stat -- hiep-plugins/plugins/workspace-spaces` -> empty.
- UI check (after TASK-175 and TASK-191, strip and page on live compared with Electron 6770) -> DEFERRED (UI checker).
- The view code (`strip.rs`, `space_menu.rs`, `page.rs`) compiles and was checked by reading only. Its look, the secondary-click menu, the Escape key, and the drag on the list are UNVERIFIED (unattended): code tests only. Live daemon 6768 is down.

## Anti-goal

- Checkpoint 1, when the visibility rule landed: 500 `is_visible` calls over 500 projects with 500 member keys took 954 us in a debug test; source `/tmp/wt188-t2.txt` (`cargo test -p bachuc-plugin-workspace-spaces -- --nocapture`); 2026-10-17 02:13.
- Checkpoint 2, after the last rules were added: 1.50 ms; source `/tmp/wt188-t3.txt`; 02:14.
- Checkpoint 3, at completion: 690 us (limit 20 ms); source `/tmp/wt188-t4.txt`; 02:14.
- Tripwire not hit. The `Snapshot` indexes member keys and catalog projects once when the settings or the catalog change. A view-key lookup then caches its Space, so no JSON parse runs per project per redraw.

## UI Checks

Checked 2026-10-17 by the UI checker with `open -g`, AX reads, AX presses, and window screenshots (no pointer, no foreground). Electron 6770 shows `workspace-spaces` as `failed`, so it draws no strip; the reference for sizes is the source `hiep-plugins/plugins/workspace-spaces/client/spaces-strip.tsx`. Live has one Space (`Workspace 1`), not two. Measures in points (screenshot px / 2) from `/tmp/wt-ui/gpui-initial.jpg`.

- UI check (sidebar strip: tab sizes, selected fill) -> FAIL on tab size. Source: tab `minWidth` 26, `height` 26, 1 px border, gap 4, strip padding 10 horizontal and 6 vertical, strip height 39. GPUI: tabs 28 pt tall (y 707.5-735.5), the `1` tab 28 wide and the `+` tab 26 wide, gap 4, first tab at x 8 (source 10), strip height 39 (border y 701.5 to 740.5). The selected fill (58,114,77) equals the accent of the Electron submit button (60,114,78): match.
- UI check (Spaces page layout, heading name, with two Spaces) -> UNVERIFIED (unattended): needs pointer. AX press on `Plugins`, `Settings`, and the sidebar rows changed nothing, so the page cannot open. Pressing `Create a Space` would create a Space on live and was not done.

## Lessons

- A per-project legacy-key scan with a JSON parse per member would cost 500 x 500 parses per redraw. Build the key indexes once in `Snapshot::new` and cache each view key's Space.
- A pure `Snapshot` and plain functions let nearly every Electron test port without GPUI. The state methods only call them and run requests.
- `refresh` returns a oneshot receiver that a caller may drop; a dropped receiver is harmless, but the type is `must_use`, so a caller that ignores it writes `let _ =`.
- `Styled::flex_grow` takes a number in this GPUI; use `flex_1` for a growing field.

## Fix Round 1

2026-10-17, code tests only. No layout change: the tabs already match the source.

- Pixel scan of `/tmp/wt-ui/gpui-initial.jpg` (2400 x 1600, so px / 2 = pt). Column x 15: the strip border at y 701.5-702, the `1` tab fill from y 708.5 to 734.5 (26 pt), the footer border at 740.5. Row y 715: the `1` tab fill from x 10 to 36 (26 pt), the `+` tab border from x 40 to 66 (26 pt). So the tabs are 26 x 26 at x 10 with a 4 pt gap, and the strip is 1 + 6 + 26 + 6 = 39 pt, as `spaces-strip.tsx` says.
- The earlier 28 pt and x 8 readings were edge reads of the JPEG ink, about 1 to 2 pt outside the box. GPUI layout puts the 1 px border inside the 26 px box.
- Change: `plugins/workspace-spaces/src/strip.rs` adds the test `tabs_are_26_px_in_a_39_px_strip`, which pins `TAB_SIZE` 26, `TAB_PADDING` 7, `STRIP_PADDING_X` 10, and the 39 px strip.
- The Spaces page check needs the `Plugins` row to open; that row now takes an AX press directly (TASK-175 Fix Round 1).
- Tests: `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-plugin-workspace-spaces` -> 50 passed; `./cargo.sh fmt --check -p bachuc-plugin-workspace-spaces` -> exit 0.
- UI check (tab sizes) -> DEFERRED (UI checker). UI check (Spaces page with two Spaces) -> DEFERRED (UI checker).
