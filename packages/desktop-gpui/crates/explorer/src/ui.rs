//! Small elements that the three views share: the centered state text, the file icon, the
//! `+N -M` count, and the tree row look of `tree-primitives.tsx`.

pub use crate::changes::Change;
use crate::changes::format_count;
use crate::diff_prefs::DiffPrefs;
use crate::file_icons::file_icon;
use bachuc_client::DiffMode;
use bachuc_icons::{IconName, icon};
use bachuc_theme::metrics::{
    FONT_BASE, FONT_SM, ICON_MD, ICON_SM, RADIUS_MD, SPACE_1, SPACE_2, SPACE_3, SPACE_4, SPACE_6,
};
use bachuc_theme::{Tokens, tokens, ui_font_size, zoomed};
use gpui_kit::component::menu::{DropdownMenu, PopupMenuItem};
use gpui_kit::component::{Selectable, Sizable as _, h_flex, v_flex};
use gpui_kit::prelude::FluentBuilder as _;
use gpui_kit::*;

/// `theme.spacing[16]`, the top inset of the Electron state screens.
pub const SPACE_16: f32 = 64.;
/// `TREE_INDENT_PER_LEVEL`, `WORKSPACE_TREE_ICON_SIZE`, and `WORKSPACE_TREE_ICON_LABEL_GAP`.
pub const TREE_INDENT: f32 = 12.;
pub const TREE_ICON: f32 = ICON_MD;
pub const TREE_ICON_GAP: f32 = SPACE_2;
/// A Files row: 6 px above and below one 17 px text line, as `WORKSPACE_FILE_ROW_VERTICAL_PADDING`
/// gives the Electron row.
pub const FILE_ROW_HEIGHT: f32 = 29.;
/// A Changes row is taller: the `DiffStat` is 20 px high.
pub const CHANGE_ROW_HEIGHT: f32 = 32.;

/// `treeRowPaddingLeft`.
pub fn tree_padding_left(depth: usize) -> f32 {
    SPACE_3 + depth as f32 * TREE_INDENT
}

/// A message centered in the pane below a 64 px inset, as the Electron `emptyContainer` and
/// `errorContainer` draw it.
pub fn state_message(
    id: &'static str,
    text: impl Into<SharedString>,
    error: bool,
    cx: &App,
) -> Stateful<Div> {
    let colors = tokens(cx);
    let text: SharedString = text.into();
    div()
        .id(id)
        .role(Role::Status)
        .aria_label(text.clone())
        .flex_1()
        .min_h_0()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .pt(zoomed(SPACE_16, cx))
        .px(zoomed(SPACE_6, cx))
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_center()
        .text_color(if error {
            colors.destructive
        } else {
            colors.foreground_muted
        })
        .child(text)
}

/// A message centered both ways in the pane, as the Files `centerState` draws it.
pub fn centered_text(
    id: &'static str,
    text: impl Into<SharedString>,
    error: bool,
    cx: &App,
) -> Stateful<Div> {
    let colors = tokens(cx);
    let text: SharedString = text.into();
    div()
        .id(id)
        .role(Role::Status)
        .aria_label(text.clone())
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(zoomed(SPACE_2, cx))
        .p(zoomed(SPACE_4, cx))
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_center()
        .text_color(if error {
            colors.destructive
        } else {
            colors.foreground_muted
        })
        .child(text)
}

/// The file viewer placeholder: a muted title and the file size under it, centered
/// (`emptyText` and `binaryMetaText` in `file-pane/pane.tsx`).
pub fn file_meta_state(
    id: &'static str,
    title: &'static str,
    meta: Option<String>,
    cx: &App,
) -> Stateful<Div> {
    let colors = tokens(cx);
    div()
        .id(id)
        .role(Role::Status)
        .aria_label(title)
        .flex_1()
        .flex()
        .flex_col()
        .items_center()
        .justify_center()
        .gap(zoomed(SPACE_3, cx))
        .p(zoomed(SPACE_4, cx))
        .text_size(ui_font_size(FONT_BASE, cx))
        .text_color(colors.foreground_muted)
        .child(div().text_center().child(title))
        .when_some(meta, |this, meta| {
            this.child(div().mt(zoomed(SPACE_2, cx)).child(meta))
        })
}

