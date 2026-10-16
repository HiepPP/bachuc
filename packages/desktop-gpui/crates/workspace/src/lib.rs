//! The workspace pane container: tab groups in resizable splits with the Electron tab bar
//! (`packages/app/src/screens/workspace/workspace-desktop-tabs-row.tsx` and
//! `components/split-container.tsx`). It does not know pane kinds: each [`PaneItem`] carries its
//! own view, and the app decides what the view is.

pub mod model;
pub mod tab_layout;
mod workspace;

use bachuc_icons::IconName;
use gpui_kit::{AnyView, SharedString};

pub use model::SplitAxis;
pub use workspace::Workspace;

/// Keyboard and menu actions the [`Workspace`] handles while focus is inside it. The app binds
/// the keys, for example `cmd-\` to `SplitRight` and `cmd-shift-\` to `SplitDown` with the
/// [`KEY_CONTEXT`] context.
pub mod actions {
    gpui_kit::actions!(workspace, [SplitRight, SplitDown, CloseActiveTab, NewTab]);
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
