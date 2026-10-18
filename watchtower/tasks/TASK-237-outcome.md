# TASK-237 Outcome

## Status

DONE, with the UI checker lines DEFERRED. The header root cause is proven from source. The tab-bar root cause is not proven from source. That button now uses a controlled menu that does not depend on the unknown cause.

## Changed

- `packages/desktop-gpui/crates/app/src/chrome.rs`: the header "Workspace actions" menu is now a `PopupMenu` the shell owns (`render_header_actions`, `toggle_header_actions`). It opens and closes from a capture-phase left press and from an explicit `AccessibleAction::Click`. `MenuButton` and the gpui-kit `DropdownMenu` use are removed. Open-in rows and the split button draw `editor_glyph(...)`. "Open on GitHub" draws the GitHub mark. New `load_editor_icons`, `app_icon` (NSWorkspace), and `editor_glyph_element`.
- `packages/desktop-gpui/crates/app/src/shell.rs`: fields `header_actions` and `editor_icons`. `focus` is now `pub(crate)`. Opening a Scripts or Editor menu closes the actions menu.
- `packages/desktop-gpui/crates/app/src/state.rs`: `EditorTarget::bundle_id`, `AppIcons`, `EditorGlyph`, and the pure `editor_glyph` with a fallback test.
- `packages/desktop-gpui/crates/app/Cargo.toml`: more `objc2-app-kit` features (NSBitmapImageRep, NSGraphics, NSGraphicsContext, NSImage, NSImageRep, NSWorkspace) and `objc2-foundation` features (NSData, NSDictionary, NSGeometry, NSURL). No new crate. `Cargo.lock` is unchanged.
- `packages/desktop-gpui/crates/workspace/src/workspace.rs`: the tab-bar More actions menu is now an `OpenSplitMenu` that the workspace owns (`toggle_split_menu`, `render_split_menu`), with the same press and AX paths.
- `packages/desktop-gpui/crates/icons/src/providers.rs`, `src/lib.rs`, and `assets/brands/github.svg`: `github_mark(fill) -> Arc<Image>`, cached per color. The path is copied from `packages/website/src/components/brand-icons.tsx` (Simple Icons).
- `panes.rs` is not edited.

## Root causes

- Header: `chrome_button` stops the left mouse-down (`crates/app/src/panes.rs:528`). The gpui-kit dropdown opens from a mouse-down listener on its wrapper, the parent of the trigger (`vendor/gpui-base/src/popover.rs:335`). A child listener runs first in the bubble phase, so the stop kept the wrapper from opening the menu. `crates/sidebar/src/sidebar.rs:1949` notes the same trap for the kebab.
- Tab bar: not found by reading. `icon_button` stops nothing, and no ancestor stops the press. The explorer resize band covers only the last 5 px of the area, and the button starts 5 px from the edge. Fix: a menu the workspace owns, the same way the working tab context menu is built (`open_tab_menu`). It no longer depends on the gpui-kit popover state.
- Both buttons toggle in the capture phase and stop propagation. The open menu's press-outside listener runs later in the capture phase, so a second press closes the menu instead of closing it and opening it again. The stop also keeps the title bar from starting a window drag.

## Contract

- Produces `EditorTarget::bundle_id(self) -> &'static str`, `pub type AppIcons = Vec<(&'static str, Arc<Image>)>`, `pub enum EditorGlyph { App(Arc<Image>), Lucide(IconName) }`, `pub fn editor_glyph(target, icons) -> EditorGlyph`, and `bachuc_icons::github_mark(Hsla) -> Arc<Image>`.
- Icons load once, in a `cx.spawn` after `Shell::new`. Each app is drawn into a 48 px RGBA bitmap and kept as PNG bytes. Render paths read only `self.editor_icons`.

## Verified

- `cd packages/desktop-gpui && CARGO_BUILD_JOBS=4 ./cargo.sh build -p bachuc-app` -> PASS. One warning, `next_theme` is never used, was there before this group.
- `CARGO_BUILD_JOBS=4 ./cargo.sh test -p bachuc-settings -p bachuc-command -p bachuc-workspace -p bachuc-theme -p bachuc-app -p bachuc-icons` -> PASS, exit 0, including `a_missing_app_icon_falls_back_to_the_lucide_icon` (a missing app, an empty cache, a wrong bundle id) and `the_github_mark_is_filled_and_cached`.
- `fmt --check -p bachuc-app -p bachuc-workspace -p bachuc-icons` -> exit 0.
- UI checker: AX click on the header three-dot button and on the tab-bar three-dot button -> menu opens -> DEFERRED (UI checker).
- UI checker: Open-in menu screenshot shows real app icons -> DEFERRED (UI checker). `app_icon` has not run against AppKit yet.

## Anti-goal

- Checkpoint "icon loading runs at most once per app per launch; a load in a render path is the tripwire": value one load per listed app per launch. Source: `load_editor_icons` is called only from `Shell::new`, and `app_icon` is called only there (`rg "app_icon\(" crates/app/src`). 2026-10-18.

## Lessons

- In GPUI, never stop the left mouse-down on a gpui-kit `DropdownMenu` trigger. For a button that must stop it, for example over a drag region, own the `PopupMenu` and toggle it in the capture phase.

## UI Checks

- Line 43 (AX click header three-dot and tab-bar three-dot): PASS. Header "Workspace actions" opened Rename agent, Archive agent, New tab, Split pane right and down, Copy path (`/tmp/wt-ui/chk-237-header-menu.png`). Tab-bar "More actions" opened Split right and Split down (`/tmp/wt-ui/chk-237-tab-menu.png`).
- Line 44 (Open-in menu shows real app icons): PASS. "Choose editor" shows the VS Code, Finder, and Terminal app icons (`/tmp/wt-ui/chk-237-open-in.png`).