/// The colored file icon of a row.
pub fn file_icon_element(file_name: &str, cx: &App) -> impl IntoElement + use<> {
    div()
        .size(zoomed(TREE_ICON, cx))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .child(img(file_icon(file_name)).size(zoomed(TREE_ICON, cx)))
}

/// The disclosure chevron of a folder row.
pub fn chevron(expanded: bool, colors: &Tokens, cx: &App) -> impl IntoElement + use<> {
    div()
        .size(zoomed(TREE_ICON, cx))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            icon(if expanded {
                IconName::ChevronDown
            } else {
                IconName::ChevronRight
            })
            .with_size(zoomed(TREE_ICON, cx))
            .text_color(colors.foreground_extra_muted),
        )
}

/// The slot of a folder whose listing is on its way.
pub fn loading_marker(colors: &Tokens, cx: &App) -> impl IntoElement + use<> {
    div()
        .size(zoomed(TREE_ICON, cx))
        .flex_shrink_0()
        .flex()
        .items_center()
        .justify_center()
        .child(
            icon(IconName::RotateCw)
                .with_size(zoomed(ICON_SM, cx))
                .text_color(colors.foreground_extra_muted),
        )
}

/// `DiffStat`: `+N` in the success color and `-M` in the danger color.
pub fn diff_stat(added: u32, removed: u32, cx: &App) -> impl IntoElement + use<> {
    let colors = tokens(cx);
    h_flex()
        .h(zoomed(20., cx))
        .gap(zoomed(SPACE_1, cx))
        .flex_shrink_0()
        .text_size(ui_font_size(FONT_SM, cx))
        .child(
            div()
                .text_color(colors.status_success)
                .child(format!("+{}", format_count(added))),
        )
        .child(
            div()
                .text_color(colors.status_danger)
                .child(format!("-{}", format_count(removed))),
        )
}

pub fn change_icon(change: Change, colors: &Tokens, cx: &App) -> impl IntoElement + use<> {
    let (name, color) = match change {
        Change::Added => (IconName::SquarePlus, colors.status_success),
        Change::Deleted => (IconName::SquareMinus, colors.status_danger),
        Change::Modified => (IconName::SquareDot, colors.status_warning),
    };
    icon(name).with_size(zoomed(ICON_SM, cx)).text_color(color)
}

/// The padding that every pane uses at the right edge: `WORKSPACE_PANE_TRAILING_GLYPH_RAIL`.
pub const TRAILING_RAIL: f32 = SPACE_2;

/// The shared diff choices; the defaults until a view writes them.
pub fn read_prefs<R>(cx: &App, read: impl FnOnce(&DiffPrefs) -> R) -> R {
    match cx.try_global::<DiffPrefs>() {
        Some(prefs) => read(prefs),
        None => read(&DiffPrefs::default()),
    }
}

/// Changes the shared diff choices. Observers hear of it only when `change` returns true, so a
/// status answer that changes nothing does not reload every diff.
pub fn update_prefs(cx: &mut App, change: impl FnOnce(&mut DiffPrefs) -> bool) {
    let mut next = read_prefs(cx, DiffPrefs::clone);
    if change(&mut next) {
        cx.set_global(next);
    }
}

/// The width of the `DiffModeMenu` popover.
const MODE_MENU_WIDTH: f32 = 260.;
/// The height of a toolbar label trigger.
const MODE_TRIGGER_HEIGHT: f32 = 24.;

