//! `bachuc-explorer`: the Explorer sidebar (Files and Changes), the file viewer, and the diff
//! viewer of the GPUI client, drawn as the Electron app draws them.
//!
//! `Explorer` emits `ExplorerEvent` and `ExplorerDiffEvent`; the app opens a `FileView` or a
//! `DiffView` in a workspace pane for them. Its file menu emits `ExplorerFileEvent`, and failed
//! operations emit `ExplorerNoticeEvent` for the app toast. The models (`tree`, `changes`, `diff`, `diff_doc`,
//! `commits`) hold no GPUI state and have their own tests.

mod branch_switcher;
pub mod changes;
mod code;
pub mod commits;
pub mod diff;
pub mod diff_doc;
pub mod diff_prefs;
mod diff_view;
mod explorer;
mod file_icon_data;
pub mod file_icons;
mod file_menu;
mod file_view;
pub mod git_actions;
mod git_actions_ui;
mod pull_request_view;
pub mod syntax;
pub mod tree;
mod ui;

pub use diff_view::DiffView;
pub use explorer::{
    Explorer, ExplorerDiffEvent, ExplorerEvent, ExplorerFileEvent, ExplorerNoticeEvent, ExplorerTab,
};
pub use file_view::{FileView, format_file_size};
pub use git_actions::ShipDefault;
pub use git_actions_ui::{GitActionsButton, GitActionsEvent};
pub use pull_request_view::{PullRequestEvent, PullRequestNoticeEvent, PullRequestView};
