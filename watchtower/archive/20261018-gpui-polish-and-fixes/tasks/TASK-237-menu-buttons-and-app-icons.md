# TASK-237 Clickable menu buttons and real app icons

Group: S (shares [packages/desktop-gpui/crates/workspace/src/workspace.rs](packages/desktop-gpui/crates/workspace/src/workspace.rs) with TASK-236)
Class: code

## Brief

Goal: Both three-dot buttons open their menu on click. The Open-in menu shows the real icon of each app.

Change: dead three-dot buttons and generic lucide icons -> working menus and real app icons.

Boundaries:

- Two buttons, both broken per the owner: the header button (`header-menu`, `IconName::MoreHorizontal`) in [packages/desktop-gpui/crates/app/src/chrome.rs](packages/desktop-gpui/crates/app/src/chrome.rs) about line 332, and the tab-bar button (`IconName::Ellipsis`) in [packages/desktop-gpui/crates/workspace/src/workspace.rs](packages/desktop-gpui/crates/workspace/src/workspace.rs) about line 1183. Find the root cause of each; the two may differ. Header suspect: `chrome_button` in [packages/desktop-gpui/crates/app/src/panes.rs](packages/desktop-gpui/crates/app/src/panes.rs) about line 528 stops mouse-down, which may block the gpui-kit `DropdownMenu` trigger. The tab-bar button does not use `chrome_button`; it is `icon_button` (workspace.rs about line 1088) wrapped in `ChromeButton` with `dropdown_menu_with_anchor` (about lines 1176-1193).
- Follows [ADR-0046](watchtower/decisions/ADR-0046-real-app-icons-in-open-in-menu-over-lucide.md): the Open-in menu uses the real app icon from the macOS app bundle for VS Code, Cursor, Zed, Finder, and Terminal, and the GitHub mark for "Open on GitHub". Other icons stay lucide per [ADR-0043](watchtower/decisions/ADR-0043-electron-tokens-and-lucide-icons-over-gpui-kit-defaults.md).
- `EditorTarget::icon()` is in [packages/desktop-gpui/crates/app/src/state.rs](packages/desktop-gpui/crates/app/src/state.rs) about line 355. Menu rows are `menu_row` in chrome.rs about line 1178.
- Load app icons once and cache them. Check the app crate `Cargo.toml` for an existing objc or cocoa dependency before adding one; never query the system in a render path (MEMORY 2026-10-17). If an app is not installed, show the lucide fallback.

How:

- Fix both buttons, with an explicit `AccessibleAction::Click` handler.
- Read each app icon once (for example through NSWorkspace iconForFile) into a cached image, keyed by bundle id.
- Draw it in the menu row and on the split button.

Files:

- [packages/desktop-gpui/crates/app/src/chrome.rs](packages/desktop-gpui/crates/app/src/chrome.rs) (header menu button, Open-in rows)
- [packages/desktop-gpui/crates/app/src/panes.rs](packages/desktop-gpui/crates/app/src/panes.rs) (`chrome_button` if it is the cause)
- [packages/desktop-gpui/crates/app/src/state.rs](packages/desktop-gpui/crates/app/src/state.rs) (`EditorTarget` icon source)
- [packages/desktop-gpui/crates/workspace/src/workspace.rs](packages/desktop-gpui/crates/workspace/src/workspace.rs) (tab-bar menu button)
- [packages/desktop-gpui/crates/icons](packages/desktop-gpui/crates/icons) (GitHub mark and fallback, if needed)

Expected result:

- Click either three-dot button -> its menu opens.
- The Open-in menu shows the real VS Code, Finder, and Terminal icons.

Anti-goal: icon loading runs at most once per app per launch; a load in a render path is the tripwire; checked by reading the code.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh build -p bachuc-app` -> builds.
- UI checker: AX click the header three-dot button -> menu opens; AX click the tab-bar three-dot button -> menu opens.
- UI checker: open the Open-in menu -> screenshot shows real app icons.
- Fallback: set a missing editor bundle id in a unit test of the icon lookup -> returns the lucide fallback.
