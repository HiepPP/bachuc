//! `bachuc-composer`: the message box under the timeline, styled and behaving like the Electron
//! composer in `packages/app/src/composer`. Each module mirrors one Electron component: the
//! root view, the card, the dock, the track strip and its pills, the context window meter, the
//! queue, the attachment tray, and the agent controls.

mod actions;
mod agent_controls;
mod agent_tracks;
mod attachment_tray;
mod attachments;
mod composer;
mod context_window_meter;
mod diff_stat_pill;
mod dock;
mod draft;
mod input;
mod mention;
mod pill_styles;
mod plugin_slots;
mod queue;
mod slash;
mod subagents_track;
mod task_list;
mod tracks;
mod turn_metrics_pill;

pub use attachments::{Attachment, can_send, mime_for_path};
pub use composer::{Composer, ComposerEvent};
pub use dock::composer_dock;
pub use draft::{DraftOptions, DraftSeed, DraftSubmission, SubmitIcon};
pub use plugin_slots::CornerPills;
