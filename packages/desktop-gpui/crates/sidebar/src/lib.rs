//! `bachuc-sidebar`: the left sidebar of the GPUI client, as the Electron sidebar draws its
//! default tree mode: nav rows, then pinned workspaces, then projects with their workspaces,
//! each with a status mark, a hover kebab menu, and collapse. There are no agent rows; agents
//! open as tabs of their workspace.
//!
//! [`Sidebar`] never opens anything itself. The app handles [`SidebarEvent`]. The plugin rows,
//! sections, and the Add project button request their work through [`SidebarPluginEvent`], a
//! second event type, and the built-in nav rows and the project "+" through [`SidebarNavEvent`].

pub mod badge;
pub mod electron_order;
mod metrics;
pub mod model;
pub mod plugin;
pub mod prefs;
mod ring;
mod sidebar;

pub use badge::StatusBadge;
pub use ring::{FRAME, Knockout, RunningRing};
pub use sidebar::{Sidebar, SidebarEvent, SidebarNavEvent, SidebarPluginEvent};