/// A toolbar label that opens a dropdown menu, filled while the menu is open.
#[derive(IntoElement)]
struct MenuTrigger {
    base: Stateful<Div>,
    open: bool,
    fill: Hsla,
}

impl Selectable for MenuTrigger {
    fn selected(mut self, selected: bool) -> Self {
        self.open = selected;
        self
    }

    fn is_selected(&self) -> bool {
        self.open
    }
}

impl Styled for MenuTrigger {
    fn style(&mut self) -> &mut StyleRefinement {
        self.base.style()
    }
}

impl InteractiveElement for MenuTrigger {
    fn interactivity(&mut self) -> &mut Interactivity {
        self.base.interactivity()
    }
}

impl DropdownMenu for MenuTrigger {}

impl RenderOnce for MenuTrigger {
    fn render(self, _: &mut Window, _: &mut App) -> impl IntoElement {
        let fill = self.fill;
        self.base.when(self.open, |button| button.bg(fill))
    }
}

/// `DiffModeMenu` for the folder `cwd`: the current mode with a chevron, opening "Uncommitted",
/// a separator, and "Committed" with its `<branch> -> <base>` description. A pick writes the
/// shared `DiffPrefs`; every view of the folder follows it.
pub fn diff_mode_menu(id: &'static str, cwd: &str, cx: &App) -> impl IntoElement + use<> {
    let colors = tokens(cx);
    let (mode, description) = read_prefs(cx, |prefs| {
        (prefs.mode(cwd), prefs.committed_description(cwd))
    });
    let label = match mode {
        DiffMode::Uncommitted => "Uncommitted",
        DiffMode::Base => "Committed",
    };
    let base = h_flex()
        .id(id)
        .role(Role::Button)
        .aria_label("Diff mode")
        .flex_shrink_0()
        .h(zoomed(MODE_TRIGGER_HEIGHT, cx))
        .px(zoomed(SPACE_1, cx))
        .gap(zoomed(SPACE_1, cx))
        .rounded(zoomed(RADIUS_MD, cx))
        .cursor_pointer()
        .hover(|style| style.bg(colors.interaction_highlight))
        .child(
            div()
                .text_size(ui_font_size(FONT_SM, cx))
                .text_color(colors.foreground_muted)
                .child(label),
        )
        .child(
            icon(IconName::ChevronDown)
                .with_size(zoomed(12., cx))
                .text_color(colors.foreground_extra_muted),
        );
    let cwd = cwd.to_string();
    let muted = colors.foreground_muted;
    let size = ui_font_size(FONT_SM, cx);
    let menu_width = zoomed(MODE_MENU_WIDTH, cx);
    MenuTrigger {
        base,
        open: false,
        fill: colors.interaction_highlight,
    }
    .dropdown_menu(move |menu, _, _| {
        let pick = |mode: DiffMode| {
            let cwd = cwd.clone();
            move |_: &ClickEvent, _: &mut Window, cx: &mut App| {
                update_prefs(cx, |prefs| {
                    prefs.select_mode(&cwd, mode);
                    true
                });
            }
        };
        let description = description.clone();
        let committed = if let Some(description) = description {
            PopupMenuItem::element(move |_, _| {
                // The menu labels only plain items, so this row names itself.
                v_flex()
                    .id("diff-mode-committed")
                    .role(Role::Label)
                    .aria_label(format!("Committed, {description}"))
                    .child("Committed")
                    .child(
                        div()
                            .text_size(size)
                            .text_color(muted)
                            .child(description.clone()),
                    )
            })
        } else {
            PopupMenuItem::new("Committed")
        };
        menu.min_w(menu_width)
            .item(
                PopupMenuItem::new("Uncommitted")
                    .checked(mode == DiffMode::Uncommitted)
                    .on_click(pick(DiffMode::Uncommitted)),
            )
            .separator()
            .item(
                committed
                    .checked(mode == DiffMode::Base)
                    .on_click(pick(DiffMode::Base)),
            )
    })
}
