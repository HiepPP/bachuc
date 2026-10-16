//! File reads and checkout diffs: binary file transfer frames, the JSON file forms, and
//! changed files. See `packages/protocol/src/binary-frames/file-transfer.ts`.

use crate::model::{ChangedFile, FileContent, FileEntry, FileKind};
use base64::Engine as _;
use serde_json::Value;

pub(crate) const FILE_BEGIN: u8 = 0x10;
pub(crate) const FILE_CHUNK: u8 = 0x11;
pub(crate) const FILE_END: u8 = 0x12;

/// One file transfer frame: `[opcode, id length, request id, body]`.
pub(crate) enum TransferFrame {
    /// The body of `FileBegin` is a big-endian u16 length and the JSON metadata.
    Begin {
        request_id: String,
        metadata: Value,
    },
    Chunk {
        request_id: String,
        bytes: Vec<u8>,
    },
    End {
        request_id: String,
    },
}

pub(crate) fn is_transfer_opcode(opcode: u8) -> bool {
    matches!(opcode, FILE_BEGIN | FILE_CHUNK | FILE_END)
}

pub(crate) fn parse_transfer(bytes: &[u8]) -> Option<TransferFrame> {
    let (&opcode, rest) = bytes.split_first()?;
    let (&id_len, rest) = rest.split_first()?;
    let id_len = id_len as usize;
    if id_len == 0 || id_len > rest.len() {
        return None;
    }
    let request_id = String::from_utf8(rest[..id_len].to_vec()).ok()?;
    let body = &rest[id_len..];
    match opcode {
        FILE_BEGIN => {
            let length = u16::from_be_bytes([*body.first()?, *body.get(1)?]) as usize;
            if length != body.len() - 2 {
                return None;
            }
            let metadata = serde_json::from_slice(&body[2..]).ok()?;
            Some(TransferFrame::Begin {
                request_id,
                metadata,
            })
        }
        FILE_CHUNK => Some(TransferFrame::Chunk {
            request_id,
            bytes: body.to_vec(),
        }),
        FILE_END => body.is_empty().then_some(TransferFrame::End { request_id }),
        _ => None,
    }
}

/// The kind of a streamed file, as the app's `binaryFileKind` decides it.
fn streamed_kind(mime: &str, encoding: &str) -> FileKind {
    if mime.starts_with("image/") {
        FileKind::Image
    } else if encoding == "utf-8" || mime.starts_with("text/") || mime == "application/json" {
        FileKind::Text
    } else {
        FileKind::Binary
    }
}

fn content(
    path: String,
    kind: FileKind,
    bytes: Vec<u8>,
    mime: Option<String>,
    size: u64,
) -> FileContent {
    let (text, bytes) = match kind {
        FileKind::Text => (Some(String::from_utf8_lossy(&bytes).into_owned()), None),
        _ => (None, Some(bytes)),
    };
    FileContent {
        path,
        kind,
        text,
        bytes,
        mime_type: mime,
        size,
    }
}

/// A file from `FileBegin` metadata and its chunks.
pub(crate) fn streamed_file(path: &str, metadata: &Value, bytes: Vec<u8>) -> FileContent {
    let mime = metadata["mime"]
        .as_str()
        .unwrap_or("application/octet-stream");
    let encoding = metadata["encoding"].as_str().unwrap_or("binary");
    let size = metadata["size"].as_u64().unwrap_or(bytes.len() as u64);
    content(
        path.to_string(),
        streamed_kind(mime, encoding),
        bytes,
        Some(mime.to_string()),
        size,
    )
}

/// A file from the JSON `file` of `file_explorer_response`, the form without binary frames.
pub(crate) fn json_file(file: &Value) -> anyhow::Result<FileContent> {
    let kind = match file["kind"].as_str() {
        Some("text") => FileKind::Text,
        Some("image") => FileKind::Image,
        _ => FileKind::Binary,
    };
    let text = file["content"].as_str().unwrap_or_default();
    let bytes = match file["encoding"].as_str() {
        Some("base64") => base64::engine::general_purpose::STANDARD.decode(text)?,
        Some("utf-8") => text.as_bytes().to_vec(),
        _ => Vec::new(),
    };
    Ok(content(
        file["path"].as_str().unwrap_or_default().to_string(),
        kind,
        bytes,
        file["mimeType"].as_str().map(str::to_string),
        file["size"].as_u64().unwrap_or_default(),
    ))
}

pub(crate) fn directory_entries(directory: &Value) -> Vec<FileEntry> {
    directory["entries"]
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|entry| {
            Some(FileEntry {
                name: entry["name"].as_str()?.to_string(),
                path: entry["path"].as_str()?.to_string(),
                is_dir: entry["kind"] == "directory",
                size: entry["size"].as_f64().unwrap_or_default() as u64,
            })
        })
        .collect()
}

