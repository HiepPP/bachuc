//! `bachuc-composer`: the message box under the timeline. It has a multiline input, image
//! attachments, mode and model pills, Send, and Stop.

mod attachments;
mod composer;

pub use attachments::{Attachment, can_send, mime_for_path};
pub use composer::{Composer, ComposerEvent};
