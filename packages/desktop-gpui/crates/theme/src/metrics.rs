//! Sizes in logical pixels, copied from the Electron app (ADR-0043). Source of each group:
//! `SPACING`, `BORDER_RADIUS`, `FONT_SIZE`, `ICON_SIZE`, `FONT_WEIGHT`, and `BORDER_WIDTH` in
//! `packages/app/src/styles/theme.ts`; `CONTROL_HEIGHTS` in
//! `packages/app/src/components/ui/control-geometry.ts`; the layout sizes in
//! `packages/app/src/constants/layout.ts`, `stores/panel-store/state.ts`, and
//! `screens/settings-screen.tsx`.
//!
//! Font sizes here are the defaults. Scale them with `crate::ui_font_size` and
//! `crate::code_font_size`, which follow the Appearance font size settings.

pub const SPACE_0_5: f32 = 2.;
pub const SPACE_1: f32 = 4.;
pub const SPACE_1_5: f32 = 6.;
pub const SPACE_2: f32 = 8.;
pub const SPACE_3: f32 = 12.;
pub const SPACE_4: f32 = 16.;
pub const SPACE_6: f32 = 24.;
pub const SPACE_8: f32 = 32.;

/// The gap between a trigger and its open menu or popover, like the Electron menus' 4 px
/// `sideOffset`. Every dropdown uses this one value.
pub const POPUP_GAP: f32 = SPACE_1;

pub const RADIUS_SM: f32 = 2.;
pub const RADIUS_BASE: f32 = 4.;
pub const RADIUS_MD: f32 = 6.;
pub const RADIUS_LG: f32 = 8.;
pub const RADIUS_XL: f32 = 12.;
pub const RADIUS_2XL: f32 = 16.;

pub const FONT_CODE: f32 = 12.;
pub const FONT_SM: f32 = 12.;
pub const FONT_BASE: f32 = 14.;
/// Message bodies, composer input, Markdown.
pub const FONT_CONTENT: f32 = 15.;
pub const FONT_LG: f32 = 16.;
pub const FONT_XL: f32 = 18.;
pub const FONT_2XL: f32 = 20.;

/// CSS font weights.
pub const WEIGHT_NORMAL: f32 = 400.;
pub const WEIGHT_MEDIUM: f32 = 500.;
pub const WEIGHT_SEMIBOLD: f32 = 600.;
pub const WEIGHT_BOLD: f32 = 700.;

pub const BORDER_WIDTH_1: f32 = 1.;
pub const BORDER_WIDTH_2: f32 = 2.;

pub const ICON_XS: f32 = 12.;
pub const ICON_SM: f32 = 14.;
pub const ICON_MD: f32 = 16.;
pub const ICON_LG: f32 = 20.;

/// `CONTROL_HEIGHTS.tight`.
pub const CONTROL_XS: f32 = 28.;
/// `CONTROL_HEIGHTS.compact`.
pub const CONTROL_SM: f32 = 32.;
/// `CONTROL_HEIGHTS.field`.
pub const CONTROL_MD: f32 = 44.;

/// `HEADER_INNER_HEIGHT`.
pub const HEADER_HEIGHT: f32 = 36.;
/// `WORKSPACE_SECONDARY_HEADER_HEIGHT`, which is the tab bar height.
pub const TAB_BAR_HEIGHT: f32 = 36.;
pub const MAX_CONTENT_WIDTH: f32 = 820.;

/// `DEFAULT_SIDEBAR_WIDTH`, `MIN_SIDEBAR_WIDTH`, and `MAX_SIDEBAR_WIDTH`.
pub const SIDEBAR_WIDTH: f32 = 320.;
pub const SIDEBAR_MIN_WIDTH: f32 = 200.;
pub const SIDEBAR_MAX_WIDTH: f32 = 600.;

/// `DESKTOP_TRAFFIC_LIGHT_WIDTH` and `DESKTOP_TRAFFIC_LIGHT_HEIGHT`: the space the macOS window
/// buttons take at the top left.
pub const TRAFFIC_LIGHT_WIDTH: f32 = 78.;
pub const TRAFFIC_LIGHT_HEIGHT: f32 = 45.;

/// `SETTINGS_DESKTOP_SIDEBAR_WIDTH` and `SETTINGS_DESKTOP_DETAIL_MIN_WIDTH`.
pub const SETTINGS_SIDEBAR_WIDTH: f32 = 320.;
pub const SETTINGS_DETAIL_MIN_WIDTH: f32 = 400.;
/// The settings content column `maxWidth`.
pub const SETTINGS_CONTENT_WIDTH: f32 = 720.;

/// Shadow recipe: offset y, blur, and the color alpha over black. `dark` and `light` are the
/// `darkShadow` and `lightShadow` objects in `theme.ts`.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct Shadow {
    pub offset_y: f32,
    pub blur: f32,
    pub alpha: f32,
}

pub const DARK_SHADOWS: [Shadow; 3] = [
    Shadow {
        offset_y: 2.,
        blur: 4.,
        alpha: 0.25,
    },
    Shadow {
        offset_y: 4.,
        blur: 8.,
        alpha: 0.20,
    },
    Shadow {
        offset_y: 12.,
        blur: 24.,
        alpha: 0.40,
    },
];

pub const LIGHT_SHADOWS: [Shadow; 3] = [
    Shadow {
        offset_y: 2.,
        blur: 8.,
        alpha: 0.02,
    },
    Shadow {
        offset_y: 4.,
        blur: 16.,
        alpha: 0.04,
    },
    Shadow {
        offset_y: 8.,
        blur: 24.,
        alpha: 0.08,
    },
];
