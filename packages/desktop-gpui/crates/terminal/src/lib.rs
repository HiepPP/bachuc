//! A daemon terminal as a GPUI view: `alacritty_terminal` draws it, the pane sets its size, and
//! the daemon's snapshot restores its history.
//!
//! The pane holds an `Entity<bachuc_client::Client>`; keep your own strong handle to the client,
//! because a subscription holds its emitter weakly.

mod emulator;
mod keys;
mod size;
mod snapshot;
mod view;

pub use emulator::{Emulator, RenderedLine};
pub use keys::{key_bytes, paste_bytes};
pub use size::{PADDING, cell_at, grid_size};
pub use snapshot::Snapshot;
pub use view::{TerminalEvent, TerminalView};
