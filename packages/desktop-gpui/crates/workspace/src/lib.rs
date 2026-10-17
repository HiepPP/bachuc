//! The workspace pane container: tab groups in resizable splits with the Electron tab bar
//! (`packages/app/src/screens/workspace/workspace-desktop-tabs-row.tsx` and
//! `components/split-container.tsx`). It does not know pane kinds: each [`PaneItem`] carries its
//! own view, and the app decides what the view is.

pub mod model;
pub mod tab_layout;
pub mod tab_menu;
mod workspace;

use bachuc_icons::IconName;
use gpui_kit::{AnyView, SharedString};

pub use model::SplitAxis;
pub use tab_menu::{TabKind, TabMenuEvent};
pub use workspace::Workspace;

/// Keyboard and menu actions the [`Workspace`] handles while focus is inside it. The app binds
/// the keys, for example `cmd-\` to `SplitRight` and `cmd-shift-\` to `SplitDown` with the
/// [`KEY_CONTEXT`] context. `OpenTabMenu` opens the context menu of the active tab; bind it to
/// the keyboard menu key, `menu`, and to `shift-f10`.
pub mod actions {
    gpui_kit::actions!(
        workspace,
        [SplitRight, SplitDown, CloseActiveTab, NewTab, OpenTabMenu]
    );
}

/// The key context of the workspace root.
pub const KEY_CONTEXT: &str = "Workspace";

/// One tab. `key` is unique in the workspace; opening an item with an open key replaces that item.
#[derive(Clone)]
pub struct PaneItem {
    pub key: String,
    pub title: SharedString,
    pub icon: IconName,
    pub modified: bool,
    pub view: AnyView,
}

impl model::Keyed for PaneItem {
    fn key(&self) -> &str {
        &self.key
    }
}

/// What the [`Workspace`] emits.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum PaneEvent {
    /// A tab closed; the workspace dropped its item.
    Closed { key: String },
    /// The active tab of the focused group changed.
    Activated { key: String },
    /// The new-tab button was pressed, or a split made an empty group. The group that should
    /// get the new tab has the focus, so `Workspace::open` puts it there.
    NewTab,
}
