//! `bachuc-composer`: the message box under the timeline, styled and behaving like the Electron
//! composer. It has a multiline input, image pills, a slash command menu, model, thinking, and
//! mode pills, the queued prompts, and Send or Stop.

mod actions;
mod attachments;
mod composer;
mod features;
mod slash;

pub use attachments::{Attachment, can_send, mime_for_path};
pub use composer::{Composer, ComposerEvent};
