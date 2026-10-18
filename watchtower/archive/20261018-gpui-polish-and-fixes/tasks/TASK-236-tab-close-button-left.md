# TASK-236 Tab close button on the left

Group: S (shares [packages/desktop-gpui/crates/workspace/src/workspace.rs](packages/desktop-gpui/crates/workspace/src/workspace.rs) with TASK-237 and settings files with TASK-234)
Class: code

## Brief

Goal: The tab close button shows on the left by default. A setting chooses Left or Right.

Change: close button overlay on the right -> on hover, the close button replaces the provider icon on the left.

Boundaries:

- Owner chose "hover replaces the icon" on 2026-10-18, like Safari and macOS. The tab does not get wider.
- Right keeps today's behavior: the overlay with gradient scrim (`CLOSE_OVERLAY_WIDTH`) in [packages/desktop-gpui/crates/workspace/src/workspace.rs](packages/desktop-gpui/crates/workspace/src/workspace.rs) about lines 902-954.
- Add the setting `tab_close_position` (Left, Right; default Left) to `Appearance` in [packages/desktop-gpui/crates/settings/src/appearance.rs](packages/desktop-gpui/crates/settings/src/appearance.rs) and a control in [packages/desktop-gpui/crates/settings/src/pane_appearance.rs](packages/desktop-gpui/crates/settings/src/pane_appearance.rs). An old `appearance.json` with no field reads as Left.
- Hide by opacity, never by `display` from hover (CONTEXT).

How:

- Add the setting with a serde default and a test that an old file reads as Left.
- In the tab chip, when Left and hovered, draw the close button in the icon slot.

Files:

- [packages/desktop-gpui/crates/workspace/src/workspace.rs](packages/desktop-gpui/crates/workspace/src/workspace.rs) (close button placement)
- [packages/desktop-gpui/crates/settings/src/appearance.rs](packages/desktop-gpui/crates/settings/src/appearance.rs) (setting)
- [packages/desktop-gpui/crates/settings/src/pane_appearance.rs](packages/desktop-gpui/crates/settings/src/pane_appearance.rs) (control)

Expected result:

- Hover a tab -> an x shows where the icon was; click closes the tab.
- Set Right -> today's right overlay comes back.

Anti-goal: tab width stays the same in Left mode, 0 px change; read from tab layout before and after.

## Verify

- `cd packages/desktop-gpui && ./cargo.sh test -p bachuc-workspace -p bachuc-settings` -> all pass, including the old-file default test.
- UI checker: hover a tab in Left mode -> x shows in the icon slot; AX click on it closes the tab.
- UI checker: switch to Right -> x shows on the right overlay.
