//! Image attachments: what the composer keeps until send, and the JSON the client sends.

use base64::Engine as _;
use serde_json::{Value, json};
use std::path::Path;

/// One image the user attached. `data` is base64, as `ImageAttachmentSchema` in
/// `packages/protocol/src/messages.ts` wants it.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct Attachment {
    pub name: String,
    pub mime_type: &'static str,
    pub data: String,
    pub bytes: usize,
}

impl Attachment {
    pub fn new(name: impl Into<String>, mime_type: &'static str, bytes: &[u8]) -> Self {
        Self {
            name: name.into(),
            mime_type,
            data: base64::engine::general_purpose::STANDARD.encode(bytes),
            bytes: bytes.len(),
        }
    }

    /// Reads an image file. Returns `None` for a non-image extension or an unreadable file.
    pub fn from_path(path: &Path) -> Option<Self> {
        let mime_type = mime_for_path(path)?;
        let bytes = match std::fs::read(path) {
            Ok(bytes) => bytes,
            Err(error) => {
                log::warn!("cannot read {}: {error}", path.display());
                return None;
            }
        };
        let name = path.file_name()?.to_string_lossy().into_owned();
        Some(Self::new(name, mime_type, &bytes))
    }

    pub fn to_json(&self) -> Value {
        json!({"data": self.data, "mimeType": self.mime_type})
    }

    /// `name (12 KB)` for the chip.
    pub fn chip_label(&self) -> String {
        format!("{} ({})", self.name, human_size(self.bytes))
    }
}

/// The image types the agents accept.
pub fn mime_for_path(path: &Path) -> Option<&'static str> {
    match path.extension()?.to_str()?.to_ascii_lowercase().as_str() {
        "png" => Some("image/png"),
        "jpg" | "jpeg" => Some("image/jpeg"),
        "gif" => Some("image/gif"),
        "webp" => Some("image/webp"),
        _ => None,
    }
}

fn human_size(bytes: usize) -> String {
    if bytes < 1024 {
        format!("{bytes} B")
    } else if bytes < 1024 * 1024 {
        format!("{} KB", bytes.div_ceil(1024))
    } else {
        format!("{:.1} MB", bytes as f64 / (1024. * 1024.))
    }
}

/// Send is allowed with some text or at least one image.
pub fn can_send(text: &str, images: usize) -> bool {
    !text.trim().is_empty() || images > 0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_without_images_cannot_send() {
        assert!(!can_send("", 0));
        assert!(!can_send("  \n\t ", 0));
        assert!(can_send("hi", 0));
        assert!(can_send("", 1));
    }

    #[test]
    fn mime_follows_the_extension() {
        assert_eq!(mime_for_path(Path::new("/tmp/a.PNG")), Some("image/png"));
        assert_eq!(mime_for_path(Path::new("b.jpeg")), Some("image/jpeg"));
        assert_eq!(mime_for_path(Path::new("c.txt")), None);
        assert_eq!(mime_for_path(Path::new("noext")), None);
    }

    #[test]
    fn json_has_data_and_mime_type_only() {
        let attachment = Attachment::new("a.png", "image/png", b"abc");
        assert_eq!(
            attachment.to_json(),
            json!({"data": "YWJj", "mimeType": "image/png"})
        );
        assert_eq!(attachment.chip_label(), "a.png (3 B)");
    }
}