/// The changed files of a diff payload.
pub(crate) fn changed_files(files: &Value) -> Vec<ChangedFile> {
    files
        .as_array()
        .into_iter()
        .flatten()
        .filter_map(|file| {
            Some(ChangedFile {
                path: file["path"].as_str()?.to_string(),
                status: file["status"].as_str().unwrap_or("ok").to_string(),
                added: file["additions"].as_f64().unwrap_or_default() as u32,
                removed: file["deletions"].as_f64().unwrap_or_default() as u32,
            })
        })
        .collect()
}

/// The unified diff text of one parsed diff file. Hunk header lines carry their `@@` text.
pub(crate) fn unified_diff(file: &Value) -> String {
    let path = file["path"].as_str().unwrap_or_default();
    let old_path = file["oldPath"].as_str().unwrap_or(path);
    let old = if file["isNew"] == true {
        "/dev/null".to_string()
    } else {
        format!("a/{old_path}")
    };
    let new = if file["isDeleted"] == true {
        "/dev/null".to_string()
    } else {
        format!("b/{path}")
    };
    let mut text = format!("--- {old}\n+++ {new}\n");
    for hunk in file["hunks"].as_array().into_iter().flatten() {
        for line in hunk["lines"].as_array().into_iter().flatten() {
            let content = line["content"].as_str().unwrap_or_default();
            let prefix = match line["type"].as_str() {
                Some("header") => "",
                Some("add") => "+",
                Some("remove") => "-",
                _ => " ",
            };
            text.push_str(prefix);
            text.push_str(content);
            text.push('\n');
        }
    }
    text
}

/// The message of a checkout error, which is `{code, message}` or null.
pub(crate) fn checkout_error(payload: &Value) -> Option<(String, String)> {
    let error = &payload["error"];
    if error.is_null() {
        return None;
    }
    let code = error["code"].as_str().unwrap_or("UNKNOWN").to_string();
    let message = error["message"]
        .as_str()
        .or_else(|| error.as_str())
        .unwrap_or("checkout request failed")
        .to_string();
    Some((code, message))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn frame(opcode: u8, id: &str, body: &[u8]) -> Vec<u8> {
        let mut bytes = vec![opcode, id.len() as u8];
        bytes.extend_from_slice(id.as_bytes());
        bytes.extend_from_slice(body);
        bytes
    }

    #[test]
    fn transfer_frames_parse() {
        let metadata = br#"{"mime":"image/png","size":3,"encoding":"binary","modifiedAt":"x"}"#;
        let mut body = (metadata.len() as u16).to_be_bytes().to_vec();
        body.extend_from_slice(metadata);
        let Some(TransferFrame::Begin {
            request_id,
            metadata,
        }) = parse_transfer(&frame(FILE_BEGIN, "r1", &body))
        else {
            panic!("begin parses");
        };
        assert_eq!(request_id, "r1");
        assert_eq!(metadata["mime"], "image/png");
        assert!(matches!(
            parse_transfer(&frame(FILE_CHUNK, "r1", b"abc")),
            Some(TransferFrame::Chunk { bytes, .. }) if bytes == b"abc"
        ));
        assert!(matches!(
            parse_transfer(&frame(FILE_END, "r1", b"")),
            Some(TransferFrame::End { .. })
        ));
        assert!(parse_transfer(&frame(FILE_END, "r1", b"x")).is_none());
        assert!(parse_transfer(&[FILE_CHUNK, 0]).is_none());
    }

    #[test]
    fn base64_file_decodes() {
        let file = json!({"path": "a.png", "kind": "image", "encoding": "base64",
            "content": "iVBORw==", "mimeType": "image/png", "size": 4, "modifiedAt": "x"});
        let content = json_file(&file).unwrap();
        assert_eq!(content.kind, FileKind::Image);
        assert_eq!(
            content.bytes.as_deref(),
            Some(&[0x89, b'P', b'N', b'G'][..])
        );
        assert_eq!(content.text, None);
    }

    #[test]
    fn unified_diff_renders_hunks() {
        let file = json!({"path": "a.rs", "isNew": false, "isDeleted": false,
            "additions": 1, "deletions": 1, "hunks": [{"oldStart": 1, "oldCount": 2,
            "newStart": 1, "newCount": 2, "lines": [
                {"type": "header", "content": "@@ -1,2 +1,2 @@"},
                {"type": "context", "content": "fn a() {}"},
                {"type": "remove", "content": "old"},
                {"type": "add", "content": "new"}]}]});
        assert_eq!(
            unified_diff(&file),
            "--- a/a.rs\n+++ b/a.rs\n@@ -1,2 +1,2 @@\n fn a() {}\n-old\n+new\n"
        );
    }
}
