//! The images sent with each prompt, saved by message id. The daemon's `user_message` holds
//! only text, so the user bubble reads its images from here after a restart. One folder per
//! message holds `0.png`, `1.jpg`, and so on, in send order.

use base64::Engine as _;
use serde_json::Value;
use std::path::{Path, PathBuf};

/// One sent image: its MIME type and its bytes.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct SentImage {
    pub mime_type: String,
    pub bytes: Vec<u8>,
}

const TYPES: [(&str, &str); 4] = [
    ("image/png", "png"),
    ("image/jpeg", "jpg"),
    ("image/gif", "gif"),
    ("image/webp", "webp"),
];

/// Decodes the `{data, mimeType}` images of a send; an entry that does not decode is dropped.
pub fn decode(images: &[Value]) -> Vec<SentImage> {
    images
        .iter()
        .filter_map(|image| {
            let bytes = base64::engine::general_purpose::STANDARD
                .decode(image["data"].as_str()?)
                .ok()?;
            Some(SentImage {
                mime_type: image["mimeType"].as_str()?.to_string(),
                bytes,
            })
        })
        .collect()
}

/// The message folder. A message id comes from the daemon too, so only a plain name is a path.
fn folder(root: &Path, message_id: &str) -> Option<PathBuf> {
    let plain = !message_id.is_empty()
        && message_id
            .chars()
            .all(|c| c.is_ascii_alphanumeric() || c == '-' || c == '_');
    plain.then(|| root.join(message_id))
}

/// Writes the images of `message_id`. The folder appears whole or not at all.
pub fn save(root: &Path, message_id: &str, images: &[SentImage]) -> std::io::Result<()> {
    let Some(target) = folder(root, message_id) else {
        return Ok(());
    };
    let staging = root.join(format!(".{message_id}.tmp"));
    std::fs::create_dir_all(&staging)?;
    for (index, image) in images.iter().enumerate() {
        let extension = TYPES
            .iter()
            .find(|(mime, _)| *mime == image.mime_type)
            .map_or("png", |(_, extension)| extension);
        std::fs::write(staging.join(format!("{index}.{extension}")), &image.bytes)?;
    }
    std::fs::rename(&staging, &target)
}

/// Reads the images of `message_id`, or `None` when none were saved.
pub fn load(root: &Path, message_id: &str) -> Option<Vec<SentImage>> {
    let mut files: Vec<(usize, SentImage)> = std::fs::read_dir(folder(root, message_id)?)
        .ok()?
        .filter_map(|entry| {
            let path = entry.ok()?.path();
            let index = path.file_stem()?.to_str()?.parse().ok()?;
            let extension = path.extension()?.to_str()?;
            let mime_type = TYPES.iter().find(|(_, ext)| *ext == extension)?.0;
            let bytes = std::fs::read(&path).ok()?;
            Some((
                index,
                SentImage {
                    mime_type: mime_type.to_string(),
                    bytes,
                },
            ))
        })
        .collect();
    files.sort_by_key(|(index, _)| *index);
    (!files.is_empty()).then(|| files.into_iter().map(|(_, image)| image).collect())
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn saved_images_load_back_in_send_order() {
        let root = std::env::temp_dir().join(format!("sent-images-{}", uuid::Uuid::new_v4()));
        let images = decode(&[
            json!({"data": "aGk=", "mimeType": "image/png"}),
            json!({"data": "%%", "mimeType": "image/png"}),
            json!({"data": "eW8=", "mimeType": "image/jpeg"}),
        ]);
        assert_eq!(images.len(), 2);
        save(&root, "m-1", &images).expect("saved");
        assert_eq!(load(&root, "m-1"), Some(images));
        assert_eq!(load(&root, "other"), None);
        // An id that is not a plain name is never a path.
        assert_eq!(load(&root, "../m-1"), None);
        save(&root, "../x", &[]).expect("skipped");
        assert!(!root.parent().unwrap().join("x").exists());
        std::fs::remove_dir_all(&root).expect("cleaned");
    }
}
